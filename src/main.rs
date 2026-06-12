//! agentns-claude binary entry point.
//!
//! Resolution order for the wrapped child's session_id (iter-3):
//!
//! 1. Mock file at `/tmp/agentns-mock` (or `$AGENTNS_MOCK_FILE` for tests).
//! 2. `$AGENTNS_SESSION_ID_OVERRIDE` env var.
//! 3. `--no-unshare`: synthesize from `(uid, btime, monotonic_ns)`.
//! 4. `prctl(PR_SET_AGENT_NS)` + `read_agent_session` (wintermute kernel).
//! 5. prctl returned ENOSYS/EINVAL (stock kernel) → synthesize + warn, mode `synth-fallback`.
//! 6. prctl returned EPERM → fatal.
//!
//! `AGENTNS_MODE` is exported to the child alongside `AGENTNS_SESSION_ID`
//! and `AGENTNS_INTENT`.

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
On wintermute kernels uses prctl(PR_SET_AGENT_NS) for a real namespace; falls back to a synthesized id on stock kernels. \n\
Use --no-unshare to skip namespace creation and synthesize a session_id from (uid, boot_time, monotonic_now). \n\
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
    /// Use on stock kernels without PR_SET_AGENT_NS support.
    #[arg(long, default_value_t = false)]
    no_unshare: bool,

    /// Log session_id, intent_tag, mode, and budget settings to stderr before exec.
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
    NeedsCapSysAdmin,
    Io(io::Error),
}

impl std::fmt::Display for LauncherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyIntent => write!(f, "--intent must be a non-empty tag"),
            Self::BadBudget(e) => write!(f, "--budget: {e}"),
            Self::StockKernelNoFallback => write!(
                f,
                "kernel does not support agent namespaces and --no-unshare was not given. \
                 Re-run with --no-unshare to synthesize a session_id on stock kernels."
            ),
            Self::NeedsCapSysAdmin => write!(
                f,
                "prctl(PR_SET_AGENT_NS) requires CAP_SYS_ADMIN in the user namespace. \
                 Re-run as root or with a privileged user namespace, or use --no-unshare."
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

    Ok(exec_child(cli, &session_id, mode))
}

/// Resolve (session_id, mode) according to the iter-3 precedence ladder.
///
/// Precedence:
/// 1. mock (mock file or `AGENTNS_SESSION_ID_OVERRIDE`)
/// 2. `--no-unshare` → synthesize, mode `"no-unshare"`
/// 3. kernel has prctl → `create_agent_ns()` + `set_intent()` + `read_agent_session()`
///    → mode `"prctl"`
/// 4. prctl returned ENOSYS/EINVAL → synthesize + warn, mode `"synth-fallback"`
/// 5. prctl returned EPERM → fatal `LauncherError::NeedsCapSysAdmin`
/// 6. no kernel support and no `--no-unshare` → fatal `LauncherError::StockKernelNoFallback`
fn resolve_session(cli: &Cli) -> Result<(String, &'static str), LauncherError> {
    // 1. Mock
    if let Some(m) = mock::resolve()? {
        mock::announce(&m)?;
        return Ok((m.session_id, m.source));
    }

    // 2. --no-unshare explicit override
    if cli.no_unshare {
        let id = unshare::synthesize_session_id()?;
        let mut err = io::stderr().lock();
        let _ = writeln!(
            err,
            "[agentns-claude] no-unshare: stock kernel, session_id synthesized"
        );
        return Ok((id, "no-unshare"));
    }

    // 3 + 4 + 5. Try prctl(PR_SET_AGENT_NS) if the kernel surface is present.
    if unshare::kernel_has_agent_ns() {
        match unshare::create_agent_ns() {
            Ok(()) => {
                // Intent tag is best-effort; failures are non-fatal.
                unshare::set_intent(&cli.intent);
                let id = unshare::read_agent_session(
                    std::path::Path::new("/proc/self/agent_session"),
                )
                .map_err(LauncherError::Io)?;
                return Ok((id, "prctl"));
            }
            Err(unshare::AgentNsError::KernelLacksPrctl) => {
                // ENOSYS/EINVAL: kernel probe said "yes" but prctl rejected it.
                // Synthesize and warn so the child still gets a useful id.
                let id = unshare::synthesize_session_id()?;
                let mut err = io::stderr().lock();
                let _ = writeln!(
                    err,
                    "[agentns-claude] WARNING: agent_session present but prctl(PR_SET_AGENT_NS) \
                     returned EINVAL/ENOSYS; falling back to synthesized id"
                );
                return Ok((id, "synth-fallback"));
            }
            Err(unshare::AgentNsError::NeedsCapSysAdmin) => {
                return Err(LauncherError::NeedsCapSysAdmin);
            }
            Err(unshare::AgentNsError::CreateSucceededButZero) => {
                // Treat as a prctl failure (unexpected but possible race).
                let id = unshare::synthesize_session_id()?;
                let mut err = io::stderr().lock();
                let _ = writeln!(
                    err,
                    "[agentns-claude] WARNING: prctl(PR_SET_AGENT_NS) succeeded but session is \
                     all-zeros; falling back to synthesized id"
                );
                return Ok((id, "synth-fallback"));
            }
            Err(unshare::AgentNsError::Io(e)) => {
                return Err(LauncherError::Io(e));
            }
        }
    }

    // 6. No kernel support and --no-unshare was not given.
    Err(LauncherError::StockKernelNoFallback)
}

fn exec_child(cli: &Cli, session_id: &str, mode: &str) -> ExitCode {
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
    command.env("AGENTNS_MODE", mode);

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
