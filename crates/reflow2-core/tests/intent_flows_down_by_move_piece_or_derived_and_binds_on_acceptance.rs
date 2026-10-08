//! `ver:a-parent-requirement-sent-to-a-part-binds-only-when-the-part-accepts-and-its-moves-are-reported`,
//! the acceptance check for slice 3 of the hub work.
//!
//! Anthony's rulings (2026-10-07 and 08): nothing is copied. A requirement
//! wholly a part's MOVES (a reference replaces it in the parent, refused while
//! the parent still satisfies it); one that spans parts stays and each part
//! holds its own PIECE; a decision stays and its consequences arrive as DERIVED
//! requirements. What lands in the part is `proposed` and binds only on the part
//! owner's word. DECOMPOSES was widened to reach a cross-design reference for
//! the piece case.

use std::collections::HashMap;

use reflow2_core::{DependencyDeclaration, DesignGraph, PropagateOptions};

fn parent() -> DesignGraph {
    let mut p = DesignGraph::open_in_memory()
        .unwrap()
        .with_graph_id("design-p");
    p.add_project("prj:p", "P").unwrap();
    p.add_contributor("who:ajs", "Anthony", Some("person"), None, None)
        .unwrap();
    p.add_requirement("req:p-whole", "Logging", "The part keeps an audit log.")
        .unwrap();
    p.add_requirement("req:p-span", "Latency", "End to end latency under 200 ms.")
        .unwrap();
    p.add_decision(
        "dec:p-d",
        "Use UTC everywhere",
        "Every timestamp is UTC.",
        None,
    )
    .unwrap();
    p.declare_external_dependency(&DependencyDeclaration {
        id: "dep:a".into(),
        name: "A".into(),
        source: "https://example.org/a".into(),
        version: "v1".into(),
        components: vec![],
        features: vec![],
        declared_in: None,
        graph_id: Some("design-a".into()),
        design_export: None,
        design_export_hash: None,
        design_export_seen_at: None,
        design_address: None,
        design_address_hash: None,
        design_address_seen_at: None,
        note: None,
        relation: vec!["part_of".into()],
        interfaces: vec![],
    })
    .unwrap();
    p
}

fn part() -> DesignGraph {
    let mut a = DesignGraph::open_in_memory()
        .unwrap()
        .with_graph_id("design-a");
    a.add_project("prj:a", "A").unwrap();
    a.add_contributor("who:ajs", "Anthony", Some("person"), None, None)
        .unwrap();
    a
}

/// The two halves the agent makes, for all three kinds.
fn send_all(p: &mut DesignGraph, a: &mut DesignGraph) {
    a.receive_from_design(
        "req:a-log",
        "Audit log",
        "This part keeps an audit log.",
        "design-p",
        "req:p-whole",
        Some("Logging"),
        "moved",
        Some("who:ajs"),
    )
    .unwrap();
    p.send_to_design(
        "req:p-whole",
        "design-a",
        "req:a-log",
        None,
        "moved",
        Some("who:ajs"),
    )
    .unwrap();
    a.receive_from_design(
        "req:a-latency",
        "Part latency",
        "This part answers within 80 ms.",
        "design-p",
        "req:p-span",
        Some("Latency"),
        "piece",
        Some("who:ajs"),
    )
    .unwrap();
    p.send_to_design(
        "req:p-span",
        "design-a",
        "req:a-latency",
        None,
        "piece",
        None,
    )
    .unwrap();
    a.receive_from_design(
        "req:a-utc",
        "Store UTC",
        "This part stores timestamps in UTC.",
        "design-p",
        "dec:p-d",
        Some("Use UTC everywhere"),
        "derived",
        Some("who:ajs"),
    )
    .unwrap();
    p.send_to_design("dec:p-d", "design-a", "req:a-utc", None, "derived", None)
        .unwrap();
}

fn status(g: &DesignGraph, id: &str) -> String {
    g.get_node("Requirement", id).unwrap().unwrap().properties["status"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn what_lands_in_the_part_is_proposed_and_says_where_it_came_from() {
    let (mut p, mut a) = (parent(), part());
    send_all(&mut p, &mut a);
    let received = a.received_intents().unwrap();
    assert_eq!(received.len(), 3);
    assert!(
        received
            .iter()
            .all(|r| r.status == "proposed" && r.from_design == "design-p")
    );
    let mut kinds: Vec<&str> = received.iter().map(|r| r.kind.as_str()).collect();
    kinds.sort();
    assert_eq!(kinds, vec!["derived", "moved", "piece"]);

    let report = a.reconcile_upstream(&[]).unwrap();
    let inbox = report
        .findings
        .iter()
        .find(|f| f.kind == "received_waiting")
        .expect("what waits on the part's owner is reported");
    assert!(inbox.is_actionable());
    assert!(
        inbox.detail.contains("3 requirement(s)"),
        "{}",
        inbox.detail
    );
}

#[test]
fn it_binds_only_when_the_parts_owner_accepts_and_cannot_be_rewritten_after() {
    let (mut p, mut a) = (parent(), part());
    send_all(&mut p, &mut a);
    a.set_requirement_status("req:a-log", "accepted").unwrap();
    assert_eq!(status(&a, "req:a-log"), "accepted");
    let waiting = a
        .reconcile_upstream(&[])
        .unwrap()
        .findings
        .into_iter()
        .find(|f| f.kind == "received_waiting")
        .unwrap();
    assert!(
        waiting.detail.contains("2 requirement(s)"),
        "{}",
        waiting.detail
    );

    let again = a.receive_from_design(
        "req:a-log",
        "Audit log",
        "rewritten by the sender",
        "design-p",
        "req:p-whole",
        None,
        "moved",
        None,
    );
    assert!(
        again.is_err(),
        "the sender cannot rewrite what the owner ruled on"
    );
}

#[test]
fn a_move_hands_the_requirement_over_and_nothing_is_copied() {
    let (mut p, mut a) = (parent(), part());
    send_all(&mut p, &mut a);
    let moved = p.get_node("Requirement", "req:p-whole").unwrap().unwrap();
    assert_eq!(moved.properties["status"].as_str(), Some("dropped"));
    assert_eq!(
        moved.properties["moved_to_design"].as_str(),
        Some("design-a")
    );
    let refs = p.design_references().unwrap();
    let to_log = refs
        .iter()
        .find(|r| r.node_id == "req:a-log")
        .expect("the parent keeps a reference to the requirement's new home");
    assert!(
        to_log
            .links
            .iter()
            .any(|l| l.relation == "OBSOLETES" && l.node_id == "req:p-whole"),
        "the reference takes the old node's place: {:?}",
        to_log.links
    );
    // The part holds its own wording; the parent's text is not duplicated there.
    let a_log = a.get_node("Requirement", "req:a-log").unwrap().unwrap();
    assert_ne!(
        a_log.properties["statement"].as_str(),
        moved.properties["statement"].as_str()
    );
}

#[test]
fn a_move_is_refused_while_the_parent_still_satisfies_it_or_without_the_owners_word() {
    let (mut p, mut a) = (parent(), part());
    a.receive_from_design(
        "req:a-log",
        "Audit log",
        "This part keeps an audit log.",
        "design-p",
        "req:p-whole",
        None,
        "moved",
        None,
    )
    .unwrap();
    assert!(
        p.send_to_design("req:p-whole", "design-a", "req:a-log", None, "moved", None)
            .is_err(),
        "a move settles the requirement here, so it needs the owner's word"
    );
    p.add_capability("cap:p-log", "Parent logging", "The parent logs.", None)
        .unwrap();
    p.create_edge(
        "SATISFIES",
        "Capability",
        "cap:p-log",
        "Requirement",
        "req:p-whole",
        HashMap::new(),
    )
    .unwrap();
    let refused = p
        .send_to_design(
            "req:p-whole",
            "design-a",
            "req:a-log",
            None,
            "moved",
            Some("who:ajs"),
        )
        .expect_err("still satisfied here, so not wholly the part's");
    assert!(format!("{refused:?}").contains("cap:p-log"), "{refused:?}");
}

#[test]
fn a_piece_and_a_derived_requirement_are_linked_across_and_the_parent_is_not_left_with_a_gap() {
    let (mut p, mut a) = (parent(), part());
    send_all(&mut p, &mut a);
    // In the part: the piece decomposes the parent's requirement; the derived
    // requirement is governed by the parent's decision.
    let a_refs = a.design_references().unwrap();
    assert!(a_refs.iter().any(|r| {
        r.node_id == "req:p-span"
            && r.links
                .iter()
                .any(|l| l.relation == "DECOMPOSES" && l.node_id == "req:a-latency")
    }));
    // In the parent: what was sent is listed.
    let sent = p.sent_intents().unwrap();
    assert_eq!(sent.len(), 3);
    assert!(
        sent.iter()
            .any(|s| s.kind == "piece" && s.to_node_id == "req:a-latency")
    );
    assert!(
        sent.iter()
            .any(|s| s.kind == "derived" && s.node_id == "dec:p-d")
    );
}

#[test]
fn a_ripple_in_the_parent_carries_down_seeded_at_what_the_part_received() {
    let (mut p, mut a) = (parent(), part());
    send_all(&mut p, &mut a);
    let r = p
        .propagate_from(&["req:p-span"], PropagateOptions { max_depth: 3 })
        .unwrap();
    let down = r
        .continue_in
        .iter()
        .find(|c| c.design == "design-a")
        .expect("the parent's radius says to carry it down into the part");
    assert_eq!(down.direction, "down");
    assert!(
        down.seeds.contains(&"req:a-latency".to_string()),
        "seeded at the piece the part holds, not at the reference: {down:?}"
    );
}
