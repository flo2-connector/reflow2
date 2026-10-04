//! `export_graph` writes the ITEM LAYOUT — one file per node and per edge — and
//! every reader takes it, with per-item lineage anchored at the merge-base.
//!
//! Anthony, 2026-10-03: `dec:how-the-saved-design-is-laid-out-so-git-merges-it`
//! (option a) and `dec:the-designs-lineage-is-kept-per-item` (option a), with
//! `taken_at` moved to a git-ignored sidecar
//! (`dec:item-13-checksums-move-to-change-edges-and-main-converts-in-one-pr`,
//! decision 5). Git's ordinary merge over the layout is driven end to end with
//! real branches by `tools/test_item_layout_merges.py`; these pin the server's
//! own seam: what a write writes, what a read reads, and where lineage anchors.
//!
//! Hermetic: pid-scoped scratch directories, the house pattern.

use reflow2_mcp::service::*;
use rmcp::handler::server::wrapper::Parameters;

macro_rules! j {
    ($call:expr) => {
        $call
            .await
            .expect("tool ok")
            .structured_content
            .expect("structured content present")
    };
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("reflow2-item-layout-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn git(dir: &std::path::Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("git {args:?} could not run: {e}"));
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn req(id: &str, statement: &str) -> RequirementReq {
    RequirementReq {
        source: None,
        provenance: None,
        id: id.into(),
        name: Some(id.into()),
        statement: Some(statement.into()),
        distinct_from: None,
        replaces: None,
        status: None,
        approver: None,
        acted_at: None,
        priority: None,
        concern: None,
        kind: None,
    }
}

fn export_to(path: &str) -> ExportGraphToReq {
    ExportGraphToReq {
        path: Some(path.to_string()),
        overwrite: Some(true),
        accept_divergence: None,
    }
}

fn edit(id: &str, statement: &str) -> CreateNodeReq {
    serde_json::from_value(serde_json::json!({
        "node_type": "Requirement",
        "id": id,
        "props": { "name": id, "statement": statement },
    }))
    .expect("create_node request")
}

fn import_from(path: &str) -> ImportGraphReq {
    serde_json::from_value(serde_json::json!({ "path": path })).expect("import request")
}

fn item(dir: &std::path::Path, rel: &str) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(dir.join(rel)).expect("item file"))
        .expect("item json")
}

#[tokio::test]
async fn a_directory_path_is_written_one_file_per_item_and_reads_back_whole() {
    let root = scratch("roundtrip");
    let layout = root.join("design");
    let path = format!("{}/", layout.display());

    let s = ReflowService::in_memory().expect("service");
    j!(s.add_requirement(Parameters(req("req:a", "the first need"))));
    j!(s.add_requirement(Parameters(req("req:b", "the second need"))));
    let receipt = j!(s.export_graph(Parameters(export_to(&path))));
    assert_eq!(receipt["layout"], "items", "{receipt:?}");
    assert_eq!(receipt["wrote"], "created");
    let rel_a = reflow2_core::item_layout::node_rel_path("Requirement", "req:a");
    assert!(layout.join(&rel_a).exists(), "one file per node");
    assert!(layout.join("design.json").exists(), "the stamp file");
    let stamp: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(layout.join("design.json")).unwrap())
            .unwrap();
    let keys: Vec<&str> = stamp
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        vec!["graph_id", "schema_version"],
        "decision 5: the committed stamp is graph_id and schema_version, nothing a PR rewrites"
    );
    assert_eq!(
        std::fs::read_to_string(layout.join(".gitignore")).unwrap(),
        reflow2_core::item_layout::GITIGNORE_TEXT,
        "the layout ignores its own taken_at sidecar"
    );
    assert!(
        item(&layout, &rel_a).get("prev_item_hash").is_none(),
        "a first write has no predecessor"
    );

    // Unchanged: nothing written.
    let again = j!(s.export_graph(Parameters(export_to(&path))));
    assert_eq!(again["wrote"], "unchanged");
    assert_eq!(again["items"]["written"], 0, "{again:?}");

    // A fresh store reads the directory back as the same design.
    let t = ReflowService::in_memory().expect("service");
    let report = j!(t.import_graph(Parameters(import_from(&path))));
    assert!(report.get("integrity_note").is_none(), "{report:?}");
    let back = j!(t.export_graph(Parameters(ExportGraphToReq {
        path: None,
        overwrite: None,
        accept_divergence: None,
    })));
    assert_eq!(back["content_hash"], receipt["content_hash"]);
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_changed_item_names_its_committed_hash_and_nothing_else_is_rewritten() {
    let dir = scratch("anchor");
    git(&dir, &["init", "--quiet", "--initial-branch=main"]);
    git(&dir, &["config", "user.email", "test@example.invalid"]);
    git(&dir, &["config", "user.name", "Test"]);
    let layout = dir.join("docs").join("design").join("demo");
    let path = format!("{}/", layout.display());

    let s = ReflowService::in_memory().expect("service");
    j!(s.add_requirement(Parameters(req("req:a", "as on main"))));
    j!(s.add_requirement(Parameters(req("req:b", "untouched"))));
    j!(s.export_graph(Parameters(export_to(&path))));
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-qm", "main"]);
    let rel_a = reflow2_core::item_layout::node_rel_path("Requirement", "req:a");
    let rel_b = reflow2_core::item_layout::node_rel_path("Requirement", "req:b");
    let at_main = item(&layout, &rel_a)["content_hash"].clone();
    let b_on_main = std::fs::read_to_string(layout.join(&rel_b)).unwrap();
    assert!(
        git(&dir, &["status", "--porcelain"]).is_empty(),
        "the taken_at sidecar is git-ignored, so a committed layout leaves a clean tree"
    );

    git(&dir, &["checkout", "-q", "-b", "feature"]);
    for text in ["first edit", "second edit"] {
        j!(s.create_node(Parameters(edit("req:a", text))));
        let r = j!(s.export_graph(Parameters(export_to(&path))));
        assert!(
            r["chained_from"]
                .as_str()
                .unwrap_or("")
                .starts_with("main@"),
            "{r:?}"
        );
        assert_eq!(
            item(&layout, &rel_a)["prev_item_hash"],
            at_main,
            "every export on the branch chains the item from main's version"
        );
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "-qm", text]);
    }
    assert_eq!(
        std::fs::read_to_string(layout.join(&rel_b)).unwrap(),
        b_on_main,
        "an item the branch never changed keeps main's file byte for byte"
    );
    let sidecar: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(layout.join("taken_at.json")).unwrap())
            .unwrap();
    assert_eq!(sidecar["branch"], "feature", "{sidecar:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn a_directory_holding_something_else_is_refused_and_a_tampered_item_is_named() {
    let root = scratch("refusals");
    let foreign = root.join("not-a-layout");
    std::fs::create_dir_all(&foreign).unwrap();
    std::fs::write(foreign.join("notes.txt"), "mine").unwrap();
    let s = ReflowService::in_memory().expect("service");
    j!(s.add_requirement(Parameters(req("req:a", "a need"))));
    let err = s
        .export_graph(Parameters(export_to(&format!("{}/", foreign.display()))))
        .await
        .expect_err("a directory with somebody's files in it is not reflow2's to write into");
    assert!(err.message.contains("notes.txt"), "{}", err.message);
    assert_eq!(
        std::fs::read_dir(&foreign).unwrap().count(),
        1,
        "nothing was written"
    );

    let layout = root.join("design");
    let path = format!("{}/", layout.display());
    j!(s.export_graph(Parameters(export_to(&path))));
    let rel = reflow2_core::item_layout::node_rel_path("Requirement", "req:a");
    let text = std::fs::read_to_string(layout.join(&rel)).unwrap();
    std::fs::write(layout.join(&rel), text.replace("a need", "a need, by hand")).unwrap();
    let t = ReflowService::in_memory().expect("service");
    let report = j!(t.import_graph(Parameters(import_from(&path))));
    let note = report["integrity_note"].as_str().unwrap_or("");
    assert!(
        note.contains(&rel),
        "the tampered item is named: {report:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// THE MIGRATION PATH for a project whose design is still one file — every
/// project but reflow2 itself today, and every design flo2.io holds. The old
/// single file still imports as it always did; exported to the directory
/// BESIDE it (`demo.json` → `demo/`), the same design becomes the layout, its
/// stamp records the single file's hash as `migrated_from`, and nothing is
/// lost either way round: the layout reads back to the single file's hash,
/// and a layout read back out as a single file is that file's design again.
#[tokio::test]
async fn a_single_file_design_still_imports_and_converts_to_the_layout_beside_it() {
    let root = scratch("migrate");
    let design_dir = root.join("docs").join("design");
    std::fs::create_dir_all(&design_dir).unwrap();
    let single = design_dir.join("demo.json");
    let single_path = single.display().to_string();
    let layout = design_dir.join("demo");
    let layout_path = format!("{}/", layout.display());

    // The project as it is today: a single-file export.
    let s = ReflowService::in_memory().expect("service");
    j!(s.add_requirement(Parameters(req("req:a", "the first need"))));
    j!(s.add_requirement(Parameters(req(
        "req:b",
        "the second need, with an Upper-Case id"
    ))));
    let receipt = j!(s.export_graph(Parameters(export_to(&single_path))));
    assert_eq!(
        receipt["layout"], "file",
        "a .json path is still the single file"
    );
    let single_hash = receipt["content_hash"].clone();

    // The old form still loads, through the same reader as the new one.
    let t = ReflowService::in_memory().expect("service");
    let report = j!(t.import_graph(Parameters(import_from(&single_path))));
    assert!(report.get("integrity_note").is_none(), "{report:?}");

    // Converted: exported to the directory beside the single file.
    let converted = j!(t.export_graph(Parameters(export_to(&layout_path))));
    assert_eq!(converted["layout"], "items", "{converted:?}");
    assert_eq!(
        converted["content_hash"], single_hash,
        "the layout holds exactly the single file's design"
    );
    let stamp: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(layout.join("design.json")).unwrap())
            .unwrap();
    assert_eq!(
        stamp["migrated_from"], single_hash,
        "the stamp links the layout's history to the single file it replaced"
    );

    // Read back, either way round, it is the same design.
    let u = ReflowService::in_memory().expect("service");
    let report = j!(u.import_graph(Parameters(import_from(&layout_path))));
    assert!(report.get("integrity_note").is_none(), "{report:?}");
    let back_out = design_dir.join("back.json").display().to_string();
    let back = j!(u.export_graph(Parameters(export_to(&back_out))));
    assert_eq!(back["layout"], "file");
    assert_eq!(
        back["content_hash"], single_hash,
        "the layout read back out as a single file is the original design"
    );
    let _ = std::fs::remove_dir_all(&root);
}
