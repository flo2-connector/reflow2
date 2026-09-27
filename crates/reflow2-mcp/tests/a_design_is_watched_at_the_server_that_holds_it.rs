//! A design that depends on another design watches it at the server that holds
//! its blueprint, never through a file
//! (`req:a-design-watches-another-design-at-the-server-that-holds-it`, accepted
//! 2026-09-27).
//!
//! Under the one-blueprint rules a design IS the store on the server that holds
//! it, and an export is a perishable photocopy nothing tracks. The first design
//! that moved to flo2.io left its last photocopy behind, and the dev_reflow2
//! hub's file watch on it reported the move itself and would then have said
//! "unchanged" forever
//! (`fact:a-moved-design-cannot-be-watched-and-its-frozen-export-reads-as-live-2026-09-27`).
//!
//! What must hold, and what each test pins:
//! · a REAL reflow2 serving a design over HTTP is asked for it, a real change
//!   there is seen as `moved`, and the watcher takes in none of its nodes;
//! · a server that stops answering is `unreachable` — UNKNOWN, never unchanged;
//! · a server that refuses is `refused`, in its own words, and a key sent to it
//!   appears in nothing the caller reads;
//! · a server answering with something that is not a design is `unreadable`;
//! · a key is never sent in the clear to another machine;
//! · a server holding OTHER PEOPLE'S designs never reaches out on a caller's
//!   behalf, and neither does `loop_status`, the orientation read;
//! · a watch in two places at once is refused before any network call.
//!
//! The judgement itself is pinned in the core
//! (`an_upstream_design_is_watched_not_imported.rs`); these cover the half only
//! a network can answer.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use reflow2_mcp::service::ReflowService;
use reflow2_mcp::upstream;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

const KEY: &str = "r2k_watch_test_5d1e-never-print-me";

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

/// Point reflow2's client settings at an empty folder, so no test here can read
/// (or be confused by) the settings and keychain of whoever runs the suite.
/// Every test in this file sets the same value, so the process-wide variable
/// never changes under a running test.
fn hermetic_settings() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let dir =
            std::env::temp_dir().join(format!("reflow2-watch-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("settings dir");
        // SAFETY: set once, before any test in this binary reads it, to the
        // same value for all of them.
        unsafe { std::env::set_var("REFLOW2_CONFIG_DIR", &dir) };
    });
}

fn tmp_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "reflow2-watch-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

trait WaitKill {
    fn wait_timeout_kill(&mut self, d: Duration);
}
impl WaitKill for Child {
    fn wait_timeout_kill(&mut self, d: Duration) {
        let start = Instant::now();
        loop {
            match self.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) if start.elapsed() < d => std::thread::sleep(Duration::from_millis(50)),
                _ => {
                    let _ = self.kill();
                    let _ = self.wait();
                    return;
                }
            }
        }
    }
}

/// Mint a real design by running the binary over stdio once; its graph_id.
fn mint(dir: &Path) -> String {
    let store = dir.join(".reflow2").join("graph");
    let mut child = Command::new(bin())
        .arg("--graph-path")
        .arg(&store)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn reflow2-mcp to mint a design");
    writeln!(
        child.stdin.as_mut().expect("stdin"),
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2025-06-18","capabilities":{{}},"clientInfo":{{"name":"mint","version":"1"}}}}}}"#
    )
    .expect("write initialize");
    drop(child.stdin.take());
    child.wait_timeout_kill(Duration::from_secs(60));
    let raw = std::fs::read_to_string(dir.join(".reflow2").join("graph.id.json"))
        .expect("identity sidecar");
    let v: Value = serde_json::from_str(&raw).expect("identity sidecar is json");
    v["graph_id"].as_str().expect("graph_id").to_string()
}

/// A real reflow2 serving every design under `root` over HTTP — the
/// `/g/<graph_id>/mcp` shape flo2.io serves — on an OS-assigned port.
struct Server {
    child: Child,
    port: u16,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn serve(root: &Path) -> Server {
    let mut child = Command::new(bin())
        .arg("--registry-root")
        .arg(root)
        .arg("--http")
        .arg("127.0.0.1:0")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the registry server");
    // Read the bound port off the server's own banner, and KEEP DRAINING stderr
    // on a thread: closing the pipe under a server still writing to it kills
    // the server (the lesson `two_designs_stay_apart_over_the_transport` paid for).
    let stderr = child.stderr.take().expect("stderr piped");
    let (tx, rx) = std::sync::mpsc::channel::<u16>();
    std::thread::spawn(move || {
        let mut sent = false;
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if !sent && let Some(i) = line.find("http://127.0.0.1:") {
                let tail = &line[i + "http://127.0.0.1:".len()..];
                let digits: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(p) = digits.parse::<u16>() {
                    let _ = tx.send(p);
                    sent = true;
                }
            }
        }
    });
    let port = rx
        .recv_timeout(Duration::from_secs(90))
        .expect("the server prints the address it bound within 90s");
    Server { child, port }
}

/// One JSON-RPC POST to the real server: (session id, raw reply).
fn post(port: u16, path: &str, body: &str, session: Option<&str>) -> (Option<String>, String) {
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
    s.set_read_timeout(Some(Duration::from_secs(120))).ok();
    let mut req = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n\
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
    let sid = raw.lines().find_map(|l| {
        let (k, v) = l.split_once(':')?;
        k.trim()
            .eq_ignore_ascii_case("mcp-session-id")
            .then(|| v.trim().to_string())
    });
    (sid, raw)
}

/// Change the design the real server holds, over its own transport.
fn add_component_upstream(port: u16, path: &str, id: &str) {
    let (sid, _) = post(
        port,
        path,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"upstream-writer","version":"1"}}}"#,
        None,
    );
    let sid = sid.expect("the upstream server hands out a session");
    let _ = post(
        port,
        path,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        Some(&sid),
    );
    let call = json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": {"name": "add_component",
                   "arguments": {"id": id, "name": "later work", "description": "work the upstream did afterwards"}}
    })
    .to_string();
    let (_, raw) = post(port, path, &call, Some(&sid));
    assert!(
        raw.contains(id) && !raw.contains("\"isError\":true"),
        "the upstream write must land: {raw}"
    );
}

async fn declare(s: &ReflowService, args: Value) -> Result<Value, String> {
    s.external_dependency(Parameters(serde_json::from_value(args).unwrap()))
        .await
        .map(|r| r.structured_content.expect("structured"))
        .map_err(|e| format!("{e:?}"))
}

async fn status(s: &ReflowService) -> Value {
    s.upstream_status(Parameters(serde_json::from_value(json!({})).unwrap()))
        .await
        .expect("upstream_status")
        .structured_content
        .expect("structured")
}

fn finding<'a>(report: &'a Value, dependency: &str) -> &'a Value {
    report["findings"]
        .as_array()
        .expect("findings")
        .iter()
        .find(|f| f["dependency"] == dependency)
        .unwrap_or_else(|| panic!("no finding for {dependency}: {report}"))
}

/// ⭐ THE WHOLE CHAIN ON REAL PARTS: a real reflow2 serves a real design over
/// HTTP; this design declares a watch at its address, sees it unchanged, sees a
/// real change as `moved`, takes none of its nodes in, and — once the server is
/// gone — says UNKNOWN rather than unchanged.
#[tokio::test]
async fn a_real_server_is_asked_and_a_real_change_is_seen_and_an_outage_is_never_unchanged() {
    hermetic_settings();
    let root = tmp_root("real");
    let dir = root.join("upstream");
    std::fs::create_dir_all(&dir).unwrap();
    let id = mint(&dir);
    let server = serve(&root);
    let path = format!("/g/{id}/mcp");
    let address = format!("http://127.0.0.1:{}{path}", server.port);

    let mine = ReflowService::in_memory().expect("service");
    let reply = declare(
        &mine,
        json!({"id": "dep:upstream", "name": "upstream", "source": address, "version": "live",
               "graph_id": id, "design_address": address, "design_address_seen_at": "2026-09-27"}),
    )
    .await
    .expect("declare");
    let b = &reply["address_baseline"];
    assert_eq!(
        b["taken"], true,
        "the baseline is taken at declaration: {reply}"
    );
    assert_eq!(b["graph_id"], id.as_str());
    let first = b["fingerprint"].as_str().expect("fingerprint").to_string();
    assert!(
        reply["value"]
            .as_str()
            .unwrap()
            .contains(&format!("design_address = \"{address}\"")),
        "the manifest names the address: {reply}"
    );

    let r = status(&mine).await;
    let f = finding(&r, "dep:upstream");
    assert_eq!(f["kind"], "unchanged", "{r}");
    assert_eq!(f["design_address"], address.as_str());

    // A real change upstream, made through the server's own transport.
    add_component_upstream(server.port, &path, "cmp:later");
    let r = status(&mine).await;
    assert_eq!(finding(&r, "dep:upstream")["kind"], "moved", "{r}");
    // …and keeps being reported: a read never refreshes the baseline.
    let r = status(&mine).await;
    assert_eq!(finding(&r, "dep:upstream")["kind"], "moved", "{r}");
    let now = upstream::fingerprint(&address)
        .await
        .expect("the server still answers");
    assert_ne!(now.content_hash, first);

    // Watching took nothing in: this design holds none of the upstream's parts.
    let comps: Value = mine
        .scan_nodes(Parameters(
            serde_json::from_value(json!({"node_type": "Component", "brief": true})).unwrap(),
        ))
        .await
        .expect("scan")
        .structured_content
        .expect("structured");
    assert_eq!(comps["total"], 0, "a watch must never import: {comps}");

    // The server goes away. Whether the upstream moved is now UNKNOWN.
    drop(server);
    let r = status(&mine).await;
    let f = finding(&r, "dep:upstream");
    assert_eq!(f["kind"], "unreachable", "{r}");
    assert!(
        f["detail"]
            .as_str()
            .unwrap()
            .contains("UNKNOWN, not unchanged"),
        "{f}"
    );
    assert!(
        !r["note"].as_str().unwrap().contains("none moved"),
        "the summary must not claim nothing moved: {r}"
    );
}

// ───────────────────────────────────────────────────────────── stand-ins

/// How a stand-in answers.
#[derive(Clone, Copy)]
enum Mode {
    /// Every request refused, 401.
    Refuses,
    /// A working handshake, then a result that is not a reflow2 design.
    AnswersSomethingElse,
}

/// A stand-in server on loopback that counts the connections it is sent and
/// answers as `mode` says. Returns (address, connection count, last Authorization header seen).
fn stand_in(
    mode: Mode,
) -> (
    String,
    Arc<AtomicUsize>,
    Arc<std::sync::Mutex<Option<String>>>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let address = format!("http://{}/g/abc/mcp", listener.local_addr().unwrap());
    let count = Arc::new(AtomicUsize::new(0));
    let auth = Arc::new(std::sync::Mutex::new(None));
    let (c, a) = (Arc::clone(&count), Arc::clone(&auth));
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            c.fetch_add(1, Ordering::SeqCst);
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut len = 0usize;
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 {
                    break;
                }
                let line = line.trim_end();
                if line.is_empty() {
                    break;
                }
                if let Some((k, v)) = line.split_once(':') {
                    let k = k.trim().to_ascii_lowercase();
                    if k == "content-length" {
                        len = v.trim().parse().unwrap_or(0);
                    }
                    if k == "authorization" {
                        *a.lock().unwrap() = Some(v.trim().to_string());
                    }
                }
            }
            let mut body = vec![0u8; len];
            let _ = reader.read_exact(&mut body);
            let msg: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
            let (code, extra, reply) = match (mode, msg.get("id")) {
                (Mode::Refuses, _) => (
                    401,
                    String::new(),
                    r#"{"error":"invalid_token"}"#.to_string(),
                ),
                (_, None) => (202, String::new(), String::new()),
                (Mode::AnswersSomethingElse, Some(id)) => {
                    let result = if msg["method"] == "initialize" {
                        json!({"protocolVersion": "2025-06-18", "capabilities": {"tools": {}},
                               "serverInfo": {"name": "stand-in", "version": "0"}})
                    } else {
                        json!({"structuredContent": {"hello": "world"},
                               "content": [{"type": "text", "text": "hello"}]})
                    };
                    (
                        200,
                        "mcp-session-id: s-1\r\n".to_string(),
                        json!({"jsonrpc": "2.0", "id": id, "result": result}).to_string(),
                    )
                }
            };
            let _ = write!(
                stream,
                "HTTP/1.1 {code} X\r\ncontent-type: application/json\r\n{extra}content-length: {}\r\nconnection: close\r\n\r\n{reply}",
                reply.len()
            );
        }
    });
    (address, count, auth)
}

#[tokio::test]
async fn a_server_that_refuses_is_reported_in_its_own_words_and_the_key_never_appears() {
    hermetic_settings();
    let (address, _, auth) = stand_in(Mode::Refuses);
    let n = upstream::fingerprint_at(&address, Some(KEY))
        .await
        .expect_err("a 401 is not a fingerprint");
    assert_eq!(n.state, "refused");
    assert!(
        n.detail.contains("401"),
        "the server's refusal is named: {}",
        n.detail
    );
    assert!(
        !n.detail.contains(KEY),
        "the key must appear in nothing the caller reads"
    );
    assert_eq!(
        auth.lock().unwrap().as_deref(),
        Some(format!("Bearer {KEY}").as_str()),
        "the key does reach the server, as a bearer header"
    );
}

/// With no key set up for that server, the refusal says how one gets set up —
/// "the key is missing, wrong, expired or revoked" cannot tell a person this
/// machine never had one.
#[tokio::test]
async fn a_refusal_with_no_key_set_up_says_how_to_set_one_up() {
    hermetic_settings();
    let (address, _, auth) = stand_in(Mode::Refuses);
    let n = upstream::fingerprint(&address)
        .await
        .expect_err("a 401 is not a fingerprint");
    assert_eq!(n.state, "refused");
    assert!(
        n.detail.contains("reflow2-mcp setup remote"),
        "{}",
        n.detail
    );
    assert_eq!(
        *auth.lock().unwrap(),
        None,
        "no key was set up, so none is sent"
    );
}

#[tokio::test]
async fn a_server_answering_with_something_that_is_not_a_design_is_unreadable() {
    hermetic_settings();
    let (address, _, _) = stand_in(Mode::AnswersSomethingElse);
    let n = upstream::fingerprint_at(&address, None)
        .await
        .expect_err("that is not a design");
    assert_eq!(n.state, "unreadable", "{}", n.detail);
}

#[tokio::test]
async fn a_key_is_never_sent_in_the_clear_to_another_machine() {
    hermetic_settings();
    let n = upstream::fingerprint_at("http://reflow2.example.invalid/g/x/mcp", Some(KEY))
        .await
        .expect_err("refused before anything is sent");
    assert_eq!(n.state, "refused");
    assert!(n.detail.contains("http://"), "{}", n.detail);
    assert!(!n.detail.contains(KEY));
}

/// 🛑 A server holding OTHER PEOPLE'S designs — flo2.io's shape — never goes
/// out over the network on a caller's say-so. The watch is recorded, the reply
/// says why no baseline was taken, `upstream_status` reports `refused`, and the
/// address named is never contacted.
#[tokio::test]
async fn a_server_holding_other_peoples_designs_never_reaches_out() {
    hermetic_settings();
    let (address, count, _) = stand_in(Mode::AnswersSomethingElse);
    let hosted = ReflowService::in_memory()
        .expect("service")
        .without_reaching_out();
    let reply = declare(
        &hosted,
        json!({"id": "dep:elsewhere", "name": "elsewhere", "source": address, "version": "live",
               "graph_id": "abc", "design_address": address}),
    )
    .await
    .expect("the watch is still recorded");
    assert_eq!(reply["address_baseline"]["taken"], false, "{reply}");
    assert_eq!(reply["address_baseline"]["state"], "refused");

    let r = status(&hosted).await;
    assert_eq!(finding(&r, "dep:elsewhere")["kind"], "refused", "{r}");
    assert_eq!(
        count.load(Ordering::SeqCst),
        0,
        "the address a caller named must never be contacted from a hosted server"
    );
}

/// `loop_status` is the orientation read every session runs first; it must
/// never wait on somebody else's server. An address watch gets no network call
/// there, and no line in `next`.
#[tokio::test]
async fn loop_status_never_goes_over_the_network_for_an_address_watch() {
    hermetic_settings();
    let (address, count, _) = stand_in(Mode::AnswersSomethingElse);
    // The declaration itself may call once for the baseline; count from after it.
    let mine = ReflowService::in_memory().expect("service");
    let _ = declare(
        &mine,
        json!({"id": "dep:elsewhere", "name": "elsewhere", "source": address, "version": "live",
               "graph_id": "abc", "design_address": address}),
    )
    .await
    .expect("declare");
    let before = count.load(Ordering::SeqCst);

    let ls: Value = mine
        .loop_status(Parameters(serde_json::from_value(json!({})).unwrap()))
        .await
        .expect("loop_status")
        .structured_content
        .expect("structured");
    assert_eq!(
        count.load(Ordering::SeqCst),
        before,
        "loop_status must not contact the watched server"
    );
    let next = ls["next"].to_string();
    assert!(
        !next.contains(&address),
        "an address watch loop_status did not read is not a to-do: {next}"
    );

    // The synchronous reader never returns an observation for an address.
    let targets = vec![reflow2_core::UpstreamTarget {
        id: "dep:elsewhere".into(),
        name: "elsewhere".into(),
        design_export: None,
        design_address: Some(address.clone()),
        graph_id: Some("abc".into()),
        baseline_hash: None,
    }];
    let (observed, not_read) = upstream::observe_upstreams(&targets);
    assert!(observed.is_empty() && not_read.is_none());
}

#[tokio::test]
async fn a_watch_in_two_places_is_refused_before_any_network_call() {
    hermetic_settings();
    let (address, count, _) = stand_in(Mode::AnswersSomethingElse);
    let mine = ReflowService::in_memory().expect("service");
    let err = declare(
        &mine,
        json!({"id": "dep:both", "name": "both", "source": address, "version": "live",
               "graph_id": "abc", "design_address": address,
               "design_export": "/somewhere/docs/design/abc.json"}),
    )
    .await
    .expect_err("one place, not two");
    assert!(err.contains("not both"), "{err}");
    assert_eq!(count.load(Ordering::SeqCst), 0, "refused before any call");
}
