//! F1 of `req:the-pieces-of-one-picture-are-found-together`: requirements and
//! accepted decisions linked to no other intent are noticed, not only open
//! ideas. Measured 2026-10-07: the three records outside a sixteen-record
//! vision were all requirements, which `unreviewed_ideas` never looks at.

use std::collections::HashMap;

use reflow2_core::DesignGraph;
use reflow2_core::nodes::{Props, edge, node};

fn graph() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.add_project("prj:p", "P").unwrap();
    for (id, name) in [
        ("req:alone", "Alone"),
        ("req:whole", "The whole"),
        ("req:piece", "A piece"),
        ("req:noted", "Reviewed, nothing related"),
        ("req:dropped", "Abandoned"),
        ("req:built", "Built but unrelated"),
    ] {
        g.add_requirement(id, name, &format!("{name}, as a requirement."))
            .unwrap();
    }
    g.decomposes("req:piece", "req:whole").unwrap();
    g.set_requirement_status("req:dropped", "dropped").unwrap();
    let mut noted = g
        .get_node(node::REQUIREMENT, "req:noted")
        .unwrap()
        .unwrap()
        .properties;
    noted.insert(
        "no_relation_note".into(),
        "searched; nothing honestly related".into(),
    );
    g.create_node(node::REQUIREMENT, "req:noted", noted)
        .unwrap();
    g.add_capability("cap:b", "Builds it", "It is built.", Some("realized"))
        .unwrap();
    g.create_edge(
        edge::SATISFIES,
        node::CAPABILITY,
        "cap:b",
        node::REQUIREMENT,
        "req:built",
        HashMap::new(),
    )
    .unwrap();
    for (id, status) in [("dec:settled", "accepted"), ("dec:open", "proposed")] {
        g.create_node(
            node::DECISION,
            id,
            Props::new()
                .set("name", id)
                .set("decision", "a decision")
                .set("status", status),
        )
        .unwrap();
    }
    g
}

#[test]
fn intent_linked_to_nothing_is_listed_and_the_linked_the_noted_and_the_dropped_are_not() {
    let g = graph();
    let unlinked = g.unlinked_intent().unwrap();
    assert_eq!(
        unlinked,
        vec!["dec:settled", "req:alone", "req:built"],
        "a capability satisfying a requirement is build, not related intent; an open idea is \
         unreviewed_ideas' to count"
    );
}

#[test]
fn the_gap_pass_reports_it_as_one_aggregate_finding_with_its_denominator() {
    let g = graph();
    let gaps = g.detect_gaps().unwrap();
    let gap = gaps
        .iter()
        .find(|c| c.gap_source.as_str() == "unlinked_intent")
        .expect("unlinked intent is a finding");
    assert_eq!(gap.affected_ids.len(), 3);
    assert!(
        gap.title.starts_with("3 of 6 "),
        "five live requirements and one accepted decision: {}",
        gap.title
    );
}
