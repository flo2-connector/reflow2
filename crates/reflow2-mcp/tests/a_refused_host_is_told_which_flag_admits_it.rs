//! A request addressed to a host the server was not told about is refused —
//! and the refusal says which host it saw, which hosts ARE allowed, and the
//! exact flag that would admit it. On every HTTP surface reflow2 serves.
//!
//! ⭐ WHY THIS IS A REAL-BINARY TEST. The defect was never in reflow2's own
//! logic: the refusal came from the MCP library, in its words ("Forbidden: Host
//! header is not allowed") and its log line ("possible DNS rebinding attempt"),
//! and reflow2's sentence naming `--http-allow-host` printed once, at startup.
//! Behind a proxy that reads as a broken ingress (GitHub issue #616). Only the
//! running server shows which text a caller actually receives, so the test
//! reads the reply off the wire and the log off stderr.
//!
//! PINNED AS A CLASS, not as one string: "say what would have worked" at the
//! moment of FAILURE, on the single-design server AND the registry router,
//! including a registry path that never reaches a design (a bare `/`, which
//! used to answer 404 with the design list before any Host check ran).
//! `fact:root-cause-the-host-refusal-is-the-librarys-opaque-403-and-the-fix-is-named-only-at-startup-2026-09-28`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn tmp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "reflow2-host-refusal-{tag}-{}-{}",
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

/// A server under test: its port, and every stderr line it has written so far.
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

    /// Every stderr line so far that mentions `needle`, after giving the
    /// server a moment to write what a request just caused.
    fn logged(&self, needle: &str) -> Vec<String> {
        std::thread::sleep(Duration::from_millis(300));
        self.stderr
            .lock()
            .unwrap()
            .iter()
            .filter(|l| l.contains(needle))
            .cloned()
            .collect()
    }
}

/// Send one raw HTTP/1.1 request to the server's loopback port with `host` as
/// the Host header — which is all that makes a request "remote" to this
/// transport — and return (status, content-type, body).
fn request(port: u16, method: &str, path: &str, host: &str) -> (u16, String, String) {
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"host-test","version":"1"}}}"#;
    let payload = if method == "POST" { body } else { "" };
    let mut s = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    s.set_read_timeout(Some(Duration::from_secs(30))).unwrap();
    write!(
        s,
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\n\
         Accept: application/json, text/event-stream\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{payload}",
        payload.len()
    )
    .unwrap();
    let mut raw = Vec::new();
    let _ = s.read_to_end(&mut raw);
    let text = String::from_utf8_lossy(&raw).to_string();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let content_type = head
        .lines()
        .find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.eq_ignore_ascii_case("content-type")
                .then(|| v.trim().to_string())
        })
        .unwrap_or_default();
    (status, content_type, body.to_string())
}

/// What every refusal must say, wherever it happens.
fn assert_refusal_explains_itself(status: u16, content_type: &str, body: &str, host: &str) {
    assert_eq!(status, 403, "an unlisted Host is still refused: {body}");
    assert!(
        content_type.starts_with("text/plain"),
        "a person reads this, so it says what it is: {content_type:?}"
    );
    assert!(
        body.contains(&format!("\"{host}\"")),
        "names the Host it saw: {body}"
    );
    for allowed in ["localhost", "127.0.0.1", "::1"] {
        assert!(
            body.contains(allowed),
            "names the allowed host {allowed}: {body}"
        );
    }
    assert!(
        body.contains(&format!("--http-allow-host {host}")),
        "names the exact flag that would admit it: {body}"
    );
}

const REMOTE: &str = "team.example.org";

#[test]
fn a_single_design_server_says_which_host_it_refused_and_which_flag_admits_it() {
    let dir = tmp_dir("single");
    let store = dir.join(".reflow2").join("graph");
    let server = Server::start(&[
        "--graph-path",
        store.to_str().unwrap(),
        "--http",
        "127.0.0.1:0",
    ]);

    let (status, ct, body) = request(server.port, "POST", "/", REMOTE);
    assert_refusal_explains_itself(status, &ct, &body, REMOTE);

    // The log says the same fix, where an operator tailing a container reads it.
    let said = server.logged(&format!("--http-allow-host {REMOTE}"));
    assert_eq!(said.len(), 1, "one log line naming the fix: {said:?}");

    // BOUNDED: the same host refused again is not logged again, so a flood
    // cannot fill the log. The reply still explains itself every time.
    let (status, ct, body) = request(server.port, "GET", "/", REMOTE);
    assert_refusal_explains_itself(status, &ct, &body, REMOTE);
    assert_eq!(
        server.logged(&format!("--http-allow-host {REMOTE}")).len(),
        1,
        "a repeat refusal of the same host is not logged twice"
    );

    // Unchanged: loopback is answered.
    let (status, _, body) = request(
        server.port,
        "POST",
        "/",
        &format!("127.0.0.1:{}", server.port),
    );
    assert_eq!(status, 200, "loopback is still answered: {body}");
}

#[test]
fn a_host_named_with_the_flag_is_answered_and_loopback_still_is() {
    let dir = tmp_dir("named");
    let store = dir.join(".reflow2").join("graph");
    let server = Server::start(&[
        "--graph-path",
        store.to_str().unwrap(),
        "--http",
        "127.0.0.1:0",
        "--http-allow-host",
        REMOTE,
    ]);
    let (status, _, body) = request(server.port, "POST", "/", REMOTE);
    assert_eq!(status, 200, "a named host is answered: {body}");
    let (status, _, body) = request(server.port, "POST", "/", "localhost");
    assert_eq!(
        status, 200,
        "naming a remote host does not lock out loopback: {body}"
    );
    let (status, ct, body) = request(server.port, "POST", "/", "other.example.org");
    assert_refusal_explains_itself(status, &ct, &body, "other.example.org");
    assert!(
        body.contains(REMOTE),
        "the allowed hosts named include the one the operator added: {body}"
    );
}

#[test]
fn the_registry_router_checks_the_host_before_it_routes_lists_or_opens_anything() {
    let root = tmp_dir("registry");
    let server = Server::start(&[
        "--registry-root",
        root.to_str().unwrap(),
        "--http",
        "127.0.0.1:0",
    ]);

    // A bare path used to answer 404 with the design list before any Host check.
    let (status, ct, body) = request(server.port, "GET", "/", REMOTE);
    assert_refusal_explains_itself(status, &ct, &body, REMOTE);
    assert!(
        !body.contains("Designs under this root") && !body.contains("holds no designs"),
        "a refused Host learns nothing about what the root holds: {body}"
    );

    // A design path is refused the same way, before the router resolves it.
    let (status, ct, body) = request(server.port, "POST", "/g/0123456789abcdef/", REMOTE);
    assert_refusal_explains_itself(status, &ct, &body, REMOTE);

    let said = server.logged(&format!("--http-allow-host {REMOTE}"));
    assert_eq!(said.len(), 1, "one log line naming the fix: {said:?}");

    // Unchanged: loopback reaches the router, which answers its own 404.
    let (status, _, body) = request(server.port, "GET", "/", "127.0.0.1");
    assert_eq!(status, 404, "loopback still reaches the router: {body}");
    assert!(body.contains("/g/<graph_id>/"), "{body}");
}
