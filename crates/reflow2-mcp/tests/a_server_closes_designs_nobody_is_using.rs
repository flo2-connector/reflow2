//! A server holding many designs closes the ones nobody is using, and says
//! "busy" rather than "not found" when it truly cannot open one.
//!
//! ⭐ WHY THIS IS END TO END. The router's own tests pin the lifecycle: least
//! recently used at the limit, busy only when every slot is serving, no reopen
//! until the old copy lets go, idle closing. What only the real binary over
//! the real transport can show is what a CLIENT meets:
//!   · three designs behind a limit of two all answer, where the third used to
//!     be refused with 404 until a restart — flo2.io, 2026-09-27
//!     (`fact:flo2-io-met-the-open-design-cap-with-no-idle-eviction-2026-09-27`);
//!   · what was written before a design closed is there after it reopens;
//!   · a closed design's old session is gone (404), and starting again works —
//!     the path flo2's gateway takes;
//!   · the idle sweep runs on its own, with no request to trigger it.
//!
//! `req:a-hosted-server-closes-idle-designs-and-says-busy-when-full`.

use std::io::{BufRead, BufReader, Write};
use std::sync::{Arc, Mutex};

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// A registry root nothing else in the suite can collide with. Same idiom as
/// `sessions_cannot_cross_designs`: two tests sharing a path would take the
/// same RocksDB lock and look like a hang rather than a collision.
fn tmp_root() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "reflow2-registry-http-{}-{}",
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
    // The integration-test binary sits beside the built one.
    let mut p = std::env::current_exe().expect("test binary has a path");
    p.pop();
    if p.ends_with("deps") {
        p.pop();
    }
    p.join("reflow2-mcp")
}

/// Mint a design by running the binary over stdio once, and return its graph_id.
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
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2025-06-18","capabilities":{{}},"clientInfo":{{"name":"mint","version":"1"}}}}}}"#
        )
        .expect("write initialize");
    }
    // CLOSE STDIN so the child sees EOF and exits. Without this it waits for
    // more JSON-RPC that never comes and the test pays the full timeout per
    // design — which is what made the first run take two minutes.
    drop(child.stdin.take());
    let _ = child.wait_timeout_kill(Duration::from_secs(60));
    let id_file = dir.join(".reflow2").join("graph.id.json");
    let raw = std::fs::read_to_string(&id_file)
        .unwrap_or_else(|e| panic!("no identity sidecar at {}: {e}", id_file.display()));
    let v: serde_json::Value = serde_json::from_str(&raw).expect("identity sidecar is json");
    v["graph_id"]
        .as_str()
        .expect("identity sidecar names a graph_id")
        .to_string()
}

trait WaitKill {
    fn wait_timeout_kill(&mut self, d: Duration) -> Option<std::process::ExitStatus>;
}
impl WaitKill for Child {
    fn wait_timeout_kill(&mut self, d: Duration) -> Option<std::process::ExitStatus> {
        let start = Instant::now();
        loop {
            match self.try_wait() {
                Ok(Some(s)) => return Some(s),
                Ok(None) if start.elapsed() < d => std::thread::sleep(Duration::from_millis(50)),
                _ => {
                    let _ = self.kill();
                    return None;
                }
            }
        }
    }
}

/// A registry server on an OS-assigned port, the port it landed on, and every
/// line it has written to stderr so far.
struct Server {
    child: Child,
    port: u16,
    log: Arc<Mutex<Vec<String>>>,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Server {
    fn logged(&self, needle: &str) -> bool {
        self.log.lock().unwrap().iter().any(|l| l.contains(needle))
    }
}

fn start_registry(root: &Path, extra: &[&str]) -> Server {
    let mut child = Command::new(bin())
        .arg("--registry-root")
        .arg(root)
        .arg("--http")
        .arg("127.0.0.1:0")
        .args(extra)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the registry server");
    // Read the port off the server's own banner, and KEEP DRAINING stderr on a
    // thread (see two_designs_stay_apart_over_the_transport.rs for why a closed
    // pipe kills the server) — here also keeping each line, so a test can see
    // what the server says it closed.
    let stderr = child.stderr.take().expect("stderr piped");
    let log = Arc::new(Mutex::new(Vec::new()));
    let (tx, rx) = std::sync::mpsc::channel::<u16>();
    let kept = Arc::clone(&log);
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
            kept.lock().unwrap().push(line);
        }
    });
    let port = rx
        .recv_timeout(Duration::from_secs(90))
        .expect("the server prints the address it bound within 90s");
    Server { child, port, log }
}

/// One HTTP POST to the server, returning (status, session id, body).
fn post(port: u16, path: &str, body: &str, session: Option<&str>) -> (u16, Option<String>, String) {
    use std::io::Read;
    use std::net::TcpStream;
    let mut s = TcpStream::connect(("127.0.0.1", port)).expect("connect");
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

fn open_session(port: u16, prefix: &str) -> String {
    let (status, sid, body) = post(
        port,
        prefix,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}"#,
        None,
    );
    assert_eq!(status, 200, "initialize on {prefix} failed: {body}");
    let sid = sid.unwrap_or_else(|| panic!("no session id from {prefix}"));
    let _ = post(
        port,
        prefix,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        Some(&sid),
    );
    sid
}

fn call(port: u16, prefix: &str, sid: &str, tool: &str, args: &str) -> String {
    let body = format!(
        r#"{{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{{"name":"{tool}","arguments":{args}}}}}"#
    );
    post(port, prefix, &body, Some(sid)).2
}

fn design(root: &Path, name: &str) -> String {
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    mint(&dir)
}

#[test]
fn a_full_server_closes_its_least_recently_used_design_instead_of_refusing() {
    let root = tmp_root();
    let a = design(&root, "alpha");
    let b = design(&root, "beta");
    let c = design(&root, "gamma");
    let server = start_registry(&root, &["--registry-max-open", "2"]);
    let port = server.port;
    let (pa, pb, pc) = (format!("/g/{a}/"), format!("/g/{b}/"), format!("/g/{c}/"));

    // Something written into A before it is closed.
    let sa = open_session(port, &pa);
    let wrote = call(
        port,
        &pa,
        &sa,
        "add_requirement",
        r#"{"id":"req:survives-a-close","name":"Survives a close","statement":"Written before the design was closed."}"#,
    );
    assert!(
        wrote.contains("req:survives-a-close"),
        "the write landed: {wrote}"
    );

    // B, then C: C is the third design behind a limit of two. Before
    // 2026-09-27 this was refused with 404 until the server restarted.
    open_session(port, &pb);
    let sc = open_session(port, &pc);
    assert!(
        !sc.is_empty(),
        "the third design opened instead of being refused"
    );
    assert!(
        server.logged(&format!("closed design {a}")),
        "the server says which design it closed to make room: {:?}",
        server.log.lock().unwrap()
    );

    // A's old session ended with it: the client is told the session is gone.
    let (status, _, body) = post(
        port,
        &pa,
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/list"}"#,
        Some(&sa),
    );
    assert_eq!(
        status, 404,
        "a closed design's session is unknown, which is what makes a client start again: {body}"
    );

    // ...and starting again works, with what was written still there.
    let sa2 = open_session(port, &pa);
    let read = call(
        port,
        &pa,
        &sa2,
        "get_node",
        r#"{"id":"req:survives-a-close"}"#,
    );
    assert!(
        read.contains("Written before the design was closed."),
        "closing a design loses nothing written to it: {read}"
    );
}

#[test]
fn a_design_nobody_uses_is_closed_by_the_idle_sweep() {
    let root = tmp_root();
    let a = design(&root, "alpha");
    let server = start_registry(&root, &["--registry-idle", "2s"]);
    let port = server.port;
    open_session(port, &format!("/g/{a}/"));
    let deadline = Instant::now() + Duration::from_secs(20);
    while !server.logged(&format!("closed design {a} (idle)")) {
        assert!(
            Instant::now() < deadline,
            "an idle design must be closed with no request to prompt it: {:?}",
            server.log.lock().unwrap()
        );
        std::thread::sleep(Duration::from_millis(200));
    }
    // And it comes back on the next request.
    open_session(port, &format!("/g/{a}/"));
}

#[test]
fn a_name_that_is_no_design_is_still_not_found() {
    let root = tmp_root();
    design(&root, "alpha");
    let server = start_registry(&root, &["--registry-max-open", "1"]);
    let (status, _, body) = post(
        server.port,
        "/g/no-such-design/",
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}"#,
        None,
    );
    assert_eq!(
        status, 404,
        "only capacity is 503; a design that is not there is 404: {body}"
    );
}
