//! What reflow2 records about a handshake is what rmcp put on the wire — and a
//! client that opens with `initialize` keeps a session, whatever it asks for.
//!
//! ROOT CAUSE this pins: reflow2 overrides `initialize` to write the handshake
//! record beside the store, and rmcp's negotiation rule is `pub(crate)`, so the
//! override carries a COPY of it (`handshake::negotiate`). The copy was taken
//! from rmcp 3.1.2; by 3.4.0 rmcp only echoed a version that still has an
//! `initialize` handshake, and the copy echoed any supported one. The only test
//! pinned the copy against itself. rmcp re-runs its own rule on whatever the
//! override returns, so the WIRE was right throughout, and the RECORD named a
//! revision the client never received whenever it asked for one past
//! `NO_INITIALIZE`. It surfaced at the rmcp 3.5.0 bump, which moved LATEST to
//! 2026-07-28 and made that case the default fallback.
//!
//! So this compares the record with the wire, over the real binary, for every
//! revision rmcp knows plus one on each side of them: if rmcp's rule moves
//! again, the two disagree here instead of in a field report.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use rmcp::model::ProtocolVersion;

struct Session {
    child: Child,
    lines: mpsc::Receiver<String>,
}

impl Session {
    fn start(dir: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_reflow2-mcp"))
            .args(["--graph-path", dir.join(".reflow2/graph").to_str().unwrap()])
            .current_dir(dir)
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

    fn send(&mut self, msg: serde_json::Value) {
        let stdin = self.child.stdin.as_mut().unwrap();
        writeln!(stdin, "{msg}").unwrap();
        stdin.flush().unwrap();
    }

    /// The reply to request `id`, skipping notifications.
    fn reply(&mut self, id: u64) -> serde_json::Value {
        loop {
            let line = self
                .lines
                .recv_timeout(Duration::from_secs(30))
                .unwrap_or_else(|_| panic!("no reply to request {id} within 30 s"));
            let v: serde_json::Value = serde_json::from_str(&line).unwrap();
            if v.get("id").and_then(|i| i.as_u64()) == Some(id) {
                return v;
            }
        }
    }

    /// `initialize` asking for `requested`; the version the server answered.
    fn initialize(&mut self, requested: &str) -> String {
        self.send(
            serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": requested, "capabilities": {},
            "clientInfo": {"name": "handshake-test", "version": "1.0.0"}}}),
        );
        let init = self.reply(1);
        let answered = init["result"]["protocolVersion"]
            .as_str()
            .unwrap_or_else(|| panic!("initialize({requested}) is answered: {init}"))
            .to_string();
        self.send(serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        answered
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Every revision rmcp knows, plus an unknown one before them and after them.
fn every_request() -> Vec<String> {
    let mut all = vec!["2024-01-01".to_string()];
    all.extend(
        ProtocolVersion::KNOWN_VERSIONS
            .iter()
            .map(|v| v.as_str().to_string()),
    );
    all.push("2027-01-01".to_string());
    all
}

#[test]
fn the_handshake_record_names_the_version_rmcp_sent() {
    for requested in every_request() {
        let dir = tempfile::tempdir().unwrap();
        let answered = Session::start(dir.path()).initialize(&requested);
        let raw = std::fs::read_to_string(dir.path().join(".reflow2/graph.client.json"))
            .unwrap_or_else(|e| panic!("initialize({requested}) wrote no handshake record: {e}"));
        let record: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            record["client_requested"], requested,
            "the record keeps what the client asked for: {record}"
        );
        assert_eq!(
            record["negotiated"], answered,
            "initialize({requested}): the record says `negotiated` was {} but rmcp sent \
             {answered} — handshake::negotiate no longer mirrors rmcp's rule: {record}",
            record["negotiated"]
        );
    }
}

#[test]
fn initialize_is_never_answered_with_a_revision_that_has_no_handshake() {
    for requested in every_request() {
        let dir = tempfile::tempdir().unwrap();
        let answered = Session::start(dir.path()).initialize(&requested);
        let requested_has_one = requested.as_str() < ProtocolVersion::NO_INITIALIZE.as_str();
        let expected = if requested_has_one
            && ProtocolVersion::KNOWN_VERSIONS
                .iter()
                .any(|k| k.as_str() == requested)
        {
            requested.as_str()
        } else {
            ProtocolVersion::LATEST_WITH_INITIALIZE.as_str()
        };
        assert_eq!(
            answered, expected,
            "initialize({requested}): a known revision with a handshake is echoed, and \
             anything else gets the newest revision that has one"
        );
    }
}

/// The property `rmcps_latest_does_not_yet_cross_the_threshold` used to watch
/// LATEST for: a client that asks for 2026-07-28 through `initialize` is
/// settled on a revision with a handshake, so it keeps a session, and a
/// session-scoped call is served instead of refused as sessionless.
#[test]
fn a_client_asking_for_2026_07_28_through_initialize_keeps_a_session() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Session::start(dir.path());
    let answered = s.initialize(ProtocolVersion::NO_INITIALIZE.as_str());
    assert_eq!(answered, ProtocolVersion::LATEST_WITH_INITIALIZE.as_str());
    s.send(
        serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
        "name": "writes_for", "arguments": {"acting_agent": "handshake-test-agent"}}}),
    );
    let reply = s.reply(2);
    let text = reply.to_string();
    assert!(
        reply.get("result").is_some() && reply["result"]["isError"] != true,
        "writes_for is served on a session opened by initialize: {reply}"
    );
    assert!(
        !text.contains("the transport has no sessions"),
        "a session opened by initialize was read as sessionless: {reply}"
    );
}
