//! THE SERVER STAMPS EACH NODE'S CREATION AND CONTENT CHANGE, AND A CALLER
//! CANNOT SET IT — `req:reflow2-stamps-the-server-time-on-each-node-creation-and-content-change`
//! (accepted 2026-10-28), under
//! `dec:prediction-order-is-read-from-server-recorded-times-and-said-not-enforced-2026-10-28`.
//!
//! Measured on main 73b29e4 before it: the store kept only what a caller sent,
//! and every date it held was one a caller typed.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use reflow2_core::DesignGraph;
use reflow2_core::foundation::store::{
    STAMP_CHANGED, STAMP_CREATED, STAMP_SEAL, StampClock, format_stamp_time,
};
use reflow2_core::nodes::{Props, node};

/// A clock that moves one second per reading, from 2026-10-28T00:00:00Z.
fn ticking() -> (StampClock, Arc<AtomicI64>) {
    let at = Arc::new(AtomicI64::new(1_793_145_600));
    let shared = at.clone();
    let clock: StampClock =
        Arc::new(move || format_stamp_time(shared.fetch_add(1, Ordering::SeqCst), 0));
    (clock, at)
}

fn stamped() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.set_stamp_clock(ticking().0).unwrap();
    g
}

fn requirement(statement: &str) -> Props {
    Props::new()
        .set("name", "A prediction")
        .set("statement", statement)
}

#[test]
fn a_new_node_is_stamped_created_and_changed_now_and_reads_recorded_here() {
    let mut g = stamped();
    g.create_node(node::REQUIREMENT, "req:p", requirement("it falls"))
        .unwrap();
    let s = g.node_stamp(node::REQUIREMENT, "req:p").unwrap().unwrap();
    assert_eq!(s.created.as_deref(), Some("2026-10-28T00:00:00.000000Z"));
    assert_eq!(s.changed, s.created);
    assert!(s.recorded_here);
}

#[test]
fn every_read_leaves_the_stamps_out() {
    let mut g = stamped();
    let written = g
        .create_node(node::REQUIREMENT, "req:p", requirement("it falls"))
        .unwrap();
    let read = g.get_node(node::REQUIREMENT, "req:p").unwrap().unwrap();
    let scanned = g.scan_nodes(node::REQUIREMENT).unwrap();
    for props in [
        &written.properties,
        &read.properties,
        &scanned[0].properties,
    ] {
        for k in [STAMP_CREATED, STAMP_CHANGED, STAMP_SEAL] {
            assert!(!props.contains_key(k), "{k} leaked into a read: {props:?}");
        }
    }
}

#[test]
fn an_unchanged_rewrite_moves_nothing_and_a_change_keeps_the_creation() {
    let mut g = stamped();
    g.create_node(node::REQUIREMENT, "req:p", requirement("it falls"))
        .unwrap();
    let first = g.node_stamp(node::REQUIREMENT, "req:p").unwrap().unwrap();

    g.create_node(node::REQUIREMENT, "req:p", requirement("it falls"))
        .unwrap();
    assert_eq!(
        g.node_stamp(node::REQUIREMENT, "req:p").unwrap().unwrap(),
        first,
        "rewriting the same content is no change"
    );

    g.create_node(node::REQUIREMENT, "req:p", requirement("it falls by half"))
        .unwrap();
    let after = g.node_stamp(node::REQUIREMENT, "req:p").unwrap().unwrap();
    assert_eq!(after.created, first.created, "creation is kept");
    assert!(after.changed > first.changed, "{after:?} after {first:?}");
    assert!(after.recorded_here);
}

#[test]
fn a_write_naming_a_stamp_is_refused_and_writes_nothing() {
    let mut g = stamped();
    for k in [STAMP_CREATED, STAMP_CHANGED, STAMP_SEAL] {
        let props = requirement("it falls").set(k, "1999-01-01T00:00:00.000000Z");
        let err = g
            .create_node(node::REQUIREMENT, "req:p", props)
            .unwrap_err()
            .to_string();
        assert!(err.contains("REFUSED") && err.contains(k), "{err}");
        assert!(g.get_node(node::REQUIREMENT, "req:p").unwrap().is_none());
    }
    // And through the bulk form.
    let specs = vec![reflow2_core::bulk::NodeSpec {
        node_type: node::REQUIREMENT.into(),
        id: "req:q".into(),
        props: requirement("x")
            .set(STAMP_CREATED, "1999-01-01T00:00:00.000000Z")
            .into(),
    }];
    let report = g.create_nodes(&specs).unwrap();
    assert!(!report.applied && report.failures.len() == 1, "{report:?}");
    assert!(g.get_node(node::REQUIREMENT, "req:q").unwrap().is_none());
}

#[test]
fn a_caller_typed_date_is_content_never_a_stamp() {
    let mut g = stamped();
    g.create_node(
        node::REQUIREMENT,
        "req:p",
        requirement("it falls").set("created_at", "1999-01-01"),
    )
    .unwrap();
    let s = g.node_stamp(node::REQUIREMENT, "req:p").unwrap().unwrap();
    assert_eq!(s.created.as_deref(), Some("2026-10-28T00:00:00.000000Z"));
}

#[test]
fn an_export_carries_the_stamps_and_the_same_store_vouches_for_them_on_return() {
    let mut g = stamped();
    g.create_node(node::REQUIREMENT, "req:p", requirement("it falls"))
        .unwrap();
    let doc = g.export_graph().unwrap();
    let n = doc.nodes.iter().find(|n| n.node_id == "req:p").unwrap();
    assert!(n.stamps.contains_key(STAMP_SEAL), "{n:?}");
    assert!(
        !n.properties.contains_key(STAMP_SEAL),
        "held apart in memory"
    );
    let text = serde_json::to_string(&doc).unwrap();
    assert!(
        text.contains(STAMP_CREATED),
        "on disk the stamps sit in the node's properties"
    );
    let back: reflow2_core::GraphExport = serde_json::from_str(&text).unwrap();
    assert_eq!(back, doc, "and a file reads back exactly");

    // Restored into the same store, its own seal verifies.
    let before = g.node_stamp(node::REQUIREMENT, "req:p").unwrap().unwrap();
    g.delete_node(node::REQUIREMENT, "req:p").unwrap();
    g.import_graph(&doc).unwrap();
    assert_eq!(
        g.node_stamp(node::REQUIREMENT, "req:p").unwrap().unwrap(),
        before
    );
}

#[test]
fn a_stamp_imported_into_another_store_reads_imported_not_recorded_here() {
    let mut g = stamped();
    g.create_node(node::REQUIREMENT, "req:p", requirement("it falls"))
        .unwrap();
    let doc = g.export_graph().unwrap();

    let mut other = stamped();
    other.import_graph(&doc).unwrap();
    let s = other
        .node_stamp(node::REQUIREMENT, "req:p")
        .unwrap()
        .unwrap();
    assert!(s.created.is_some(), "the times travel");
    assert!(!s.recorded_here, "and this store does not vouch for them");

    // Re-exported from there, the copy is the same document: a design copy
    // round trip writes nothing new.
    let again = other.export_graph().unwrap();
    assert_eq!(
        again.nodes.iter().find(|n| n.node_id == "req:p"),
        doc.nodes.iter().find(|n| n.node_id == "req:p")
    );

    // A change there drops the creation it cannot vouch for, rather than
    // sealing it as its own.
    other
        .create_node(node::REQUIREMENT, "req:p", requirement("it falls by half"))
        .unwrap();
    let s = other
        .node_stamp(node::REQUIREMENT, "req:p")
        .unwrap()
        .unwrap();
    assert!(s.recorded_here);
    assert_eq!(s.created, None, "an imported creation is never laundered");
}

#[test]
fn a_backdated_or_edited_stamp_in_a_document_is_never_vouched_for() {
    let mut g = stamped();
    g.create_node(node::REQUIREMENT, "req:p", requirement("it falls"))
        .unwrap();
    let mut doc = g.export_graph().unwrap();
    let n = doc.nodes.iter_mut().find(|n| n.node_id == "req:p").unwrap();
    // The wording edited, the store's own seal left on it.
    n.properties.insert("statement".into(), "it rises".into());
    g.import_graph(&doc).unwrap();
    assert!(
        !g.node_stamp(node::REQUIREMENT, "req:p")
            .unwrap()
            .unwrap()
            .recorded_here,
        "a seal binds the content it was made over"
    );

    // A time moved back, the seal left as it was.
    let mut doc = g.export_graph().unwrap();
    let n = doc.nodes.iter_mut().find(|n| n.node_id == "req:p").unwrap();
    n.stamps
        .insert(STAMP_CREATED.into(), "1999-01-01T00:00:00.000000Z".into());
    g.import_graph(&doc).unwrap();
    assert!(
        !g.node_stamp(node::REQUIREMENT, "req:p")
            .unwrap()
            .unwrap()
            .recorded_here
    );
}

#[test]
fn a_node_no_stamp_was_ever_recorded_for_reads_not_recorded() {
    let mut g = stamped();
    g.create_node(node::REQUIREMENT, "req:p", requirement("it falls"))
        .unwrap();
    let mut doc = g.export_graph().unwrap();
    for n in &mut doc.nodes {
        n.stamps.clear();
    }
    let mut older = stamped();
    older.import_graph(&doc).unwrap();
    let s = older
        .node_stamp(node::REQUIREMENT, "req:p")
        .unwrap()
        .unwrap();
    assert_eq!((s.created, s.changed, s.recorded_here), (None, None, false));
}
