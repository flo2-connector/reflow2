//! F2 of `req:the-pieces-of-one-picture-are-found-together`: relation
//! suggestions weigh a shared neighbour by how rare it is, so the author and the
//! project, which touch nearly everything, stop counting, and intent (a
//! requirement or a decision) is compared with requirements AND ideas together.
//!
//! Measured 2026-10-07
//! (fact:root-cause-idea-linking-covers-open-ideas-in-pairs-and-a-vision-spread-across-requirements-is-never-gathered-2026-10-07):
//! for a requirement in a family of sixteen records, 1 of the top 10
//! suggestions was in the family, and the first reason on every one was "both
//! relate to who:ajs". The pool held requirements only, so the family's ideas
//! were never offered.

use reflow2_core::DesignGraph;
use reflow2_core::nodes::{Props, edge, node};

fn req(g: &mut DesignGraph, id: &str, name: &str, text: &str) {
    g.add_requirement(id, name, text).unwrap();
}

fn idea(g: &mut DesignGraph, id: &str, name: &str, text: &str) {
    g.create_node(
        node::DECISION,
        id,
        Props::new()
            .set("name", name)
            .set("decision", text)
            .set("status", "proposed")
            .set("kind", "exploratory"),
    )
    .unwrap();
}

/// A design where one author wrote nearly everything. The subject and eight
/// unrelated requirements carry the author; the two records that really
/// belong with the subject do not.
fn design() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.add_contributor("who:ajs", "Anthony", Some("person"), None, None)
        .unwrap();
    req(
        &mut g,
        "req:subject",
        "One search reaches every design a hub tracks",
        "A hub's search runs in each member design and names which design each hit came from.",
    );
    g.authored_by(node::REQUIREMENT, "req:subject", "who:ajs", None, None)
        .unwrap();
    for (i, topic) in [
        "invoice rounding",
        "login throttling",
        "colour palette",
        "printer queue",
        "backup retention",
        "font licensing",
        "timezone display",
        "cache eviction",
    ]
    .iter()
    .enumerate()
    {
        let id = format!("req:noise-{i}");
        req(
            &mut g,
            &id,
            topic,
            &format!("The system handles {topic} correctly."),
        );
        g.authored_by(node::REQUIREMENT, &id, "who:ajs", None, None)
            .unwrap();
    }
    req(
        &mut g,
        "req:family",
        "A set of member designs is worked as one design graph",
        "Every member design of a hub is searched and analysed as if one graph.",
    );
    idea(
        &mut g,
        "dec:idea-family",
        "OPEN — does a hub search every member design at once?",
        "Brainstorm: a hub could search each member design and say which design a hit is from.",
    );
    g
}

#[test]
fn the_author_alone_is_no_reason_to_offer_a_candidate() {
    let g = design();
    let r = g
        .relation_candidates("Requirement", "req:subject", None, 5)
        .unwrap();
    for c in &r.candidates {
        assert!(
            !c.because.iter().any(|b| b.contains("who:ajs")),
            "the author touches nearly every record here, so sharing it says nothing: {:?}",
            c
        );
    }
    let top = &r.candidates.first().expect("something is offered").node_id;
    assert!(
        top == "req:family" || top == "dec:idea-family",
        "the top suggestion is a real match, not a record that shares only the author: {:?}",
        r.candidates
    );
}

#[test]
fn a_requirement_is_compared_with_ideas_too() {
    let g = design();
    let r = g
        .relation_candidates("Requirement", "req:subject", None, 5)
        .unwrap();
    assert!(
        r.candidates.iter().any(|c| c.node_id == "dec:idea-family"),
        "the idea the requirement belongs with must be offered: {:?}",
        r.candidates
    );
}

#[test]
fn a_rare_shared_neighbour_still_outranks_shared_words() {
    let mut g = design();
    req(
        &mut g,
        "req:umbrella",
        "The whole",
        "An umbrella over two pieces.",
    );
    req(
        &mut g,
        "req:sibling",
        "Sibling piece",
        "Another piece of the same whole.",
    );
    for child in ["req:subject", "req:sibling"] {
        g.create_edge(
            edge::DECOMPOSES,
            node::REQUIREMENT,
            child,
            node::REQUIREMENT,
            "req:umbrella",
            std::collections::HashMap::new(),
        )
        .unwrap();
    }
    let r = g
        .relation_candidates("Requirement", "req:subject", None, 5)
        .unwrap();
    assert_eq!(
        r.candidates.first().map(|c| c.node_id.as_str()),
        Some("req:sibling"),
        "a neighbour only two records share is something a person wrote into the graph: {:?}",
        r.candidates
    );
}

#[test]
fn an_explicit_pool_is_still_obeyed() {
    let g = design();
    let r = g
        .relation_candidates("Requirement", "req:subject", Some("Requirement"), 5)
        .unwrap();
    assert!(
        r.candidates.iter().all(|c| c.node_type == "Requirement"),
        "a named pool type is the caller's choice: {:?}",
        r.candidates
    );
}
