//! The remote client measures the folder's own files for `link_artifact`,
//! `set_artifact_checksum(s)` and `reconcile_artifacts`, because the server
//! cannot see this disk.
//!
//! `epoch:planned-local-files-are-measured-for-a-remote-design`;
//! `req:a-local-client-works-against-a-remote-reflow2-server-by-url` point 3.
//!
//! ⭐ WHY THIS IS END TO END. `client_measure`'s own tests pin the parsing,
//! the injection and the reply rewriting against a stand-in. What only the real
//! binary shows is the whole chain: a design on a REGISTRY server — which holds
//! no tree, so the server itself cannot measure anything and refuses or
//! records what it is told — and a client started in a folder whose
//! `.reflow2.toml` names that design, with a real file beside it:
//!   · `link_artifact` without a checksum stores the sha256 of that file;
//!   · `set_artifact_checksum(s)` without checksums are measured and stored;
//!   · `reconcile_artifacts` with nothing observed reports the file's state;
//!   · a location outside the root is refused locally with the server's
//!     OutsideRoot wording and nothing is stored;
//!   · a bare `--remote` client with no root measures NOTHING and says so —
//!     even when started in the folder that holds the file;
//!   · a bare `--remote` client with `--tree-root` measures under it;
//!   · an item that already carries a checksum passes through untouched.
//!
//! The helpers mirror tests/a_code_folder_attaches_to_the_design_it_names.rs.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn tmp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "reflow2-client-measures-{tag}-{}-{}",
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
    PathBuf::from(env!("CARGO_BIN_EXE_reflow2-mcp"))
}

/// `sha256:<hex>` of a file's bytes, as the server's own dialect spells it.
fn sha256_of(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap();
    let digest = Sha256::digest(&bytes);
    let mut hex = String::from("sha256:");
    for b in digest.iter() {
        use std::fmt::Write as _;
        let _ = write!(hex, "{b:02x}");
    }
    hex
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
    let v: Value = serde_json::from_str(&raw).unwrap();
    v["graph_id"].as_str().unwrap().to_string()
}

/// A registry server holding the designs under `root`, and its port. A
/// registry holds DESIGNS, not checkouts: it serves every design WITHOUT a
/// tree, so nothing can be measured on its side.
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

/// A client driven over stdio one JSON-RPC line at a time.
struct Client {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
    next_id: i64,
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Client {
    /// Started in `folder` with the machine-wide MCP entry's flags; the
    /// folder's `.reflow2.toml` attaches it to the design it names.
    fn in_folder(folder: &Path) -> Client {
        Client::spawn(
            folder,
            &[
                "--graph-path",
                ".reflow2/graph",
                "--shared",
                "--only-if-present",
            ],
        )
    }

    /// A bare `--remote` client, with whatever else `extra` says.
    fn remote(cwd: &Path, url: &str, extra: &[&str]) -> Client {
        let mut args = vec!["--remote", url];
        args.extend_from_slice(extra);
        Client::spawn(cwd, &args)
    }

    fn spawn(cwd: &Path, args: &[&str]) -> Client {
        let mut child = Command::new(bin())
            .current_dir(cwd)
            .args(args)
            // No key and no keychain: this machine's own setup never leaks in.
            .env("REFLOW2_CONFIG_DIR", tmp_dir("config"))
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
        let mut c = Client {
            child,
            stdin,
            lines,
            next_id: 1,
        };
        c.handshake();
        c
    }

    fn send(&mut self, v: Value) {
        writeln!(self.stdin, "{v}").expect("write to the client");
        self.stdin.flush().unwrap();
    }

    /// The reply to request `id`.
    fn reply(&mut self, id: i64) -> Value {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let line = self
                .lines
                .recv_timeout(left)
                .unwrap_or_else(|_| panic!("no reply to request {id}"));
            let v: Value = serde_json::from_str(&line).unwrap_or_default();
            if v["id"] == id {
                return v;
            }
            // An internal call's reply must never reach this stream.
            assert!(
                !v["id"]
                    .as_str()
                    .is_some_and(|s| s.contains("client-measure")),
                "an internal call leaked into the agent's stream: {line}"
            );
        }
    }

    fn handshake(&mut self) -> Value {
        self.send(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}));
        let hello = self.reply(1);
        self.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        self.next_id = 2;
        hello
    }

    /// Call `tool`; the whole JSON-RPC reply.
    fn call(&mut self, tool: &str, args: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":tool,"arguments":args}}));
        self.reply(id)
    }

    /// Call `tool` expecting success; its payload.
    fn ok(&mut self, tool: &str, args: Value) -> Value {
        let reply = self.call(tool, args);
        assert!(
            reply.get("error").is_none(),
            "{tool} was refused: {}",
            reply["error"]
        );
        assert_ne!(
            reply["result"]["isError"], true,
            "{tool} failed: {}",
            reply["result"]
        );
        payload_of(&reply)
    }

    /// The stored Artifact node's properties.
    fn artifact(&mut self, id: &str) -> Value {
        self.ok("get_node", json!({ "id": id, "node_type": "Artifact" }))["node"]["properties"]
            .clone()
    }
}

fn payload_of(reply: &Value) -> Value {
    let r = &reply["result"];
    if r["structuredContent"].is_object() {
        return r["structuredContent"].clone();
    }
    r["content"][0]["text"]
        .as_str()
        .and_then(|t| serde_json::from_str(t).ok())
        .unwrap_or_else(|| panic!("no payload in {reply}"))
}

/// A design on a registry, and a code folder with a real file whose
/// `.reflow2.toml` names it. Returns (registry, folder, the design's address).
fn stage(tag: &str) -> (Registry, PathBuf, String) {
    let root = tmp_dir(&format!("{tag}-root"));
    let design_dir = root.join("the-design");
    std::fs::create_dir_all(&design_dir).unwrap();
    let named = mint(&design_dir);
    let registry = start_registry(&root);
    let address = format!("http://127.0.0.1:{}/g/{named}/", registry.port);

    let folder = tmp_dir(&format!("{tag}-folder"));
    std::fs::create_dir_all(folder.join("src")).unwrap();
    std::fs::write(folder.join("src/a.rs"), b"fn a() {}\n").unwrap();
    std::fs::write(
        folder.join(".reflow2.toml"),
        format!("[design]\nid = \"{named}\"\naddress = \"{address}\"\n"),
    )
    .unwrap();
    (registry, folder, address)
}

fn link(client: &mut Client, artifact_id: &str, location: &str) -> Value {
    client.ok(
        "link_artifact",
        json!({
            "artifact_id": artifact_id, "name": artifact_id, "location": location,
            "target_id": "comp:x", "description": "a file"
        }),
    )
}

#[test]
fn link_artifact_without_a_checksum_stores_the_sha256_of_the_folders_file() {
    let (_registry, folder, _) = stage("link");
    let mut client = Client::in_folder(&folder);
    client.ok(
        "add_component",
        json!({ "id": "comp:x", "name": "x", "description": "holds the files under test" }),
    );

    let want = sha256_of(&folder.join("src/a.rs"));
    let out = link(&mut client, "art:a", "src/a.rs");
    let m = &out["measurement"];
    assert_eq!(m["measured_by"], "client", "{m}");
    assert_eq!(m["basis"], "asserted", "{m}");
    assert_eq!(m["checksum"], want, "{m}");
    assert_eq!(
        m["root"],
        std::fs::canonicalize(&folder)
            .unwrap()
            .display()
            .to_string(),
        "{m}"
    );
    assert!(
        m["note"]
            .as_str()
            .unwrap_or_default()
            .contains("records the value as asserted"),
        "the agent is told the truth: {m}"
    );
    assert_eq!(
        m["server"]["basis"], "asserted",
        "the server's own block rides beside: {m}"
    );

    let stored = client.artifact("art:a");
    assert_eq!(stored["checksum"], want, "{stored}");
    assert_eq!(stored["checksum_basis"], "asserted", "{stored}");
}

#[test]
fn set_artifact_checksums_without_checksums_are_measured_and_stored() {
    let (_registry, folder, _) = stage("accept");
    let mut client = Client::in_folder(&folder);
    client.ok(
        "add_component",
        json!({ "id": "comp:x", "name": "x", "description": "holds the files under test" }),
    );
    std::fs::write(folder.join("src/b.rs"), b"fn b() {}\n").unwrap();
    // Registered with a checksum the agent asserted, as before this change.
    let stale = format!("sha256:{}", "0".repeat(64));
    client.ok(
        "link_artifact",
        json!({ "artifact_id": "art:a", "name": "a", "location": "src/a.rs",
                "target_id": "comp:x", "description": "a", "checksum": stale }),
    );
    link(&mut client, "art:b", "src/b.rs");
    std::fs::write(folder.join("src/b.rs"), b"fn b() { 2 }\n").unwrap();

    // The bulk form: one item measured, nothing typed.
    let out = client.ok(
        "set_artifact_checksums",
        json!({ "accepts": [
            { "artifact_id": "art:a", "disposition": "design_holds", "change_type": "resync" },
            { "artifact_id": "art:b", "disposition": "design_holds", "change_type": "refactor" },
        ] }),
    );
    let ms = out["measurements"].as_array().expect("measurements");
    assert_eq!(ms.len(), 2, "{out}");
    for m in ms {
        assert_eq!(m["measurement"]["measured_by"], "client", "{m}");
        assert_eq!(m["measurement"]["basis"], "asserted", "{m}");
    }
    assert_eq!(
        client.artifact("art:a")["checksum"],
        sha256_of(&folder.join("src/a.rs"))
    );
    assert_eq!(
        client.artifact("art:b")["checksum"],
        sha256_of(&folder.join("src/b.rs"))
    );

    // The single form, a first baseline.
    std::fs::write(folder.join("src/c.rs"), b"fn c() {}\n").unwrap();
    client.ok(
        "add_artifact",
        json!({ "id": "art:c", "name": "c", "location": "src/c.rs", "description": "c" }),
    );
    let out = client.ok(
        "set_artifact_checksum",
        json!({ "artifact_id": "art:c", "disposition": "baseline_established" }),
    );
    assert_eq!(out["measurement"]["measured_by"], "client", "{out}");
    assert_eq!(
        out["measurement"]["checksum"],
        sha256_of(&folder.join("src/c.rs"))
    );
    assert_eq!(
        client.artifact("art:c")["checksum"],
        sha256_of(&folder.join("src/c.rs"))
    );
}

#[test]
fn reconcile_artifacts_with_nothing_observed_reports_the_files_state() {
    let (_registry, folder, _) = stage("reconcile");
    let mut client = Client::in_folder(&folder);
    client.ok(
        "add_component",
        json!({ "id": "comp:x", "name": "x", "description": "holds the files under test" }),
    );
    link(&mut client, "art:a", "src/a.rs");
    std::fs::write(folder.join("src/gone.rs"), b"x").unwrap();
    link(&mut client, "art:gone", "src/gone.rs");
    link(&mut client, "art:web", "https://example.invalid/spec.yaml");

    // Then the code moves: one file changes, one vanishes.
    std::fs::write(folder.join("src/a.rs"), b"fn a() { 1 }\n").unwrap();
    std::fs::remove_file(folder.join("src/gone.rs")).unwrap();

    let out = client.ok("reconcile_artifacts", json!({}));
    let m = &out["measurement"];
    assert_eq!(m["measured_by"], "client", "{m}");
    assert_eq!(m["basis"], "asserted", "{m}");
    assert_eq!(m["measured"], 1, "{m}");
    assert_eq!(m["absent"], 1, "{m}");
    assert_eq!(m["unmeasurable"][0]["artifact_id"], "art:web", "{m}");
    assert_eq!(m["unmeasurable"][0]["not_measured"]["why"], "remote", "{m}");
    let text = out.to_string();
    assert!(
        text.contains("art:a") && text.contains("checksum_change"),
        "the changed file is reported: {text}"
    );
    assert!(
        text.contains("art:gone"),
        "the vanished file is reported: {text}"
    );
}

#[test]
fn a_location_outside_the_root_is_refused_locally_and_nothing_is_stored() {
    let (_registry, folder, _) = stage("outside");
    let outside = tmp_dir("outside-file");
    std::fs::write(outside.join("secret.txt"), b"not yours to hash").unwrap();
    let escape = outside.join("secret.txt").display().to_string();
    let mut client = Client::in_folder(&folder);
    client.ok(
        "add_component",
        json!({ "id": "comp:x", "name": "x", "description": "holds the files under test" }),
    );

    // Registering it is allowed, as on a server holding the tree: no baseline,
    // and the reply names why.
    let out = link(&mut client, "art:out", &escape);
    assert_eq!(out["measurement"]["basis"], "none", "{out}");
    assert_eq!(
        out["measurement"]["not_measured"]["why"], "outside_root",
        "{out}"
    );
    assert!(
        client.artifact("art:out").get("checksum").is_none(),
        "no checksum was stored"
    );

    // Moving a baseline onto it is refused HERE, with the server's words.
    let reply = client.call(
        "set_artifact_checksum",
        json!({ "artifact_id": "art:out", "disposition": "baseline_established" }),
    );
    let msg = reply["error"]["message"]
        .as_str()
        .unwrap_or_else(|| panic!("a JSON-RPC refusal: {reply}"));
    assert!(
        msg.contains("resolves outside the project root and is refused unread"),
        "{msg}"
    );
    assert!(msg.contains("pass `checksum` to assert one"), "{msg}");
    assert!(
        client.artifact("art:out").get("checksum").is_none(),
        "nothing was stored"
    );
    // And the bulk form names the item.
    let reply = client.call(
        "set_artifact_checksums",
        json!({ "accepts": [
            { "artifact_id": "art:out", "disposition": "baseline_established" },
        ] }),
    );
    let msg = reply["error"]["message"].as_str().unwrap_or_default();
    assert!(msg.starts_with("art:out: "), "{msg}");
    assert!(msg.contains("outside the project root"), "{msg}");
}

#[test]
fn a_client_with_no_root_measures_nothing_and_says_so() {
    let (_registry, folder, address) = stage("no-root");
    // Started IN the folder that holds the file, but as a bare --remote client
    // with no --tree-root: the current directory is never guessed.
    let mut client = Client::remote(&folder, &address, &[]);
    client.ok(
        "add_component",
        json!({ "id": "comp:x", "name": "x", "description": "holds the files under test" }),
    );

    let out = link(&mut client, "art:a", "src/a.rs");
    let m = &out["measurement"];
    assert_eq!(m["basis"], "none", "the server recorded no baseline: {m}");
    assert_eq!(m["client"]["measured"], false, "{m}");
    assert_eq!(m["client"]["why"], "no_root", "{m}");
    assert!(
        m["client"]["note"]
            .as_str()
            .unwrap_or_default()
            .contains("--tree-root"),
        "the remedy is named: {m}"
    );
    assert!(
        client.artifact("art:a").get("checksum").is_none(),
        "nothing was measured from the current directory"
    );

    let reply = client.call(
        "set_artifact_checksum",
        json!({ "artifact_id": "art:a", "disposition": "baseline_established" }),
    );
    let msg = reply["error"]["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("does not hold the project tree"),
        "the server's refusal stands: {msg}"
    );
    assert!(
        msg.contains("Nothing was measured on this machine either"),
        "and the client says it measured nothing: {msg}"
    );

    let reply = client.call("reconcile_artifacts", json!({}));
    let msg = reply["error"]["message"].as_str().unwrap_or_default();
    assert!(msg.contains("nothing to reconcile"), "{msg}");
    assert!(
        msg.contains("Nothing was measured on this machine either"),
        "{msg}"
    );
}

#[test]
fn a_bare_remote_client_measures_under_tree_root() {
    let (_registry, folder, address) = stage("tree-root");
    let elsewhere = tmp_dir("elsewhere");
    let mut client = Client::remote(
        &elsewhere,
        &address,
        &["--tree-root", folder.to_str().unwrap()],
    );
    client.ok(
        "add_component",
        json!({ "id": "comp:x", "name": "x", "description": "holds the files under test" }),
    );
    let want = sha256_of(&folder.join("src/a.rs"));
    let out = link(&mut client, "art:a", "src/a.rs");
    assert_eq!(out["measurement"]["measured_by"], "client", "{out}");
    assert_eq!(out["measurement"]["checksum"], want, "{out}");
    assert_eq!(client.artifact("art:a")["checksum"], want);
}

#[test]
fn an_item_that_carries_a_checksum_passes_through_untouched() {
    let (_registry, folder, _) = stage("given");
    let mut client = Client::in_folder(&folder);
    client.ok(
        "add_component",
        json!({ "id": "comp:x", "name": "x", "description": "holds the files under test" }),
    );
    let given = format!("sha256:{}", "a".repeat(64));
    let out = client.ok(
        "link_artifact",
        json!({ "artifact_id": "art:a", "name": "a", "location": "src/a.rs",
                "target_id": "comp:x", "description": "a", "checksum": given }),
    );
    assert!(
        out["measurement"].get("measured_by").is_none(),
        "the server's block is as the server sent it: {out}"
    );
    assert_eq!(client.artifact("art:a")["checksum"], given);
    assert_eq!(client.artifact("art:a")["checksum_basis"], "asserted");
}
