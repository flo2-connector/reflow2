//! `req:one-composed-check-runs-over-a-hub-and-several-of-its-designs` — the
//! core of it: `DesignGraph::compose_many` composes this design with several
//! others in ONE combined graph and attributes every finding to the set of
//! designs its ids belong to.
//!
//! The class, across none, one and several other designs: every finding's
//! `designs` is exactly the designs its ids name (this design's ids as they
//! are, another's as `<design>:<id>`); with one other design the findings are
//! the two-design composition's, bucket for bucket; nothing is written; and a
//! name that would collide is refused.

use std::collections::BTreeSet;

use reflow2_core::compose::{AttributedFinding, ComposeWith, ManyComposedReport, Side};
use reflow2_core::graph::DesignGraph;
use reflow2_core::nodes::{Props, edge, node};

/// A design with a Project, a Requirement, a Capability, a Component and an
/// Interface it consumes but nothing provides — and `cmp:store`, an id every
/// design here holds.
fn design(gid: &str, word: &str) -> DesignGraph {
    let mut g = DesignGraph::open_in_memory_as(gid).unwrap();
    g.add_project(&format!("proj:{word}"), word).unwrap();
    g.add_requirement(&format!("req:{word}"), word, &format!("{word} works"))
        .unwrap();
    g.add_capability(&format!("cap:{word}"), word, "does it", None)
        .unwrap();
    g.add_component("cmp:store", &format!("{word} store"), "a store", None)
        .unwrap();
    g.satisfies(&format!("cap:{word}"), &format!("req:{word}"))
        .unwrap();
    g.allocate(&format!("cap:{word}"), "cmp:store").unwrap();
    g.add_interface("ifc:bytes", "Byte store API").unwrap();
    g.create_edge(
        edge::CONSUMES,
        node::COMPONENT,
        "cmp:store",
        node::INTERFACE,
        "ifc:bytes",
        Props::new(),
    )
    .unwrap();
    g
}

/// The design an id in a composed finding belongs to.
fn design_of(id: &str, this: &str, others: &[&str]) -> String {
    others
        .iter()
        .find(|d| id.starts_with(&format!("{d}:")))
        .map_or_else(|| this.to_string(), |d| (*d).to_string())
}

/// THE CLASS, asked of one report.
fn assert_attributed(r: &ManyComposedReport, others: &[&str]) {
    let order: Vec<&str> = std::iter::once(r.this_design.as_str())
        .chain(others.iter().copied())
        .collect();
    assert!(!r.findings.is_empty(), "the fixture yields findings: {r:?}");
    for f in &r.findings {
        let named: BTreeSet<String> = f
            .affected_ids
            .iter()
            .map(|i| design_of(i, &r.this_design, others))
            .collect();
        let filed: BTreeSet<String> = f.designs.iter().cloned().collect();
        assert_eq!(
            named, filed,
            "filed under exactly the designs it names: {f:?}"
        );
        // In composition order, this design first.
        let positions: Vec<usize> = f
            .designs
            .iter()
            .map(|d| {
                order
                    .iter()
                    .position(|o| o == d)
                    .expect("a composed design")
            })
            .collect();
        assert!(positions.windows(2).all(|w| w[0] < w[1]), "{f:?}");
        assert!(
            f.affected_ids.iter().all(|i| !i.contains("::")),
            "ids are given in the form a reader opens, never the combined graph's: {f:?}"
        );
    }
}

fn with<'a>(designs: &[(&'a str, &'a reflow2_core::GraphExport)]) -> Vec<ComposeWith<'a>> {
    designs
        .iter()
        .map(|(design, export)| ComposeWith { design, export })
        .collect()
}

#[test]
fn with_one_other_design_the_findings_are_the_two_design_compositions() {
    let ours = design("hub", "hub");
    let theirs = design("deploy", "deploy").export_graph().unwrap();
    let pairwise = ours.compose_and_analyse(&theirs, "deploy").unwrap();
    let many = ours.compose_many(&with(&[("deploy", &theirs)])).unwrap();
    assert_attributed(&many, &["deploy"]);
    let side = |f: &AttributedFinding| match f.designs.as_slice() {
        [] => Side::Ours,
        [d] if d == "hub" => Side::Ours,
        [_] => Side::Theirs,
        _ => Side::Seam,
    };
    let key = |kind: &str, source: &str, title: &str, side: Side| {
        format!("{side:?} {kind} {source} {title}")
    };
    let mut got: Vec<String> = many
        .findings
        .iter()
        .map(|f| key(f.kind, &f.source, &f.title, side(f)))
        .collect();
    let mut want: Vec<String> = pairwise
        .our_findings
        .iter()
        .chain(&pairwise.their_findings)
        .chain(&pairwise.seam_findings)
        .map(|f| key(f.kind, &f.source, &f.title, f.side))
        .collect();
    got.sort();
    want.sort();
    assert_eq!(got, want, "the same findings, on the same sides");
    assert_eq!(many.composed[0].nodes, pairwise.imported_nodes);
    assert_eq!(many.composed[0].edges, pairwise.imported_edges);
}

#[test]
fn with_several_designs_each_finding_is_filed_under_the_designs_it_names() {
    let hub = design("hub", "hub");
    let a = design("deploy", "deploy").export_graph().unwrap();
    let b = design("storage", "storage").export_graph().unwrap();
    let r = hub
        .compose_many(&with(&[("deploy", &a), ("storage", &b)]))
        .unwrap();
    assert_attributed(&r, &["deploy", "storage"]);
    // Every design's own finding is there.
    for d in ["hub", "deploy", "storage"] {
        assert!(
            r.findings.iter().any(|f| f.designs == [d]),
            "{d} has a finding of its own: {r:?}"
        );
    }
    // A member's nodes of every type a finding names carry its prefix.
    let ids: BTreeSet<&str> = r
        .findings
        .iter()
        .flat_map(|f| f.affected_ids.iter().map(String::as_str))
        .collect();
    let types: BTreeSet<&str> = ids
        .iter()
        .filter_map(|i| i.strip_prefix("storage:"))
        .filter_map(|i| i.split(':').next())
        .collect();
    assert!(
        types.len() >= 2,
        "findings name several node types: {ids:?}"
    );
    assert_eq!(r.composed.len(), 2);
    assert_eq!(r.this_design, "hub");
}

#[test]
fn a_seam_between_two_other_designs_is_filed_under_that_pair() {
    // The hub holds no Project, so the check that reads every design's
    // Project names the two members' and nothing of the hub's.
    let mut hub = DesignGraph::open_in_memory_as("hub").unwrap();
    hub.add_requirement("req:fix", "fix", "the fix lands")
        .unwrap();
    let a = design("deploy", "deploy").export_graph().unwrap();
    let b = design("storage", "storage").export_graph().unwrap();
    let r = hub
        .compose_many(&with(&[("deploy", &a), ("storage", &b)]))
        .unwrap();
    assert_attributed(&r, &["deploy", "storage"]);
    assert!(
        r.findings
            .iter()
            .any(|f| f.designs == ["deploy", "storage"]),
        "a finding naming both members and not the design calling: {r:?}"
    );
}

#[test]
fn with_no_other_design_every_finding_is_this_designs_or_the_wholes() {
    let hub = design("hub", "hub");
    let r = hub.compose_many(&[]).unwrap();
    assert_attributed(&r, &[]);
    assert!(r.composed.is_empty());
    assert!(
        r.findings
            .iter()
            .all(|f| f.designs.is_empty() || f.designs == ["hub"])
    );
}

#[test]
fn composing_several_writes_nothing() {
    let hub = design("hub", "hub");
    let before = hub.export_graph().unwrap();
    let a = design("deploy", "deploy").export_graph().unwrap();
    let b = design("storage", "storage").export_graph().unwrap();
    hub.compose_many(&with(&[("deploy", &a), ("storage", &b)]))
        .unwrap();
    let after = hub.export_graph().unwrap();
    assert_eq!(before.content_hash, after.content_hash);
    assert!(!after.nodes.iter().any(|n| n.node_id.contains("::")));
}

#[test]
fn a_name_that_would_collide_is_refused() {
    let hub = design("hub", "hub");
    let a = design("deploy", "deploy").export_graph().unwrap();
    for names in [vec![" "], vec!["hub"], vec!["deploy", "deploy"]] {
        let pairs: Vec<(&str, &reflow2_core::GraphExport)> =
            names.iter().map(|d| (*d, &a)).collect();
        assert!(hub.compose_many(&with(&pairs)).is_err(), "{names:?}");
    }
}
