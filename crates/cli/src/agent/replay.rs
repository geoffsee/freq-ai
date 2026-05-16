//! Flight-recorder and deterministic replay for workflow runs.
//!
//! Each `work_on_issue` call writes a per-run NDJSON log at
//! `.caretta/replay/<run-id>.replay.ndjson` that captures the full agent
//! prompt and the captured response text. The `caretta replay <log>` subcommand
//! reads that log and replays the run without live agent invocation, flagging
//! any divergence between recorded input and response provenance.

use crate::agent::tracker::Provenance;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

// ── NDJSON record types ───────────────────────────────────────────────────────

/// One line in a `.replay.ndjson` log file.
///
/// The `type` field (injected by serde's `tag`) distinguishes input records
/// from response records so parsers can ignore unknown future types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReplayRecord {
    Input(InputRecord),
    Response(ResponseRecord),
}

/// Workflow input: the full agent prompt and its provenance.
///
/// Together with the provenance block this record is sufficient to
/// reconstruct the complete input set for any past run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputRecord {
    pub run_id: String,
    pub timestamp: String,
    pub provenance: Provenance,
    pub prompt: String,
}

/// Agent response: captured output text, success flag, and matching provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseRecord {
    pub run_id: String,
    pub timestamp: String,
    pub provenance: Provenance,
    pub text: String,
    pub success: bool,
}

// ── Flight recorder ───────────────────────────────────────────────────────────

/// Writes NDJSON records for a single workflow run.
///
/// Drop or let go out of scope when the run finishes — the writer is flushed
/// on every record so partial logs are still readable on crash.
pub struct FlightRecorder {
    run_id: String,
    provenance: Provenance,
    writer: BufWriter<File>,
}

impl FlightRecorder {
    /// Open (or append to) a log file, creating parent directories as needed.
    pub fn open(
        run_id: String,
        log_path: PathBuf,
        provenance: Provenance,
    ) -> std::io::Result<Self> {
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;
        Ok(Self {
            run_id,
            provenance,
            writer: BufWriter::new(file),
        })
    }

    /// Append an input record (the full agent prompt).
    pub fn record_input(&mut self, prompt: &str) {
        self.write_record(&ReplayRecord::Input(InputRecord {
            run_id: self.run_id.clone(),
            timestamp: replay_timestamp(),
            provenance: self.provenance.clone(),
            prompt: prompt.to_string(),
        }));
    }

    /// Append a response record (captured agent output text + success flag).
    pub fn record_response(&mut self, text: &str, success: bool) {
        self.write_record(&ReplayRecord::Response(ResponseRecord {
            run_id: self.run_id.clone(),
            timestamp: replay_timestamp(),
            provenance: self.provenance.clone(),
            text: text.to_string(),
            success,
        }));
    }

    fn write_record(&mut self, record: &ReplayRecord) {
        if let Ok(line) = serde_json::to_string(record) {
            let _ = writeln!(self.writer, "{line}");
            let _ = self.writer.flush();
        }
    }
}

// ── Path / ID helpers ─────────────────────────────────────────────────────────

/// Replay log path: `<root>/.caretta/replay/<run-id>.replay.ndjson`.
pub fn replay_log_path(root: &str, run_id: &str) -> PathBuf {
    PathBuf::from(root)
        .join(".caretta")
        .join("replay")
        .join(format!("{run_id}.replay.ndjson"))
}

/// Generate a run ID for an issue workflow run.
///
/// Format: `issue-<N>-<unix-seconds>` — unique per issue per second.
pub fn issue_run_id(issue_num: u32) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("issue-{issue_num}-{ts}")
}

// ── Replay command ────────────────────────────────────────────────────────────

/// Replay a past run from its NDJSON log without live agent invocation.
///
/// Reads all records from `log_path`, prints the reconstructed input prompt
/// and the recorded agent response, then checks for provenance divergence.
/// Exits with code 2 when the input and response provenance hashes disagree,
/// and with code 1 on fatal I/O or parse errors.
pub fn run_replay(log_path: &str) {
    let file = match File::open(log_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("replay: cannot open {log_path}: {e}");
            std::process::exit(1);
        }
    };

    let mut inputs: Vec<InputRecord> = Vec::new();
    let mut responses: Vec<ResponseRecord> = Vec::new();
    let mut parse_errors = 0usize;

    for (idx, line) in BufReader::new(file).lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("replay: read error at line {}: {e}", idx + 1);
                continue;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<ReplayRecord>(&line) {
            Ok(ReplayRecord::Input(r)) => inputs.push(r),
            Ok(ReplayRecord::Response(r)) => responses.push(r),
            Err(e) => {
                eprintln!("replay: parse error at line {}: {e}", idx + 1);
                parse_errors += 1;
            }
        }
    }

    if inputs.is_empty() && responses.is_empty() {
        eprintln!("replay: {log_path} contains no recognizable records");
        std::process::exit(1);
    }

    // ── Header ────────────────────────────────────────────────────────────────
    let first_prov = inputs
        .first()
        .map(|r| &r.provenance)
        .or_else(|| responses.first().map(|r| &r.provenance));
    let first_run_id = inputs
        .first()
        .map(|r| r.run_id.as_str())
        .or_else(|| responses.first().map(|r| r.run_id.as_str()))
        .unwrap_or("unknown");

    if let Some(prov) = first_prov {
        println!("=== Replay: run_id={first_run_id} ===");
        println!("Agent:        {}", prov.agent);
        println!("Model:        {}", prov.model_id);
        println!("Timestamp:    {}", prov.run_timestamp);
        println!("Input digest: {}", prov.input_digest);
        println!();
    }

    // ── Input prompt(s) ───────────────────────────────────────────────────────
    for (i, input) in inputs.iter().enumerate() {
        println!(
            "--- input[{i}] (prompt_version={}) ---",
            input.provenance.prompt_version
        );
        println!("{}", input.prompt);
        println!();
    }

    // ── Recorded response(s) ─────────────────────────────────────────────────
    for (i, resp) in responses.iter().enumerate() {
        let status = if resp.success { "success" } else { "failure" };
        println!("--- response[{i}] ({status}) ---");
        if resp.text.is_empty() {
            println!("<no text captured>");
        } else {
            println!("{}", resp.text);
        }
        println!();
    }

    // ── Divergence check ──────────────────────────────────────────────────────
    // Compare provenance hashes between the first input and first response
    // record. Mismatch means the log was assembled from different runs or the
    // inputs changed between the time the input was recorded and the response.
    if let (Some(inp), Some(resp)) = (inputs.first(), responses.first()) {
        let digest_ok = inp.provenance.input_digest == resp.provenance.input_digest;
        let version_ok = inp.provenance.prompt_version == resp.provenance.prompt_version;
        if !digest_ok || !version_ok {
            eprintln!("DIVERGENCE DETECTED: input/response provenance mismatch");
            if !digest_ok {
                eprintln!(
                    "  input_digest:   {} (input) vs {} (response)",
                    inp.provenance.input_digest, resp.provenance.input_digest
                );
            }
            if !version_ok {
                eprintln!(
                    "  prompt_version: {} (input) vs {} (response)",
                    inp.provenance.prompt_version, resp.provenance.prompt_version
                );
            }
            std::process::exit(2);
        }
    }

    // ── Notes ─────────────────────────────────────────────────────────────────
    if parse_errors > 0 {
        eprintln!("WARNING: {parse_errors} line(s) failed to parse");
    }
    if inputs.is_empty() {
        println!("NOTE: no input records found — response-only log");
    } else if responses.is_empty() {
        println!("NOTE: no response records found — run may not have completed");
    }
}

// ── Timestamp helper ──────────────────────────────────────────────────────────

fn replay_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let time = secs % 86400;
    let days = secs / 86400;
    let h = time / 3600;
    let m = (time % 3600) / 60;
    let s = time % 60;
    let (y, mo, d) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

fn civil_from_days(days: u64) -> (i64, u64, u64) {
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    (y, mo, d)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_prov() -> Provenance {
        Provenance {
            schema_version: "1".to_string(),
            agent: "claude".to_string(),
            model_id: "claude-3-5-sonnet".to_string(),
            prompt_version: "abc123".to_string(),
            input_digest: "def456".to_string(),
            run_timestamp: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn flight_recorder_writes_valid_ndjson() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.replay.ndjson");
        let mut rec =
            FlightRecorder::open("issue-42-12345".to_string(), log_path.clone(), test_prov())
                .unwrap();
        rec.record_input("the prompt");
        rec.record_response("the response", true);
        drop(rec);

        let contents = fs::read_to_string(&log_path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 2, "expected 2 NDJSON lines, got:\n{contents}");

        let r0: ReplayRecord = serde_json::from_str(lines[0]).unwrap();
        let r1: ReplayRecord = serde_json::from_str(lines[1]).unwrap();
        assert!(matches!(&r0, ReplayRecord::Input(i) if i.prompt == "the prompt"));
        assert!(matches!(&r1, ReplayRecord::Response(r) if r.text == "the response" && r.success));
    }

    #[test]
    fn flight_recorder_failure_response() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("fail.replay.ndjson");
        let mut rec =
            FlightRecorder::open("issue-1-99".to_string(), log_path.clone(), test_prov()).unwrap();
        rec.record_input("prompt");
        rec.record_response("error output", false);
        drop(rec);

        let contents = fs::read_to_string(&log_path).unwrap();
        let last = contents.lines().last().unwrap();
        let r: ReplayRecord = serde_json::from_str(last).unwrap();
        assert!(matches!(&r, ReplayRecord::Response(r) if !r.success));
    }

    #[test]
    fn replay_log_path_structure() {
        let path = replay_log_path("/repo", "issue-10-1700000000");
        assert_eq!(
            path,
            PathBuf::from("/repo/.caretta/replay/issue-10-1700000000.replay.ndjson")
        );
    }

    #[test]
    fn issue_run_id_has_expected_prefix() {
        let id = issue_run_id(7);
        assert!(id.starts_with("issue-7-"), "unexpected run_id: {id}");
        // Second part must be a numeric unix timestamp
        let ts_part = id.strip_prefix("issue-7-").unwrap();
        assert!(
            ts_part.chars().all(|c| c.is_ascii_digit()),
            "timestamp part is not numeric: {ts_part}"
        );
    }

    #[test]
    fn recorder_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let nested = dir
            .path()
            .join(".caretta")
            .join("replay")
            .join("run.replay.ndjson");
        let mut rec = FlightRecorder::open("r1".to_string(), nested.clone(), test_prov()).unwrap();
        rec.record_input("p");
        drop(rec);
        assert!(nested.exists(), "log file should have been created");
    }

    #[test]
    fn input_record_roundtrips_through_json() {
        let rec = ReplayRecord::Input(InputRecord {
            run_id: "issue-5-1000".to_string(),
            timestamp: "2026-05-16T12:00:00Z".to_string(),
            provenance: test_prov(),
            prompt: "hello world".to_string(),
        });
        let json = serde_json::to_string(&rec).unwrap();
        let back: ReplayRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(rec, back);
        // Confirm the `type` field is present and set to "input"
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "input");
    }

    #[test]
    fn response_record_roundtrips_through_json() {
        let rec = ReplayRecord::Response(ResponseRecord {
            run_id: "issue-5-1000".to_string(),
            timestamp: "2026-05-16T12:00:00Z".to_string(),
            provenance: test_prov(),
            text: "agent said this".to_string(),
            success: true,
        });
        let json = serde_json::to_string(&rec).unwrap();
        let back: ReplayRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(rec, back);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "response");
    }
}
