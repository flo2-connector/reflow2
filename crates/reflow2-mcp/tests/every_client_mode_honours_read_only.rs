//! `--read-only` is honoured by every CLIENT mode — a process that forwards an
//! agent's calls to a server rather than opening a store — through the real
//! binary, against real servers.
//!
//! WHAT FAILED, measured on main c4e1cbd (2026-10-03), each pinned below:
//! · `--remote URL --read-only`: `add_requirement` was forwarded and the write
//!   landed in the server's store, exit 0;
//! · `--shared --read-only`: the same, through the shared daemon;
//! · a client attached through a folder's `.reflow2.toml`, with the
//!   machine-wide entry's flags plus `--read-only`: the same, on the design the
//!   pointer names (inferred in the finding, measured here);
//!   (fact:read-only-is-silently-ignored-by-the-remote-and-shared-clients-2026-10-02)
//! · the latent surface (`--only-if-present`, no design started):
//!   `reflow2_start_design` created `.reflow2/` and the promoted surface wrote;
//! · `--shared --read-only` in an opted-in folder with no store spawned a
//!   daemon, which created the store;
//! · a `--shared` call whose ARGUMENTS held the string "initialize" was taken
//!   for a handshake: no reply, and every later call failed (422).
//!
//! THE CLASS: a client forwards whatever it is given, so a flag that governs
//! what the SERVER may do was never consulted by the process the agent talks
//! to. Every forwarding client now screens each line before it leaves
//! (`reflow2_mcp::read_only_client`): a call leaves only when this build's own
//! surface, the server's own `tools/list` and the file-write rule all say it
//! only reads, and a call it cannot classify is refused as a write.
//!
//! Each test here was run against main before the fix and failed there.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const REQ_ID: &str = "req:read-only-client-probe";
/// The marker every client-side refusal carries.
const REFUSED_HERE: &str = "REFUSED BY THIS CLIENT";

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

fn write_args() -> serde_json::Value {
    serde_json::json!({
        "id": REQ_ID,
        "name": "Read-only client probe",
        "statement": "written through a client that was started with --read-only"
    })
}

/// A scratch directory, removed when dropped.
fn scratch(tag: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("reflow2-ro-client-{tag}-"))
        .tempdir()
        .unwrap()
}

/// The environment every process here runs in: its own HOME and settings, and
/// none of this machine's reflow2 configuration.
fn isolate(cmd: &mut Command, home: &Path) {
    std::fs::create_dir_all(home).unwrap();
    cmd.env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("REFLOW2_CONFIG_DIR", home.join("reflow2-config"))
        .env("RUST_LOG", "error")
        .env_remove("REFLOW2_CONTENT_POLICY")
        .env_remove("REFLOW2_TRUSTED_GATEWAY")
        .env_remove("REFLOW2_OIDC_ISSUER")
        .env_remove("REFLOW2_CONTRIBUTOR_ID")
        .env_remove("REFLOW2_CONTRIBUTOR_MAP");
}

/// A one-shot run, with a deadline.
fn one_shot(cwd: &Path, home: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(bin());
    cmd.current_dir(cwd).args(args);
    isolate(&mut cmd, home);
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary runs");
    let mut out = child.stdout.take().unwrap();
    let mut err = child.stderr.take().unwrap();
    let ot = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = out.read_to_end(&mut b);
        b
    });
    let et = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = err.read_to_end(&mut b);
        b
    });
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s;
        }
        if start.elapsed() > Duration::from_secs(120) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("`reflow2-mcp {}` did not exit", args.join(" "));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    std::process::Output {
        status,
        stdout: ot.join().unwrap(),
        stderr: et.join().unwrap(),
    }
}

/// Seed a design at `graph` with one project in it.
fn seed(cwd: &Path, home: &Path, graph: &str) {
    let o = one_shot(
        cwd,
        home,
        &[
            "--graph-path",
            graph,
            "--call",
            "add_project",
            "--args",
            r#"{"id":"proj:seed","name":"Seed"}"#,
        ],
    );
    assert_eq!(
        o.status.code(),
        Some(0),
        "seed: {}",
        String::from_utf8_lossy(&o.stderr)
    );
}

/// Whether the probe requirement is in the store at `graph`, read with the
/// one-shot door after every server has stopped.
fn probe_landed(cwd: &Path, home: &Path, graph: &str) -> bool {
    let o = one_shot(
        cwd,
        home,
        &[
            "--graph-path",
            graph,
            "--call",
            "get_node",
            "--args",
            &serde_json::json!({ "id": REQ_ID }).to_string(),
        ],
    );
    assert_eq!(
        o.status.code(),
        Some(0),
        "reading the store back failed: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap_or_else(|e| {
        panic!(
            "get_node printed no JSON ({e}): {}",
            String::from_utf8_lossy(&o.stdout)
        )
    });
    !v["node"].is_null()
}

/// Stops the shared server for `.reflow2/graph` in `folder` when dropped, so a
/// test that fails half-way never leaves a daemon holding the store (it would
/// idle for two hours).
struct StopsShared {
    folder: PathBuf,
    home: PathBuf,
}

impl Drop for StopsShared {
    fn drop(&mut self) {
        let _ = one_shot(
            &self.folder,
            &self.home,
            &["--graph-path", ".reflow2/graph", "--stop-shared"],
        );
    }
}

/// A server process that is killed when dropped, and the port it bound.
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

impl Server {
    /// Stop it the way an operator does, so its store lock is released before
    /// the store is read back.
    fn stop(mut self) {
        #[cfg(unix)]
        {
            let _ = Command::new("kill")
                .arg(self.child.id().to_string())
                .status();
        }
        let start = Instant::now();
        while self.child.try_wait().ok().flatten().is_none() {
            if start.elapsed() > Duration::from_secs(30) {
                let _ = self.child.kill();
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = self.child.wait();
    }
}

/// Start `reflow2-mcp <args>` and wait for it to print where it listens.
fn start_server(cwd: &Path, home: &Path, args: &[&str]) -> Server {
    let mut cmd = Command::new(bin());
    cmd.current_dir(cwd).args(args);
    isolate(&mut cmd, home);
    cmd.env("RUST_LOG", "info");
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the server");
    let stderr = child.stderr.take().unwrap();
    let (tx, rx) = channel::<u16>();
    std::thread::spawn(move || {
        let mut sent = false;
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if !sent && let Some(i) = line.find("http://127.0.0.1:") {
                let digits: String = line[i + "http://127.0.0.1:".len()..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if let Ok(p) = digits.parse::<u16>()
                    && p != 0
                {
                    let _ = tx.send(p);
                    sent = true;
                }
            }
        }
    });
    let port = rx
        .recv_timeout(Duration::from_secs(120))
        .unwrap_or_else(|_| {
            panic!(
                "`reflow2-mcp {}` never said where it listens",
                args.join(" ")
            )
        });
    Server { child, port }
}

/// A client process, driven over stdio one JSON-RPC line at a time.
struct Client {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Client {
    fn start(cwd: &Path, home: &Path, args: &[&str]) -> Client {
        let mut cmd = Command::new(bin());
        cmd.current_dir(cwd).args(args);
        isolate(&mut cmd, home);
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn the client");
        let stdin = child.stdin.take();
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

    fn send_raw(&mut self, line: &str) {
        let stdin = self.stdin.as_mut().expect("stdin open");
        writeln!(stdin, "{line}").expect("write to the client");
        stdin.flush().unwrap();
    }

    fn send(&mut self, v: serde_json::Value) {
        self.send_raw(&v.to_string());
    }

    /// The next line whose `id` is `id` (a number or a JSON value).
    fn reply_to(&mut self, id: &serde_json::Value) -> serde_json::Value {
        let deadline = Instant::now() + Duration::from_secs(90);
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let line = self
                .lines
                .recv_timeout(left)
                .unwrap_or_else(|_| panic!("no reply to request {id}"));
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            if v.get("id") == Some(id) {
                return v;
            }
        }
    }

    fn reply(&mut self, id: i64) -> serde_json::Value {
        self.reply_to(&serde_json::json!(id))
    }

    fn handshake(&mut self) -> serde_json::Value {
        self.send(serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"read-only-test","version":"1"}}}));
        let hello = self.reply(1);
        self.send(serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        hello
    }

    fn call(&mut self, id: i64, tool: &str, args: serde_json::Value) -> serde_json::Value {
        self.send(serde_json::json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":tool,"arguments":args}}));
        self.reply(id)
    }

    /// Close stdin and wait for the client to go.
    fn finish(mut self) {
        drop(self.stdin.take());
        let start = Instant::now();
        while self.child.try_wait().ok().flatten().is_none() {
            if start.elapsed() > Duration::from_secs(30) {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

/// The text of a tool result, whichever block carries it.
fn text_of(reply: &serde_json::Value) -> String {
    reply["result"]["content"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|c| c["text"].as_str())
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

/// A refusal by the CLIENT: an MCP tool error naming `--read-only`, saying it
/// was not sent.
fn assert_refused_here(reply: &serde_json::Value, what: &str) {
    assert_eq!(
        reply["result"]["isError"],
        serde_json::json!(true),
        "{what}: expected a tool error from the client, got {reply}"
    );
    let text = text_of(reply);
    assert!(
        text.contains(REFUSED_HERE) && text.contains("--read-only"),
        "{what}: the refusal must say the client refused it and name --read-only: {text}"
    );
    assert!(
        text.to_lowercase().contains("nothing was sent"),
        "{what}: the refusal must say nothing reached the server: {text}"
    );
}

/// A read answered by the server, not refused.
fn assert_answered(reply: &serde_json::Value, what: &str) {
    assert!(
        reply.get("error").is_none() && reply["result"]["isError"] != serde_json::json!(true),
        "{what}: a read must pass through a read-only client: {reply}"
    );
}

// ───────────────────────────────────────────── the walk, over real servers

/// One client mode, set up against a real server holding a seeded design.
struct Rig {
    name: &'static str,
    /// Where the client runs, and the flags it is started with.
    cwd: PathBuf,
    client_args: Vec<String>,
    /// The store the server holds, and where to read it back from.
    store_cwd: PathBuf,
    store: String,
    /// What to stop before reading the store back.
    server: Option<Server>,
    shared_folder: Option<PathBuf>,
    _dirs: Vec<tempfile::TempDir>,
}

fn rig_remote(home: &Path) -> Rig {
    let d = scratch("remote");
    let store = d.path().join("design").join("graph");
    let store = store.to_str().unwrap().to_string();
    seed(d.path(), home, &store);
    let server = start_server(
        d.path(),
        home,
        &["--graph-path", &store, "--http", "127.0.0.1:0"],
    );
    let url = format!("http://127.0.0.1:{}/", server.port);
    Rig {
        name: "--remote",
        cwd: d.path().to_path_buf(),
        client_args: vec!["--remote".into(), url, "--read-only".into()],
        store_cwd: d.path().to_path_buf(),
        store,
        server: Some(server),
        shared_folder: None,
        _dirs: vec![d],
    }
}

fn rig_shared(home: &Path) -> Rig {
    let d = scratch("shared");
    seed(d.path(), home, ".reflow2/graph");
    Rig {
        name: "--shared",
        cwd: d.path().to_path_buf(),
        client_args: vec![
            "--graph-path".into(),
            ".reflow2/graph".into(),
            "--shared".into(),
            "--read-only".into(),
        ],
        store_cwd: d.path().to_path_buf(),
        store: ".reflow2/graph".into(),
        server: None,
        shared_folder: Some(d.path().to_path_buf()),
        _dirs: vec![d],
    }
}

fn rig_pointer(home: &Path) -> Rig {
    let root = scratch("registry");
    let design = root.path().join("the-design");
    std::fs::create_dir_all(&design).unwrap();
    let store = design.join(".reflow2").join("graph");
    let store = store.to_str().unwrap().to_string();
    seed(root.path(), home, &store);
    let raw = std::fs::read_to_string(design.join(".reflow2").join("graph.id.json")).unwrap();
    let id = serde_json::from_str::<serde_json::Value>(&raw).unwrap()["graph_id"]
        .as_str()
        .unwrap()
        .to_string();
    let server = start_server(
        root.path(),
        home,
        &[
            "--registry-root",
            root.path().to_str().unwrap(),
            "--http",
            "127.0.0.1:0",
        ],
    );
    let folder = scratch("pointer-folder");
    std::fs::write(
        folder.path().join(".reflow2.toml"),
        format!(
            "[design]\nid = \"{id}\"\naddress = \"http://127.0.0.1:{}/g/{id}/\"\n",
            server.port
        ),
    )
    .unwrap();
    Rig {
        name: "a client attached through .reflow2.toml",
        cwd: folder.path().to_path_buf(),
        // The machine-wide entry's flags, plus --read-only.
        client_args: vec![
            "--graph-path".into(),
            ".reflow2/graph".into(),
            "--shared".into(),
            "--only-if-present".into(),
            "--read-only".into(),
        ],
        store_cwd: root.path().to_path_buf(),
        store,
        server: Some(server),
        shared_folder: None,
        _dirs: vec![root, folder],
    }
}

/// THE FLAG × MODE WALK, EXTENDED TO THE CLIENT MODES (item 1 walked the
/// one-shot modes): `--read-only` against every mode that forwards calls to a
/// server. In each, a write is refused by the client with nothing sent, a read
/// passes through and is answered, and the design is unchanged afterwards.
#[test]
fn read_only_is_honoured_by_every_client_mode() {
    let home_dir = scratch("home");
    let home = home_dir.path().join("home");
    let mut failures = Vec::new();
    for make in [rig_remote, rig_shared, rig_pointer] {
        let mut rig = make(&home);
        let _stops = rig.shared_folder.clone().map(|folder| StopsShared {
            folder,
            home: home.clone(),
        });
        let args: Vec<&str> = rig.client_args.iter().map(String::as_str).collect();
        let mut client = Client::start(&rig.cwd, &home, &args);
        let hello = client.handshake();
        if hello.get("error").is_some() {
            failures.push(format!("{}: the handshake failed: {hello}", rig.name));
        }
        let write = client.call(2, "add_requirement", write_args());
        let read = client.call(3, "get_node", serde_json::json!({"id": "proj:seed"}));
        let read_back = client.call(4, "get_node", serde_json::json!({"id": REQ_ID}));
        client.finish();

        let refused = write["result"]["isError"] == serde_json::json!(true)
            && text_of(&write).contains(REFUSED_HERE)
            && text_of(&write).contains("--read-only");
        if !refused {
            failures.push(format!(
                "{}: a write was not refused by the client: {write}",
                rig.name
            ));
        }
        if read.get("error").is_some()
            || read["result"]["isError"] == serde_json::json!(true)
            || !text_of(&read).contains("proj:seed")
        {
            failures.push(format!("{}: a read did not pass through: {read}", rig.name));
        }
        if text_of(&read_back).contains("Read-only client probe") {
            failures.push(format!(
                "{}: the server answered the probe it should never have received: {read_back}",
                rig.name
            ));
        }
        if let Some(server) = rig.server.take() {
            server.stop();
        }
        if let Some(folder) = &rig.shared_folder {
            drop(StopsShared {
                folder: folder.clone(),
                home: home.clone(),
            });
        }
        if probe_landed(&rig.store_cwd, &home, &rig.store) {
            failures.push(format!(
                "{}: the write LANDED in the server's store",
                rig.name
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// The shape of the refusal an agent reads, and that the session goes on.
#[test]
fn a_remote_read_only_refusal_is_a_tool_error_that_names_the_flag_and_the_session_goes_on() {
    let home_dir = scratch("home-shape");
    let home = home_dir.path().join("home");
    let mut rig = rig_remote(&home);
    let args: Vec<&str> = rig.client_args.iter().map(String::as_str).collect();
    let mut client = Client::start(&rig.cwd, &home, &args);
    client.handshake();
    let write = client.call(2, "add_requirement", write_args());
    assert_refused_here(&write, "add_requirement over --remote");
    let text = text_of(&write);
    assert!(
        text.contains("add_requirement"),
        "the refusal names the tool: {text}"
    );
    // A file the server would write is refused too; the same call without
    // `path` answers in the reply.
    let file = rig.cwd.join("exported.json");
    let export = client.call(
        3,
        "export_graph",
        serde_json::json!({"path": file.to_str().unwrap()}),
    );
    assert_refused_here(&export, "export_graph with a path");
    assert!(!file.exists(), "the server wrote the file");
    let inline = client.call(4, "export_graph", serde_json::json!({}));
    assert_answered(&inline, "export_graph without a path");
    let after = client.call(5, "graph_report", serde_json::json!({}));
    assert_answered(&after, "a read after two refusals");
    client.finish();
    rig.server.take().unwrap().stop();
    assert!(!probe_landed(&rig.store_cwd, &home, &rig.store));
}

/// A `--shared` call whose arguments hold the word "initialize" is a tool
/// call, not a handshake: it is answered, and so is the next one. On main it
/// got no reply and broke the session (422 on every later call).
#[test]
fn a_shared_call_that_mentions_initialize_is_not_taken_for_a_handshake() {
    let home_dir = scratch("home-init");
    let home = home_dir.path().join("home");
    let d = scratch("init-word");
    seed(d.path(), &home, ".reflow2/graph");
    let stops = StopsShared {
        folder: d.path().to_path_buf(),
        home: home.clone(),
    };
    for read_only in [false, true] {
        let mut args = vec!["--graph-path", ".reflow2/graph", "--shared"];
        if read_only {
            args.push("--read-only");
        }
        let mut client = Client::start(d.path(), &home, &args);
        client.handshake();
        let search = client.call(
            2,
            "search_design",
            serde_json::json!({"query": "initialize"}),
        );
        assert_answered(&search, "a read whose argument is \"initialize\"");
        if read_only {
            let mut w = write_args();
            w["statement"] = serde_json::json!("initialize");
            let write = client.call(3, "add_requirement", w);
            assert_refused_here(&write, "a write whose argument is \"initialize\"");
        }
        let next = client.call(4, "get_node", serde_json::json!({"id": "proj:seed"}));
        assert_answered(&next, "the call after it");
        client.finish();
    }
    drop(stops);
    assert!(!probe_landed(d.path(), &home, ".reflow2/graph"));
}

/// `--shared --read-only` where there is no store starts no server and
/// creates nothing: a server would create the store, and `--read-only`
/// creates nothing. The session is told why, in band.
#[test]
fn a_shared_read_only_client_starts_no_server_where_there_is_no_store() {
    let home_dir = scratch("home-nostore");
    let home = home_dir.path().join("home");
    let d = scratch("opted-in");
    std::fs::create_dir_all(d.path().join(".reflow2")).unwrap();
    let stops = StopsShared {
        folder: d.path().to_path_buf(),
        home: home.clone(),
    };
    let mut client = Client::start(
        d.path(),
        &home,
        &["--graph-path", ".reflow2/graph", "--shared", "--read-only"],
    );
    let hello = client.handshake();
    let said = hello["result"]["instructions"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(
        said.contains("--read-only"),
        "the session is told why it has no design: {hello}"
    );
    client.finish();
    drop(stops);
    let left: Vec<String> = std::fs::read_dir(d.path().join(".reflow2"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert!(
        left.is_empty(),
        "--shared --read-only created {left:?} where there was no store"
    );
}

/// The latent surface — what a machine-wide entry serves where no design has
/// been started — starts no design under `--read-only`, and a design that
/// appears later is served read-only.
#[test]
fn the_latent_surface_starts_no_design_under_read_only() {
    let home_dir = scratch("home-latent");
    let home = home_dir.path().join("home");
    for entry in [
        vec![
            "--graph-path",
            ".reflow2/graph",
            "--only-if-present",
            "--read-only",
        ],
        vec![
            "--graph-path",
            ".reflow2/graph",
            "--shared",
            "--only-if-present",
            "--read-only",
        ],
    ] {
        let d = scratch("latent");
        let mut client = Client::start(d.path(), &home, &entry);
        client.handshake();
        let start = client.call(2, "reflow2_start_design", serde_json::json!({}));
        let refused =
            start.get("error").is_some() || start["result"]["isError"] == serde_json::json!(true);
        let said = format!("{start}");
        assert!(
            refused && said.contains("--read-only"),
            "`{}`: reflow2_start_design must be refused, naming --read-only: {start}",
            entry.join(" ")
        );
        assert!(
            !d.path().join(".reflow2").exists(),
            "`{}`: a design directory was created",
            entry.join(" ")
        );

        // A design appears under the running server (a restore, say): it is
        // served from then on — read-only.
        seed(d.path(), &home, ".reflow2/graph");
        let read = client.call(3, "get_node", serde_json::json!({"id": "proj:seed"}));
        assert_answered(&read, "a read once a design exists");
        let write = client.call(4, "add_requirement", write_args());
        let refused =
            write.get("error").is_some() || write["result"]["isError"] == serde_json::json!(true);
        assert!(
            refused && format!("{write}").contains("READ-ONLY"),
            "`{}`: the promoted surface must refuse writes: {write}",
            entry.join(" ")
        );
        client.finish();
        assert!(!probe_landed(d.path(), &home, ".reflow2/graph"));
    }
}

// ─────────────────────────── a stand-in server: what actually goes on the wire

/// How the stand-in answers `tools/list`.
#[derive(Clone, Copy, PartialEq)]
enum Lists {
    /// With the surface below.
    Works,
    /// Every `tools/list` fails with a 500.
    Fails,
}

/// A stand-in MCP server that records every body it is sent. Its tool list is
/// chosen to exercise each way a call is classified.
fn stand_in(lists: Lists) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let record = Arc::clone(&seen);
    let ro = |name: &str, hint: Option<bool>| {
        let mut t = serde_json::json!({"name": name, "inputSchema": {"type": "object"}});
        if let Some(h) = hint {
            t["annotations"] = serde_json::json!({"readOnlyHint": h});
        }
        t
    };
    let tools = serde_json::json!([
        // A read on both sides: forwarded.
        ro("get_node", Some(true)),
        // A write on both sides.
        ro("add_requirement", Some(false)),
        // A file writer: forwarded without `path`, refused with one.
        ro("export_graph", Some(true)),
        // This build reads; THIS server says it writes.
        ro("graph_report", Some(false)),
        // This build reads; this server declares no hint.
        ro("search_design", None),
        // A read this server serves and this build has never heard of.
        ro("mystery_read", Some(true)),
        // (`detect_gaps`, a read in this build, is not served here at all.)
    ]);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut len = 0usize;
            let mut session = String::from("-");
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
                    if k.trim().eq_ignore_ascii_case("content-length") {
                        len = v.trim().parse().unwrap_or(0);
                    }
                    if k.trim().eq_ignore_ascii_case("mcp-session-id") {
                        session = v.trim().to_string();
                    }
                }
            }
            let mut body = vec![0u8; len];
            reader.read_exact(&mut body).unwrap();
            let body = String::from_utf8(body).unwrap();
            // Recorded as "<session id or -> <body>", so a test can see which
            // session each message was sent on.
            record.lock().unwrap().push(format!("{session} {body}"));
            let msg: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
            let method = msg["method"].as_str().unwrap_or_default().to_string();
            let (code, extra, reply) = match msg.get("id") {
                _ if lists == Lists::Fails && method == "tools/list" => {
                    (500, String::new(), String::new())
                }
                None => (202, String::new(), String::new()),
                Some(id) => {
                    let (result, extra) = match method.as_str() {
                        "initialize" => (
                            serde_json::json!({"protocolVersion": "2025-06-18",
                                "capabilities": {"tools": {}},
                                "serverInfo": {"name": "stand-in", "version": "0"}}),
                            "mcp-session-id: s-1\r\n".to_string(),
                        ),
                        "tools/list" => (serde_json::json!({ "tools": tools }), String::new()),
                        "tools/call" => (
                            serde_json::json!({"content": [{"type": "text",
                                "text": format!("the stand-in ran {}", msg["params"]["name"])}]}),
                            String::new(),
                        ),
                        _ => (serde_json::json!({}), String::new()),
                    };
                    let reply = serde_json::json!({"jsonrpc": "2.0", "id": id, "result": result})
                        .to_string();
                    (200, extra, reply)
                }
            };
            let _ = write!(
                stream,
                "HTTP/1.1 {code} X\r\ncontent-type: application/json\r\n{extra}content-length: {}\r\nconnection: close\r\n\r\n{reply}",
                reply.len()
            );
        }
    });
    (url, seen)
}

/// The body of a recorded message, without its session prefix.
fn body_of(recorded: &str) -> &str {
    recorded.split_once(' ').map(|(_, b)| b).unwrap_or(recorded)
}

/// The session a recorded message was sent on (`-` for none).
fn session_of(recorded: &str) -> &str {
    recorded.split_once(' ').map(|(s, _)| s).unwrap_or("-")
}

/// The `tools/call`s the stand-in received, by tool name.
fn calls_received(seen: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
    seen.lock()
        .unwrap()
        .iter()
        .filter_map(|b| serde_json::from_str::<serde_json::Value>(body_of(b)).ok())
        .flat_map(|v| match v {
            serde_json::Value::Array(items) => items,
            one => vec![one],
        })
        .filter(|v| v["method"] == "tools/call")
        .filter_map(|v| v["params"]["name"].as_str().map(str::to_string))
        .collect()
}

/// EVERY WAY A CALL IS CLASSIFIED, AND WHAT REACHES THE SERVER. A call leaves
/// only when this build, the server's own list and the file rule all say it
/// reads; everything else is refused here, and the stand-in's record proves it
/// never arrived.
#[test]
fn nothing_a_read_only_client_refuses_reaches_the_server() {
    let (url, seen) = stand_in(Lists::Works);
    let home_dir = scratch("home-stand-in");
    let home = home_dir.path().join("home");
    let cwd = scratch("stand-in-cwd");
    let mut client = Client::start(cwd.path(), &home, &["--remote", &url, "--read-only"]);
    client.handshake();

    // Passes: a read on both sides, and the file writer without a path.
    assert_answered(
        &client.call(2, "get_node", serde_json::json!({"id": "proj:x"})),
        "get_node",
    );
    assert_answered(
        &client.call(3, "export_graph", serde_json::json!({})),
        "export_graph without a path",
    );

    // Refused, each for its own reason, and named in the refusal.
    for (id, tool, args, because) in [
        (10, "add_requirement", write_args(), "writes"),
        (
            11,
            "export_graph",
            serde_json::json!({"path": "/tmp/never-written.json"}),
            "path",
        ),
        (12, "graph_report", serde_json::json!({}), "server"),
        (
            13,
            "search_design",
            serde_json::json!({"query": "x"}),
            "server",
        ),
        (14, "mystery_read", serde_json::json!({}), "mystery_read"),
        (15, "detect_gaps", serde_json::json!({}), "tool list"),
    ] {
        let r = client.call(id, tool, args);
        assert_refused_here(&r, tool);
        assert!(
            text_of(&r).contains(because),
            "{tool}: the refusal should say why ({because}): {}",
            text_of(&r)
        );
    }

    // A method that is not a read is refused by name; a notification passes.
    client.send(serde_json::json!({"jsonrpc":"2.0","id":20,"method":"resources/subscribe","params":{"uri":"x"}}));
    let sub = client.reply(20);
    let msg = sub["error"]["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("--read-only") && msg.contains("resources/subscribe"),
        "{sub}"
    );
    client.send(serde_json::json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":99}}));

    // A batch that carries a write is not sent at all.
    client.send_raw(
        &serde_json::json!([
            {"jsonrpc":"2.0","id":30,"method":"tools/call","params":{"name":"get_node","arguments":{"id":"proj:x"}}},
            {"jsonrpc":"2.0","id":31,"method":"tools/call","params":{"name":"add_requirement","arguments":write_args()}}
        ])
        .to_string(),
    );
    let batch_line = client
        .lines
        .recv_timeout(Duration::from_secs(60))
        .expect("the batch is answered");
    let batch: serde_json::Value = serde_json::from_str(&batch_line).unwrap();
    let answers = batch.as_array().expect("a batch is answered with an array");
    assert_eq!(answers.len(), 2, "{batch}");
    assert!(
        batch_line.contains("--read-only"),
        "the batch refusal names the flag: {batch_line}"
    );

    // A line that is not JSON is not sent either.
    client.send_raw("this is not json, tools/call add_requirement");
    let parse = client.reply_to(&serde_json::Value::Null);
    assert_eq!(parse["error"]["code"], -32700, "{parse}");

    // A write whose arguments hold "initialize" is still a write, and the
    // session goes on.
    let mut w = write_args();
    w["statement"] = serde_json::json!("initialize");
    assert_refused_here(&client.call(40, "add_requirement", w), "initialize in args");
    assert_answered(
        &client.call(41, "get_node", serde_json::json!({"id": "proj:y"})),
        "the next read",
    );
    client.finish();

    let calls = calls_received(&seen);
    assert_eq!(
        calls,
        vec!["get_node", "export_graph", "get_node"],
        "only the reads may reach the server"
    );
    let bodies = seen.lock().unwrap().clone();
    assert_eq!(
        bodies
            .iter()
            .filter(|b| body_of(b).contains("\"method\":\"initialize\""))
            .count(),
        1,
        "one handshake: {bodies:?}"
    );
    for r in bodies.iter().skip(1) {
        assert_eq!(session_of(r), "s-1", "a message left the session: {r}");
    }
    assert!(
        !bodies.iter().any(|b| b.contains("resources/subscribe")),
        "the refused method was sent: {bodies:?}"
    );
    assert!(
        bodies.iter().any(|b| b.contains("notifications/cancelled")),
        "a notification is forwarded: {bodies:?}"
    );
}

/// WHERE THE SERVER'S LIST CANNOT BE READ, EVEN A READ IS REFUSED: a client
/// that cannot classify a tool treats it as a write.
#[test]
fn a_read_only_client_that_cannot_read_the_servers_tool_list_sends_nothing() {
    let (url, seen) = stand_in(Lists::Fails);
    let home_dir = scratch("home-fails");
    let home = home_dir.path().join("home");
    let cwd = scratch("fails-cwd");
    let mut client = Client::start(cwd.path(), &home, &["--remote", &url, "--read-only"]);
    client.handshake();
    let r = client.call(2, "get_node", serde_json::json!({"id": "proj:x"}));
    assert_refused_here(&r, "get_node with no tool list");
    assert!(
        text_of(&r).contains("could not read the server's tool list"),
        "{}",
        text_of(&r)
    );
    client.finish();
    assert!(
        calls_received(&seen).is_empty(),
        "a call was sent: {:?}",
        seen.lock().unwrap()
    );
}

/// Without `--read-only` nothing is screened: every call is forwarded as
/// before, including one whose arguments hold "initialize".
#[test]
fn without_read_only_a_remote_client_forwards_every_call() {
    let (url, seen) = stand_in(Lists::Works);
    let home_dir = scratch("home-plain");
    let home = home_dir.path().join("home");
    let cwd = scratch("plain-cwd");
    let mut client = Client::start(cwd.path(), &home, &["--remote", &url]);
    client.handshake();
    let mut w = write_args();
    w["statement"] = serde_json::json!("initialize");
    assert_answered(&client.call(2, "add_requirement", w), "a write");
    assert_answered(
        &client.call(3, "mystery_read", serde_json::json!({})),
        "an unknown tool",
    );
    client.finish();
    assert_eq!(
        calls_received(&seen),
        vec!["add_requirement", "mystery_read"]
    );
    let recorded = seen.lock().unwrap().clone();
    assert!(
        !recorded
            .iter()
            .any(|b| b.contains("\"method\":\"tools/list\"")),
        "a client that is not read-only asks the server nothing of its own: {recorded:?}"
    );
    // The call whose arguments hold "initialize" was NOT taken for a
    // handshake: one handshake, and every later message on its session. On
    // main the call was posted as a fresh handshake, with no session, and the
    // session id was lost for every call after it.
    let handshakes = recorded
        .iter()
        .filter(|r| body_of(r).contains("\"method\":\"initialize\""))
        .count();
    assert_eq!(handshakes, 1, "{recorded:?}");
    for r in recorded.iter().skip(1) {
        assert_eq!(
            session_of(r),
            "s-1",
            "a message left the session the handshake opened: {r}"
        );
    }
}
