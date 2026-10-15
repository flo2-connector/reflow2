//! A build without on-disk storage refuses every store in a folder, and says
//! which build it is and how to get the one that opens it.
//!
//! Anthony, 2026-10-15: *"for the vast majority of development, I don't think
//! [RocksDB is] necessary"*. So `reflow2-mcp` has a `rocksdb` feature, ON BY
//! DEFAULT — every release, image, `cargo install`, installer and launcher
//! builds with it, exactly as before — and the everyday development command
//! (`cargo dev-test`) builds without it, on the in-memory backend.
//!
//! THE ONE THING SUCH A BUILD MUST NEVER DO is answer from memory where a store
//! in a folder was asked for. A design that silently stops persisting reads as
//! a design that works, right up to the session that finds it empty. So every
//! path that would open a store in a folder REFUSES, before anything is opened,
//! locked or written, and the refusal names the build and the fix. These tests
//! drive each door that opens one: the library openers, the one-shot door
//! (`--call`, `--export`, `--import`), a stdio server and a `--shared` client.
//!
//! They run only in a build WITHOUT the feature (`cargo test -p reflow2-mcp
//! --no-default-features`, which `cargo dev-test` and CI's core job run). In
//! the default build the last test says the feature is on, and every test in
//! the `disk` program opens a real store.

/// Everything a build WITHOUT the feature must do. Compiled only there, so the
/// default build carries none of it.
#[cfg(not(feature = "rocksdb"))]
mod without_the_feature {
    use std::io::{BufRead, BufReader, Write};
    use std::path::Path;
    use std::process::{Child, ChildStdin, Command, Output, Stdio};
    use std::sync::mpsc::{Receiver, channel};
    use std::time::{Duration, Instant};

    /// What the refusal must say, whichever door it comes through: that this
    /// build lacks on-disk storage, the feature by name, and the fix.
    fn assert_refusal(said: &str, path: &Path) {
        assert!(
            said.contains("built without on-disk storage"),
            "the refusal must say which build this is: {said}"
        );
        assert!(
            said.contains("`rocksdb` feature"),
            "and name the feature: {said}"
        );
        assert!(
            said.contains("default features"),
            "and say how to get a build that opens it: {said}"
        );
        assert!(
            said.contains(&path.display().to_string()),
            "and name the store it would not open: {said}"
        );
    }

    /// Nothing was created where the store was asked for: no store, no version
    /// stamp, no identity file, no rendezvous — the folder is as it was.
    fn assert_untouched(dir: &Path) {
        let left: Vec<String> = std::fs::read_dir(dir)
            .expect("the scratch folder")
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            left.is_empty(),
            "a refused open must leave the folder as it found it; found {left:?}"
        );
    }

    fn bin() -> &'static str {
        env!("CARGO_BIN_EXE_reflow2-mcp")
    }

    /// The binary, run once in `dir` with nothing of the caller's settings.
    fn run(dir: &Path, home: &Path, args: &[&str]) -> Output {
        let mut cmd = Command::new(bin());
        cmd.current_dir(dir).args(args);
        isolate(&mut cmd, home);
        cmd.output().expect("the binary runs")
    }

    fn isolate(cmd: &mut Command, home: &Path) {
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

    /// A scratch folder for the store, and a HOME of its own beside it, so the
    /// store's folder holds nothing but what an open would put there.
    struct Rig {
        _root: tempfile::TempDir,
        dir: std::path::PathBuf,
        home: std::path::PathBuf,
        graph: std::path::PathBuf,
    }

    fn rig() -> Rig {
        let root = tempfile::Builder::new()
            .prefix("reflow2-no-disk-")
            .tempdir()
            .expect("tempdir");
        let dir = root.path().join("project");
        let home = root.path().join("home");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        let graph = dir.join("graph");
        Rig {
            _root: root,
            dir,
            home,
            graph,
        }
    }

    /// An MCP client: the binary over stdio. Any `--shared` daemon a build WITH
    /// the feature would start is stopped on drop, so a run of this file on such a
    /// build leaves no process behind.
    struct Client {
        child: Child,
        stdin: Option<ChildStdin>,
        lines: Receiver<String>,
        stop_shared: Option<(std::path::PathBuf, std::path::PathBuf)>,
    }

    impl Drop for Client {
        fn drop(&mut self) {
            drop(self.stdin.take());
            let _ = self.child.kill();
            let _ = self.child.wait();
            if let Some((graph, home)) = self.stop_shared.take() {
                let mut cmd = Command::new(bin());
                cmd.args(["--graph-path", graph.to_str().unwrap(), "--stop-shared"]);
                isolate(&mut cmd, &home);
                let _ = cmd.output();
            }
        }
    }

    impl Client {
        fn start(r: &Rig, args: &[&str]) -> Client {
            let mut cmd = Command::new(bin());
            cmd.current_dir(&r.dir).args(args);
            isolate(&mut cmd, &r.home);
            let mut child = cmd
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn reflow2-mcp");
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
                stop_shared: args
                    .contains(&"--shared")
                    .then(|| (r.graph.clone(), r.home.clone())),
            }
        }

        fn send(&mut self, v: serde_json::Value) {
            let stdin = self.stdin.as_mut().expect("stdin open");
            writeln!(stdin, "{v}").expect("write to the server");
            stdin.flush().unwrap();
        }

        fn reply(&mut self, id: i64) -> serde_json::Value {
            let id = serde_json::json!(id);
            let deadline = Instant::now() + Duration::from_secs(90);
            loop {
                let left = deadline.saturating_duration_since(Instant::now());
                let Ok(line) = self.lines.recv_timeout(left) else {
                    panic!("no reply to request {id}");
                };
                let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
                    continue;
                };
                if v.get("id") == Some(&id) {
                    return v;
                }
            }
        }

        /// The handshake's instructions, and the names of the tools served.
        fn handshake_and_tools(&mut self) -> (String, Vec<String>) {
            self.send(serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"no-disk-test","version":"1"}}}));
            let hello = self.reply(1);
            self.send(serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
            self.send(serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}));
            let listed = self.reply(2);
            let tools = listed["result"]["tools"]
                .as_array()
                .map(|ts| {
                    ts.iter()
                        .filter_map(|t| t["name"].as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            (
                hello["result"]["instructions"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                tools,
            )
        }
    }

    /// The library's two openers — the one every serving mode and the registry
    /// use, and the one the one-shot door uses — and the held-design copy reader.
    #[test]
    fn every_library_opener_refuses_a_store_in_a_folder() {
        use reflow2_mcp::service::ReflowService;
        let r = rig();
        let path = r.graph.to_str().unwrap();

        let serving = ReflowService::new_reporting(path).err().expect(
            "a build without on-disk storage must refuse to open a store, not serve memory",
        );
        assert_refusal(&serving.to_string(), &r.graph);

        let one_shot = ReflowService::new(path)
            .err()
            .expect("the one-shot opener must refuse too");
        assert_refusal(&one_shot.to_string(), &r.graph);

        let copy = r.dir.join("copy");
        let held = ReflowService::reading_a_copy(copy.to_str().unwrap(), path)
            .err()
            .expect("reading a held design's copy opens a store, so it refuses too");
        assert_refusal(&held.to_string(), &copy);

        assert_untouched(&r.dir);
    }

    /// Every file under `dir`, with its bytes, so a refused open can be shown
    /// to have changed nothing in a store that was already there.
    fn snapshot(dir: &Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
        let mut out = std::collections::BTreeMap::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).expect("readable").flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    out.insert(p.clone(), std::fs::read(&p).expect("readable file"));
                }
            }
        }
        out
    }

    /// A store a real reflow2 (v0.76.0, built with RocksDB) wrote, copied into
    /// `dir` — the store a build without the feature must not read as empty.
    fn a_real_store_in(dir: &Path) -> std::path::PathBuf {
        let from = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/legacy-identity/v0.76.0-as-dot/graph");
        let to = dir.join("graph");
        let mut stack = vec![(from, to.clone())];
        while let Some((src, dst)) = stack.pop() {
            std::fs::create_dir_all(&dst).unwrap();
            for e in std::fs::read_dir(&src).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push((p.clone(), dst.join(e.file_name())));
                } else {
                    std::fs::copy(&p, dst.join(e.file_name())).unwrap();
                }
            }
        }
        to
    }

    fn refused_with_exit_one(o: &Output, what: &str, path: &Path) {
        assert_eq!(
            o.status.code(),
            Some(1),
            "{what} must be refused with exit 1, not answered from memory: stdout {} stderr {}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        assert!(o.stdout.is_empty(), "{what} prints no reply");
        assert_refusal(&String::from_utf8_lossy(&o.stderr), path);
    }

    /// The one-shot door, on a store that exists — an export and a read are
    /// refused, not answered as an empty design, and the store is unchanged —
    /// and where none exists yet: an import and a write are refused, and
    /// nothing is created.
    #[test]
    fn the_one_shot_door_refuses_and_creates_nothing() {
        let r = rig();
        let store = a_real_store_in(&r.dir);
        let before = snapshot(&r.dir);
        let at = store.to_str().unwrap();

        let export = run(&r.dir, &r.home, &["--graph-path", at, "--export"]);
        refused_with_exit_one(&export, "--export of a store", &store);
        let read = run(
            &r.dir,
            &r.home,
            &["--graph-path", at, "--call", "graph_report", "--args", "{}"],
        );
        refused_with_exit_one(&read, "a read through --call", &store);
        assert!(
            snapshot(&r.dir) == before,
            "a refused open must leave the store exactly as it was"
        );
        std::fs::remove_dir_all(&store).unwrap();

        // Where no store exists yet, a write would create one.
        let path = r.graph.to_str().unwrap();
        let doc = reflow2_core::DesignGraph::open_in_memory()
            .expect("an in-memory design")
            .export_graph()
            .expect("export it");
        let source = r.home.join("design.json");
        std::fs::write(&source, serde_json::to_string(&doc).unwrap()).unwrap();
        let import = run(
            &r.dir,
            &r.home,
            &["--graph-path", path, "--import", source.to_str().unwrap()],
        );
        refused_with_exit_one(&import, "--import", &r.graph);
        let write = run(
            &r.dir,
            &r.home,
            &[
                "--graph-path",
                path,
                "--call",
                "add_project",
                "--args",
                r#"{"id":"proj:kept","name":"Kept"}"#,
            ],
        );
        refused_with_exit_one(&write, "a write through --call", &r.graph);
        assert_untouched(&r.dir);
    }

    /// A stdio server serves the surface that says why — in its handshake, where
    /// the agent reads it — and never an in-memory design in the store's place.
    #[test]
    fn a_server_says_why_in_band_and_serves_no_design() {
        let r = rig();
        let mut c = Client::start(&r, &["--graph-path", r.graph.to_str().unwrap()]);
        let (instructions, tools) = c.handshake_and_tools();
        assert_refusal(&instructions, &r.graph);
        assert_eq!(
            tools,
            vec!["reflow2_unavailable".to_string()],
            "only the tool that says why is served: no design tool answers from memory"
        );
        drop(c);
        assert_untouched(&r.dir);
    }

    /// A `--shared` client starts no daemon for a store this build cannot open:
    /// it says why in band, and leaves no rendezvous and no store behind.
    #[test]
    fn a_shared_client_starts_nothing_and_says_why() {
        let r = rig();
        let mut c = Client::start(&r, &["--graph-path", r.graph.to_str().unwrap(), "--shared"]);
        let (instructions, tools) = c.handshake_and_tools();
        assert_refusal(&instructions, &r.graph);
        assert_eq!(tools, vec!["reflow2_unavailable".to_string()]);
        drop(c);
        assert_untouched(&r.dir);
    }
}

/// The default build opens stores in folders: the feature is on, and the
/// guard every opener asks first lets the open through.
#[cfg(feature = "rocksdb")]
#[test]
fn the_default_build_opens_a_store_in_a_folder() {
    assert!(reflow2_mcp::opening::STORES_IN_A_FOLDER);
    assert!(
        reflow2_mcp::opening::refuse_without_on_disk_storage("/any/where/graph").is_ok(),
        "the guard refuses nothing in a build with the feature"
    );
}
