//! The largest request the HTTP door takes is the operator's to choose, and a
//! request past it is refused in words that name the limit and the setting.
//!
//! ⭐ WHY THIS IS A REAL-BINARY TEST. The limit is enforced by the MCP library
//! while it streams the body, and only the running server shows which status
//! and which words a caller actually receives, and that the flag reached the
//! transport (`cap:the-http-door-takes-a-request-size-its-operator-chooses`;
//! `fact:measured-the-http-door-takes-4-mib-in-one-request-2026-11-22`).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn tmp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "reflow2-request-size-{tag}-{}-{}",
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
    // Cargo names where it built the server. Working it out from this test's
    // own path assumed tests sit in target/<profile>/deps beside it, and cargo
    // 1.100 builds them under target/<profile>/build/ instead.
    PathBuf::from(env!("CARGO_BIN_EXE_reflow2-mcp"))
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
        Server::start_with_env(args, &[])
    }

    /// Start with these environment variables set on the CHILD only, so no
    /// other test running beside this one sees them.
    fn start_with_env(args: &[&str], env: &[(&str, &str)]) -> Server {
        let mut child = Command::new(bin())
            .args(args)
            .envs(env.iter().copied())
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

/// POST one JSON-RPC `initialize` padded to `size` bytes to the server's
/// loopback port; return (status, body).
fn post_of_size(port: u16, size: usize) -> (u16, String) {
    let head = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"size-test","version":"1"},"pad":""#;
    let tail = r#""}}"#;
    let pad = size.saturating_sub(head.len() + tail.len());
    let body = format!("{head}{}{tail}", "x".repeat(pad));
    assert_eq!(body.len(), size.max(head.len() + tail.len()));
    let mut s = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    s.set_read_timeout(Some(Duration::from_secs(60))).unwrap();
    write!(
        s,
        "POST / HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n\
         Accept: application/json, text/event-stream\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n",
        body.len()
    )
    .unwrap();
    // The server may answer and close before the whole body is sent: a write
    // error then is the refusal arriving early, not a failure of the test.
    let _ = s.write_all(body.as_bytes());
    let mut raw = Vec::new();
    let _ = s.read_to_end(&mut raw);
    let text = String::from_utf8_lossy(&raw).to_string();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    (status, body.to_string())
}

fn store_args(dir: &std::path::Path) -> Vec<String> {
    let store = dir.join(".reflow2").join("graph");
    vec![
        "--graph-path".into(),
        store.to_str().unwrap().into(),
        "--http".into(),
        "127.0.0.1:0".into(),
    ]
}

const MIB: usize = 1024 * 1024;

#[test]
fn unset_the_door_takes_four_mib_and_refuses_more_naming_the_flag() {
    let dir = tmp_dir("default");
    let args = store_args(&dir);
    let server = Server::start(&args.iter().map(String::as_str).collect::<Vec<_>>());

    let (status, body) = post_of_size(server.port, 3 * MIB);
    assert_ne!(status, 413, "3 MiB is under the default: {body:.300}");
    assert_ne!(status, 0, "the server answered");

    let (status, body) = post_of_size(server.port, 5 * MIB);
    assert_eq!(status, 413, "5 MiB is over the default: {body:.300}");
    assert!(body.contains("at most 4194304 bytes (4 MiB)"), "{body}");
    assert!(body.contains("--http-max-request-bytes"), "{body}");
    assert!(body.contains("REFLOW2_HTTP_MAX_REQUEST_BYTES"), "{body}");
    assert!(body.contains("nothing in it was read"), "{body}");
    assert!(
        server.logged("in one request (").is_empty(),
        "the default is not announced"
    );
}

#[test]
fn a_chosen_limit_reaches_the_transport_both_ways_and_is_announced() {
    let dir = tmp_dir("raised");
    let mut args = store_args(&dir);
    args.extend(["--http-max-request-bytes".into(), "8MiB".into()]);
    let server = Server::start(&args.iter().map(String::as_str).collect::<Vec<_>>());

    let (status, body) = post_of_size(server.port, 6 * MIB);
    assert_ne!(status, 413, "6 MiB is under the chosen 8 MiB: {body:.300}");
    assert_ne!(status, 0, "the server answered");
    let (status, body) = post_of_size(server.port, 9 * MIB);
    assert_eq!(status, 413, "9 MiB is over the chosen 8 MiB: {body:.300}");
    assert!(body.contains("at most 8388608 bytes (8 MiB)"), "{body}");
    assert_eq!(
        server.logged("8388608 bytes (8 MiB) in one request").len(),
        1,
        "the banner says the chosen limit once"
    );

    // The environment twin sets it the same way, and a LOWER limit holds too.
    let dir = tmp_dir("lowered");
    let args = store_args(&dir);
    let server = Server::start_with_env(
        &args.iter().map(String::as_str).collect::<Vec<_>>(),
        &[("REFLOW2_HTTP_MAX_REQUEST_BYTES", "128KiB")],
    );
    let (status, body) = post_of_size(server.port, 200 * 1024);
    assert_eq!(
        status, 413,
        "200 KiB is over the chosen 128 KiB: {body:.300}"
    );
    assert!(body.contains("at most 131072 bytes"), "{body}");
}

#[test]
fn a_limit_outside_the_bounds_stops_the_server_saying_them() {
    for bad in ["1024", "1GiB", "16MB"] {
        let out = Command::new(bin())
            .args(["--http", "127.0.0.1:0", "--http-max-request-bytes", bad])
            .stdin(Stdio::null())
            .output()
            .expect("run reflow2-mcp");
        assert!(!out.status.success(), "{bad} is refused");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains("--http-max-request-bytes"), "{bad}: {err}");
        assert!(
            err.contains("64KiB to 512MiB") || err.contains("KiB, MiB or GiB"),
            "{bad} says what is accepted: {err}"
        );
    }
}
