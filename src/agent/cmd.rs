use crate::agent::types::{AgentEvent, EVENT_SENDER};
use std::path::Path;
use std::process::{self, Command, Stdio};
use tracing::info;

#[cfg(not(target_arch = "wasm32"))]
pub use toak_rs::count_tokens;

#[cfg(target_arch = "wasm32")]
pub fn count_tokens(s: &str) -> usize {
    s.len() / 4
}

/// Log the elapsed time for a labelled operation.
#[macro_export]
macro_rules! timed {
    ($label:expr, $body:expr) => {{
        let _t0 = Instant::now();
        let _result = $body;
        $crate::agent::cmd::log(&format!(
            "[timing] {} completed in {:.2?}",
            $label,
            _t0.elapsed()
        ));
        _result
    }};
}

pub fn die(msg: &str) -> ! {
    eprintln!("ERROR: {msg}");
    process::exit(1);
}

pub fn log(msg: &str) {
    info!("{msg}");
    if let Some(tx) = EVENT_SENDER.get() {
        let _ = tx.send(AgentEvent::Log(msg.to_string()));
    }
}

/// Run a command, return trimmed stdout or None on failure.
pub fn cmd_stdout(program: &str, args: &[&str]) -> Option<String> {
    Command::new(program)
        .args(args)
        .stderr(Stdio::inherit())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// Run a command, return trimmed stdout or die.
pub fn cmd_stdout_or_die(program: &str, args: &[&str], context: &str) -> String {
    cmd_stdout(program, args).unwrap_or_else(|| die(context))
}

/// Run a command, inheriting stdio. Returns success bool.
pub fn cmd_run(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Run a command in a specific directory, inheriting stdio. Returns success bool.
pub fn cmd_run_in(program: &str, args: &[&str], dir: &Path) -> bool {
    Command::new(program)
        .args(args)
        .current_dir(dir)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Run a command, capture combined stdout+stderr. Returns (success, output).
pub fn cmd_capture(program: &str, args: &[&str]) -> (bool, String) {
    match Command::new(program)
        .args(args)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .output()
    {
        Ok(o) => {
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
            (o.status.success(), combined)
        }
        Err(e) => (false, e.to_string()),
    }
}

pub fn has_command(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
