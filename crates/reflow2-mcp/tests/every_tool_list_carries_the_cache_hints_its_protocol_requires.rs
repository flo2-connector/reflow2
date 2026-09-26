//! Every surface's tool list carries the cache hints its protocol version
//! requires — and none for an older client.
//!
//! ROOT CAUSE this pins
//! (fact:root-cause-the-latent-tool-list-omits-the-cache-fields-the-2026-07-28-spec-requires-2026-09-26):
//! Claude Code 2.1.283 opens with `server/discover` on the stateless 2026-07-28
//! protocol, whose `tools/list` result REQUIRES `ttlMs` (a number) and
//! `cacheScope` ("public" | "private"). The latent surface — the machine-wide
//! entry in a folder with no design, which is where /genesis starts one —
//! built its listing with `ListToolsResult::with_all_items`, which leaves both
//! unset, so Claude Code refused it: "Invalid result for tools/list: ttlMs …
//! cacheScope". The full surface had the rule copied into its own override by
//! hand; the latent override never had it.
//!
//! THE CLASS: reflow2 overrides rmcp's generated `list_tools` to add behaviour,
//! and rmcp writes the version rule inline in its macro, so each override has to
//! reproduce it. This pins the OUTPUT of every surface, over the real binary,
//! under both protocol versions — so a new override, a new required field or an
//! rmcp change that moves the rule fails here rather than in a user's client.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

/// What Claude Code 2.1.283 sends as `_meta` on the 2026-07-28 protocol,
/// captured from the client itself on 2026-09-26.
fn meta_2026() -> serde_json::Value {
    serde_json::json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "claude-code", "version": "2.1.283" },
        "io.modelcontextprotocol/clientCapabilities": { "roots": { "listChanged": true }, "elicitation": { "form": {}, "url": {} } }
    })
}

struct Session {
    child: Child,
    lines: mpsc::Receiver<String>,
}

impl Session {
    fn start(cwd: &Path, args: &[&str]) -> Self {
        let mut child = Command::new(bin())
            .args(args)
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

    /// The 2026-07-28 opening: server/discover, then a stateless tools/list.
    fn tools_2026(&mut self) -> serde_json::Value {
        self.send(serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "server/discover", "params": {"_meta": meta_2026()}}));
        let discover = self.reply(1);
        assert!(
            discover.get("result").is_some(),
            "server/discover is answered: {discover}"
        );
        self.send(serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {"_meta": meta_2026()}}));
        self.reply(2)
    }

    /// The 2025-11-25 opening: initialize, initialized, tools/list.
    fn tools_2025(&mut self) -> serde_json::Value {
        self.send(serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "claude-code", "version": "2.1.283"}}}));
        let init = self.reply(1);
        assert!(
            init.get("result").is_some(),
            "initialize is answered: {init}"
        );
        self.send(serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        self.send(
            serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
        );
        self.reply(2)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The three surfaces one binary can present, set up in their own folders.
enum Surface {
    /// A folder with a design.
    Full,
    /// The machine-wide entry's exact arguments in a folder with no design.
    Latent,
    /// A design that cannot be opened: a FILE where the graph directory should be.
    Degraded,
}

fn open(surface: Surface, dir: &Path) -> Session {
    match surface {
        Surface::Full => Session::start(
            dir,
            &["--graph-path", dir.join(".reflow2/graph").to_str().unwrap()],
        ),
        Surface::Latent => Session::start(
            dir,
            &[
                "--graph-path",
                ".reflow2/graph",
                "--shared",
                "--only-if-present",
            ],
        ),
        Surface::Degraded => {
            std::fs::create_dir_all(dir.join(".reflow2")).unwrap();
            std::fs::write(dir.join(".reflow2/graph"), "not a store").unwrap();
            Session::start(
                dir,
                &["--graph-path", dir.join(".reflow2/graph").to_str().unwrap()],
            )
        }
    }
}

fn assert_carries_the_hints(label: &str, reply: &serde_json::Value) {
    let result = reply
        .get("result")
        .unwrap_or_else(|| panic!("{label}: tools/list failed: {reply}"));
    assert!(
        result["tools"].as_array().is_some_and(|t| !t.is_empty()),
        "{label}: no tools: {reply}"
    );
    assert!(
        result["ttlMs"].is_u64(),
        "{label}: ttlMs must be a number under 2026-07-28, got {}",
        result["ttlMs"]
    );
    let scope = result["cacheScope"].as_str();
    assert!(
        matches!(scope, Some("public" | "private")),
        "{label}: cacheScope must be public or private under 2026-07-28, got {}",
        result["cacheScope"]
    );
}

fn assert_carries_no_hints(label: &str, reply: &serde_json::Value) {
    let result = reply
        .get("result")
        .unwrap_or_else(|| panic!("{label}: tools/list failed: {reply}"));
    assert!(
        result["tools"].as_array().is_some_and(|t| !t.is_empty()),
        "{label}: no tools: {reply}"
    );
    assert!(
        result.get("ttlMs").is_none(),
        "{label}: an older client gets no ttlMs: {result}"
    );
    assert!(
        result.get("cacheScope").is_none(),
        "{label}: an older client gets no cacheScope: {result}"
    );
}

#[test]
fn the_full_surface_carries_the_hints_on_2026_07_28_and_none_on_2025_11_25() {
    let dir = tempfile::tempdir().unwrap();
    assert_carries_the_hints("full", &open(Surface::Full, dir.path()).tools_2026());
    let dir = tempfile::tempdir().unwrap();
    assert_carries_no_hints("full", &open(Surface::Full, dir.path()).tools_2025());
}

#[test]
fn the_latent_surface_carries_the_hints_on_2026_07_28_and_none_on_2025_11_25() {
    let dir = tempfile::tempdir().unwrap();
    assert_carries_the_hints("latent", &open(Surface::Latent, dir.path()).tools_2026());
    assert!(
        !dir.path().join(".reflow2").exists(),
        "listing tools in a folder with no design creates nothing"
    );
    let dir = tempfile::tempdir().unwrap();
    assert_carries_no_hints("latent", &open(Surface::Latent, dir.path()).tools_2025());
}

#[test]
fn the_degraded_surface_carries_the_hints_on_2026_07_28_and_none_on_2025_11_25() {
    let dir = tempfile::tempdir().unwrap();
    assert_carries_the_hints(
        "degraded",
        &open(Surface::Degraded, dir.path()).tools_2026(),
    );
    let dir = tempfile::tempdir().unwrap();
    assert_carries_no_hints(
        "degraded",
        &open(Surface::Degraded, dir.path()).tools_2025(),
    );
}
