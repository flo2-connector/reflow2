//! `REALIZES` points only at a Capability, a Component or an Interface
//! (`dec:realizes-is-restricted-to-capability-component-and-interface`,
//! Anthony 2026-09-23). Measured first: of 45 REALIZES edges on anything else
//! in reflow2's own design, 44 were the wrong edge.
//!
//! Pins: the schema refuses the dropped targets; `link_artifact` refuses them
//! BEFORE writing anything and names the edge that fits; an export written
//! before the change still imports — a legacy REALIZES onto a check arrives as
//! IMPLEMENTS and is reported; any other legacy target is refused by name.
//!
//! The on-open rewrite (`migrate_edge_rewrites`) could not be exercised through
//! the public API until 2026-10-03, because the schema refuses the very edge it
//! migrates. `write_under_schema` now writes that edge the way an older reflow2
//! did, so the last test pins it — and it reads the same table the import
//! does (`narrowing::EDGE_REWRITES`).

use reflow2_core::artifact::LinkArtifactOptions;
use reflow2_core::export::GraphExport;
use reflow2_core::graph::DesignGraph;
use reflow2_core::nodes::{Props, edge, node};

fn world() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("open");
    g.add_project("proj:1", "Pump").expect("project");
    g.add_requirement("req:flow", "Flow", "Deliver 10 L/min.")
        .expect("req");
    g.add_capability("cap:impeller", "Impeller", "moves water", Some("realized"))
        .expect("cap");
    g.add_verification("ver:flow", "flow test", None, None, None)
        .expect("ver");
    g.create_node(
        node::CHANGE_EVENT,
        "chg:impeller",
        Props::new()
            .set("name", "impeller added")
            .set("change_type", "new_feature"),
    )
    .expect("chg");
    g
}

fn link(target_type: &str, target_id: &str) -> LinkArtifactOptions {
    serde_json::from_value(serde_json::json!({
        "artifact_id": "art:impeller",
        "name": "impeller.rs",
        "location": "src/impeller.rs",
        "target_type": target_type,
        "target_id": target_id,
    }))
    .expect("options")
}

#[test]
fn the_schema_refuses_a_file_realizing_a_requirement() {
    let mut g = world();
    g.create_node(
        node::ARTIFACT,
        "art:x",
        Props::new().set("name", "x").set("location", "x.rs"),
    )
    .expect("artifact");
    assert!(
        g.create_edge(
            edge::REALIZES,
            node::ARTIFACT,
            "art:x",
            node::REQUIREMENT,
            "req:flow",
            Props::new()
        )
        .is_err()
    );
    for (t, id) in [
        (node::CAPABILITY, "cap:impeller"),
        (node::VERIFICATION, "ver:flow"),
    ] {
        let r = g.create_edge(edge::REALIZES, node::ARTIFACT, "art:x", t, id, Props::new());
        assert_eq!(r.is_ok(), t == node::CAPABILITY, "{t}: {r:?}");
    }
}

#[test]
fn link_artifact_refuses_a_requirement_before_writing_anything_and_names_the_fit() {
    let mut g = world();
    let err = g
        .link_artifact(link(node::REQUIREMENT, "req:flow"))
        .expect_err("refused");
    let msg = format!("{err}");
    assert!(
        msg.contains("SATISFIES") && msg.contains("VERIFIES"),
        "{msg}"
    );
    assert!(
        g.get_node(node::ARTIFACT, "art:impeller")
            .expect("read")
            .is_none(),
        "nothing may be written before the refusal"
    );
}

#[test]
fn link_artifact_names_changed_for_a_change() {
    let mut g = world();
    let err = g
        .link_artifact(link(node::CHANGE_EVENT, "chg:impeller"))
        .expect_err("refused");
    assert!(format!("{err}").contains("CHANGED"), "{err}");
}

#[test]
fn link_artifact_still_realizes_a_capability_and_implements_a_check() {
    let mut g = world();
    g.link_artifact(link(node::CAPABILITY, "cap:impeller"))
        .expect("capability");
    let mut o = link(node::VERIFICATION, "ver:flow");
    o.artifact_id = "art:flow-test".into();
    o.name = Some("flow_test.rs".into());
    g.link_artifact(o).expect("check");
    assert!(
        g.outgoing("art:flow-test", Some(edge::IMPLEMENTS))
            .expect("edges")
            .iter()
            .any(|e| e.to_id == "ver:flow")
    );
}

/// An export as a pre-2026-09-23 reflow2 could have written it.
fn legacy_export(target: &str) -> GraphExport {
    let g = world();
    let mut doc = g.export_graph().expect("export");
    doc.nodes.push(
        serde_json::from_value(serde_json::json!({
            "node_type": "Artifact", "node_id": "art:legacy",
            "properties": {"name": "legacy.rs", "location": "legacy.rs"}
        }))
        .expect("node"),
    );
    doc.edges.push(
        serde_json::from_value(serde_json::json!({
            "edge_type": "REALIZES", "from_id": "art:legacy", "to_id": target, "properties": {}
        }))
        .expect("edge"),
    );
    doc
}

#[test]
fn a_legacy_realizes_onto_a_check_imports_as_implements_and_is_reported() {
    let doc = legacy_export("ver:flow");
    let mut g = DesignGraph::open_in_memory().expect("open");
    let r = g.import_graph(&doc).expect("imports");
    assert_eq!(r.migrated_edges.len(), 1, "{:?}", r.migrated_edges);
    assert!(
        g.outgoing("art:legacy", Some(edge::IMPLEMENTS))
            .expect("edges")
            .iter()
            .any(|e| e.to_id == "ver:flow")
    );
    assert!(
        g.outgoing("art:legacy", Some(edge::REALIZES))
            .expect("edges")
            .is_empty()
    );
}

#[test]
fn a_legacy_realizes_onto_a_change_is_refused_by_name_with_the_fit() {
    let doc = legacy_export("chg:impeller");
    let mut g = DesignGraph::open_in_memory().expect("open");
    let err = g.import_graph(&doc).expect_err("refused");
    assert!(format!("{err}").contains("CHANGED"), "{err}");
}

/// The on-open rewrite, finally exercised: a store an older reflow2 wrote with
/// `Artifact REALIZES Verification` holds IMPLEMENTS after the migration every
/// open runs, and nothing of the old edge.
#[test]
fn the_open_rewrite_brings_a_legacy_realizes_onto_a_check_over() {
    let mut g = world();
    g.create_node(
        node::ARTIFACT,
        "art:legacy",
        Props::new()
            .set("name", "legacy.rs")
            .set("location", "legacy.rs"),
    )
    .expect("artifact");
    let mut older = reflow2_core::schema::load_schema().expect("schema");
    older
        .edge_types
        .get_mut(edge::REALIZES)
        .expect("REALIZES")
        .to = reflow2_core::foundation::core::EdgeEndpoint::Single("*".into());
    g.write_under_schema(older, |g| {
        g.create_edge(
            edge::REALIZES,
            node::ARTIFACT,
            "art:legacy",
            node::VERIFICATION,
            "ver:flow",
            Props::new(),
        )
    })
    .expect("the older schema accepted it");
    assert_eq!(g.migrate_edge_rewrites().expect("migrate"), 1);
    assert!(
        g.outgoing("art:legacy", Some(edge::REALIZES))
            .expect("edges")
            .is_empty()
    );
    assert!(
        g.outgoing("art:legacy", Some(edge::IMPLEMENTS))
            .expect("edges")
            .iter()
            .any(|e| e.to_id == "ver:flow")
    );
    assert_eq!(
        g.migrate_edge_rewrites().expect("again"),
        0,
        "idempotent — it runs on every open"
    );
}
