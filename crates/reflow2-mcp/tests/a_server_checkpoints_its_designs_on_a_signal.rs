//! A registry server copies every design it serves on `SIGUSR1`, and keeps
//! serving (`cap:a-server-checkpoints-every-design-it-serves-on-a-signal-without-stopping`),
//! through the real binary.
//!
//! FOUND 2026-11-23: flo2.io's nightly backup stopped the engine for about two
//! and a half minutes (04:14 to 04:17 UTC on 2026-11-22 and 11-23) to get a
//! consistent copy of the designs. Anthony chose that day how the backup asks
//! the server for one instead: a signal. These pin that the signal makes one
//! whole set while the server answers, and that a server not set up for it
//! survives the signal, which until now ended the process.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_reflow2-mcp"))
}

fn tmp_root() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "reflow2-checkpoint-signal-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Mint a design by running the binary over stdio once.
fn mint(dir: &Path) {
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
        child.stdin.as_mut().unwrap(),
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2025-06-18","capabilities":{{}},"clientInfo":{{"name":"mint","version":"1"}}}}}}"#
    )
    .unwrap();
    drop(child.stdin.take());
    let start = Instant::now();
    while child.try_wait().unwrap().is_none() {
        assert!(start.elapsed() < Duration::from_secs(60), "minting hung");
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(dir.join(".reflow2").join("graph.id.json").is_file());
}

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

    fn signal(&self) {
        let ok = Command::new("kill")
            .arg("-USR1")
            .arg(self.child.id().to_string())
            .status()
            .expect("run kill")
            .success();
        assert!(ok, "the signal was sent");
    }

    fn alive(&mut self) -> bool {
        self.child.try_wait().unwrap().is_none()
    }

    fn healthz(&self) -> u16 {
        let mut s = std::net::TcpStream::connect(("127.0.0.1", self.port)).expect("connect");
        s.set_read_timeout(Some(Duration::from_secs(30))).ok();
        s.write_all(b"GET /healthz HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
            .unwrap();
        let mut raw = String::new();
        let _ = s.read_to_string(&mut raw);
        raw.split_whitespace()
            .nth(1)
            .and_then(|c| c.parse().ok())
            .unwrap_or(0)
    }
}

fn start_registry(root: &Path, extra: &[&str]) -> Server {
    let mut child = Command::new(bin())
        .arg("--registry-root")
        .arg(root)
        .arg("--http")
        .arg("127.0.0.1:0")
        .args(extra)
        .env("RUST_LOG", "info")
        .env_remove("REFLOW2_CHECKPOINT_DIR")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the registry server");
    // Keep draining stderr on a thread: a closed pipe kills the server.
    let stderr = child.stderr.take().unwrap();
    let log = Arc::new(Mutex::new(Vec::new()));
    let (tx, rx) = std::sync::mpsc::channel::<u16>();
    let kept = Arc::clone(&log);
    std::thread::spawn(move || {
        let mut sent = false;
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if !sent && let Some(i) = line.find("http://127.0.0.1:") {
                let digits: String = line[i + "http://127.0.0.1:".len()..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
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

/// The whole sets under `dir`: folders with no `.partial` suffix.
fn whole_sets(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && !p.to_string_lossy().ends_with(".partial"))
        .collect()
}

#[test]
fn a_signal_copies_every_design_into_one_whole_set_and_the_server_keeps_serving() {
    let root = tmp_root();
    mint(&root.join("design-a"));
    mint(&root.join("design-b"));
    let dir = root.join(".checkpoints");
    let mut server = start_registry(&root, &["--checkpoint-dir", dir.to_str().unwrap()]);
    let start = Instant::now();
    while !server.logged("SIGUSR1 takes a checkpoint") {
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "the handler is installed"
        );
        std::thread::sleep(Duration::from_millis(50));
    }

    server.signal();
    let start = Instant::now();
    let set = loop {
        if let Some(set) = whole_sets(&dir).into_iter().next() {
            break set;
        }
        assert!(
            start.elapsed() < Duration::from_secs(120),
            "a whole set appears; the log says: {:?}",
            server.log.lock().unwrap()
        );
        std::thread::sleep(Duration::from_millis(100));
    };

    let result: serde_json::Value =
        serde_json::from_slice(&std::fs::read(set.join("result.json")).unwrap()).unwrap();
    assert_eq!(result["copied"], 2, "{result}");
    assert_eq!(result["failed"], 0, "{result}");
    for folder in ["design-a", "design-b"] {
        assert!(
            set.join(folder).join("graph").join("CURRENT").is_file(),
            "{folder} copied"
        );
        assert!(
            set.join(folder).join("graph.id.json").is_file(),
            "{folder}'s identity too"
        );
    }
    assert!(server.alive(), "the server did not stop to be copied");
    assert_eq!(server.healthz(), 200, "and it still answers");
}

#[test]
fn a_server_with_nowhere_to_put_a_copy_survives_the_signal_and_says_why() {
    let root = tmp_root();
    mint(&root.join("design-a"));
    let mut server = start_registry(&root, &[]);
    let start = Instant::now();
    while !server.logged("SIGUSR1 is logged and does nothing") {
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "the handler is installed"
        );
        std::thread::sleep(Duration::from_millis(50));
    }

    server.signal();
    let start = Instant::now();
    while !server.logged("nothing was copied") {
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "the signal is logged"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(server.alive(), "SIGUSR1 no longer ends the process");
    assert_eq!(server.healthz(), 200);
}
