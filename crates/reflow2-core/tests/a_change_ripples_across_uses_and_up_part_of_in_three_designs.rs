//! `ver:a-change-ripples-across-uses-and-up-part-of-in-three-designs`, the
//! acceptance check for `cap:a-cross-design-ripple-follows-each-members-relation`.
//!
//! Three designs: P (the parent), A (PART OF P) and B (a peer A USES across
//! interface I, mirrored into A). A change in B at I, carried on the way the
//! /hub skill carries it, must reach A's consumers of I (across) and P's
//! capability that A serves (up), and every row must name its design.
//!
//! Measured 2026-10-05: a blast radius stopped at the design it ran in and no
//! row named a design (fact:root-cause-a-blast-radius-stops-at-its-design-by-
//! design-and-nothing-says-where-it-left-2026-10-05). An edge still cannot
//! cross a store; what this pins is that each radius says where it left and
//! each design says how a ripple enters it, so the hop between them is
//! mechanical.

use std::collections::{BTreeMap, VecDeque};

use reflow2_core::{BlastRadius, DependencyDeclaration, DesignGraph, PropagateOptions};

const OPTS: PropagateOptions = PropagateOptions { max_depth: 5 };

fn pin(id: &str, graph_id: &str, relation: &[&str], interfaces: &[&str]) -> DependencyDeclaration {
    DependencyDeclaration {
        id: id.into(),
        name: graph_id.into(),
        source: format!("https://example.org/{graph_id}"),
        version: "v1".into(),
        components: vec![],
        features: vec![],
        declared_in: None,
        graph_id: Some(graph_id.into()),
        design_export: None,
        design_export_hash: None,
        design_export_seen_at: None,
        design_address: None,
        design_address_hash: None,
        design_address_seen_at: None,
        note: None,
        relation: relation.iter().map(|s| s.to_string()).collect(),
        interfaces: interfaces.iter().map(|s| s.to_string()).collect(),
    }
}

/// B provides I; A mirrors B's surface, consumes I and pins B as a peer it
/// uses; P pins A as its part and has a capability that requires A.
/// `relations: false` builds the same three with pins that state no relation.
fn three_designs(relations: bool) -> BTreeMap<&'static str, DesignGraph> {
    let mut b = DesignGraph::open_in_memory()
        .unwrap()
        .with_graph_id("design-b");
    b.add_project("prj:b", "B").unwrap();
    b.add_component("cmp:b-engine", "B's engine", "serves I", None)
        .unwrap();
    b.add_interface("ifc:i", "I, B's published interface")
        .unwrap();
    b.set_interface_designation("ifc:i", "published").unwrap();
    b.provides("cmp:b-engine", "ifc:i").unwrap();

    let mut a = DesignGraph::open_in_memory()
        .unwrap()
        .with_graph_id("design-a");
    a.add_project("prj:a", "A").unwrap();
    let surface = b.export_surface().unwrap();
    a.mirror_surface(&surface.document, Some("2026-10-06"))
        .unwrap();
    a.add_component("cmp:a-client", "A's client of I", "calls I", None)
        .unwrap();
    a.consumes("cmp:a-client", "ifc:i").unwrap();
    a.add_capability("cap:a-feature", "A's feature", "built on I", None)
        .unwrap();
    a.allocate("cap:a-feature", "cmp:a-client").unwrap();
    let (rel_b, ifc_b): (&[&str], &[&str]) = if relations {
        (&["uses"], &["ifc:i"])
    } else {
        (&[], &[])
    };
    a.declare_external_dependency(&pin("dep:b", "design-b", rel_b, ifc_b))
        .unwrap();

    let mut p = DesignGraph::open_in_memory()
        .unwrap()
        .with_graph_id("design-p");
    p.add_project("prj:p", "P").unwrap();
    p.add_capability(
        "cap:p-serves",
        "what A serves P",
        "P's capability A delivers",
        None,
    )
    .unwrap();
    let rel_a: &[&str] = if relations { &["part_of"] } else { &[] };
    p.declare_external_dependency(&pin("dep:a", "design-a", rel_a, &[]))
        .unwrap();
    p.require_resource("Capability", "cap:p-serves", "dep:a", None)
        .unwrap();

    BTreeMap::from([("design-a", a), ("design-b", b), ("design-p", p)])
}

/// The hop the /hub skill makes, written out: run where the change starts;
/// then, breadth first, carry each radius on along its own `continue_in` and
/// into every other design whose pins say how a ripple from it enters.
fn ripple(
    designs: &BTreeMap<&'static str, DesignGraph>,
    start: &str,
    seeds: &[&str],
) -> BTreeMap<String, BlastRadius> {
    let mut out: BTreeMap<String, BlastRadius> = BTreeMap::new();
    out.insert(
        start.to_string(),
        designs[start].propagate_from(seeds, OPTS).unwrap(),
    );
    let mut queue = VecDeque::from([start.to_string()]);
    while let Some(d) = queue.pop_front() {
        let (onward, reached) = {
            let r = &out[&d];
            (r.continue_in.clone(), r.interfaces_reached.clone())
        };
        for c in onward {
            if out.contains_key(&c.design) || c.seeds.is_empty() {
                continue;
            }
            if let Some(g) = designs.get(c.design.as_str()) {
                let refs: Vec<&str> = c.seeds.iter().map(String::as_str).collect();
                out.insert(c.design.clone(), g.propagate_from(&refs, OPTS).unwrap());
                queue.push_back(c.design.clone());
            }
        }
        for (x, g) in designs {
            if out.contains_key(*x) {
                continue;
            }
            if let Some((_, r)) = g.propagate_arriving(&d, &reached, OPTS).unwrap() {
                out.insert(x.to_string(), r);
                queue.push_back(x.to_string());
            }
        }
    }
    out
}

/// Reached, or started from: a ripple entering a design starts AT what it
/// reaches there first (the capability that needs a part, the interface a peer
/// is used across), so those are seeds of that design's radius.
fn reaches(r: &BlastRadius, id: &str) -> bool {
    r.seeds.iter().any(|s| s == id) || r.impacted.iter().any(|n| n.node_id == id)
}

#[test]
fn a_change_in_b_at_i_reaches_a_across_uses_and_p_up_part_of() {
    let designs = three_designs(true);
    let out = ripple(&designs, "design-b", &["ifc:i"]);

    assert_eq!(
        out.keys().map(String::as_str).collect::<Vec<_>>(),
        vec!["design-a", "design-b", "design-p"],
        "the ripple reaches all three designs"
    );
    assert!(
        reaches(&out["design-a"], "cmp:a-client") && reaches(&out["design-a"], "cap:a-feature"),
        "across `uses`: A's consumer of I and the capability on it"
    );
    assert!(
        reaches(&out["design-p"], "cap:p-serves"),
        "up `part_of`: P's capability that A serves"
    );
    assert_eq!(
        out["design-p"].seeds,
        vec!["cap:p-serves".to_string()],
        "the ripple enters P at what requires A, not at the pin, which would reach all of P"
    );
    for (design, r) in &out {
        assert_eq!(&r.design, design, "every radius names the design it ran in");
    }
}

#[test]
fn every_row_names_its_design_and_a_mirrored_node_names_where_it_came_from() {
    let designs = three_designs(true);
    // A change in A's client reaches the mirrored interface and B's engine.
    let r = designs["design-a"]
        .propagate_from(&["cmp:a-client"], OPTS)
        .unwrap();
    assert_eq!(r.design, "design-a");
    for id in ["ifc:i", "cmp:b-engine"] {
        let row = r
            .impacted
            .iter()
            .find(|n| n.node_id == id)
            .unwrap_or_else(|| panic!("{id} is reached"));
        assert_eq!(
            row.design.as_deref(),
            Some("design-b"),
            "{id} is B's, mirrored into A"
        );
    }
    let local = r
        .impacted
        .iter()
        .find(|n| n.node_id == "cap:a-feature")
        .unwrap();
    assert_eq!(local.design, None, "A's own node is the radius's design");

    let across = r
        .continue_in
        .iter()
        .find(|c| c.design == "design-b")
        .expect("the radius says to carry it on in B");
    assert_eq!(across.direction, "across");
    assert!(
        across.seeds.contains(&"ifc:i".to_string()),
        "{:?}",
        across.seeds
    );
    let summary = r.summarize();
    assert_eq!(summary.design, "design-a");
    assert!(!summary.continue_in.is_empty());
}

#[test]
fn a_parent_reaching_its_pin_of_a_part_says_to_carry_it_down() {
    let designs = three_designs(true);
    let r = designs["design-p"]
        .propagate_from(&["cap:p-serves"], OPTS)
        .unwrap();
    let down = r
        .continue_in
        .iter()
        .find(|c| c.design == "design-a")
        .expect("P's radius reaches its pin of A and says to continue in A");
    assert_eq!(down.direction, "down");
    assert!(
        down.seeds.is_empty() && down.why.contains("propagate from what the change touches"),
        "nothing of A is mirrored into P, and it says so rather than inventing seeds: {down:?}"
    );
}

#[test]
fn without_stated_relations_the_ripple_stays_in_the_design_it_started_in() {
    // The negative control: the same three designs, the pins naming each other
    // but stating no relation. Nothing carries the ripple, and nothing guesses.
    let designs = three_designs(false);
    let out = ripple(&designs, "design-b", &["ifc:i"]);
    assert_eq!(out.keys().collect::<Vec<_>>(), vec!["design-b"]);
    assert!(
        designs["design-p"]
            .arrival_seeds("design-b", &["ifc:i".to_string()])
            .unwrap()
            .is_empty(),
        "P names no B at all"
    );
}

#[test]
fn a_part_reached_only_through_the_parents_project_is_not_a_place_to_carry_it_down() {
    // Every pin hangs off the design's Project, so a radius that reaches the
    // Project reaches every pin through it. Only a part the radius reached
    // DIRECTLY is somewhere to continue; the rest would be noise.
    let mut designs = three_designs(true);
    let p = designs.get_mut("design-p").unwrap();
    p.declare_external_dependency(&pin("dep:c", "design-c", &["part_of"], &[]))
        .unwrap();
    let r = p.propagate_from(&["cap:p-serves"], OPTS).unwrap();
    let to: Vec<&str> = r.continue_in.iter().map(|c| c.design.as_str()).collect();
    assert!(
        to.contains(&"design-a"),
        "A is required by the capability: {to:?}"
    );
    assert!(
        !to.contains(&"design-c"),
        "C is reached only through P's Project, and nothing here needs it: {to:?}"
    );
}
