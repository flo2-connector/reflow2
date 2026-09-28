//! A code folder's `.reflow2.toml` names the design it implements, and a client
//! started there attaches to THAT design — never to a store left in the folder.
//!
//! ⭐ WHY THIS IS END TO END. `pointer`'s own tests pin the file and the words.
//! What only the real binary shows is what an AGENT started in the folder meets,
//! with the same flags the machine-wide MCP entry passes
//! (`--graph-path .reflow2/graph --shared --only-if-present`):
//!   · a folder holding BOTH a leftover local store and a pointer is attached to
//!     the named design, the handshake says so, and nothing is opened or spawned
//!     for the local store (the defect in
//!     fact:an-agent-opened-in-a-moved-designs-folder-is-served-the-frozen-store-2026-09-27);
//!   · a pointer naming a design the server does not hold attaches NOTHING, and
//!     the session is told why rather than handed the wrong design;
//!   · a pointer that cannot be used is refused by name, never skipped in favour
//!     of the local store.
//!
//! `req:a-code-folder-names-the-design-it-implements-and-its-server`.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

fn tmp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "reflow2-pointer-{tag}-{}-{}",
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
    let start = Instant::now();
    while child.try_wait().ok().flatten().is_none() {
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "minting run did not exit"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let raw = std::fs::read_to_string(dir.join(".reflow2").join("graph.id.json"))
        .expect("an opened design has an identity sidecar");
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    v["graph_id"].as_str().unwrap().to_string()
}

/// A registry server holding the designs under `root`, and its port.
struct Registry {
    child: Child,
    port: u16,
}

impl Drop for Registry {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn start_registry(root: &Path) -> Registry {
    let mut child = Command::new(bin())
        .arg("--registry-root")
        .arg(root)
        .arg("--http")
        .arg("127.0.0.1:0")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the registry server");
    let stderr = child.stderr.take().expect("stderr piped");
    let (tx, rx) = channel::<u16>();
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
        }
    });
    let port = rx
        .recv_timeout(Duration::from_secs(90))
        .expect("the registry prints its address within 90s");
    Registry { child, port }
}

/// A client started in `folder` with the machine-wide entry's flags, driven
/// over stdio one JSON-RPC line at a time.
struct Client {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Client {
    fn start(folder: &Path, config: &Path) -> Client {
        let mut child = Command::new(bin())
            .current_dir(folder)
            .args([
                "--graph-path",
                ".reflow2/graph",
                "--shared",
                "--only-if-present",
            ])
            // No key and no keychain: this machine's own setup never leaks in.
            .env("REFLOW2_CONFIG_DIR", config)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn the client");
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, lines) = channel::<String>();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let _ = tx.send(line);
            }
        });
        Client {
            child,
            stdin,
            lines,
        }
    }

    fn send(&mut self, v: serde_json::Value) {
        writeln!(self.stdin, "{v}").expect("write to the client");
        self.stdin.flush().unwrap();
    }

    /// The reply to request `id`.
    fn reply(&mut self, id: i64) -> serde_json::Value {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let line = self
                .lines
                .recv_timeout(left)
                .unwrap_or_else(|_| panic!("no reply to request {id}"));
            let v: serde_json::Value = serde_json::from_str(&line).unwrap_or_default();
            if v["id"] == id {
                return v;
            }
        }
    }

    fn handshake(&mut self) -> serde_json::Value {
        self.send(serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}));
        let hello = self.reply(1);
        self.send(serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        hello
    }

    fn call(&mut self, id: i64, tool: &str) -> serde_json::Value {
        self.send(serde_json::json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":tool,"arguments":{}}}));
        self.reply(id)
    }

    fn tools(&mut self, id: i64) -> Vec<String> {
        self.send(serde_json::json!({"jsonrpc":"2.0","id":id,"method":"tools/list","params":{}}));
        self.reply(id)["result"]["tools"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|t| t["name"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn graph_id_of(reply: &serde_json::Value) -> String {
    let r = &reply["result"];
    r["structuredContent"]["graph_id"]
        .as_str()
        .map(str::to_string)
        .or_else(|| {
            let text = r["content"][0]["text"].as_str()?;
            let v: serde_json::Value = serde_json::from_str(text).ok()?;
            v["graph_id"].as_str().map(str::to_string)
        })
        .unwrap_or_else(|| panic!("no graph_id in {reply}"))
}

fn write_pointer(folder: &Path, id: &str, address: &str) {
    std::fs::write(
        folder.join(".reflow2.toml"),
        format!("[design]\nid = \"{id}\"\naddress = \"{address}\"\n"),
    )
    .unwrap();
}

#[test]
fn a_folder_with_a_pointer_and_a_leftover_store_is_attached_to_the_named_design() {
    let root = tmp_dir("root");
    let named = mint(&{
        let d = root.join("the-design");
        std::fs::create_dir_all(&d).unwrap();
        d
    });
    let registry = start_registry(&root);

    // The code folder: a store left behind by a move, and the pointer.
    let folder = tmp_dir("folder");
    let leftover = mint(&folder);
    assert_ne!(leftover, named);
    let address = format!("http://127.0.0.1:{}/g/{named}/", registry.port);
    write_pointer(&folder, &named, &address);

    let mut client = Client::start(&folder, &tmp_dir("config"));
    let hello = client.handshake();
    let said = hello["result"]["instructions"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(
        said.contains("THIS FOLDER NAMES ITS DESIGN") && said.contains(&named),
        "{said}"
    );
    assert!(
        said.contains("was NOT opened"),
        "the leftover store is named as left alone: {said}"
    );

    let identity = client.call(2, "design_identity");
    assert_eq!(
        graph_id_of(&identity),
        named,
        "the session works the named design"
    );
    assert!(
        !folder.join(".reflow2").join("graph.server.json").exists(),
        "nothing was spawned for the leftover local store"
    );
}

#[test]
fn a_pointer_naming_a_design_the_server_does_not_hold_attaches_nothing_and_says_why() {
    let root = tmp_dir("root-wrong");
    let held = mint(&{
        let d = root.join("held");
        std::fs::create_dir_all(&d).unwrap();
        d
    });
    let registry = start_registry(&root);
    let folder = tmp_dir("folder-wrong");
    write_pointer(
        &folder,
        "not0the0design0",
        &format!("http://127.0.0.1:{}/g/{held}/", registry.port),
    );

    let mut client = Client::start(&folder, &tmp_dir("config-wrong"));
    let said = client.handshake()["result"]["instructions"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(said.contains(&format!("holds design {held}")), "{said}");
    assert_eq!(
        client.tools(2),
        vec!["reflow2_unavailable".to_string()],
        "attached to nothing"
    );
}

#[test]
fn a_pointer_that_cannot_be_used_is_refused_by_name_and_the_local_store_is_not_opened() {
    let folder = tmp_dir("folder-bad");
    mint(&folder);
    std::fs::write(folder.join(".reflow2.toml"), "[design]\nid = \"abc\"\n").unwrap();

    let mut client = Client::start(&folder, &tmp_dir("config-bad"));
    let said = client.handshake()["result"]["instructions"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(
        said.contains(".reflow2.toml") && said.contains("address"),
        "{said}"
    );
    assert_eq!(client.tools(2), vec!["reflow2_unavailable".to_string()]);
    assert!(!folder.join(".reflow2").join("graph.server.json").exists());
}
