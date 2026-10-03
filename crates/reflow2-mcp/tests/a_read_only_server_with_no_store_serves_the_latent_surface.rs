//! A SERVER started with `--read-only` where there is no design store serves the
//! LATENT surface and creates nothing — through the real binary, in every
//! serving mode.
//!
//! WHAT FAILED, measured on main 293f957 (2026-10-03), each pinned below:
//! · stdio `--graph-path .reflow2/graph --read-only` in an empty folder: the
//!   server opened the path, which created `.reflow2/graph` with its version
//!   stamp and identity beside it, and served an empty design that refused
//!   every write;
//! · `--http` the same, and `--serve-shared` the same plus its rendezvous
//!   (`graph.server.json`);
//!   (fact:a-read-only-server-creates-an-empty-store-where-there-is-none-2026-10-03)
//! · `--only-if-present --http` where no design was started served the latent
//!   surface on STDIO, not on the HTTP address it was given: with stdin closed
//!   it exited at once, and nothing ever listened;
//! · a design the latent surface promoted itself to was opened bare: the
//!   operator's `--export-to` was never started for it.
//!
//! THE CLASS: a guard written for one mode protects only that mode. The test
//! "is there a store at this path?" had three copies (the one-shot door, the
//! `--shared` client, the latent surface) and the serving modes had none, so
//! they opened the path and created the store `--read-only` promised not to.
//! The rule is now one function (`reflow2_mcp::opening`), and every serving mode
//! where it says "may not open" serves the latent surface on the transport it
//! was given (Anthony's choice, 2026-10-03,
//! `dec:a-read-only-server-with-no-store-serves-the-latent-surface`).
//!
//! Each test here was run against main before the fix and failed there, except
//! the registry pin, which passes on main by construction (said on the test).

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

const REQ_ID: &str = "req:read-only-server-probe";

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

fn write_args() -> serde_json::Value {
    serde_json::json!({
        "id": REQ_ID,
        "name": "Read-only server probe",
        "statement": "written to a server that was started with --read-only"
    })
}

/// A scratch directory, removed when dropped.
fn scratch(tag: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("reflow2-ro-server-{tag}-"))
        .tempdir()
        .unwrap()
}

/// Its own HOME and settings, and none of this machine's reflow2 configuration.
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

/// Every file and directory under `root`, with each file's bytes: what
/// "byte-identical" is compared on.
fn snapshot(root: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            let rel = path.strip_prefix(root).unwrap().display().to_string();
            if path.is_dir() {
                out.insert(rel, None);
                stack.push(path);
            } else {
                out.insert(rel, Some(std::fs::read(&path).unwrap_or_default()));
            }
        }
    }
    out
}

fn names(snap: &BTreeMap<String, Option<Vec<u8>>>) -> Vec<&str> {
    snap.keys().map(String::as_str).collect()
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

/// Create a design at `graph` from ANOTHER process, with one project in it.
fn seed(cwd: &Path, home: &Path, graph: &str) -> Result<(), String> {
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
    if o.status.code() == Some(0) {
        Ok(())
    } else {
        Err(format!(
            "another process could not create the design: {}",
            String::from_utf8_lossy(&o.stderr)
        ))
    }
}

/// Whether the probe requirement is in the store at `graph`, read with the
/// one-shot door once every server has stopped.
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
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    !v["node"].is_null()
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
    /// Stop it the way an operator does, so a store it opened is released.
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

/// Start `reflow2-mcp <args>` and wait for it to say where it listens. `Err`
/// carries what it said instead when it exits without listening.
fn start_server(cwd: &Path, home: &Path, args: &[&str]) -> Result<Server, String> {
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
    let (tx, rx) = channel::<Result<u16, String>>();
    std::thread::spawn(move || {
        let mut sent = false;
        let mut said = String::new();
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if !sent {
                said.push_str(&line);
                said.push('\n');
            }
            if !sent && let Some(i) = line.find("http://127.0.0.1:") {
                let digits: String = line[i + "http://127.0.0.1:".len()..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if let Ok(p) = digits.parse::<u16>()
                    && p != 0
                {
                    let _ = tx.send(Ok(p));
                    sent = true;
                }
            }
        }
        if !sent {
            let _ = tx.send(Err(said));
        }
    });
    match rx.recv_timeout(Duration::from_secs(120)) {
        Ok(Ok(port)) => Ok(Server { child, port }),
        Ok(Err(said)) => {
            let _ = child.kill();
            let _ = child.wait();
            Err(format!(
                "`reflow2-mcp {}` exited without listening; it said:\n{said}",
                args.join(" ")
            ))
        }
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            Err(format!(
                "`reflow2-mcp {}` never said where it listens",
                args.join(" ")
            ))
        }
    }
}

/// `GET <path>` on the server, answered with its status code.
fn http_status(port: u16, path: &str) -> u16 {
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
    write!(
        s,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut head = String::new();
    BufReader::new(s).read_line(&mut head).unwrap();
    head.split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0)
}

/// An MCP client: the binary itself over stdio, or a `--remote` client
/// (without `--read-only`, so it forwards every call) in front of an HTTP
/// server.
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

    fn send(&mut self, v: serde_json::Value) {
        let stdin = self.stdin.as_mut().expect("stdin open");
        writeln!(stdin, "{v}").expect("write to the client");
        stdin.flush().unwrap();
    }

    fn reply(&mut self, id: i64) -> serde_json::Value {
        let id = serde_json::json!(id);
        let deadline = Instant::now() + Duration::from_secs(90);
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let Ok(line) = self.lines.recv_timeout(left) else {
                return serde_json::json!({"error": {"message": format!("no reply to request {id}")}});
            };
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            if v.get("id") == Some(&id) {
                return v;
            }
        }
    }

    fn handshake(&mut self) -> serde_json::Value {
        self.send(serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"read-only-server-test","version":"1"}}}));
        let hello = self.reply(1);
        self.send(serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        hello
    }

    fn tools(&mut self, id: i64) -> Vec<String> {
        self.send(serde_json::json!({"jsonrpc":"2.0","id":id,"method":"tools/list"}));
        let r = self.reply(id);
        r["result"]["tools"]
            .as_array()
            .map(|ts| {
                ts.iter()
                    .filter_map(|t| t["name"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn call(&mut self, id: i64, tool: &str, args: serde_json::Value) -> serde_json::Value {
        self.send(serde_json::json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":tool,"arguments":args}}));
        self.reply(id)
    }

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

fn refused(reply: &serde_json::Value) -> bool {
    reply.get("error").is_some() || reply["result"]["isError"] == serde_json::json!(true)
}

fn instructions(hello: &serde_json::Value) -> String {
    hello["result"]["instructions"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

/// How a serving mode is started, and how an MCP client reaches it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Stdio,
    Http,
    ServeShared,
}

impl Mode {
    fn flags(self) -> &'static [&'static str] {
        match self {
            Mode::Stdio => &[],
            Mode::Http => &["--http", "127.0.0.1:0"],
            Mode::ServeShared => &["--serve-shared"],
        }
    }
}

/// A running server in `folder` and a client on it. For the HTTP modes the
/// client is a `--remote` process started OUTSIDE the folder, so the only
/// process that could write there is the server.
struct Rig {
    server: Option<Server>,
    client: Client,
}

fn rig(mode: Mode, folder: &Path, home: &Path, extra: &[&str]) -> Result<Rig, String> {
    let mut args = vec!["--graph-path", ".reflow2/graph"];
    args.extend_from_slice(mode.flags());
    args.extend_from_slice(extra);
    match mode {
        Mode::Stdio => Ok(Rig {
            server: None,
            client: Client::start(folder, home, &args),
        }),
        Mode::Http | Mode::ServeShared => {
            let server = start_server(folder, home, &args)?;
            let url = format!("http://127.0.0.1:{}/", server.port);
            let client = Client::start(home, home, &["--remote", &url]);
            Ok(Rig {
                server: Some(server),
                client,
            })
        }
    }
}

/// ⭐ THE WALK: every serving mode × an empty folder and an opted-in one
/// (`.reflow2/` there and empty). Each starts with `--read-only`, is told there
/// is no design and why, lists the latent tools, has `reflow2_start_design`
/// refused, and leaves the folder byte-identical. Then a design is created by
/// ANOTHER process, and the same session reads it and has its writes refused.
#[test]
fn every_read_only_server_with_no_store_serves_the_latent_surface_and_creates_nothing() {
    let home_dir = scratch("home");
    let home = home_dir.path().join("home");
    let mut failures = Vec::new();
    for mode in [Mode::Stdio, Mode::Http, Mode::ServeShared] {
        for opted_in in [false, true] {
            let name = format!(
                "{mode:?} --read-only, {} folder",
                if opted_in { "opted-in" } else { "empty" }
            );
            let d = scratch("walk");
            if opted_in {
                std::fs::create_dir_all(d.path().join(".reflow2")).unwrap();
            }
            let before = snapshot(d.path());
            let mut rig = match rig(mode, d.path(), &home, &["--read-only"]) {
                Ok(r) => r,
                Err(why) => {
                    failures.push(format!("{name}: {why}"));
                    continue;
                }
            };
            let hello = rig.client.handshake();
            let said = instructions(&hello);
            if !(said.contains("--read-only") && said.to_lowercase().contains("no design store")) {
                failures.push(format!(
                    "{name}: the handshake must say there is no design store and that \
                     --read-only creates none: {said:.300}"
                ));
            }
            let tools = rig.client.tools(2);
            if !(tools.iter().any(|t| t == "reflow2_start_design")
                && tools.iter().any(|t| t == "describe_designs")
                && !tools.iter().any(|t| t == "add_requirement"))
            {
                failures.push(format!(
                    "{name}: the latent surface's tools are expected, got {} tools: {:?}",
                    tools.len(),
                    tools.iter().take(6).collect::<Vec<_>>()
                ));
            }
            let start = rig
                .client
                .call(3, "reflow2_start_design", serde_json::json!({}));
            if !(refused(&start) && start.to_string().contains("--read-only")) {
                failures.push(format!(
                    "{name}: reflow2_start_design must be refused naming --read-only: {start:.300}"
                ));
            }
            let look = rig.client.call(
                4,
                "describe_designs",
                serde_json::json!({"paths": [".reflow2/graph"]}),
            );
            if refused(&look) {
                failures.push(format!("{name}: describe_designs must answer: {look:.300}"));
            }
            if let Some(server) = &rig.server {
                let status = http_status(server.port, "/readyz");
                if status != 503 {
                    failures.push(format!(
                        "{name}: /readyz must say 503 while no design is served, said {status}"
                    ));
                }
            }
            let during = snapshot(d.path());
            if during != before {
                failures.push(format!(
                    "{name}: the folder changed while the server ran: {:?} -> {:?}",
                    names(&before),
                    names(&during)
                ));
            }

            // A design appears, made by ANOTHER process: served read-only.
            if let Err(why) = seed(d.path(), &home, ".reflow2/graph") {
                failures.push(format!("{name}: {why}"));
                continue;
            }
            let read = rig
                .client
                .call(5, "get_node", serde_json::json!({"id": "proj:seed"}));
            if refused(&read) || !read.to_string().contains("proj:seed") {
                failures.push(format!(
                    "{name}: a design that appears must be served: {read:.300}"
                ));
            }
            let write = rig.client.call(6, "add_requirement", write_args());
            if !(refused(&write) && write.to_string().contains("READ-ONLY")) {
                failures.push(format!(
                    "{name}: the design that appeared must be served READ-ONLY: {write:.300}"
                ));
            }
            if let Some(server) = &rig.server {
                let status = http_status(server.port, "/readyz");
                if status != 200 {
                    failures.push(format!(
                        "{name}: /readyz must say 200 once the design is served, said {status}"
                    ));
                }
            }
            let Rig { server, client } = rig;
            client.finish();
            if let Some(server) = server {
                server.stop();
            }
            if mode == Mode::ServeShared && d.path().join(".reflow2/graph.server.json").exists() {
                failures.push(format!("{name}: a rendezvous was left behind"));
            }
            if probe_landed(d.path(), &home, ".reflow2/graph") {
                failures.push(format!("{name}: the write LANDED"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// ⭐ THE OTHER CAUSE, ON THE TRANSPORT ASKED. `--only-if-present` where no
/// design was started serves the latent surface on the transport it was given,
/// and a design it starts is prepared the way a healthy start prepares one:
/// here, the operator's `--export-to` is kept current. On main the `--http`
/// server served stdio and exited (stdin closed), and the stdio one promoted
/// to a design whose `--export-to` was never started.
#[test]
fn the_latent_surface_serves_the_transport_asked_and_prepares_the_design_it_starts() {
    let home_dir = scratch("home-oip");
    let home = home_dir.path().join("home");
    let mut failures = Vec::new();
    for mode in [Mode::Stdio, Mode::Http] {
        let d = scratch("oip");
        let export = d.path().join("design.json");
        let export_s = export.to_str().unwrap().to_string();
        let mut rig = match rig(
            mode,
            d.path(),
            &home,
            &["--only-if-present", "--export-to", &export_s],
        ) {
            Ok(r) => r,
            Err(why) => {
                failures.push(format!("{mode:?} --only-if-present: {why}"));
                continue;
            }
        };
        let hello = rig.client.handshake();
        if !instructions(&hello).contains("reflow2_start_design") {
            failures.push(format!(
                "{mode:?}: the latent handshake is expected: {:.300}",
                instructions(&hello)
            ));
        }
        if let Some(server) = &rig.server
            && http_status(server.port, "/readyz") != 503
        {
            failures.push(format!(
                "{mode:?}: /readyz must say 503 before a design is started"
            ));
        }
        let start = rig
            .client
            .call(2, "reflow2_start_design", serde_json::json!({}));
        if refused(&start) {
            failures.push(format!("{mode:?}: start_design: {start:.300}"));
        }
        let write = rig.client.call(
            3,
            "add_project",
            serde_json::json!({"id": "proj:started", "name": "Started"}),
        );
        if refused(&write) {
            failures.push(format!(
                "{mode:?}: a write to the started design: {write:.300}"
            ));
        }
        if let Some(server) = &rig.server
            && http_status(server.port, "/readyz") != 200
        {
            failures.push(format!(
                "{mode:?}: /readyz must say 200 once the design is served"
            ));
        }
        // The write-through waits for two seconds of quiet.
        let deadline = Instant::now() + Duration::from_secs(20);
        while !export.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(100));
        }
        match std::fs::read_to_string(&export) {
            Ok(text) if text.contains("proj:started") => {}
            Ok(_) => failures.push(format!(
                "{mode:?}: --export-to was written without the write"
            )),
            Err(_) => failures.push(format!(
                "{mode:?}: --export-to was never written for the design the latent surface started"
            )),
        }
        let Rig { server, client } = rig;
        client.finish();
        if let Some(server) = server {
            server.stop();
        }
    }
    assert!(
        failures.is_empty(),
        "{} failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// THE REGISTRY, CHECKED: `--registry-root --read-only` opens only a store its
/// discovery found (re-read on every request), so it creates nothing under an
/// empty root, and a design created there later is served read-only. This pin
/// PASSES ON MAIN, by construction (`registry::Registry::discover` binds only a
/// store that exists); it is here so the serving-mode walk covers every mode.
#[test]
fn a_read_only_registry_creates_nothing_and_serves_a_design_that_appears_read_only() {
    let home_dir = scratch("home-registry");
    let home = home_dir.path().join("home");
    let root = scratch("registry");
    let before = snapshot(root.path());
    let server = start_server(
        root.path(),
        &home,
        &[
            "--registry-root",
            root.path().to_str().unwrap(),
            "--http",
            "127.0.0.1:0",
            "--read-only",
        ],
    )
    .unwrap();
    assert_eq!(
        http_status(server.port, "/g/0123456789abcdef/mcp"),
        404,
        "an unknown design is not opened"
    );
    assert_eq!(
        snapshot(root.path()),
        before,
        "an empty root must stay empty"
    );

    let design = root.path().join("one");
    std::fs::create_dir_all(&design).unwrap();
    seed(&design, &home, ".reflow2/graph").unwrap();
    let raw = std::fs::read_to_string(design.join(".reflow2/graph.id.json")).unwrap();
    let id = serde_json::from_str::<serde_json::Value>(&raw).unwrap()["graph_id"]
        .as_str()
        .unwrap()
        .to_string();
    let url = format!("http://127.0.0.1:{}/g/{id}/", server.port);
    let mut client = Client::start(&home, &home, &["--remote", &url]);
    client.handshake();
    let read = client.call(2, "get_node", serde_json::json!({"id": "proj:seed"}));
    assert!(!refused(&read), "a read: {read}");
    let write = client.call(3, "add_requirement", write_args());
    assert!(
        refused(&write) && write.to_string().contains("READ-ONLY"),
        "a write: {write}"
    );
    client.finish();
    server.stop();
    assert!(!probe_landed(&design, &home, ".reflow2/graph"));
}
