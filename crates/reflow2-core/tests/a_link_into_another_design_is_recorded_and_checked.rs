//! A relation to a node in another design is recorded as a typed link in the
//! design that makes it, and checked: a link into a design nothing here
//! declares, and a link whose far design changed since it was made, are
//! reported (`req:a-relation-between-nodes-in-two-designs-is-recorded-in-the-design-that-makes-it`,
//! build order item 4, Anthony's pick 2026-10-07).
//!
//! Before this, such a relation could be written only as prose, which no check,
//! search or ripple reads.

use reflow2_core::relate::RelationLink;
use reflow2_core::{DependencyDeclaration, DesignGraph, ObservedUpstream, PropagateOptions};

fn graph() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory()
        .unwrap()
        .with_graph_id("design-a");
    g.add_project("prj:a", "A").unwrap();
    g.add_requirement("req:here", "Here", "A requirement in design A.")
        .unwrap();
    g
}

fn link(g: &mut DesignGraph, relation: &str) -> String {
    let id = g
        .ensure_design_reference("design-b", "req:there", Some("Requirement"), Some("There"))
        .unwrap();
    g.review_relations(
        "Requirement",
        "req:here",
        &[RelationLink {
            relation: relation.into(),
            other_type: "Resource".into(),
            other_id: id.clone(),
            evidence: "the same need, said in two designs".into(),
            incoming: false,
        }],
        None,
    )
    .unwrap();
    id
}

fn pin(hash: &str) -> DependencyDeclaration {
    DependencyDeclaration {
        id: "dep:b".into(),
        name: "B".into(),
        source: "https://example.org/b".into(),
        version: "v1".into(),
        components: vec![],
        features: vec![],
        declared_in: None,
        graph_id: Some("design-b".into()),
        design_export: Some("b.json".into()),
        design_export_hash: Some(hash.into()),
        design_export_seen_at: None,
        design_address: None,
        design_address_hash: None,
        design_address_seen_at: None,
        note: None,
        relation: vec!["uses".into()],
        interfaces: vec![],
    }
}

fn seen(hash: &str) -> ObservedUpstream {
    ObservedUpstream {
        id: "dep:b".into(),
        state: "read".into(),
        content_hash: Some(hash.into()),
        graph_id: Some("design-b".into()),
        nodes: Some(10),
        detail: None,
    }
}

fn kinds(g: &DesignGraph, observed: &[ObservedUpstream]) -> Vec<&'static str> {
    g.reconcile_upstream(observed)
        .unwrap()
        .findings
        .into_iter()
        .filter(|f| f.kind.starts_with("link_"))
        .map(|f| f.kind)
        .collect()
}

#[test]
fn a_link_into_another_design_is_a_typed_reference_and_reads_back() {
    let mut g = graph();
    let id = link(&mut g, "DUPLICATES");
    assert_eq!(id, "xref:design-b:req:there");
    let refs = g.design_references().unwrap();
    assert_eq!(refs.len(), 1);
    let r = &refs[0];
    assert_eq!(
        (r.design.as_str(), r.node_id.as_str()),
        ("design-b", "req:there")
    );
    assert_eq!(r.name, "There");
    assert_eq!(r.links.len(), 1);
    assert_eq!(r.links[0].node_id, "req:here");
    assert_eq!(r.links[0].relation, "DUPLICATES");
    assert_eq!(r.links[0].direction, "out");
}

#[test]
fn a_link_to_this_design_or_with_no_target_is_refused() {
    let mut g = graph();
    assert!(
        g.ensure_design_reference("design-a", "req:here", None, None)
            .is_err(),
        "a node in this design is linked directly, not through a reference"
    );
    assert!(
        g.ensure_design_reference("design-b", " ", None, None)
            .is_err()
    );
    assert!(g.design_references().unwrap().is_empty());
}

#[test]
fn a_link_into_an_undeclared_design_is_reported() {
    let mut g = graph();
    link(&mut g, "DUPLICATES");
    let report = g.reconcile_upstream(&[]).unwrap();
    let f = report
        .findings
        .iter()
        .find(|f| f.kind == "link_into_undeclared_design")
        .expect("a link into a design nothing here declares cannot be checked, and says so");
    assert!(f.is_actionable());
    assert!(f.detail.contains("design-b"), "{}", f.detail);
}

#[test]
fn a_link_whose_far_design_moved_is_reported_until_it_is_made_again() {
    let mut g = graph();
    g.declare_external_dependency(&pin("sha256:aaa")).unwrap();
    link(&mut g, "DEPENDS_ON");
    assert!(
        kinds(&g, &[seen("sha256:aaa")]).is_empty(),
        "nothing moved, nothing to say"
    );
    assert_eq!(kinds(&g, &[seen("sha256:bbb")]), vec!["link_far_end_moved"]);

    // The owner re-reads the far design, re-declares the pin and makes the
    // link again: that is the acknowledgement.
    g.declare_external_dependency(&pin("sha256:bbb")).unwrap();
    link(&mut g, "DEPENDS_ON");
    assert!(kinds(&g, &[seen("sha256:bbb")]).is_empty());
}

#[test]
fn a_link_made_before_any_baseline_says_it_cannot_tell() {
    let mut g = graph();
    link(&mut g, "DUPLICATES");
    let mut unbaselined = pin("sha256:aaa");
    unbaselined.design_export_hash = None;
    g.declare_external_dependency(&unbaselined).unwrap();
    assert_eq!(kinds(&g, &[seen("sha256:aaa")]), vec!["link_unbaselined"]);
}

#[test]
fn a_radius_reaching_the_reference_names_the_other_design() {
    let mut g = graph();
    link(&mut g, "DUPLICATES");
    let r = g
        .propagate_from(&["req:here"], PropagateOptions { max_depth: 3 })
        .unwrap();
    let row = r
        .impacted
        .iter()
        .find(|n| n.node_id == "xref:design-b:req:there")
        .expect("the reference is one hop from the node that links to it");
    assert_eq!(row.design.as_deref(), Some("design-b"));
}
