//! A server told to stop finishes what it has, writes what is waiting, closes
//! every store, and only then exits.
//!
//! ⭐ WHY THIS IS END TO END. `drain`'s own tests pin the order against a fake
//! service: a request in progress is answered in full, work that outlasts the
//! grace is cut off and said, an open stream does not hold the stop up. What
//! only the real binary under a real signal can show is what an OPERATOR meets:
//!   · SIGTERM is handled at all. Before this there was no handler, so the
//!     process died where it stood — and as a container's process 1 it ignored
//!     the signal until Docker's SIGKILL;
//!   · a change written a moment before the stop reaches the export, where it
//!     used to be lost inside the write-through's quiet period;
//!   · a server holding many designs closes every one and says so.
//!
//! `req:a-server-drains-before-it-stops`.

#![cfg(unix)]

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn tmp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "reflow2-drain-{tag}-{}-{}",
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

/// Mint a design by running the binary over stdio once; its graph_id.
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
    wait_for_exit(&mut child, Duration::from_secs(60)).expect("the minting run exits");
    let raw = std::fs::read_to_string(dir.join(".reflow2").join("graph.id.json"))
        .expect("an opened design has an identity sidecar");
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    v["graph_id"].as_str().unwrap().to_string()
}

fn wait_for_exit(child: &mut Child, d: Duration) -> Option<std::process::ExitStatus> {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(s)) => return Some(s),
            Ok(None) if start.elapsed() < d => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                return None;
            }
        }
    }
}

/// A server on an OS-assigned port, and every line it has written to stderr.
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
    fn start(args: &[&std::ffi::OsStr]) -> Server {
        let mut child = Command::new(bin())
            .args(args)
            .arg("--http")
            .arg("127.0.0.1:0")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn the server");
        // Keep draining stderr on a thread: a closed pipe would kill the server
        // (see two_designs_stay_apart_over_the_transport.rs), and the lines are
        // what the test reads the server's account of its stop from.
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

    fn terminate(&self) {
        let ok = Command::new("kill")
            .arg("-TERM")
            .arg(self.child.id().to_string())
            .status()
            .expect("run kill")
            .success();
        assert!(ok, "the server was running to be told to stop");
    }

    fn said(&self) -> String {
        self.log.lock().unwrap().join("\n")
    }
}

/// One HTTP POST; (status, session id, body).
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

#[test]
fn a_change_made_just_before_the_stop_reaches_the_export_and_the_store_closes_cleanly() {
    let dir = tmp_dir("one");
    mint(&dir);
    let store = dir.join(".reflow2").join("graph");
    let export = dir.join("design.json");
    let server = Server::start(&[
        "--graph-path".as_ref(),
        store.as_os_str(),
        "--export-to".as_ref(),
        export.as_os_str(),
    ]);

    let sid = open_session(server.port, "/");
    let wrote = call(
        server.port,
        "/",
        &sid,
        "add_requirement",
        r#"{"id":"req:written-just-before-the-stop","name":"Written just before the stop","statement":"The write-through had not run yet when the server was told to stop."}"#,
    );
    assert!(
        wrote.contains("req:written-just-before-the-stop"),
        "the write landed: {wrote}"
    );

    // Inside the write-through's two-second quiet period, on purpose: this is
    // the window in which a stop used to lose the change.
    let told = Instant::now();
    server.terminate();
    let mut server = server;
    let status = wait_for_exit(&mut server.child, Duration::from_secs(15))
        .unwrap_or_else(|| panic!("the server exits once drained: {}", server.said()));
    // The stderr thread may still be reading the last lines.
    std::thread::sleep(Duration::from_millis(200));
    let said = server.said();

    assert!(
        status.success(),
        "a drained stop is a clean exit: {status:?}\n{said}"
    );
    assert!(
        told.elapsed() < Duration::from_secs(8),
        "with nothing in progress, the stop does not wait out the grace: {:?}",
        told.elapsed()
    );
    assert!(said.contains("stopped (SIGTERM)"), "{said}");
    assert!(
        said.contains("every store released cleanly"),
        "the store is closed by its owner, not by the process dying: {said}"
    );
    let on_disk = std::fs::read_to_string(&export).expect("the export exists");
    assert!(
        on_disk.contains("req:written-just-before-the-stop"),
        "the change waiting to be written through reached the export before the exit: {said}"
    );

    // And the store opens again straight away, with the change in it.
    let out = Command::new(bin())
        .arg("--graph-path")
        .arg(&store)
        .arg("--export")
        .output()
        .expect("run --export");
    assert!(
        out.status.success(),
        "the store reopens: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("req:written-just-before-the-stop"));
}

#[test]
fn a_server_holding_many_designs_closes_every_one_when_told_to_stop() {
    let root = tmp_dir("many");
    let mut ids = Vec::new();
    for name in ["alpha", "beta"] {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        ids.push(mint(&dir));
    }
    let server = Server::start(&["--registry-root".as_ref(), root.as_os_str()]);
    for id in &ids {
        open_session(server.port, &format!("/g/{id}/"));
    }

    server.terminate();
    let mut server = server;
    let status = wait_for_exit(&mut server.child, Duration::from_secs(15))
        .unwrap_or_else(|| panic!("the server exits once drained: {}", server.said()));
    std::thread::sleep(Duration::from_millis(200));
    let said = server.said();
    assert!(status.success(), "{status:?}\n{said}");
    for id in &ids {
        assert!(
            said.contains(&format!("closed design {id} (the server is stopping)")),
            "every open design is closed by name: {said}"
        );
    }
    assert!(
        said.contains("2 design(s) closed, every store released cleanly"),
        "{said}"
    );
}
