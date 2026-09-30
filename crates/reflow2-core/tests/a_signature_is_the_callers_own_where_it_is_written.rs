//! A SIGNATURE IS CHECKED WHERE THE STORE WRITES IT (#616 fix 4,
//! `reflow2_core::intent::Signer`).
//!
//! The store has one AUTHORED_BY write, one AUTHORED_BY delete and two node
//! writes. With a signer installed — an engine served for others — each asks
//! it, so every door that reaches the store is held to the same rule: the
//! typed helper, the generic edge write, the bulk form, and an import. With
//! none installed — a local engine — nothing changes.

use reflow2_core::DesignGraph;
use reflow2_core::bulk::EdgeSpec;
use reflow2_core::foundation::core::Value;
use reflow2_core::intent::Signer;
use reflow2_core::nodes::{Props, edge, node};

const ME: &str = "who:alice";
const OTHER: &str = "who:mallory";

fn design() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("in-memory design");
    g.add_contributor(ME, "Alice", Some("person"), None, None)
        .unwrap();
    g.add_contributor(OTHER, "Mallory", Some("person"), None, None)
        .unwrap();
    g.add_decision("dec:d", "Choose", "Use JSON.", None)
        .unwrap();
    g.add_decision("dec:e", "Choose again", "Use CBOR.", None)
        .unwrap();
    // Signed by the other contributor before any signer was installed.
    g.authored_by(node::DECISION, "dec:e", OTHER, Some("approver"), None)
        .unwrap();
    g
}

fn as_caller(g: &mut DesignGraph) {
    g.begin_signing(Signer::Caller {
        contributor: ME.into(),
        how: "named by the test gateway".into(),
    });
}

fn as_nobody(g: &mut DesignGraph) {
    g.begin_signing(Signer::Nobody {
        why: "this test engine establishes nobody (--http-trusted-gateway)".into(),
    });
}

fn approver_props() -> Props {
    Props::new().set("roles", Value::List(vec![Value::String("approver".into())]))
}

fn signers(g: &DesignGraph, id: &str) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = g
        .outgoing(id, Some(edge::AUTHORED_BY))
        .unwrap()
        .into_iter()
        .map(|e| (e.to_id, format!("{:?}", e.properties.get("roles"))))
        .collect();
    v.sort();
    v
}

#[test]
fn a_local_engine_is_unchanged() {
    let mut g = design();
    g.authored_by(node::DECISION, "dec:d", OTHER, Some("approver"), None)
        .expect("local: anyone may be named, as always");
    g.create_edge(
        edge::AUTHORED_BY,
        node::DECISION,
        "dec:d",
        node::CONTRIBUTOR,
        ME,
        approver_props(),
    )
    .expect("local generic write");
    assert!(g.delete_edge(edge::AUTHORED_BY, "dec:e", OTHER).unwrap());
    assert!(g.may_sign(OTHER, "approver").is_ok());
}

#[test]
fn with_a_caller_established_every_door_signs_only_as_the_caller() {
    let mut g = design();
    as_caller(&mut g);
    let before = signers(&g, "dec:d");

    // The typed helper.
    let e = g
        .authored_by(node::DECISION, "dec:d", OTHER, Some("approver"), None)
        .expect_err("a signature in someone else's name");
    let text = e.to_string();
    assert!(text.contains(ME) && text.contains(OTHER), "{text}");
    assert!(text.contains("nothing was written"), "{text}");
    // Author is a signature too.
    g.authored_by(node::DECISION, "dec:d", OTHER, Some("author"), None)
        .expect_err("authorship in someone else's name");
    // The generic write.
    g.create_edge(
        edge::AUTHORED_BY,
        node::DECISION,
        "dec:d",
        node::CONTRIBUTOR,
        OTHER,
        approver_props(),
    )
    .expect_err("generic write naming someone else");
    // The bulk form: nothing of the batch lands.
    let items = vec![
        EdgeSpec {
            edge_type: edge::AUTHORED_BY.into(),
            from_type: node::DECISION.into(),
            from_id: "dec:d".into(),
            to_type: node::CONTRIBUTOR.into(),
            to_id: ME.into(),
            props: approver_props().into(),
        },
        EdgeSpec {
            edge_type: edge::AUTHORED_BY.into(),
            from_type: node::DECISION.into(),
            from_id: "dec:d".into(),
            to_type: node::CONTRIBUTOR.into(),
            to_id: OTHER.into(),
            props: approver_props().into(),
        },
    ];
    let report = g.create_edges(&items).unwrap();
    assert!(!report.applied && report.failures.len() == 1, "{report:?}");
    assert_eq!(
        signers(&g, "dec:d"),
        before,
        "a refused batch writes nothing"
    );
    // Removing someone else's signature.
    g.delete_edge(edge::AUTHORED_BY, "dec:e", OTHER)
        .expect_err("unsigning someone else");
    assert_eq!(signers(&g, "dec:e").len(), 1);
    // A rewrite that changes nothing is not a new signature.
    let stored = g
        .outgoing("dec:e", Some(edge::AUTHORED_BY))
        .unwrap()
        .remove(0)
        .properties;
    g.create_edge(
        edge::AUTHORED_BY,
        node::DECISION,
        "dec:e",
        node::CONTRIBUTOR,
        OTHER,
        stored,
    )
    .expect("an unchanged edge is not a signature");
    // The caller's own signature, through every door.
    g.authored_by(node::DECISION, "dec:d", ME, Some("approver"), None)
        .expect("own signature");
    assert!(g.delete_edge(edge::AUTHORED_BY, "dec:d", ME).unwrap());
    assert!(g.may_sign(ME, "approver").is_ok());
    assert!(g.may_sign(OTHER, "author").is_err());
    // A settle stands on its signature, which is checked where it is written.
    g.upsert_node(
        node::DECISION,
        "dec:d",
        Props::new().set("status", "accepted"),
    )
    .expect("with a caller established a status may move");
    // Ended, it is local again.
    g.end_signing();
    g.authored_by(node::DECISION, "dec:d", OTHER, Some("approver"), None)
        .expect("local after end_signing");
}

#[test]
fn with_nobody_established_nothing_signs_or_settles_and_proposals_land() {
    let mut g = design();
    as_nobody(&mut g);
    let e = g
        .authored_by(node::DECISION, "dec:d", ME, Some("approver"), None)
        .expect_err("no approval with nobody established");
    assert!(e.to_string().contains("--http-trusted-gateway"), "{e}");
    g.authored_by(node::DECISION, "dec:d", ME, Some("author"), None)
        .expect("authorship is attribution, not a signature of intent");
    g.delete_edge(edge::AUTHORED_BY, "dec:e", OTHER)
        .expect_err("removing an approval");
    // A status moving INTO settled intent, through the node write.
    let mut settled = g
        .get_node(node::DECISION, "dec:d")
        .unwrap()
        .unwrap()
        .properties;
    settled.insert("status".into(), Value::String("accepted".into()));
    g.create_node(node::DECISION, "dec:d", settled.clone())
        .expect_err("a settle with nobody established");
    assert_eq!(
        g.get_node(node::DECISION, "dec:d")
            .unwrap()
            .unwrap()
            .properties
            .get("status"),
        Some(&Value::String("proposed".into()))
    );
    // A proposal lands.
    g.add_requirement("req:p", "A proposal", "Maybe.")
        .expect("proposal");
    // A value already settled before is not a new settle.
    g.end_signing();
    g.create_node(node::DECISION, "dec:d", settled.clone())
        .unwrap();
    as_nobody(&mut g);
    settled.insert("name".into(), Value::String("Renamed".into()));
    g.create_node(node::DECISION, "dec:d", settled)
        .expect("re-writing a settled value is not a new settle");
}

#[test]
fn an_import_naming_someone_else_writes_nothing() {
    let mut g = design();
    let doc: reflow2_core::export::GraphExport = serde_json::from_value(serde_json::json!({
        "nodes": [{"node_type": "Decision", "node_id": "dec:imported", "properties": {"name": "I", "decision": "D", "kind": "choice", "status": "proposed"}}],
        "edges": [{"edge_type": "AUTHORED_BY", "from_id": "dec:imported", "to_id": OTHER, "properties": {"roles": ["approver"]}}]
    }))
    .unwrap();
    as_caller(&mut g);
    let refused = g.import_graph(&doc);
    assert!(refused.is_err(), "{refused:?}");
    assert!(
        g.get_node(node::DECISION, "dec:imported")
            .unwrap()
            .is_none()
    );
}
