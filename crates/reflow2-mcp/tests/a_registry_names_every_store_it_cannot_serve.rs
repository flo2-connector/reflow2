//! A registry server names every store it found and cannot serve — at startup,
//! in its listing, and in its refusal for an id it does not hold — and never
//! says "no designs" while one sits under its root unread.
//!
//! ⭐ WHY THIS EXISTS. A single-design server refuses a store whose identity
//! file is missing, loudly, naming the file and the usual cause (a volume
//! mounted at the store instead of its parent) — since 2026-08-07. A
//! `--registry-root` server met the SAME store, classified it the same way
//! (`describe_at` says Unnamed), and threw the classification away: startup
//! said "no designs found", the listing said the root "holds no designs", and
//! the design read as silently empty (GitHub issue #616;
//! `fact:root-cause-the-registry-drops-a-store-without-identity-and-says-no-designs-2026-09-28`).
//!
//! PINNED AS A CLASS — discovery never silently discards what it classified —
//! over the three things `describe_at` can find that must not be served:
//!   · a store with data and NO identity file (the volume-mounted-at-the-store case);
//!   · a store whose identity file is there but cannot be read;
//!   · an identity file whose store is GONE, which the registry used to open —
//!     creating an empty store under the old id and serving the design as empty.
//! And over the three places a person or agent looks: the startup log, the
//! listing on a bare path, and the refusal for an unknown `/g/<id>/`.
//!
//! What it must NOT do is open, mint or repair an identity for any of them:
//! minting would split a design's name from its history, which is the reason
//! the single-design guard refuses.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn tmp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "reflow2-unserved-{tag}-{}-{}",
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

/// A real design with content under `root/name`, the way a host makes one:
/// the store opened once, a node written, and let go. Returns its graph_id.
fn design(root: &Path, name: &str) -> String {
    let store = root.join(name).join(".reflow2").join("graph");
    std::fs::create_dir_all(store.parent().unwrap()).unwrap();
    let mut g = reflow2_core::DesignGraph::open_rocksdb(store.to_str().unwrap())
        .expect("open a real store");
    g.add_project(&format!("proj:{name}"), name).unwrap();
    let id = g.graph_id().to_string();
    drop(g);
    id
}

/// The volume-mounted-at-the-store case: the data is there, every sidecar
/// beside it is gone.
fn lose_identity(root: &Path, name: &str) {
    let dot = root.join(name).join(".reflow2");
    for f in ["graph.id.json", "graph.meta.json", "graph.sync.json"] {
        let _ = std::fs::remove_file(dot.join(f));
    }
    assert!(dot.join("graph").exists(), "the store itself stays");
}

/// An identity file that is there and cannot be read.
fn break_identity(root: &Path, name: &str) {
    std::fs::write(
        root.join(name).join(".reflow2").join("graph.id.json"),
        "{ this is not json",
    )
    .unwrap();
}

/// The identity survives; the store it names is gone.
fn lose_store(root: &Path, name: &str) {
    let dot = root.join(name).join(".reflow2");
    std::fs::remove_dir_all(dot.join("graph")).unwrap();
    assert!(dot.join("graph.id.json").exists(), "the identity stays");
}

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
    fn registry(root: &Path) -> Server {
        let mut child = Command::new(bin())
            .arg("--registry-root")
            .arg(root)
            .args(["--http", "127.0.0.1:0"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn the registry server");
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
                "the registry printed no address within 90s: {:?}",
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

    /// Everything the server has said on stderr so far, as one string.
    fn said(&self) -> String {
        self.stderr.lock().unwrap().join("\n")
    }
}

/// One raw HTTP/1.1 request on loopback; (status, body).
fn request(port: u16, method: &str, path: &str) -> (u16, String) {
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"unserved-test","version":"1"}}}"#;
    let payload = if method == "POST" { init } else { "" };
    let mut s = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    s.set_read_timeout(Some(Duration::from_secs(60))).unwrap();
    write!(
        s,
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\n\
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
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    (status, body.to_string())
}

#[test]
fn a_store_that_lost_its_identity_is_named_at_startup_in_the_listing_and_in_a_refusal() {
    let root = tmp_dir("lost");
    let alpha = design(&root, "alpha");
    design(&root, "mounted-wrong");
    lose_identity(&root, "mounted-wrong");

    let server = Server::registry(&root);
    let said = server.said();
    assert!(
        said.contains("mounted-wrong")
            && said.contains("graph.id.json")
            && said.contains("mount the parent"),
        "startup names the store it cannot identify, the missing file and the remedy:\n{said}"
    );

    let (status, listing) = request(server.port, "GET", "/");
    assert_eq!(
        status, 404,
        "a bare path still asks for /g/<id>/: {listing}"
    );
    assert!(
        listing.contains(&alpha),
        "the design it serves is listed: {listing}"
    );
    assert!(
        listing.contains("mounted-wrong") && listing.contains("graph.id.json"),
        "the listing names the store it found and cannot serve, and why: {listing}"
    );

    let (status, refusal) = request(server.port, "POST", "/g/0000000000000000/mcp");
    assert_eq!(status, 404, "{refusal}");
    assert!(
        refusal.contains("found 1 store") && refusal.contains("cannot serve"),
        "an unknown id's refusal says a store was found and not served, so the design a \
         caller expects is not simply 'not here': {refusal}"
    );

    let (status, hello) = request(server.port, "POST", &format!("/g/{alpha}/mcp"));
    assert_eq!(status, 200, "the good design beside it is served: {hello}");

    assert!(
        !root
            .join("mounted-wrong")
            .join(".reflow2")
            .join("graph.id.json")
            .exists(),
        "nothing minted an identity for the store it could not identify"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_root_whose_only_store_lost_its_identity_never_says_it_holds_no_designs() {
    let root = tmp_dir("only-lost");
    design(&root, "mounted-wrong");
    lose_identity(&root, "mounted-wrong");

    let server = Server::registry(&root);
    let said = server.said();
    assert!(
        !said.contains("no designs found"),
        "startup must not say 'no designs' while a store sits here unread:\n{said}"
    );
    assert!(said.contains("mounted-wrong"), "{said}");

    let (_, listing) = request(server.port, "GET", "/");
    assert!(
        !listing.contains("holds no designs"),
        "the listing must not say the root holds no designs: {listing}"
    );
    assert!(listing.contains("mounted-wrong"), "{listing}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_unreadable_identity_is_named_rather_than_skipped() {
    let root = tmp_dir("broken");
    design(&root, "alpha");
    design(&root, "garbled");
    break_identity(&root, "garbled");

    let server = Server::registry(&root);
    let said = server.said();
    assert!(
        said.contains("garbled") && said.contains("cannot be read"),
        "startup names the store whose identity file cannot be read:\n{said}"
    );
    let (_, listing) = request(server.port, "GET", "/");
    assert!(
        listing.contains("garbled") && listing.contains("cannot be read"),
        "{listing}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_identity_whose_store_is_gone_is_not_served_as_an_empty_design() {
    let root = tmp_dir("ghost");
    design(&root, "alpha");
    let ghost = design(&root, "ghost");
    lose_store(&root, "ghost");

    let server = Server::registry(&root);
    let said = server.said();
    assert!(
        said.contains("ghost") && said.contains("store"),
        "startup names the identity whose store is gone:\n{said}"
    );

    let (status, reply) = request(server.port, "POST", &format!("/g/{ghost}/mcp"));
    assert_ne!(
        status, 200,
        "an identity with no store must NOT be opened: opening it creates an empty store \
         under the old id and serves the design as empty: {reply}"
    );
    assert!(
        reply.contains("store") && reply.contains("not here"),
        "the refusal says the store is not here: {reply}"
    );
    assert!(
        !root.join("ghost").join(".reflow2").join("graph").exists(),
        "no empty store was created under the old id"
    );

    let (_, listing) = request(server.port, "GET", "/");
    assert!(
        listing.contains("ghost"),
        "the listing names it among the stores found and not served: {listing}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
