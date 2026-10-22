//! A session on stdio answers a message of any length, as it did on rmcp 3.5.0.
//!
//! rmcp 3.6.0 (#1049) bounds an incoming stdio line at 16 MiB and DISCARDS a
//! longer one with only a log line, so the client waits for a reply that never
//! comes. Measured 2026-10-22 on the built binary: a 17 MiB `tools/call` was
//! answered on rmcp 3.5.0 and dropped on 3.6.0. Anthony, 2026-10-22: "lift
//! rmcp's new 16 MiB limit on ordinary MCP connections, so they behave as
//! before" — so every stdio serve path builds its transport with no line bound
//! (main.rs `stdio_unbounded`, and the shared proxy's in-process degraded
//! surface). Whether a connection should bound a message and ANSWER when it is
//! exceeded is the open
//! dec:idea-an-mcp-connection-bounds-a-message-and-answers-with-an-error-when-it-is-exceeded.
//!
//! Each case sends one request longer than 16 MiB and then a ping, and wants a
//! reply to BOTH: the ping alone answering is exactly what the bound looks like.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

/// Past rmcp 3.6.0's 16 MiB default, with room to spare.
const LONG: usize = 17 * 1024 * 1024;

struct Session {
    child: Child,
    lines: mpsc::Receiver<String>,
}

impl Session {
    fn start(cwd: &Path, graph: &Path) -> Self {
        let mut child = Command::new(bin())
            .args(["--graph-path", graph.to_str().unwrap()])
            .current_dir(cwd)
            .env("RUST_LOG", "warn")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the binary runs");
        let stdout = child.stdout.take().unwrap();
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    return;
                }
            }
        });
        Self { child, lines }
    }

    fn send(&mut self, msg: &serde_json::Value) {
        let stdin = self.child.stdin.as_mut().unwrap();
        writeln!(stdin, "{msg}").unwrap();
        stdin.flush().unwrap();
    }

    /// Every reply that arrives within `within`, by id, stopping once all of
    /// `want` are in.
    fn replies(&mut self, want: &[u64], within: Duration) -> Vec<u64> {
        let deadline = Instant::now() + within;
        let mut got = Vec::new();
        while !want.iter().all(|w| got.contains(w)) {
            let left = deadline.saturating_duration_since(Instant::now());
            let Ok(line) = self.lines.recv_timeout(left) else {
                break;
            };
            let v: serde_json::Value = serde_json::from_str(&line).unwrap_or_default();
            if let Some(id) = v.get("id").and_then(|i| i.as_u64()) {
                got.push(id);
            }
        }
        got
    }

    fn open(&mut self) {
        self.send(
            &serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": {"name": "line-length-test", "version": "0"}}}),
        );
        assert_eq!(
            self.replies(&[1], Duration::from_secs(60)),
            vec![1],
            "initialize is answered"
        );
        self.send(&serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
    }

    /// One request longer than 16 MiB, then a ping: both must be answered.
    fn a_long_request_is_answered(&mut self, label: &str) {
        let long_id = format!("req:{}", "x".repeat(LONG));
        self.send(
            &serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": {"name": "get_node", "arguments": {"id": long_id}}}),
        );
        self.send(&serde_json::json!({"jsonrpc": "2.0", "id": 3, "method": "ping"}));
        let got = self.replies(&[2, 3], Duration::from_secs(120));
        assert!(
            got.contains(&3),
            "{label}: the session stopped answering at all: {got:?}"
        );
        assert!(
            got.contains(&2),
            "{label}: a request longer than 16 MiB got no reply — the line bound dropped it: {got:?}"
        );
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The ordinary connection: a design in a folder, served on stdio.
#[cfg(feature = "rocksdb")]
#[test]
fn a_session_on_a_design_answers_a_request_longer_than_16_mib() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut s = Session::start(dir.path(), &dir.path().join(".reflow2/graph"));
    s.open();
    s.a_long_request_is_answered("a design on stdio");
}

/// The degraded surface, served on stdio when the design cannot be opened —
/// its own serve path, so it is held to the same rule.
#[test]
fn a_degraded_session_answers_a_request_longer_than_16_mib() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join(".reflow2")).unwrap();
    std::fs::write(dir.path().join(".reflow2/graph"), "not a store").unwrap();
    let mut s = Session::start(dir.path(), &dir.path().join(".reflow2/graph"));
    s.open();
    s.a_long_request_is_answered("a degraded design on stdio");
}
