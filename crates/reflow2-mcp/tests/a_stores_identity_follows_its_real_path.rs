//! A store's identity file follows the store's REAL path, however the path is
//! spelled — and every store that opened before still opens.
//!
//! fact:root-cause-a-stores-identity-file-is-placed-by-how-its-path-was-spelled-2026-10-05,
//! measured on 0.76.0, 0.78.0 and main: the identity file (`<graph>.id.json`)
//! was placed beside the path AS TYPED. A store first opened through a symlink
//! got `<link>.id.json` beside the LINK; `--graph-path .` put `..id.json`
//! INSIDE the store. The next open by the real path was refused as having lost
//! its identity, and that refused open still wrote `graph.meta.json` — and
//! rotated the store's own LOG, WAL, MANIFEST and OPTIONS — because the version
//! stamp and the read-write open came before the identity check. In the field
//! (art:vscode-call-field-log-2026-10-05) that left a store unopenable with a
//! freshly touched `graph.meta.json` and no `graph.id.json`.
//!
//! The owner's constraint, verbatim (Anthony, 2026-10-05): "I just want to make
//! sure the changes we implement for vscode don't break it for how it was
//! originally used/designed as an mcp server. My brother likes using as is".
//! So the stores in `tests/fixtures/legacy-identity/` were WRITTEN BY THE
//! RELEASED reflow2 0.76.0 binary (the field reporter's version), one through a
//! symlink and one as `--graph-path .`, exactly as it left them — minus the
//! info `LOG`, the `LOCK` file, and the client record and usage ledger (which
//! carry a host name). They pin that a store an older reflow2 made still opens.
//!
//! How they were made, 2026-10-05, with the published 0.76.0 asset:
//!
//! ```text
//! mkdir -p sym/proj/.reflow2/graph sym/stores && ln -s ../proj/.reflow2/graph sym/stores/bq
//! (cd sym && reflow2-mcp --graph-path stores/bq --call add_requirement --args \
//!   '{"id":"req:legacy-symlink","name":"Made through a symlink","statement":"…"}')
//! mkdir -p dot/proj/.reflow2/graph && cd dot/proj/.reflow2/graph && \
//!   reflow2-mcp --graph-path . --call add_requirement --args \
//!   '{"id":"req:legacy-dot","name":"Made as dot","statement":"…"}'
//! ```
//!
//! The store directories are committed as `graph/` and laid out again at
//! `proj/.reflow2/graph` by each test, because `.reflow2/` is git-ignored.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use reflow2_core::DesignGraph;
use reflow2_core::identity;
use reflow2_core::nodes::node;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

fn tmp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "reflow2-identity-path-{}-{}-{name}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/legacy-identity")
        .join(name)
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// `reflow2-mcp --graph-path <graph_path> --call <tool>` run from `cwd`.
fn call(cwd: &Path, graph_path: &str, tool: &str, args: &str) -> Output {
    let mut cmd = Command::new(bin());
    cmd.current_dir(cwd).args(["--graph-path", graph_path]);
    if tool.starts_with("add_") {
        cmd.arg("--no-export");
    }
    cmd.args(["--call", tool, "--args", args])
        .output()
        .expect("the binary runs")
}

fn text(out: &Output) -> (String, String) {
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[track_caller]
fn assert_reads(out: &Output, needle: &str) {
    let (stdout, stderr) = text(out);
    assert!(
        out.status.success() && stdout.contains(needle),
        "expected the call to succeed and answer with {needle:?}\nexit: {:?}\nstdout: \
         {stdout}\nstderr: {stderr}",
        out.status.code()
    );
}

/// Every entry under `root` — files, directories and links, not followed —
/// with its size, modification time and, for a file, its bytes.
fn folder_state(root: &Path) -> BTreeMap<PathBuf, (String, u64, i128, Vec<u8>)> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, (String, u64, i128, Vec<u8>)>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            let meta = std::fs::symlink_metadata(&path).unwrap();
            let mtime = meta
                .modified()
                .unwrap()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as i128;
            let rel = path.strip_prefix(root).unwrap().to_path_buf();
            let kind = if meta.is_dir() {
                "dir"
            } else if meta.file_type().is_symlink() {
                "link"
            } else {
                "file"
            };
            let bytes = if kind == "file" {
                std::fs::read(&path).unwrap()
            } else {
                Vec::new()
            };
            out.insert(rel, (kind.to_string(), meta.len(), mtime, bytes));
            if kind == "dir" {
                walk(root, &path, out);
            }
        }
    }
    let mut out = BTreeMap::new();
    let meta = std::fs::metadata(root).unwrap();
    out.insert(
        PathBuf::from("."),
        (
            "dir".into(),
            0,
            meta.modified()
                .unwrap()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as i128,
            Vec::new(),
        ),
    );
    walk(root, root, &mut out);
    out
}

#[track_caller]
fn assert_untouched(
    before: &BTreeMap<PathBuf, (String, u64, i128, Vec<u8>)>,
    after: &BTreeMap<PathBuf, (String, u64, i128, Vec<u8>)>,
) {
    let mut changed = Vec::new();
    for (path, was) in before {
        match after.get(path) {
            None => changed.push(format!("removed: {}", path.display())),
            Some(now) if now != was => changed.push(format!(
                "changed: {} ({} bytes, mtime {} → {} bytes, mtime {})",
                path.display(),
                was.1,
                was.2,
                now.1,
                now.2
            )),
            _ => {}
        }
    }
    for path in after.keys() {
        if !before.contains_key(path) {
            changed.push(format!("created: {}", path.display()));
        }
    }
    assert!(
        changed.is_empty(),
        "a refused open must leave the folder byte-identical, and it changed:\n  {}",
        changed.join("\n  ")
    );
}

/// A store holding data under a minted id, with its identity file removed —
/// the store a refused open is asked about. Returns the store path and the id.
fn a_store_that_lost_its_identity_file(dir: &Path) -> (PathBuf, String) {
    let store = dir.join("proj/.reflow2/graph");
    std::fs::create_dir_all(store.parent().unwrap()).unwrap();
    let id = {
        let mut g = DesignGraph::open_rocksdb(store.to_str().unwrap()).unwrap();
        g.add_project("proj:lost", "Lost").unwrap();
        g.add_requirement(
            "req:lost",
            "Lost",
            "A design whose identity file went missing.",
        )
        .unwrap();
        g.graph_id().to_string()
    };
    std::fs::remove_file(dir.join("proj/.reflow2/graph.id.json")).unwrap();
    (store, id)
}

// ---------------------------------------------------------------------------
// New stores: one identity file, beside the store's real directory.
// ---------------------------------------------------------------------------

#[test]
fn a_store_first_opened_through_a_symlink_opens_by_its_real_path() {
    let dir = tmp("symlink-first");
    let store = dir.join("proj/.reflow2/graph");
    std::fs::create_dir_all(&store).unwrap();
    std::fs::create_dir_all(dir.join("stores")).unwrap();
    std::os::unix::fs::symlink(&store, dir.join("stores/bq")).unwrap();
    let link = dir.join("stores/bq");

    let id = {
        let mut g = DesignGraph::open_rocksdb(link.to_str().unwrap()).unwrap();
        g.add_requirement(
            "req:through-a-link",
            "Through a link",
            "Written through a symlink.",
        )
        .unwrap();
        g.graph_id().to_string()
    };

    assert!(
        dir.join("proj/.reflow2/graph.id.json").exists(),
        "the identity file belongs beside the store's real directory"
    );
    assert!(
        !dir.join("stores/bq.id.json").exists(),
        "and not beside the link it was reached through"
    );

    let by_real_path = DesignGraph::open_rocksdb(store.to_str().unwrap())
        .unwrap_or_else(|e| panic!("opening by the real path was refused: {e}"));
    assert_eq!(by_real_path.graph_id(), id, "one store, one identity");
    assert!(
        by_real_path
            .get_node(node::REQUIREMENT, "req:through-a-link")
            .unwrap()
            .is_some(),
        "and its design is there"
    );
    drop(by_real_path);
    let through_the_link = DesignGraph::open_rocksdb(link.to_str().unwrap()).unwrap();
    assert_eq!(through_the_link.graph_id(), id);
    assert!(
        through_the_link.identity_on_open().is_none(),
        "an ordinary open says nothing about identity"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_store_opened_as_dot_keeps_its_identity_beside_the_store() {
    let dir = tmp("dot");
    let store = dir.join("proj/.reflow2/graph");
    std::fs::create_dir_all(&store).unwrap();

    let wrote = call(
        &store,
        ".",
        "add_requirement",
        r#"{"id":"req:as-dot","name":"As dot","statement":"Written with --graph-path dot."}"#,
    );
    assert!(wrote.status.success(), "{:?}", text(&wrote));

    assert!(
        dir.join("proj/.reflow2/graph.id.json").exists(),
        "`.` resolves to the store, so the identity file goes beside it"
    );
    assert!(
        !store.join("..id.json").exists(),
        "and never inside the store, where 0.79.0 and earlier put it"
    );

    assert_reads(
        &call(
            &dir,
            "proj/.reflow2/graph",
            "get_node",
            r#"{"id":"req:as-dot"}"#,
        ),
        "Written with --graph-path dot.",
    );
    assert_reads(
        &call(&store, ".", "get_node", r#"{"id":"req:as-dot"}"#),
        "Written with --graph-path dot.",
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_store_opened_by_its_plain_path_behaves_exactly_as_before() {
    // The way reflow2 is used as an MCP server — `--graph-path .reflow2/graph`
    // — must be exactly what it was: the same two files beside the store, the
    // same identity record, the same id on every reopen, and nothing said.
    let dir = tmp("plain");
    let store = dir.join("proj/.reflow2/graph");
    std::fs::create_dir_all(store.parent().unwrap()).unwrap();

    let id = {
        let mut g = DesignGraph::open_rocksdb(store.to_str().unwrap()).unwrap();
        g.add_project("proj:plain", "Plain").unwrap();
        assert!(g.identity_on_open().is_none());
        g.graph_id().to_string()
    };
    let mut beside: Vec<String> = std::fs::read_dir(dir.join("proj/.reflow2"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    beside.sort();
    assert_eq!(
        beside,
        ["graph", "graph.id.json", "graph.meta.json"],
        "exactly the files every earlier version left beside the store"
    );
    let record: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join("proj/.reflow2/graph.id.json")).unwrap(),
    )
    .unwrap();
    let mut keys: Vec<&str> = record
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort();
    assert_eq!(keys, ["graph_id", "label", "minted_by", "origin"]);
    assert_eq!(record["graph_id"], id.as_str());
    assert_eq!(record["label"], "proj");
    assert_eq!(record["origin"], "minted");
    assert!(
        std::fs::read_dir(&store).unwrap().all(|e| !e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".id.json")),
        "nothing about identity is written inside the store"
    );

    let reopened = DesignGraph::open_rocksdb(store.to_str().unwrap()).unwrap();
    assert_eq!(reopened.graph_id(), id);
    assert!(reopened.identity_on_open().is_none());
    drop(reopened);

    let status = call(&dir, "proj/.reflow2/graph", "loop_status", "{}");
    assert!(status.status.success(), "{:?}", text(&status));
    let (stdout, stderr) = text(&status);
    assert!(
        !stdout.contains("identity_on_open") && !stderr.contains("identity file"),
        "an ordinary open says nothing about identity:\n{stdout}\n{stderr}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

// ---------------------------------------------------------------------------
// Stores an older reflow2 made: they still open, and now by the real path too.
// ---------------------------------------------------------------------------

/// The 0.76.0 symlink fixture laid out at `dir`: the store at
/// `proj/.reflow2/graph`, a link to it at `stores/bq`, and the identity file
/// and version stamp 0.76.0 wrote beside the LINK.
fn lay_out_the_symlink_fixture(dir: &Path) -> PathBuf {
    let fx = fixture("v0.76.0-through-a-symlink");
    copy_tree(&fx.join("graph"), &dir.join("proj/.reflow2/graph"));
    std::fs::create_dir_all(dir.join("stores")).unwrap();
    std::os::unix::fs::symlink("../proj/.reflow2/graph", dir.join("stores/bq")).unwrap();
    for f in ["bq.id.json", "bq.meta.json"] {
        std::fs::copy(fx.join("link-side").join(f), dir.join("stores").join(f)).unwrap();
    }
    dir.join("stores/bq.id.json")
}

#[test]
fn a_store_reflow2_0_76_0_made_through_a_symlink_still_opens_and_now_by_its_real_path() {
    let dir = tmp("legacy-symlink");
    let legacy = lay_out_the_symlink_fixture(&dir);
    let legacy_bytes = std::fs::read(&legacy).unwrap();
    let beside_store = dir.join("proj/.reflow2/graph.id.json");
    assert!(!beside_store.exists(), "the fixture is the 0.76.0 layout");

    // Through the link, as it was always opened: it opens, as it did before.
    let first = call(
        &dir,
        "stores/bq",
        "get_node",
        r#"{"id":"req:legacy-symlink"}"#,
    );
    assert_reads(&first, "written by reflow2 0.76.0 through a symlink");
    let (_, stderr) = text(&first);
    assert!(
        stderr.contains("was found only at")
            && stderr.contains("A copy was written beside the store"),
        "an identity found only where an older reflow2 put it is SAID, once: {stderr}"
    );
    assert_eq!(
        std::fs::read(&beside_store).unwrap(),
        legacy_bytes,
        "the copy beside the store is the old file, byte for byte"
    );
    assert_eq!(
        std::fs::read(&legacy).unwrap(),
        legacy_bytes,
        "and the old file is left exactly where and as it was"
    );

    // By the real path — refused on 0.76.0 through 0.79.0.
    let by_real_path = call(
        &dir,
        "proj/.reflow2/graph",
        "get_node",
        r#"{"id":"req:legacy-symlink"}"#,
    );
    assert_reads(&by_real_path, "written by reflow2 0.76.0 through a symlink");
    let (_, stderr) = text(&by_real_path);
    assert!(
        !stderr.contains("was found only at"),
        "said once: the next open finds the file beside the store: {stderr}"
    );

    // And through the link again, now reading the file beside the store.
    let again = call(
        &dir,
        "stores/bq",
        "get_node",
        r#"{"id":"req:legacy-symlink"}"#,
    );
    assert_reads(&again, "written by reflow2 0.76.0 through a symlink");
    assert!(!text(&again).1.contains("was found only at"));
    assert_eq!(std::fs::read(&legacy).unwrap(), legacy_bytes);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_store_reflow2_0_76_0_made_as_dot_opens_by_its_real_path_and_as_dot() {
    let dir = tmp("legacy-dot");
    let store = dir.join("proj/.reflow2/graph");
    copy_tree(&fixture("v0.76.0-as-dot").join("graph"), &store);
    let inside = store.join("..id.json");
    let inside_bytes = std::fs::read(&inside).unwrap();

    // By the real path FIRST — refused on 0.76.0 through 0.79.0, because the
    // identity file was inside the store.
    let by_real_path = call(
        &dir,
        "proj/.reflow2/graph",
        "get_node",
        r#"{"id":"req:legacy-dot"}"#,
    );
    assert_reads(&by_real_path, "with --graph-path . from inside the store");
    assert!(
        text(&by_real_path).1.contains("inside the store directory"),
        "{}",
        text(&by_real_path).1
    );
    assert_eq!(
        std::fs::read(dir.join("proj/.reflow2/graph.id.json")).unwrap(),
        inside_bytes,
        "copied beside the store, byte for byte"
    );

    // As `.`, the way it was made: still opens.
    assert_reads(
        &call(&store, ".", "get_node", r#"{"id":"req:legacy-dot"}"#),
        "with --graph-path . from inside the store",
    );
    assert_eq!(
        std::fs::read(&inside).unwrap(),
        inside_bytes,
        "the file 0.76.0 left inside the store is never moved or rewritten"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_store_reflow2_0_76_0_made_as_dot_still_opens_as_dot_first() {
    // The other order: opened the way it always was, before anything else.
    let dir = tmp("legacy-dot-first");
    let store = dir.join("proj/.reflow2/graph");
    copy_tree(&fixture("v0.76.0-as-dot").join("graph"), &store);
    assert_reads(
        &call(&store, ".", "get_node", r#"{"id":"req:legacy-dot"}"#),
        "with --graph-path . from inside the store",
    );
    assert_reads(
        &call(
            &dir,
            "proj/.reflow2/graph",
            "get_node",
            r#"{"id":"req:legacy-dot"}"#,
        ),
        "with --graph-path . from inside the store",
    );
    std::fs::remove_dir_all(&dir).ok();
}

// ---------------------------------------------------------------------------
// A refused open writes nothing, and says where it looked and what to do.
// ---------------------------------------------------------------------------

#[test]
fn a_refused_open_leaves_the_folder_byte_identical() {
    let dir = tmp("refused-untouched");
    let (store, _) = a_store_that_lost_its_identity_file(&dir);
    let folder = dir.join("proj/.reflow2");
    let before = folder_state(&folder);

    // The door a person meets, then the core open every door goes through.
    let refused = call(
        &dir,
        "proj/.reflow2/graph",
        "get_node",
        r#"{"id":"req:lost"}"#,
    );
    assert!(!refused.status.success(), "{:?}", text(&refused));
    assert!(DesignGraph::open_rocksdb(store.to_str().unwrap()).is_err());

    assert_untouched(&before, &folder_state(&folder));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_refusal_names_where_it_looked_and_the_id_the_store_holds_and_that_id_recovers_it() {
    let dir = tmp("refusal-says");
    let (store, id) = a_store_that_lost_its_identity_file(&dir);
    let message = match DesignGraph::open_rocksdb(store.to_str().unwrap()) {
        Ok(_) => panic!("a store that lost its identity file must not open"),
        Err(e) => e.to_string(),
    };
    let beside = identity::identity_path(store.to_str().unwrap());
    let real = std::fs::canonicalize(&store).unwrap();
    for needle in [
        "will not guess".to_string(),
        "It looked in these places".to_string(),
        beside.display().to_string(),
        real.join("..id.json").display().to_string(),
        "put the design's id file beside the store".to_string(),
        format!("all under the design id `{id}`"),
        "beside that LINK".to_string(),
        "Nothing was written".to_string(),
    ] {
        assert!(
            message.contains(&needle),
            "missing {needle:?} in:\n{message}"
        );
    }

    // The recovery it names works: an identity file naming that id, beside
    // the store, opens the design with its data.
    std::fs::write(
        &beside,
        format!(
            r#"{{"graph_id": "{id}", "label": "proj", "origin": "minted", "minted_by": "0.0.0"}}"#
        ),
    )
    .unwrap();
    let g = DesignGraph::open_rocksdb(store.to_str().unwrap()).unwrap();
    assert_eq!(g.graph_id(), id);
    assert!(g.get_node(node::REQUIREMENT, "req:lost").unwrap().is_some());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_field_case_a_store_made_through_a_symlink_opened_by_its_real_path_first() {
    // What the work machine met: a store an older reflow2 made through a
    // symlink, opened by its real path, which cannot see the link. Refused —
    // there is no way to know the link exists — but now with the folder left
    // as it was, the store's own id named, and where to look; and once opened
    // through the link (or the file copied by hand), it opens by its real path.
    let dir = tmp("field-case");
    lay_out_the_symlink_fixture(&dir);
    let legacy_id = {
        let v: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join("stores/bq.id.json")).unwrap()).unwrap();
        v["graph_id"].as_str().unwrap().to_string()
    };
    let folder = dir.join("proj/.reflow2");
    let before = folder_state(&folder);

    let refused = call(
        &dir,
        "proj/.reflow2/graph",
        "get_node",
        r#"{"id":"req:legacy-symlink"}"#,
    );
    let (_, stderr) = text(&refused);
    assert!(!refused.status.success(), "{stderr}");
    assert!(
        stderr.contains(&format!("`{legacy_id}`")) && stderr.contains("beside that LINK"),
        "the refusal names the id the store holds and where an older reflow2 put the file: \
         {stderr}"
    );
    assert_untouched(&before, &folder_state(&folder));

    assert_reads(
        &call(
            &dir,
            "stores/bq",
            "get_node",
            r#"{"id":"req:legacy-symlink"}"#,
        ),
        "through a symlink",
    );
    assert_reads(
        &call(
            &dir,
            "proj/.reflow2/graph",
            "get_node",
            r#"{"id":"req:legacy-symlink"}"#,
        ),
        "through a symlink",
    );
    std::fs::remove_dir_all(&dir).ok();
}

// ---------------------------------------------------------------------------
// The one exception, made loud.
// ---------------------------------------------------------------------------

#[test]
fn two_identity_files_naming_different_designs_open_the_one_beside_the_store_and_say_so() {
    // Two identity files for one store that name DIFFERENT designs — only a
    // reflow2 older than the lost-identity refusal, or a hand edit, could
    // leave that. The file beside the store comes first (one store, one
    // identity, however it is reached); the other is reported, not used, and
    // neither is touched.
    let dir = tmp("disagree");
    let store = dir.join("proj/.reflow2/graph");
    std::fs::create_dir_all(store.parent().unwrap()).unwrap();
    let id = {
        let mut g = DesignGraph::open_rocksdb(store.to_str().unwrap()).unwrap();
        g.add_requirement("req:beside", "Beside", "The design beside the store.")
            .unwrap();
        g.graph_id().to_string()
    };
    std::fs::create_dir_all(dir.join("stores")).unwrap();
    std::os::unix::fs::symlink(&store, dir.join("stores/bq")).unwrap();
    let other = r#"{"graph_id": "0000000000000bad", "label": "proj", "origin": "minted", "minted_by": "0.20.0"}"#;
    std::fs::write(dir.join("stores/bq.id.json"), other).unwrap();
    let beside_bytes = std::fs::read(dir.join("proj/.reflow2/graph.id.json")).unwrap();

    let g = DesignGraph::open_rocksdb(dir.join("stores/bq").to_str().unwrap()).unwrap();
    assert_eq!(g.graph_id(), id, "the file beside the store comes first");
    let note = g
        .identity_on_open()
        .expect("two names for one store is said, not passed over");
    assert!(note.needs_attention);
    assert_eq!(note.disagreeing.len(), 1);
    assert_eq!(note.disagreeing[0].1, "0000000000000bad");
    drop(g);

    let status = call(&dir, "stores/bq", "loop_status", "{}");
    let (stdout, _) = text(&status);
    let reply: serde_json::Value = serde_json::from_str(&stdout).expect("loop_status JSON");
    assert!(
        reply["next"].as_array().is_some_and(|n| n
            .iter()
            .any(|s| s.as_str().is_some_and(|s| s.contains("0000000000000bad")))),
        "and it is in `next`, where an agent acts on it: {stdout}"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("stores/bq.id.json")).unwrap(),
        other
    );
    assert_eq!(
        std::fs::read(dir.join("proj/.reflow2/graph.id.json")).unwrap(),
        beside_bytes
    );
    std::fs::remove_dir_all(&dir).ok();
}
