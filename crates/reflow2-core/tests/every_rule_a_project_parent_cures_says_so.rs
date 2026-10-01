//! Every hierarchy rule that a Project parent satisfies says so, in one set of
//! words, and the cure it names really does clear it.
//!
//! The class, from two root causes a fortnight apart. On 2026-09-16
//! (`fact:root-cause-a-childless-root-part-has-no-legal-level-because-two-self-limiting-hierarchy-rules-were-never-checked-jointly`)
//! BOTH rules that accept a Project parent were found to hide it, and the fix
//! reached one message. On 2026-09-29 the other one, `level_spine_disagreement`,
//! sent an agent modelling an OUTSIDE gateway to nest it inside the engine, which
//! misstated the design (dev_reflow2 two-agent exercise, I11,
//! `fact:root-cause-an-external-part-trips-level-spine-disagreement-and-the-cure-is-named-only-on-its-sibling-rule-2026-09-29`).
//!
//! So this walks [`HierarchyIssueKind::ALL`]: every kind that declares a Project
//! parent cures it must carry [`PROJECT_PARENT_CURE`] and must, measured, go away
//! when the part is contained under the Project. A kind flipped to `true`
//! without a fixture here fails the enumeration, not a later field report.

use reflow2_core::DesignGraph;
use reflow2_core::HierarchyIssueKind;
use reflow2_core::hierarchy::PROJECT_PARENT_CURE;

/// A design raising `kind` on `cmp:subject`, for every kind a Project parent
/// cures. `None` for the rest.
fn raising(kind: HierarchyIssueKind) -> Option<DesignGraph> {
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.add_project("prj:p", "P").unwrap();
    match kind {
        // A part above the bottom rung, with no parent and no child.
        HierarchyIssueKind::OrphanLevel => {
            g.add_component("cmp:subject", "Subject", "floats", Some("subsystem"))
                .unwrap();
        }
        // A part below the top level present, sitting at the spine's root.
        HierarchyIssueKind::LevelSpineDisagreement => {
            g.add_component("cmp:engine", "Engine", "the system", Some("system"))
                .unwrap();
            g.contains("prj:p", "Component", "cmp:engine").unwrap();
            g.add_component(
                "cmp:subject",
                "An outside gateway",
                "exists beside the system, not inside it",
                Some("component"),
            )
            .unwrap();
        }
        _ => return None,
    }
    Some(g)
}

#[test]
fn every_kind_a_project_parent_cures_names_the_cure_and_the_cure_clears_it() {
    let mut walked = 0;
    for kind in HierarchyIssueKind::ALL {
        if !kind.cured_by_project_parent() {
            continue;
        }
        let mut g = raising(kind).unwrap_or_else(|| {
            panic!(
                "{} says a Project parent cures it and this suite has no design raising it — \
                 write the fixture, so the claim is measured rather than declared",
                kind.as_str()
            )
        });
        let issues = g.hierarchy_issues().unwrap();
        let issue = issues
            .iter()
            .find(|i| i.kind == kind && i.components == ["cmp:subject"])
            .unwrap_or_else(|| panic!("the fixture must raise {}: {issues:?}", kind.as_str()));
        assert!(
            issue.message.contains(PROJECT_PARENT_CURE),
            "{} is cured by a Project parent and its words must say so, in the shared \
             sentence: {}",
            kind.as_str(),
            issue.message
        );

        g.contains("prj:p", "Component", "cmp:subject").unwrap();
        let after = g.hierarchy_issues().unwrap();
        assert!(
            !after
                .iter()
                .any(|i| i.kind == kind && i.components == ["cmp:subject"]),
            "the cure {} names must clear it: {after:?}",
            kind.as_str()
        );
        walked += 1;
    }
    assert!(
        walked >= 2,
        "orphan_level and level_spine_disagreement at least"
    );
}

#[test]
fn a_kind_the_project_does_not_cure_does_not_offer_it() {
    for kind in HierarchyIssueKind::ALL {
        if kind.cured_by_project_parent() {
            continue;
        }
        assert!(
            raising(kind).is_none(),
            "{} has a fixture here but declares no Project cure — say which",
            kind.as_str()
        );
    }
}
