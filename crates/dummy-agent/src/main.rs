//! Minimal stand-in provider CLI for `dummy-agent` live tests and CI verification.

use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("caretta-dummy-agent: expected at least one argument (try --help)");
        return ExitCode::from(2);
    }

    if matches!(args.as_slice(), [h] if h == "--help" || h == "-h") {
        println!("caretta-dummy-agent - test double for caretta CI");
        return ExitCode::SUCCESS;
    }

    if matches!(args.as_slice(), [v] if v == "--version" || v == "-V") {
        println!("caretta-dummy-agent {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    // Accept any argv the adapter might emit so `live_probe` and future checks stay non-fatal.
    ExitCode::SUCCESS
}
