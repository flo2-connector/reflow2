//! The shipped binary reports, on a store an older reflow2 wrote, exactly what
//! its own import of that store's export would refuse — before the move.
//!
//! `dec:idea-stored-data-is-rechecked-against-the-current-schema`, accepted by
//! Anthony 2026-10-02. The core pins the rule
//! (`crates/reflow2-core/tests/a_stored_item_the_schema_now_refuses_is_reported.rs`);
//! this drives the door a person meets it through — `--call detect_defects` on a
//! RocksDB store, then `--export` and `--import` — because the field case was a
//! store on disk: musicjug's, minted by 0.45.0, whose `Artifact REALIZES
//! Decision` 0.75.0 exported without a word and then refused on import, all
//! or nothing (2026-10-01).
//!
//! The store is written the way the older binary wrote it: under a schema
//! whose REALIZES and VERIFIES still accepted any target, then closed and
//! opened by today's binary. Both shapes are the ones triage agent C built by
//! hand on 2026-10-02 (`legacy-edge-export.json`): REALIZES onto a Decision and
//! VERIFIES onto a Decision.

use std::path::Path;
use std::process::{Command, Output};

use reflow2_core::foundation::core::EdgeEndpoint;
use reflow2_core::graph::DesignGraph;
use reflow2_core::nodes::{Props, edge, node};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

fn run(args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("the binary runs")
}

fn older_store(path: &Path) {
    let mut g = DesignGraph::open_rocksdb(path.to_str().expect("utf-8")).expect("open");
    g.add_project("proj:musicjug", "Musicjug").expect("project");
    g.create_node(
        node::ARTIFACT,
        "art:palettes-js",
        Props::new()
            .set("name", "palettes.js")
            .set("location", "src/palettes.js"),
    )
    .expect("artifact");
    g.create_node(
        node::DECISION,
        "dec:styles-are-palettes",
        Props::new()
            .set("name", "Styles are palettes")
            .set("decision", "A style is a palette."),
    )
    .expect("decision");
    g.add_verification("ver:palette-check", "palette check", None, None, None)
        .expect("verification");
    let mut older = reflow2_core::schema::load_schema().expect("schema");
    for e in [edge::REALIZES, edge::VERIFIES] {
        older.edge_types.get_mut(e).expect("edge").to =
            EdgeEndpoint::Single(EdgeEndpoint::WILDCARD.to_string());
    }
    g.write_under_schema(older, |g| {
        g.create_edge(
            edge::REALIZES,
            node::ARTIFACT,
            "art:palettes-js",
            node::DECISION,
            "dec:styles-are-palettes",
            Props::new(),
        )?;
        g.create_edge(
            edge::VERIFIES,
            node::VERIFICATION,
            "ver:palette-check",
            node::DECISION,
            "dec:styles-are-palettes",
            Props::new(),
        )
    })
    .expect("the older schema accepted both");
}

#[test]
fn the_binary_names_what_its_import_would_refuse_before_the_move_is_tried() {
    let dir = tempfile::tempdir().expect("tempdir");
    let graph = dir.path().join("graph");
    older_store(&graph);
    let graph = graph.to_str().expect("utf-8");

    // 1. The store opens and works, as it did in the field.
    let o = run(&[
        "--graph-path",
        graph,
        "--call",
        "detect_defects",
        "--args",
        "{}",
    ]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let sweep: serde_json::Value =
        serde_json::from_slice(&o.stdout).expect("one JSON reply on stdout");
    let found: Vec<&serde_json::Value> = sweep["defects"]
        .as_array()
        .expect("defects")
        .iter()
        .filter(|d| d["category"] == "refused_by_schema")
        .collect();
    assert_eq!(found.len(), 2, "{sweep:#}");
    let by_edge = |t: &str| {
        found
            .iter()
            .find(|d| d["refusal"]["edge_type"] == t)
            .unwrap_or_else(|| panic!("a {t} finding: {sweep:#}"))
    };
    let realizes = by_edge("REALIZES");
    assert_eq!(realizes["severity"], "critical");
    assert!(
        realizes["refusal"]["replacement"]
            .as_str()
            .is_some_and(|r| r.contains("DOCUMENTS")),
        "the replacement the import names: {realizes:#}"
    );
    // ⭐ THE ONE THE IMPORT NAMED NOTHING FOR ON 0.77.0 — a check on a ruling.
    let verifies = by_edge("VERIFIES");
    assert!(
        verifies["refusal"]["replacement"]
            .as_str()
            .is_some_and(|r| r.contains("GOVERNED_BY")),
        "{verifies:#}"
    );

    // 2. It still exports, silently, as before: export does not validate,
    //    and the finding is the warning, once, on demand.
    let e = run(&["--graph-path", graph, "--export"]);
    assert!(e.status.success(), "{}", String::from_utf8_lossy(&e.stderr));
    let doc = dir.path().join("export.json");
    std::fs::write(&doc, &e.stdout).expect("write export");

    // 3. And the binary's own import refuses exactly those two, naming the
    //    same replacements.
    let fresh = dir.path().join("fresh");
    let i = run(&[
        "--graph-path",
        fresh.to_str().expect("utf-8"),
        "--import",
        doc.to_str().expect("utf-8"),
    ]);
    assert!(!i.status.success(), "the import must refuse");
    let err = String::from_utf8_lossy(&i.stderr);
    // anyhow re-indents the cause, so count the items, not the indentation.
    assert_eq!(
        err.matches("- edges[").count() + err.matches("- nodes[").count(),
        found.len(),
        "one import fault per finding:\n{err}"
    );
    assert!(
        err.contains("REALIZES art:palettes-js -> dec:styles-are-palettes")
            && err.contains("VERIFIES ver:palette-check -> dec:styles-are-palettes")
            && err.contains("DOCUMENTS")
            && err.contains("GOVERNED_BY"),
        "{err}"
    );
}
