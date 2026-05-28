//! agentns-claude binary entry point.
//!
//! Resolution order for the wrapped child's session_id:
//!
//! 1. Mock file at `/tmp/agentns-mock` (or `$AGENTNS_MOCK_FILE` for tests).
//! 2. `$AGENTNS_SESSION_ID_OVERRIDE` env var.
//! 3. Real `unshare(CLONE_NEWAGENT)` under linux-wintermute (iter-3+).
//! 4. `--no-unshare` fallback: synthesize from `(uid, btime, monotonic_ns)`.
//!
//! Without any of the above on a stock kernel, the launcher refuses to run
//! and prints a clear remediation message (AC9).

use std::io::{self, Write};
use std::os::unix::process::CommandExt;
use std::process::{Command, ExitCode};

use clap::Parser;

use agentns_claude::{budget, mock, unshare};

/// Wrap a command in a wintermute agent namespace.
#[derive(Debug, Parser)]
#[command(
    name = "agentns-claude",
    version,
    about = "Wrap a command in a wintermute agent namespace.",
    long_about = "Wrap a command in a wintermute agent namespace so /proc/$PID/agent_session reads stably from birth. \n\
On stock kernels without CLONE_NEWAGENT, use --no-unshare to synthesize a session_id from (uid, boot_time, monotonic_now). \n\
Mock-mode: set AGENTNS_SESSION_ID_OVERRIDE or write a session_id into /tmp/agentns-mock; file wins over env."
)]
struct Cli {
    /// Intent tag written to prctl(PR_SET_AGENT_INTENT_TAG).
    /// Conventions: /build, /dream, /self-review, interactive, headless, headless:<service>.
    #[arg(long)]
    intent: String,

    /// Optional budget spec, e.g. wall=3600s,syscalls=1e7,write_bytes=10G,fork=1000.
    /// SIGTERM on soft limit, SIGKILL on hard. Applied via prctl on linux-wintermute.
    #[arg(long)]
    budget: Option<String>,

    /// Skip the unshare; synthesize a session_id from (uid, btime, monotonic_ns).
    /// Use on stock kernels without CLONE_NEWAGENT support.
    #[arg(long, default_value_t = false)]
    no_unshare: bool,

    /// Log session_id, intent_tag, and budget settings to stderr before exec.
    #[arg(long, default_value_t = false)]
    verbose: bool,

    /// The wrapped command and its argv. Required.
    #[arg(last = true, required = true)]
    cmd: Vec<String>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(code) => code,
        Err(e) => {
            let _ = writeln!(io::stderr().lock(), "agentns-claude: {e}");
            ExitCode::from(2)
        }
    }
}

#[derive(Debug)]
enum LauncherError {
    EmptyIntent,
    BadBudget(budget::ParseError),
    StockKernelNoFallback,
    Io(io::Error),
}

impl std::fmt::Display for LauncherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyIntent => write!(f, "--intent must be a non-empty tag"),
            Self::BadBudget(e) => write!(f, "--budget: {e}"),
            Self::StockKernelNoFallback => write!(
                f,
                "kernel does not support agent namespaces (no CLONE_NEWAGENT). \
                 Re-run with --no-unshare to skip namespace creation on stock kernels."
            ),
            Self::Io(e) => write!(f, "io: {e}"),
        }
    }
}

impl From<io::Error> for LauncherError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

fn run(cli: &Cli) -> Result<ExitCode, LauncherError> {
    if cli.intent.trim().is_empty() {
        return Err(LauncherError::EmptyIntent);
    }
    if let Some(spec) = cli.budget.as_deref() {
        budget::parse(spec).map_err(LauncherError::BadBudget)?;
    }

    let (session_id, mode) = resolve_session(cli)?;

    if cli.verbose {
        let mut err = io::stderr().lock();
        let _ = writeln!(
            err,
            "[agentns-claude] mode={mode} session_id={session_id} intent={}",
            cli.intent
        );
        if let Some(b) = cli.budget.as_deref() {
            let _ = writeln!(err, "[agentns-claude] budget={b}");
        }
    }

    Ok(exec_child(cli, &session_id))
}

fn resolve_session(cli: &Cli) -> Result<(String, &'static str), LauncherError> {
    if let Some(m) = mock::resolve()? {
        mock::announce(&m)?;
        return Ok((m.session_id, m.source));
    }
    if unshare::kernel_has_agent_ns() {
        // iter-3 wires the real CLONE_NEWAGENT + prctl plumbing here. For
        // iter-2 we still exec the child so AC4 (exec semantics) is testable
        // on a wintermute boot; the session_id is synthesized and a warning
        // surfaces so nothing silently assumes a real namespace.
        let id = unshare::synthesize_session_id()?;
        let mut err = io::stderr().lock();
        let _ = writeln!(
            err,
            "[agentns-claude] PENDING: agent_session detected but iter-2 has not wired CLONE_NEWAGENT; session_id synthesized"
        );
        return Ok((id, "pending-unshare"));
    }
    if cli.no_unshare {
        let id = unshare::synthesize_session_id()?;
        let mut err = io::stderr().lock();
        let _ = writeln!(
            err,
            "[agentns-claude] no-unshare fallback: stock kernel, session_id synthesized"
        );
        return Ok((id, "no-unshare"));
    }
    Err(LauncherError::StockKernelNoFallback)
}

fn exec_child(cli: &Cli, session_id: &str) -> ExitCode {
    // `cmd` is `required = true` + `last = true`, so it has at least one
    // element by clap's argv validation; defensively guard anyway because
    // the lint config forbids unwrap.
    let Some((program, args)) = cli.cmd.split_first() else {
        let _ = writeln!(io::stderr().lock(), "agentns-claude: empty command");
        return ExitCode::from(2);
    };
    let mut command = Command::new(program);
    command.args(args);
    command.env("AGENTNS_SESSION_ID", session_id);
    command.env("AGENTNS_INTENT", &cli.intent);

    // exec replaces the current process on success; only returns on error.
    let err = command.exec();
    let _ = writeln!(
        io::stderr().lock(),
        "agentns-claude: exec {program}: {err}"
    );
    // Map common exec errors to conventional exit codes (127 = command not
    // found, 126 = found-but-not-executable) so callers can distinguish.
    match err.kind() {
        io::ErrorKind::NotFound => ExitCode::from(127),
        io::ErrorKind::PermissionDenied => ExitCode::from(126),
        _ => ExitCode::from(2),
    }
}
