//! A read of a held design is as true as a read of the live one.
//!
//! FOUND 2026-10-02 (fact:root-cause-a-held-design-answers-search-through-the-door-with-a-false-nothing-matched-2026-10-02),
//! triaging a VS Code session that drove reflow2 through `--call` while a Claude
//! Code session held the same design with `--serve-shared`. A read-only tool on
//! a held design is answered from a COPY of the store, and the copy left out
//! the nested `fulltext/` index directory, so it opened onto an empty index:
//! `search_design` answered `{"hits": []}` and `topic_report` "NOTHING MATCHED …
//! across 2 node(s)", exit 0, for a word one of those two nodes held. The only
//! warning was the generic one about unflushed writes, which names staleness,
//! not blindness.
//!
//! THE CLASS IS A READ ANSWERED FROM A SOURCE THAT LACKS WHAT THE READ DEPENDS
//! ON, and the index was not the only thing the copy lacked. Measured the same
//! day on 0.77.0 with a registered file and an export on record: the copy was
//! opened at its own path in the temporary directory, so every read that finds
//! something FROM the design's path looked there. `loop_status` measured the
//! registered file under `/tmp`, reported it MISSING and told the agent to
//! record a disposition for it; `sync_status` said there was no export to
//! check; `wall_check` read `/tmp` as the project.
//!
//! So these tests do not pick tools. [`every_read_answers_the_same_held_as_live`]
//! takes the read-only tools from the served list — the list `--call` itself
//! uses to decide a held read may be answered — and asks each the same
//! question with the design free and with it held. The search index's only
//! entry point is `DesignGraph::search_design`; its read callers are the
//! `search_design` and `topic_report` tools (its other callers are the capture
//! guard inside the constructors, which write and so refuse on a held design).
//! Both take only a `query`, so both are in the probed set, and the test
//! asserts that rather than assuming it.

use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

/// A scratch project: `<root>/.reflow2/graph`, a source file registered
/// against the design, an export on record, and a HOME of its own so nothing
/// here reads or writes the developer's.
struct Project {
    _dir: tempfile::TempDir,
    root: PathBuf,
    graph: PathBuf,
    home: PathBuf,
}

impl Project {
    fn call(&self, tool: &str, args: &str) -> Output {
        Command::new(bin())
            .current_dir(&self.root)
            .env("HOME", &self.home)
            .args([
                "--graph-path",
                self.graph.to_str().unwrap(),
                "--call",
                tool,
                "--args",
                args,
            ])
            .output()
            .expect("the binary runs")
    }

    fn ok(&self, tool: &str, args: &str) -> serde_json::Value {
        let o = self.call(tool, args);
        assert!(
            o.status.success(),
            "`{tool}` failed: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        stdout_json(&o)
    }

    /// The records beside the store, by name, with their bytes — leaving out
    /// the holder's own rendezvous and log, which are the holder's to write
    /// (and which it publishes a moment after it takes the store).
    fn sidecars(&self) -> Vec<(String, Vec<u8>)> {
        let dir = self.graph.parent().unwrap();
        let mut out: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .filter(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                !name.ends_with(".server.json") && !name.ends_with(".server.log")
            })
            .map(|e| {
                (
                    e.file_name().to_string_lossy().into_owned(),
                    std::fs::read(e.path()).unwrap_or_default(),
                )
            })
            .collect();
        out.sort();
        out
    }
}

fn stdout_json(o: &Output) -> serde_json::Value {
    let text = String::from_utf8_lossy(&o.stdout);
    serde_json::from_str(&text).unwrap_or_else(|e| {
        panic!(
            "stdout is not one JSON document ({e}):\n{text}\nstderr:\n{}",
            String::from_utf8_lossy(&o.stderr)
        )
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    let d = sha2::Sha256::digest(bytes);
    d.iter().map(|b| format!("{b:02x}")).collect()
}

fn seeded() -> Project {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("zoo");
    let home = dir.path().join("home");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("docs/design")).unwrap();
    std::fs::create_dir_all(&home).unwrap();
    let graph = root.join(".reflow2/graph");
    let p = Project {
        _dir: dir,
        root,
        graph,
        home,
    };
    let source = b"fn main() {}\n";
    std::fs::write(p.root.join("src/main.rs"), source).unwrap();

    p.ok("add_project", r#"{"id":"proj:zoo","name":"Zoo"}"#);
    p.ok(
        "add_requirement",
        r#"{"id":"req:stripes","name":"The enclosure shows a zebra pattern","statement":"Visitors see the zebra pattern from the path."}"#,
    );
    p.ok(
        "add_requirement",
        r#"{"id":"req:water","name":"The enclosure has water","statement":"A trough is filled daily."}"#,
    );
    p.ok(
        "add_artifact",
        &serde_json::json!({
            "id": "art:main",
            "name": "main",
            "location": "src/main.rs",
            "artifact_type": "code",
            "checksum": sha256_hex(source),
        })
        .to_string(),
    );
    p.ok("export_graph", r#"{"path":"docs/design/zoo.json"}"#);
    p
}

/// A `--serve-shared` server holding the design — the field setup: one
/// session's server holds it, and a second harness reaches it through
/// `--call`. Stopped, and its lock released, when dropped.
struct Holder<'a> {
    project: &'a Project,
    child: Child,
}

impl Drop for Holder<'_> {
    fn drop(&mut self) {
        let _ = Command::new(bin())
            .env("HOME", &self.project.home)
            .args([
                "--graph-path",
                self.project.graph.to_str().unwrap(),
                "--stop-shared",
            ])
            .output();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn hold(p: &Project) -> Holder<'_> {
    let child = Command::new(bin())
        .current_dir(&p.root)
        .env("HOME", &p.home)
        .args(["--graph-path", p.graph.to_str().unwrap(), "--serve-shared"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn the holder");
    let holder = Holder { project: p, child };
    // Held means the door answers a read from a copy, and says so: wait for
    // exactly that, with a read, so the probe cannot change the design the
    // reads below compare.
    for _ in 0..300 {
        let o = p.call("get_node", r#"{"id":"proj:zoo"}"#);
        if String::from_utf8_lossy(&o.stderr).contains("SNAPSHOT") {
            return holder;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    panic!("the --serve-shared server never took the store");
}

/// THE MEASURED INSTANCE, and it fails on main: a word the design holds is
/// found through `--call` while another process holds the design.
#[test]
fn a_held_design_finds_by_search_and_topic_what_it_holds() {
    let p = seeded();
    // The live design's own answer first: how many nodes a search runs over.
    let live = p.ok("search_design", r#"{"query":"zebra pattern"}"#);
    let searchable = live["searched"].as_u64().unwrap_or(0);
    let _held = hold(&p);

    let o = p.call("search_design", r#"{"query":"zebra pattern"}"#);
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(o.status.success(), "{err}");
    assert!(
        err.contains("SNAPSHOT"),
        "a held read still says it reads a copy: {err}"
    );
    let found = stdout_json(&o);
    assert_eq!(
        found["hits"][0]["node_id"], "req:stripes",
        "the word the design holds is found from the copy, not answered with an empty list: \
         {found}"
    );
    assert!(
        searchable >= 3,
        "the live design searches its project and both requirements at least: {live}"
    );
    assert_eq!(
        found["searched"], searchable,
        "the copy searches as many nodes as the live design does, and says so: {found}"
    );

    let topic = p.ok("topic_report", r#"{"query":"zebra pattern"}"#);
    assert_eq!(topic["count"], 1, "{topic}");
    assert_eq!(topic["searched"], searchable, "{topic}");

    // A true miss on the copy says how many nodes it searched, so it cannot
    // read like the empty answer this test exists for.
    let miss = p.ok("topic_report", r#"{"query":"okapi"}"#);
    let line = miss["not_found"].as_str().unwrap_or_default();
    assert!(
        line.contains("NOTHING MATCHED")
            && line.contains(&format!("in the {searchable} node(s) searched")),
        "{line}"
    );
}

/// The existing snapshot behaviour stays as it was: a read answers, a write
/// refuses, and the read writes nothing beside the held store — those records
/// are the holder's, and a second writer would race it.
#[test]
fn a_held_read_answers_a_held_write_refuses_and_nothing_is_written_beside_the_store() {
    let p = seeded();
    let _held = hold(&p);
    let before = p.sidecars();

    let node = p.ok("get_node", r#"{"id":"req:stripes"}"#);
    assert_eq!(node["node"]["node_id"], "req:stripes", "{node}");
    p.ok("loop_status", "{}");
    p.ok("sync_status", "{}");
    p.ok("search_design", r#"{"query":"zebra"}"#);
    // export_graph only READS the design; the file it writes is the caller's,
    // and the sync record it would update beside the store is the holder's.
    let receipt = p.ok(
        "export_graph",
        r#"{"path":"docs/design/from-the-copy.json"}"#,
    );
    assert!(receipt.is_object(), "{receipt}");

    let o = p.call(
        "add_requirement",
        r#"{"id":"req:second","name":"Second","statement":"x"}"#,
    );
    assert_eq!(
        o.status.code(),
        Some(1),
        "a write on a held design refuses: {}",
        String::from_utf8_lossy(&o.stdout)
    );

    let after = p.sidecars();
    let names = |v: &[(String, Vec<u8>)]| v.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>();
    assert_eq!(
        names(&before),
        names(&after),
        "no record appeared or vanished beside the held store"
    );
    for ((name, was), (_, now)) in before.iter().zip(after.iter()) {
        assert!(
            was == now,
            "{name} changed beside the held store during reads through the door"
        );
    }
}

/// Keys that legitimately differ between two processes answering the same
/// question about the same design: who is asking (a seat is minted per
/// process), and whether the answering service may write (a copy may not —
/// that difference is the point, and it is reported, not hidden).
const PER_PROCESS: &[&str] = &["seat", "read_only"];

fn normalised(mut v: serde_json::Value) -> serde_json::Value {
    fn walk(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(m) => {
                for k in PER_PROCESS {
                    m.remove(*k);
                }
                m.values_mut().for_each(walk);
            }
            serde_json::Value::Array(a) => a.iter_mut().for_each(walk),
            _ => {}
        }
    }
    walk(&mut v);
    v
}

/// Arguments that ask `tool` a real question of the seeded design, from its
/// published schema — or `None` when it requires something this design cannot
/// supply generically. Free text gets the word the design holds; an id gets a
/// node the design holds.
fn probe_args(schema: &serde_json::Map<String, serde_json::Value>) -> Option<serde_json::Value> {
    let required: Vec<&str> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    let mut args = serde_json::Map::new();
    for name in required {
        let value = match name {
            "query" => serde_json::json!("zebra pattern"),
            "id" | "node_id" => serde_json::json!("req:stripes"),
            _ => return None,
        };
        args.insert(name.to_string(), value);
    }
    Some(serde_json::Value::Object(args))
}

/// Answer = exit code plus the reply (or the refusal's sentence).
fn answer(o: &Output, tool: &str) -> (Option<i32>, serde_json::Value) {
    let code = o.status.code();
    if code == Some(0) || code == Some(2) {
        return (code, normalised(stdout_json(o)));
    }
    let err = String::from_utf8_lossy(&o.stderr);
    let refusal = err
        .lines()
        .find(|l| l.contains(&format!("`{tool}` refused")) || l.starts_with("Error"))
        .unwrap_or_default()
        .to_string();
    (code, serde_json::Value::String(refusal))
}

/// THE CLASS: every read the door will answer from a copy, asked the same
/// question with the design free and with it held, answers the same.
#[tokio::test(flavor = "multi_thread")]
async fn every_read_answers_the_same_held_as_live() {
    let served = reflow2_mcp::service::ReflowService::in_memory()
        .expect("an in-memory service")
        .tools_with_lessons()
        .await;
    let mut probed: Vec<(String, String)> = Vec::new();
    let mut not_probed: Vec<String> = Vec::new();
    for t in &served {
        let read_only = t
            .annotations
            .as_ref()
            .and_then(|a| a.read_only_hint)
            .unwrap_or(false);
        if !read_only {
            continue;
        }
        match probe_args(&t.input_schema) {
            Some(args) => probed.push((t.name.to_string(), args.to_string())),
            None => not_probed.push(t.name.to_string()),
        }
    }
    for must in [
        "search_design",
        "topic_report",
        "loop_status",
        "sync_status",
    ] {
        assert!(
            probed.iter().any(|(n, _)| n == must),
            "`{must}` reads what a copy can lack and must be in the probed set; probed: {:?}",
            probed.iter().map(|(n, _)| n).collect::<Vec<_>>()
        );
    }

    let p = seeded();
    let live: Vec<_> = probed
        .iter()
        .map(|(tool, args)| answer(&p.call(tool, args), tool))
        .collect();
    let held_answers = {
        let _held = hold(&p);
        probed
            .iter()
            .map(|(tool, args)| answer(&p.call(tool, args), tool))
            .collect::<Vec<_>>()
    };

    let differs: Vec<String> = probed
        .iter()
        .zip(live.iter().zip(held_answers.iter()))
        .filter(|(_, (l, h))| l != h)
        .map(|((tool, args), (l, h))| {
            format!(
                "`{tool}` {args}\n  live: {:?} {}\n  held: {:?} {}",
                l.0,
                l.1.to_string().chars().take(600).collect::<String>(),
                h.0,
                h.1.to_string().chars().take(600).collect::<String>()
            )
        })
        .collect();
    assert!(
        differs.is_empty(),
        "{} of {} read(s) answered differently from a held design than from the live one \
         (not probed, needing arguments this design cannot supply generically: {}):\n{}",
        differs.len(),
        probed.len(),
        not_probed.join(", "),
        differs.join("\n")
    );
}

/// The same class without a holder: a store whose index directory is gone
/// (copied or restored without it) opens onto an index that holds nothing.
/// Through the door it now finds what it holds, and the open that rebuilt the
/// index says so once, in `loop_status`, rather than repairing in silence.
#[test]
fn a_store_without_its_index_finds_its_words_and_says_the_index_was_rebuilt() {
    let p = seeded();
    std::fs::remove_dir_all(p.graph.join("fulltext"))
        .expect("the store keeps its search index in a nested fulltext/ directory");

    let status = p.ok("loop_status", "{}");
    let rebuilt = &status["search_index_rebuilt_on_open"];
    assert_eq!(rebuilt["indexed_before"], 0, "{status}");
    assert!(
        rebuilt["searchable"].as_u64().unwrap_or(0) >= 3,
        "{rebuilt}"
    );

    let found = p.ok("search_design", r#"{"query":"zebra pattern"}"#);
    assert_eq!(found["hits"][0]["node_id"], "req:stripes", "{found}");

    // Rebuilt on disk, so the next open has nothing to report.
    let again = p.ok("loop_status", "{}");
    assert!(
        again.get("search_index_rebuilt_on_open").is_none(),
        "a rebuild is reported by the open that made it, not by every open after: {again}"
    );
}

/// The version stamp beside the store is something a read depends on too: a
/// binary BEHIND the one that wrote a design refuses to open it, rather than
/// show less of the design than it holds. The copy used to leave the stamp out,
/// so a held design skipped that refusal and was read anyway. Here the stamp is
/// made to name a node type this binary has never heard of — exactly what a
/// newer writer's stamp looks like to an older reader.
#[test]
fn a_held_design_written_by_a_newer_reflow2_is_refused_as_the_live_one_is() {
    let p = seeded();
    let _held = hold(&p);
    let stamp = reflow2_core::provenance::stamp_path(p.graph.to_str().unwrap());
    let original = std::fs::read_to_string(&stamp).expect("the store carries a version stamp");
    let mut newer: serde_json::Value = serde_json::from_str(&original).unwrap();
    newer["node_type_names"]
        .as_array_mut()
        .expect("a current stamp names its node types")
        .push(serde_json::json!("ZzTypeFromTheFuture"));
    newer["node_types"] = serde_json::json!(newer["node_types"].as_u64().unwrap() + 1);
    std::fs::write(&stamp, newer.to_string()).unwrap();

    let o = p.call("get_node", r#"{"id":"req:stripes"}"#);
    let err = String::from_utf8_lossy(&o.stderr);
    std::fs::write(&stamp, &original).unwrap();
    assert_eq!(
        o.status.code(),
        Some(1),
        "a held design written by a newer reflow2 is refused, not read: {err}"
    );
    assert!(
        err.contains("ZzTypeFromTheFuture") && err.contains("BEHIND"),
        "the refusal is the one the live store gives, naming what this binary cannot read: {err}"
    );
}

/// A copy that fails part-way leaves nothing behind. Found by CI on this
/// change (2026-10-03): the degraded-server suite found a
/// `reflow2-snapshot-<pid>` directory in the temp dir after the workspace
/// tests. `snapshot_dir` created the copy's directory and then returned at
/// the first file it could not copy, leaving a partial second copy of a
/// design on disk — the thing `GraphSnapshot::cleanup` exists to prevent. The
/// held reads these tests make while a holder is starting are exactly where a
/// copy can fail: an opening RocksDB deletes superseded files while the copy
/// lists and copies them. Here the failure is made deterministic — the info
/// LOG made unreadable — and the temp dir is the test's own.
#[cfg(unix)]
#[test]
fn a_copy_that_fails_part_way_leaves_nothing_behind() {
    use std::os::unix::fs::PermissionsExt;
    let p = seeded();
    let _held = hold(&p);
    let tmp = tempfile::tempdir().expect("tempdir");
    let log = p.graph.join("LOG");
    let was = std::fs::metadata(&log)
        .expect("RocksDB keeps an info LOG in the store directory")
        .permissions();
    std::fs::set_permissions(&log, std::fs::Permissions::from_mode(0o000)).unwrap();
    let readable_anyway = std::fs::read(&log).is_ok();
    let o = Command::new(bin())
        .current_dir(&p.root)
        .env("HOME", &p.home)
        .env("TMPDIR", tmp.path())
        .args([
            "--graph-path",
            p.graph.to_str().unwrap(),
            "--call",
            "get_node",
            "--args",
            r#"{"id":"req:stripes"}"#,
        ])
        .output()
        .expect("the binary runs");
    std::fs::set_permissions(&log, was).unwrap();
    if readable_anyway {
        eprintln!("not measured: this process reads a mode-000 file, so the copy cannot fail here");
        return;
    }
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(
        o.status.code(),
        Some(1),
        "a copy that cannot be made is refused: {err}"
    );
    assert!(err.contains("could not copy"), "{err}");
    let residue: Vec<String> = std::fs::read_dir(tmp.path())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("reflow2-snapshot-"))
        .collect();
    assert!(
        residue.is_empty(),
        "a failed copy left a partial design behind: {residue:?}"
    );
}
