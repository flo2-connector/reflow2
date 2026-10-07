//! A member design's pin says how it stands to this design, and a member linked
//! by nothing a ripple could follow is reported, never guessed
//! (`cap:a-hub-member-records-its-relation-and-the-interfaces-it-crosses`,
//! slice 1 of `req:a-hub-records-each-members-relation-and-a-cross-design-ripple-follows-it`).
//!
//! Measured 2026-10-05 in the dev_reflow2 hub: its six member designs were
//! linked only by pins that said nothing about how they stood to one another,
//! so a change in one had no recorded way into the others.

use reflow2_core::{DependencyDeclaration, DesignGraph, UpstreamReport};

fn graph() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("in-memory graph");
    g.add_project("prj:hub", "Hub").unwrap();
    g
}

fn member(id: &str, relation: &[&str], interfaces: &[&str]) -> DependencyDeclaration {
    DependencyDeclaration {
        id: format!("dep:{id}"),
        name: id.into(),
        source: format!("https://example.org/{id}"),
        version: "v1.0.0".into(),
        components: vec![],
        features: vec![],
        declared_in: None,
        graph_id: Some(format!("{id}0000000000")),
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

fn report(g: &DesignGraph) -> UpstreamReport {
    g.reconcile_upstream(&[])
        .expect("the upstream report reads")
}

fn kinds_for<'a>(r: &'a UpstreamReport, dep: &str) -> Vec<&'a str> {
    r.findings
        .iter()
        // Only the relation's own findings: an unwatched member also gets the
        // watch's `not_watched`, which says nothing about how it is linked.
        .filter(|f| f.dependency == dep)
        .filter(|f| matches!(f.kind, "relation_not_stated" | "no_interface_to_follow"))
        .map(|f| f.kind)
        .collect()
}

#[test]
fn a_stated_relation_survives_the_round_trip_and_is_read_back() {
    let mut g = graph();
    g.declare_external_dependency(&member("engine", &["part_of"], &[]))
        .unwrap();
    let back = g.declared_dependencies().unwrap();
    assert_eq!(back[0].relation, vec!["part_of".to_string()]);
    let r = report(&g);
    assert_eq!(r.members.len(), 1);
    assert_eq!(r.members[0].relation, vec!["part_of".to_string()]);
    assert!(
        kinds_for(&r, "dep:engine").is_empty(),
        "a part_of member is linked; nothing is owed: {:?}",
        r.findings
    );
}

#[test]
fn a_member_that_does_not_say_is_reported_as_not_stated_and_never_defaulted() {
    let mut g = graph();
    g.declare_external_dependency(&member("engine", &[], &[]))
        .unwrap();
    let r = report(&g);
    assert_eq!(r.members[0].relation, vec!["not stated".to_string()]);
    let f = r
        .findings
        .iter()
        .find(|f| f.kind == "relation_not_stated")
        .expect("an unstated relation is a finding");
    assert!(f.is_actionable(), "it asks the reader to do something");
    assert!(
        f.detail.contains("PART OF") && f.detail.contains("USES"),
        "it names both answers the person can give: {}",
        f.detail
    );
    assert!(r.note.contains("linked by nothing"), "{}", r.note);
}

#[test]
fn a_uses_link_needs_an_interface_here_that_a_ripple_can_follow() {
    let mut g = graph();
    g.declare_external_dependency(&member("calc", &["uses"], &[]))
        .unwrap();
    assert_eq!(
        kinds_for(&report(&g), "dep:calc"),
        vec!["no_interface_to_follow"],
        "uses with no interface named"
    );

    g.declare_external_dependency(&member("calc", &["uses"], &["ifc:calc-api"]))
        .unwrap();
    let r = report(&g);
    let f = r
        .findings
        .iter()
        .find(|f| f.dependency == "dep:calc" && f.kind != "not_watched")
        .expect("an interface this design does not hold is still nothing to follow");
    assert_eq!(f.kind, "no_interface_to_follow");
    assert!(f.detail.contains("ifc:calc-api"), "{}", f.detail);

    g.add_interface("ifc:calc-api", "the calculator's API (mirrored)")
        .unwrap();
    assert!(
        kinds_for(&report(&g), "dep:calc").is_empty(),
        "once the interface is here, the use is linked"
    );
}

#[test]
fn a_member_may_be_both_a_part_and_a_peer() {
    let mut g = graph();
    g.add_interface("ifc:door", "the door").unwrap();
    g.declare_external_dependency(&member("both", &["part_of", "uses"], &["ifc:door"]))
        .unwrap();
    let r = report(&g);
    assert_eq!(
        r.members[0].relation,
        vec!["part_of".to_string(), "uses".to_string()]
    );
    assert_eq!(r.members[0].interfaces, vec!["ifc:door".to_string()]);
    assert!(kinds_for(&r, "dep:both").is_empty());
}

#[test]
fn an_unknown_relation_and_interfaces_without_uses_are_refused() {
    let mut g = graph();
    let err = g
        .declare_external_dependency(&member("x", &["depends"], &[]))
        .expect_err("`depends` is not a relation");
    assert!(format!("{err:?}").contains("part_of"), "{err:?}");
    let err = g
        .declare_external_dependency(&member("y", &["part_of"], &["ifc:door"]))
        .expect_err("interfaces belong to a uses relation");
    assert!(format!("{err:?}").contains("uses"), "{err:?}");
    assert!(
        g.declared_dependencies().unwrap().is_empty(),
        "a refused declaration stores nothing"
    );
}

#[test]
fn an_empty_relation_on_a_re_declare_clears_the_stored_one() {
    let mut g = graph();
    g.declare_external_dependency(&member("engine", &["part_of"], &[]))
        .unwrap();
    g.declare_external_dependency(&member("engine", &[], &[]))
        .unwrap();
    assert!(g.declared_dependencies().unwrap()[0].relation.is_empty());
}

#[test]
fn a_code_dependency_that_names_no_design_is_never_asked_for_a_relation() {
    let mut g = graph();
    let mut serde = member("serde", &[], &[]);
    serde.graph_id = None;
    g.declare_external_dependency(&serde).unwrap();
    let r = report(&g);
    assert!(r.members.is_empty());
    assert!(
        r.findings.iter().all(|f| f.kind != "relation_not_stated"),
        "serde is not a member: {:?}",
        r.findings
    );
}
