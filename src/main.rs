//! agentns-claude binary entry point.
//!
//! Stage-2 scaffold. Parses argv via clap and dispatches to the (stubbed)
//! library functions. The iterate-and-prove loop replaces each stub with a
//! real implementation across subsequent iterations.

use std::process::ExitCode;

use clap::Parser;

/// Wrap a command in a wintermute agent namespace.
#[derive(Debug, Parser)]
#[command(name = "agentns-claude", version, about, long_about = None)]
struct Cli {
    /// Intent tag written to prctl(PR_SET_AGENT_INTENT_TAG).
    /// Conventions: /build, /dream, /self-review, interactive, headless, headless:<service>.
    #[arg(long)]
    intent: String,

    /// Optional budget spec, e.g. wall=3600s,syscalls=1e7,write_bytes=10G,fork=1000.
    #[arg(long)]
    budget: Option<String>,

    /// Skip the unshare; synthesize a session_id from (uid, boot_time_ns, monotonic_now_ns).
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
    let _cli = Cli::parse();
    // iter-1 scaffold: argv parsing wired; dispatch lands in iter-2.
    ExitCode::from(2)
}
