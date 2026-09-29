//! A relation reflow2 stores twice has one authority, and the store keeps the
//! other copy (`req:a-relation-stored-in-more-than-one-place-has-one-authoritative-copy-and-no-copy-drifts-unnoticed`).
//!
//! MEASURED 2026-09-28 on reflow2's own design:
//! - 354 of 763 findings had a `subject_id` and no `HAS_TEMPORAL_FACT` edge.
//! - 4 hung from a different node than their `subject_id`.
//! - A Flow's stored exit named step 8 of 9.
//!
//! The root cause
//! (`fact:root-cause-each-writer-draws-the-second-copy-by-hand-and-three-writers-never-did-2026-09-28`)
//! is that each writer drew the second copy by hand. So these tests pin the
//! CLASS, not a writer: every declared twin, written by the writers that never
//! drew it, moved, imported out of step, and reopened.
//!
//! OBSERVED FAILING, 2026-09-29, with the fix switched off (the write-time
//! keep, the import repair, the open repair): the generic writer, the by-hand
//! report, the moved subject and the import each failed for the reason the
//! fix exists. The open test stopped at its own setup, because with nothing
//! drawn on write there was no edge to delete, so that run did not isolate
//! the open repair. The four that passed read the schema, the refusal, a
//! snapshot and a writer that draws its own copy, none of which the switch
//! touched.
//!
//! WHAT IT DOES NOT CHECK: whether each twin's authority is the right copy.
//! That is Anthony's ruling, recorded in
//! `dec:has-temporal-fact-means-only-the-subject-and-also-concerns-is-about-entity`
//! and `dec:a-flows-order-is-its-step-order-and-entry-and-exit-are-computed`.

use reflow2_core::DesignGraph;
use reflow2_core::export::GraphExport;
use reflow2_core::foundation::core::{TwinEnd, Value};
use reflow2_core::nodes::{Props, edge, node};
use serde_json::json;

fn graph() -> DesignGraph {
    DesignGraph::open_in_memory().expect("in-memory graph")
}

fn sources(g: &DesignGraph, to: &str, edge_type: &str) -> Vec<String> {
    let mut v: Vec<String> = g
        .incoming(to, Some(edge_type))
        .expect("incoming")
        .into_iter()
        .map(|e| e.from_id)
        .collect();
    v.sort();
    v
}

#[test]
fn every_declared_twin_is_an_edge_that_can_join_its_node() {
    let g = graph();
    let twins = g.stored_twins();
    let declared: Vec<String> = twins
        .iter()
        .map(|t| format!("{}.{}", t.node_type, t.property))
        .collect();
    for expected in [
        "TemporalFact.subject_id",
        "Snapshot.target_id",
        "DimensionAssessment.target_id",
        "DimensionObservation.target_id",
        "DimensionObservation.source_fragment_id",
        "ReadinessAssessment.target_id",
    ] {
        assert!(
            declared.iter().any(|d| d == expected),
            "{expected} is a relation stored twice and must declare its twin: {declared:?}"
        );
    }
    let schema = g.schema();
    for t in &twins {
        let def = schema.edge_types.get(&t.twin.edge).unwrap_or_else(|| {
            panic!(
                "{}.{}: twin edge {} is not declared",
                t.node_type, t.property, t.twin.edge
            )
        });
        let owner_end = match t.twin.names {
            TwinEnd::Source => &def.to,
            TwinEnd::Target => &def.from,
        };
        assert!(
            owner_end.accepts(&t.node_type),
            "{}.{}: {} cannot have a {} at the end the property's node sits on",
            t.node_type,
            t.property,
            t.twin.edge,
            t.node_type
        );
        if let Some(other) = &t.twin.strays_become {
            assert!(
                schema.edge_types.contains_key(other),
                "{}.{}: strays_become names {other}, which is not an edge type",
                t.node_type,
                t.property
            );
        }
    }
}

/// THE MEASURED DEFECT. The generic writer set `subject_id` and drew nothing,
/// and 84% of findings written before `record_finding` existed carried no
/// edge. The store draws it now, whoever writes.
#[test]
fn a_finding_written_through_the_generic_writer_hangs_from_its_subject() {
    let mut g = graph();
    g.add_component("cmp:a", "A", "a part", None).unwrap();
    g.upsert_node(
        node::TEMPORAL_FACT,
        "fact:x",
        Props::new()
            .set("subject_id", "cmp:a")
            .set("statement", "measured something")
            .set("basis", "measured"),
    )
    .unwrap();
    assert_eq!(sources(&g, "fact:x", edge::HAS_TEMPORAL_FACT), ["cmp:a"]);
}

/// The live writer defect: every by-hand report was edgeless, 27 of them
/// after `record_finding` shipped.
#[test]
fn a_by_hand_report_hangs_from_the_project() {
    let mut g = graph();
    g.add_project("proj:p", "P").unwrap();
    let id = g
        .report_manual_work("grepped the export for ids", "tool_missing", None, None)
        .unwrap();
    assert_eq!(sources(&g, &id, edge::HAS_TEMPORAL_FACT), ["proj:p"]);
}

/// When the authority moves, its copy moves with it, and the old subject no
/// longer carries the finding.
#[test]
fn moving_the_subject_moves_the_edge() {
    let mut g = graph();
    g.add_component("cmp:a", "A", "a part", None).unwrap();
    g.add_component("cmp:b", "B", "another part", None).unwrap();
    let fact = |subject: &str| {
        Props::new()
            .set("subject_id", subject)
            .set("statement", "measured something")
            .set("basis", "measured")
    };
    g.upsert_node(node::TEMPORAL_FACT, "fact:x", fact("cmp:a"))
        .unwrap();
    g.upsert_node(node::TEMPORAL_FACT, "fact:x", fact("cmp:b"))
        .unwrap();
    assert_eq!(sources(&g, "fact:x", edge::HAS_TEMPORAL_FACT), ["cmp:b"]);
}

/// A writer that already draws the edge (record_finding, forecast_readiness,
/// add_readiness, the dimension writers, snapshots) keeps working, with ONE
/// edge, not two, and its own edge properties intact.
#[test]
fn a_writer_that_draws_its_own_copy_gets_exactly_one() {
    let mut g = graph();
    g.add_component("cmp:a", "A", "a part", None).unwrap();
    g.add_epoch("epoch:1", "One", reflow2_core::EpochType::Milestone, 1)
        .unwrap();
    let snap = g
        .snapshot_node("epoch:1", node::COMPONENT, "cmp:a")
        .unwrap();
    assert_eq!(sources(&g, &snap.node_id, edge::HAS_SNAPSHOT), ["cmp:a"]);
}

fn doc(value: serde_json::Value) -> GraphExport {
    serde_json::from_value(value).expect("a GraphExport")
}

/// A document written before the store kept these copies arrives out of step:
/// a finding with no edge, a finding hung by hand off a second node, a Flow's
/// stored exit, and a capability's retired flag. The import brings each into
/// step and names every change. It never refuses, because a restore reproduces
/// a state that existed.
#[test]
fn an_import_repairs_every_twin_out_of_step_and_names_each_repair() {
    let mut g = graph();
    let report = g
        .import_graph(&doc(json!({
            "nodes": [
                {"node_type": "Project", "node_id": "proj:p", "properties": {"name": "P"}},
                {"node_type": "Component", "node_id": "cmp:a", "properties": {"name": "A", "description": "a", "purpose": "p"}},
                {"node_type": "Component", "node_id": "cmp:b", "properties": {"name": "B", "description": "b", "purpose": "p"}},
                {"node_type": "TemporalFact", "node_id": "fact:edgeless",
                 "properties": {"subject_id": "cmp:a", "statement": "s", "basis": "measured"}},
                {"node_type": "TemporalFact", "node_id": "fact:hung-twice",
                 "properties": {"subject_id": "cmp:a", "statement": "s", "basis": "measured"}},
                {"node_type": "Capability", "node_id": "cap:start",
                 "properties": {"name": "Start", "description": "d", "is_entry_point": true}},
                {"node_type": "Flow", "node_id": "flow:f",
                 "properties": {"name": "F", "exit_point": "cap:start"}}
            ],
            "edges": [
                {"edge_type": "HAS_TEMPORAL_FACT", "from_id": "cmp:a", "to_id": "fact:hung-twice", "properties": {}},
                {"edge_type": "HAS_TEMPORAL_FACT", "from_id": "cmp:b", "to_id": "fact:hung-twice", "properties": {}}
            ]
        })))
        .expect("an out-of-step design still restores");

    let r = &report.twin_repairs;
    assert!(
        r.added
            .iter()
            .any(|l| l.contains("HAS_TEMPORAL_FACT cmp:a -> fact:edgeless")),
        "{r:?}"
    );
    assert_eq!(r.moved.len(), 1, "{r:?}");
    assert!(
        r.moved[0].contains("became ABOUT_ENTITY fact:hung-twice -> cmp:b"),
        "{r:?}"
    );
    assert_eq!(r.retired_properties.len(), 2, "{r:?}");

    assert_eq!(
        sources(&g, "fact:edgeless", edge::HAS_TEMPORAL_FACT),
        ["cmp:a"]
    );
    assert_eq!(
        sources(&g, "fact:hung-twice", edge::HAS_TEMPORAL_FACT),
        ["cmp:a"]
    );
    let also: Vec<String> = g
        .outgoing("fact:hung-twice", Some(edge::ABOUT_ENTITY))
        .unwrap()
        .into_iter()
        .map(|e| e.to_id)
        .collect();
    assert_eq!(
        also,
        ["cmp:b"],
        "'also concerns' is kept, on the edge that means it"
    );
    let flow = g.get_node(node::FLOW, "flow:f").unwrap().unwrap();
    assert!(!flow.properties.contains_key("exit_point"));
    let cap = g.get_node(node::CAPABILITY, "cap:start").unwrap().unwrap();
    assert!(!cap.properties.contains_key("is_entry_point"));

    // Idempotent: a design in step reports nothing.
    assert!(g.repair_stored_twins().unwrap().is_empty());
}

/// A Snapshot is the only record left of a deleted node, so its `target_id`
/// outliving the node is the mechanism working. It is neither a repair nor a
/// refusal.
#[test]
fn a_snapshot_outlives_its_node_without_a_repair() {
    let mut g = graph();
    g.add_component("cmp:gone", "Gone", "soon deleted", None)
        .unwrap();
    g.add_epoch("epoch:1", "One", reflow2_core::EpochType::Milestone, 1)
        .unwrap();
    let snap = g
        .snapshot_node("epoch:1", node::COMPONENT, "cmp:gone")
        .unwrap();
    assert!(g.delete_node(node::COMPONENT, "cmp:gone").unwrap());
    let r = g.repair_stored_twins().unwrap();
    assert!(r.is_empty(), "{r:?}");
    let kept = g.get_node(node::SNAPSHOT, &snap.node_id).unwrap().unwrap();
    assert_eq!(
        kept.properties.get("target_id").and_then(Value::as_str),
        Some("cmp:gone")
    );
}

/// The refusal the generic edge tools give: drawing a finding's
/// HAS_TEMPORAL_FACT from anything but its subject names the property and the
/// edge that means "also concerns"; deleting the subject's own copy names the
/// property to change instead.
#[test]
fn a_hand_drawn_copy_that_disagrees_is_refused_and_says_what_to_do() {
    let mut g = graph();
    g.add_component("cmp:a", "A", "a part", None).unwrap();
    g.add_component("cmp:b", "B", "another part", None).unwrap();
    g.upsert_node(
        node::TEMPORAL_FACT,
        "fact:x",
        Props::new()
            .set("subject_id", "cmp:a")
            .set("statement", "s")
            .set("basis", "measured"),
    )
    .unwrap();

    let why = g
        .twin_edge_refusal(edge::HAS_TEMPORAL_FACT, "cmp:b", "fact:x", false)
        .unwrap()
        .expect("a second subject is refused");
    assert!(
        why.contains("subject_id") && why.contains("ABOUT_ENTITY"),
        "{why}"
    );
    assert!(
        g.twin_edge_refusal(edge::HAS_TEMPORAL_FACT, "cmp:a", "fact:x", false)
            .unwrap()
            .is_none(),
        "the copy the property names is not refused"
    );
    let why = g
        .twin_edge_refusal(edge::HAS_TEMPORAL_FACT, "cmp:a", "fact:x", true)
        .unwrap()
        .expect("deleting the subject's copy is refused");
    assert!(why.contains("change subject_id"), "{why}");
    assert!(
        g.twin_edge_refusal(edge::DEPENDS_ON, "cmp:a", "cmp:b", false)
            .unwrap()
            .is_none(),
        "an edge that is no twin's copy is none of this check's business"
    );
}

/// Opening a store repairs what it holds out of step, and KEEPS the report for
/// `loop_status`. The two migrations beside it on open discard their counts.
#[cfg(feature = "rocksdb")]
#[test]
fn opening_a_store_out_of_step_repairs_it_and_keeps_the_report() {
    let dir = std::env::temp_dir().join(format!("reflow2-twins-open-{}", std::process::id()));
    let path = dir.to_str().expect("utf-8 temp path").to_string();
    let _ = std::fs::remove_dir_all(&dir);
    {
        let mut g = DesignGraph::open_rocksdb(&path).expect("open");
        g.add_component("cmp:a", "A", "a part", None).unwrap();
        g.upsert_node(
            node::TEMPORAL_FACT,
            "fact:x",
            Props::new()
                .set("subject_id", "cmp:a")
                .set("statement", "s")
                .set("basis", "measured"),
        )
        .unwrap();
        // Out of step the way a store written before this change is: the
        // copy is simply not there.
        assert!(
            g.delete_edge(edge::HAS_TEMPORAL_FACT, "cmp:a", "fact:x")
                .unwrap()
        );
        assert!(g.repaired_on_open().is_empty());
    }
    {
        let g = DesignGraph::open_rocksdb(&path).expect("reopen");
        assert_eq!(sources(&g, "fact:x", edge::HAS_TEMPORAL_FACT), ["cmp:a"]);
        let r = g.repaired_on_open();
        assert_eq!(r.added.len(), 1, "{r:?}");
        assert!(r.summary().is_some());
    }
    {
        let g = DesignGraph::open_rocksdb(&path).expect("reopen in step");
        assert!(g.repaired_on_open().is_empty());
    }
    let _ = std::fs::remove_dir_all(&dir);
}
