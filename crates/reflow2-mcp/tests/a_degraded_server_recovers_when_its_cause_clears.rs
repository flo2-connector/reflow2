//! A server that cannot open its design because ANOTHER PROCESS HOLDS IT serves
//! the design, in place, the moment that process lets go — and an HTTP server's
//! readiness says whether the design is served, whatever Host the probe uses.
//!
//! ⭐ WHY THIS IS END TO END. GitHub issue #616 measured the failure on a real
//! cluster shape: a rolling update briefly runs two pods on one store, the second
//! cannot take the store's lock, and it served the one-tool degraded surface
//! FOREVER — still degraded 11 s after the first server stopped — while the
//! image's TCP health check reported it healthy
//! (`fact:root-cause-a-server-degraded-by-a-held-lock-never-retries-and-looks-alive-2026-09-28`).
//! The same class was fixed for the latent surface on 2026-09-14 (a stand-in
//! surface never re-checks after its cause clears). Only the real binary shows
//! what a client and an orchestrator actually meet, so that is what runs here:
//!   · over HTTP, the SAME process and port serve the design after the holder
//!     stops, and /readyz goes 503 → 200;
//!   · over stdio, the connected client is told `notifications/tools/list_changed`
//!     and then sees the full tool list;
//!   · the `--shared` client that timed out waiting for a server (the lock held
//!     by a non-shared process) re-elects and serves the design in place;
//!   · /readyz and /healthz answer whatever the Host header, and nothing else
//!     does — the Host gate still guards the design;
//!   · a cause that CANNOT clear by itself (a stamp that will not read) is not
//!     polled, and readiness stays 503.
//!
//! `req:never-silently-absent`, `req:reflow2-consumable-as-an-image`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn tmp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "reflow2-recovers-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn bin() -> PathBuf {
    let mut p = std::env::current_exe().expect("test binary has a path");
    p.pop();
    if p.ends_with("deps") {
        p.pop();
    }
    p.join("reflow2-mcp")
}

/// An HTTP server under test: its port, and every stderr line so far.
struct Server {
    child: Child,
    port: u16,
    stderr: Arc<Mutex<Vec<String>>>,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Server {
    fn start(args: &[&str]) -> Server {
        let mut child = Command::new(bin())
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn reflow2-mcp");
        let stderr = child.stderr.take().expect("stderr piped");
        let lines = Arc::new(Mutex::new(Vec::<String>::new()));
        let sink = Arc::clone(&lines);
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                sink.lock().unwrap().push(line);
            }
        });
        let deadline = Instant::now() + Duration::from_secs(90);
        let port = loop {
            if let Some(p) = lines.lock().unwrap().iter().find_map(|l| {
                let i = l.find("http://127.0.0.1:")?;
                l[i + "http://127.0.0.1:".len()..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<u16>()
                    .ok()
            }) {
                break p;
            }
            assert!(
                Instant::now() < deadline,
                "the server printed no address within 90s: {:?}",
                lines.lock().unwrap()
            );
            std::thread::sleep(Duration::from_millis(20));
        };
        Server {
            child,
            port,
            stderr: lines,
        }
    }

    fn still_running(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }

    fn log(&self) -> String {
        self.stderr.lock().unwrap().join("\n")
    }
}

/// Stop a server and wait until the process is gone, so its lock is released.
fn stop(mut s: Server) {
    let _ = s.child.kill();
    let _ = s.child.wait();
}

/// One raw HTTP/1.1 request; returns (status, body).
fn http(port: u16, method: &str, path: &str, host: &str, body: &str) -> (u16, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    s.set_read_timeout(Some(Duration::from_secs(60))).unwrap();
    write!(
        s,
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\n\
         Accept: application/json, text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    let mut raw = String::new();
    let _ = s.read_to_string(&mut raw);
    let status = raw
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let body = raw.split_once("\r\n\r\n").map(|(_, b)| b).unwrap_or("");
    (status, body.to_string())
}

fn post(port: u16, body: &str, session: Option<&str>) -> (u16, Option<String>, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    s.set_read_timeout(Some(Duration::from_secs(120))).ok();
    let mut req = format!(
        "POST / HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n\
         Accept: application/json, text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(sid) = session {
        req.push_str(&format!("mcp-session-id: {sid}\r\n"));
    }
    req.push_str("\r\n");
    req.push_str(body);
    s.write_all(req.as_bytes()).expect("write request");
    let mut raw = String::new();
    let _ = s.read_to_string(&mut raw);
    let status = raw
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let sid = raw.lines().find_map(|l| {
        let (k, v) = l.split_once(':')?;
        (k.trim().eq_ignore_ascii_case("mcp-session-id")).then(|| v.trim().to_string())
    });
    (status, sid, raw)
}

fn open_session(port: u16) -> String {
    let (status, sid, body) = post(
        port,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}"#,
        None,
    );
    assert_eq!(status, 200, "initialize failed: {body}");
    let sid = sid.expect("a session id");
    let _ = post(
        port,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        Some(&sid),
    );
    sid
}

/// The JSON-RPC message carried in a response body, JSON or SSE.
fn message_in(raw: &str) -> serde_json::Value {
    raw.lines()
        .filter_map(|l| l.strip_prefix("data:").map(str::trim).or(Some(l.trim())))
        .filter(|l| l.starts_with('{'))
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .find(|v| v.get("result").is_some() || v.get("error").is_some())
        .unwrap_or_else(|| panic!("no JSON-RPC message in: {raw}"))
}

fn tool_names_of(v: &serde_json::Value) -> Vec<String> {
    v["result"]["tools"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|t| t["name"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn http_tools(port: u16, sid: &str) -> Vec<String> {
    let (_, _, raw) = post(
        port,
        r#"{"jsonrpc":"2.0","id":7,"method":"tools/list","params":{}}"#,
        Some(sid),
    );
    tool_names_of(&message_in(&raw))
}

/// Poll `/readyz` until it answers `want`, within `limit`.
fn readiness_becomes(port: u16, want: u16, limit: Duration) -> (u16, String) {
    let deadline = Instant::now() + limit;
    loop {
        let (status, body) = http(port, "GET", "/readyz", "127.0.0.1", "");
        if status == want || Instant::now() >= deadline {
            return (status, body);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// A stdio client, driven one JSON-RPC line at a time.
struct StdioClient {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
}

impl Drop for StdioClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl StdioClient {
    fn start(args: &[&str], dir: &Path) -> StdioClient {
        let mut child = Command::new(bin())
            .args(args)
            .current_dir(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn the stdio client");
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, lines) = channel::<String>();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let _ = tx.send(line);
            }
        });
        StdioClient {
            child,
            stdin,
            lines,
        }
    }

    fn send(&mut self, v: serde_json::Value) {
        writeln!(self.stdin, "{v}").expect("write");
        self.stdin.flush().unwrap();
    }

    /// The next message matching `pred`, within `limit`.
    fn next_matching(
        &mut self,
        limit: Duration,
        pred: impl Fn(&serde_json::Value) -> bool,
    ) -> Option<serde_json::Value> {
        let deadline = Instant::now() + limit;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let line = self.lines.recv_timeout(left).ok()?;
            let v: serde_json::Value = serde_json::from_str(&line).unwrap_or_default();
            if pred(&v) {
                return Some(v);
            }
        }
    }

    fn reply(&mut self, id: i64, limit: Duration) -> serde_json::Value {
        self.next_matching(limit, |v| v["id"] == id)
            .unwrap_or_else(|| panic!("no reply to request {id} within {limit:?}"))
    }

    fn handshake(&mut self, limit: Duration) -> serde_json::Value {
        self.send(serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}));
        let hello = self.reply(1, limit);
        self.send(serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        hello
    }

    fn tools(&mut self, id: i64) -> Vec<String> {
        self.send(serde_json::json!({"jsonrpc":"2.0","id":id,"method":"tools/list","params":{}}));
        tool_names_of(&self.reply(id, Duration::from_secs(60)))
    }
}

fn store_in(dir: &Path) -> String {
    dir.join(".reflow2")
        .join("graph")
        .to_string_lossy()
        .to_string()
}

#[test]
fn an_http_server_behind_a_held_lock_serves_the_design_in_place_when_it_frees() {
    let dir = tmp_dir("http");
    let store = store_in(&dir);
    let holder = Server::start(&["--graph-path", &store, "--http", "127.0.0.1:0"]);
    let mut waiting = Server::start(&["--graph-path", &store, "--http", "127.0.0.1:0"]);

    // While the other process holds the store: not ready, and says so.
    let (status, body) = http(waiting.port, "GET", "/readyz", "127.0.0.1", "");
    assert_eq!(
        status, 503,
        "a server that is not serving its design must not report ready: {body}"
    );
    assert!(
        body.contains("held"),
        "the readiness reply says why, in one line: {body}"
    );
    assert!(
        !body.contains(&store),
        "and discloses no path — it is answered to anyone who can reach the port: {body}"
    );
    let (live, _) = http(waiting.port, "GET", "/healthz", "127.0.0.1", "");
    assert_eq!(
        live, 200,
        "the process is alive and serving, so /healthz is 200"
    );
    let early = open_session(waiting.port);
    assert_eq!(
        http_tools(waiting.port, &early),
        vec!["reflow2_unavailable".to_string()],
        "while the lock is held, the one-tool surface explains why"
    );

    // The holder goes away; the waiting server takes over IN PLACE.
    stop(holder);
    let (status, body) = readiness_becomes(waiting.port, 200, Duration::from_secs(45));
    assert_eq!(
        status,
        200,
        "the server must serve the design once the lock frees, with no restart. /readyz: \
         {body}\nstderr:\n{}",
        waiting.log()
    );
    assert!(
        waiting.still_running(),
        "the SAME process serves it — nothing restarted"
    );
    let tools = http_tools(waiting.port, &early);
    assert!(
        tools.len() > 1 && tools.iter().any(|t| t == "get_node"),
        "a session opened while degraded now sees the design's tools: {tools:?}"
    );
    let fresh = open_session(waiting.port);
    assert!(
        http_tools(waiting.port, &fresh)
            .iter()
            .any(|t| t == "get_node"),
        "and a new session gets the design"
    );
    assert!(
        waiting.log().contains("serves it from here on"),
        "the operator is told the design is served now:\n{}",
        waiting.log()
    );
}

/// A server SERVED FOR OTHERS that waited out a held lock serves the design
/// under the caller rule a healthy start would have installed — #616 fix 3
/// meeting fix 4.
///
/// ⭐ WHY THIS EXISTS. Fix 4 (`reflow2_mcp::caller`) gives every engine served
/// over HTTP the rule for who is calling, at the one place the healthy start
/// builds it. Fix 3 added a SECOND place a design gets served over HTTP: the
/// degraded surface opens the store itself once the holder lets go. The two
/// were written apart and merged cleanly, and the recovered design came up
/// LOCAL — `--http-allow-host` naming another machine, and any caller's
/// signature accepted, after a rolling update. Nothing failed to compile and
/// no test was red. So the same rule is asserted here, through the recovered
/// door: the handshake says what signing means, a settle is refused naming the
/// flag that establishes the caller, and the operator's banner says so.
#[test]
fn a_recovered_server_served_for_others_keeps_the_caller_rule_a_healthy_start_installs() {
    let dir = tmp_dir("exposed");
    let store = store_in(&dir);
    let holder = Server::start(&["--graph-path", &store, "--http", "127.0.0.1:0"]);
    let mut waiting = Server::start(&[
        "--graph-path",
        &store,
        "--http",
        "127.0.0.1:0",
        "--http-allow-host",
        "team.example.org",
    ]);
    stop(holder);
    let (status, body) = readiness_becomes(waiting.port, 200, Duration::from_secs(45));
    assert_eq!(
        status,
        200,
        "the design is served once the lock frees: {body}\nstderr:\n{}",
        waiting.log()
    );
    assert!(waiting.still_running(), "the same process serves it");

    let (status, sid, raw) = post(
        waiting.port,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}"#,
        None,
    );
    assert_eq!(status, 200, "initialize failed: {raw}");
    let sid = sid.expect("a session id");
    let said = message_in(&raw)["result"]["instructions"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(
        said.contains("SERVED FOR OTHERS"),
        "a design served after the wait tells every session what a signature means here, as a \
         healthy start does: {}",
        &said[..said.len().min(600)]
    );
    let _ = post(
        waiting.port,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        Some(&sid),
    );

    let (_, _, raw) = post(
        waiting.port,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"import_graph","arguments":{"document":{"nodes":[{"node_type":"Decision","node_id":"dec:settled-after-the-wait","properties":{"name":"I","decision":"D","kind":"choice","status":"accepted"}}],"edges":[]}}}}"#,
        Some(&sid),
    );
    let reply = message_in(&raw);
    let refused = reply.get("error").is_some() || reply["result"]["isError"] == true;
    assert!(
        refused && reply.to_string().contains("--http-trusted-gateway"),
        "a server served for others that cannot establish the caller refuses a settle, naming \
         the flag that would: {reply}"
    );
    assert!(
        waiting.log().contains("served for others"),
        "and the operator's banner says how signatures are held here:\n{}",
        waiting.log()
    );
}

#[test]
fn a_stdio_session_behind_a_held_lock_is_told_the_tool_list_changed_when_it_frees() {
    let dir = tmp_dir("stdio");
    let store = store_in(&dir);
    let holder = Server::start(&["--graph-path", &store, "--http", "127.0.0.1:0"]);

    let mut client = StdioClient::start(&["--graph-path", &store], &dir);
    let hello = client.handshake(Duration::from_secs(60));
    assert_eq!(
        hello["result"]["capabilities"]["tools"]["listChanged"], true,
        "a server that may grow its surface declares it: {hello}"
    );
    let said = hello["result"]["instructions"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(said.contains("UNAVAILABLE"), "{said}");
    assert!(
        said.contains("as soon as"),
        "and says it will serve the design by itself once the holder lets go: {said}"
    );
    assert_eq!(client.tools(2), vec!["reflow2_unavailable".to_string()]);

    stop(holder);
    let changed = client.next_matching(Duration::from_secs(45), |v| {
        v["method"] == "notifications/tools/list_changed"
    });
    assert!(
        changed.is_some(),
        "the connected client must be TOLD the tool list changed — the MCP way to grow a surface"
    );
    let tools = client.tools(3);
    assert!(
        tools.iter().any(|t| t == "get_node"),
        "after the notification, the full surface is served: {tools:?}"
    );
}

#[test]
fn a_shared_session_that_found_no_server_serves_the_design_in_place_once_the_holder_stops() {
    let dir = tmp_dir("shared");
    let store = store_in(&dir);
    // A NON-shared process holds the store: every daemon the session spawns
    // loses the lock race, no rendezvous appears, and the election times out.
    let holder = Server::start(&["--graph-path", &store, "--http", "127.0.0.1:0"]);

    let mut client = StdioClient::start(&["--graph-path", &store, "--shared"], &dir);
    let hello = client.handshake(Duration::from_secs(90));
    let said = hello["result"]["instructions"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(said.contains("UNAVAILABLE"), "{said}");
    assert_eq!(client.tools(2), vec!["reflow2_unavailable".to_string()]);

    stop(holder);
    let changed = client.next_matching(Duration::from_secs(120), |v| {
        v["method"] == "notifications/tools/list_changed"
    });
    let ok = changed.is_some();
    let tools = if ok { client.tools(3) } else { Vec::new() };
    // Stop the shared server the session elected, whatever happened above.
    let _ = Command::new(bin())
        .args(["--graph-path", &store, "--stop-shared"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    assert!(
        ok,
        "a --shared session must re-elect and be told the tool list changed once the holder stops"
    );
    assert!(
        tools.iter().any(|t| t == "get_node"),
        "and then serve the design through the shared server: {tools:?}"
    );
}

#[test]
fn readiness_and_liveness_answer_whatever_the_host_and_nothing_else_does() {
    let dir = tmp_dir("host");
    let store = store_in(&dir);
    let server = Server::start(&["--graph-path", &store, "--http", "127.0.0.1:0"]);
    for host in ["127.0.0.1", "team.example.org", "10.1.2.3:8080"] {
        let (status, body) = http(server.port, "GET", "/readyz", host, "");
        assert_eq!(
            status, 200,
            "an orchestrator probes by pod IP or name; /readyz must answer Host {host}: {body}"
        );
        let (status, _) = http(server.port, "GET", "/healthz", host, "");
        assert_eq!(status, 200, "/healthz must answer Host {host}");
    }
    let (status, body) = http(
        server.port,
        "POST",
        "/",
        "team.example.org",
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
    );
    assert_eq!(
        status, 403,
        "the probes are the ONLY exemption: the design itself stays behind the Host gate: {body}"
    );

    // A registry is ready when it is up: one design it cannot serve does not
    // make the whole server unready (each is named on GET /, fix 2 of #616).
    let root = tmp_dir("registry");
    let registry = Server::start(&[
        "--registry-root",
        &root.to_string_lossy(),
        "--http",
        "127.0.0.1:0",
    ]);
    let (status, body) = http(registry.port, "GET", "/readyz", "127.0.0.1", "");
    assert_eq!(status, 200, "a running registry is ready: {body}");
}

#[test]
fn a_cause_that_cannot_clear_by_itself_is_not_polled_and_readiness_stays_503() {
    let dir = tmp_dir("permanent");
    let store = store_in(&dir);
    // Make a real store, stop its server, then break the stamp beside it: no
    // other process holds anything, and waiting cannot fix a stamp that will
    // not read.
    stop(Server::start(&[
        "--graph-path",
        &store,
        "--http",
        "127.0.0.1:0",
    ]));
    let stamp = format!("{store}.meta.json");
    assert!(Path::new(&stamp).exists(), "the store has a stamp: {stamp}");
    std::fs::write(&stamp, "this is not a stamp").unwrap();

    let server = Server::start(&["--graph-path", &store, "--http", "127.0.0.1:0"]);
    let (status, body) = http(server.port, "GET", "/readyz", "127.0.0.1", "");
    assert_eq!(status, 503, "{body}");
    std::thread::sleep(Duration::from_secs(4));
    let (status, _) = http(server.port, "GET", "/readyz", "127.0.0.1", "");
    assert_eq!(status, 503, "nothing clears a broken stamp by itself");
    let log = server.log();
    assert!(
        log.contains("restart"),
        "the operator is told a restart after fixing the cause is what serves it:\n{log}"
    );
    assert!(
        !log.contains("as soon as"),
        "and is NOT promised a recovery that cannot happen:\n{log}"
    );
}
