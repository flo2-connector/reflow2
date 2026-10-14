//! "Has this followed through?" is answered in ONE place, and the follow-through
//! rule and the promotion detector both read it.
//!
//! Measured 2026-10-13 on reflow2's own design (main c58305ba8):
//! `unresolved_setup` raised 47 findings, and 20 of them pointed at something
//! the design had already acted on, in ways the rule never read
//! (`fact:root-cause-follow-through-reads-arrival-from-a-status-nothing-sets-and-misses-how-findings-close-2026-10-13`):
//!
//! - **5 at a requirement already DELIVERED.** The rule read arrival from
//!   `met`, which 0 of 385 requirements carry; delivery is derived from the
//!   thread, never read from a field (`report.rs`, BL-104).
//! - **13 at a finding already acted on.** The rule read a finding's
//!   follow-through from EVOLVES_INTO alone, while this design closes a finding
//!   with INVALIDATES (or a `valid_to`), answers it with MITIGATES from an
//!   accepted Decision, and records what it led to with CAUSES or TRIGGERS.
//! - **2 at a ChangeEvent or a DesignRule**, which the promotion detector
//!   already counted as taken up by their existence and this rule did not.
//!
//! The class is "arrived" re-derived by hand in each detector, with the copies
//! disagreeing. These tests pin the shared answer from both sides — the case it
//! clears and the nearest case it must NOT clear — and pin that the two rules
//! now agree, except in the two ways each rule's own definition names.
//!
//! The 18 findings that link two findings where the later one is still open
//! are NOT changed: whether such a link promises a follow-through at all is an
//! open ruling, so `a_link_to_a_later_finding_still_open_still_raises` pins the
//! behaviour as it stands.

use std::collections::HashMap;

use reflow2_core::detect::GapSource;
use reflow2_core::nodes::{Props, edge, node};
use reflow2_core::{DesignGraph, HealCategory, Value};

// ---- fixture -----------------------------------------------------------------

fn base() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.add_project("prj:p", "P").unwrap();
    g
}

fn decision(g: &mut DesignGraph, id: &str, status: &str, kind: Option<&str>) {
    let mut props = Props::new()
        .set("name", id)
        .set("decision", "something")
        .set("status", status);
    if let Some(k) = kind {
        props = props.set("kind", k);
    }
    g.create_node(node::DECISION, id, props).unwrap();
}

fn finding(g: &mut DesignGraph, id: &str) {
    g.create_node(
        node::TEMPORAL_FACT,
        id,
        Props::new()
            .set("name", id)
            .set("fact_type", "defect")
            .set("statement", "something is wrong")
            .set("subject_id", "prj:p"),
    )
    .unwrap();
}

fn change(g: &mut DesignGraph, id: &str) {
    g.create_node(
        node::CHANGE_EVENT,
        id,
        Props::new()
            .set("name", id)
            .set("change_type", "defect_fix")
            .set("subject", "system"),
    )
    .unwrap();
}

fn rule(g: &mut DesignGraph, id: &str) {
    g.create_node(
        node::DESIGN_RULE,
        id,
        Props::new()
            .set("name", id)
            .set("statement", "a rule in force"),
    )
    .unwrap();
}

fn requirement(g: &mut DesignGraph, id: &str, status: &str) {
    g.add_requirement(id, id, "it must hold").unwrap();
    g.set_requirement_status(id, status).unwrap();
}

fn status_node(g: &mut DesignGraph, ty: &str, id: &str, status: &str) {
    let mut props = Props::new().set("name", id).set("status", status);
    props = if ty == node::COMPONENT {
        props.set("purpose", "something")
    } else {
        props.set("description", "something")
    };
    g.create_node(ty, id, props).unwrap();
}

/// A requirement with a complete thread: a capability satisfies it, a file
/// realizes the capability, and a check on it carries `check_status`.
fn threaded_requirement(g: &mut DesignGraph, id: &str, check_status: &str) {
    requirement(g, id, "accepted");
    let cap = format!("cap:for-{id}");
    let art = format!("art:for-{id}");
    let ver = format!("ver:for-{id}");
    g.add_capability(&cap, &cap, "does it", Some("realized"))
        .unwrap();
    g.satisfies(&cap, id).unwrap();
    g.add_artifact(&art, &art, Some("code"), Some("src/it.rs"))
        .unwrap();
    g.realizes(&art, node::CAPABILITY, &cap, None, None)
        .unwrap();
    g.add_verification(&ver, &ver, Some("test"), None, None)
        .unwrap();
    g.verifies(&ver, node::CAPABILITY, &cap).unwrap();
    g.set_verification_status(&ver, check_status, None, None)
        .unwrap();
}

fn link(g: &mut DesignGraph, edge_type: &str, from: &str, to: &str) {
    link_with(g, edge_type, from, to, HashMap::new());
}

fn link_with(
    g: &mut DesignGraph,
    edge_type: &str,
    from: &str,
    to: &str,
    props: HashMap<String, Value>,
) {
    g.create_edge(edge_type, type_of(from), from, type_of(to), to, props)
        .unwrap();
}

/// The fixture names every node by its type's prefix.
fn type_of(id: &str) -> &'static str {
    let prefix = id.split(':').next().unwrap_or_default();
    match prefix {
        "dec" => node::DECISION,
        "fact" => node::TEMPORAL_FACT,
        "chg" => node::CHANGE_EVENT,
        "rule" => node::DESIGN_RULE,
        "req" => node::REQUIREMENT,
        "cap" => node::CAPABILITY,
        "cmp" => node::COMPONENT,
        "art" => node::ARTIFACT,
        "epoch" => node::DESIGN_EPOCH,
        other => panic!("no type for prefix {other}"),
    }
}

/// A settled setup anticipating `target`: an ACCEPTED decision, so the
/// parked-idea exemption cannot be what clears it.
fn anticipate(g: &mut DesignGraph, setup: &str, target: &str) {
    decision(g, setup, "accepted", None);
    link(g, edge::ANTICIPATES, setup, target);
}

/// Every unresolved_setup finding, as (anticipating, anticipated, message).
fn unresolved(g: &DesignGraph) -> Vec<(String, String, String)> {
    g.open_defects()
        .unwrap()
        .into_iter()
        .filter(|d| d.category == HealCategory::UnresolvedSetup)
        .map(|d| {
            let msg = d.message.clone();
            let mut ids = d.affected_ids.clone();
            ids.sort();
            (ids[0].clone(), ids[1].clone(), msg)
        })
        .collect()
}

fn raised_for(g: &DesignGraph, target: &str) -> bool {
    unresolved(g)
        .iter()
        .any(|(a, b, _)| a == target || b == target)
}

/// The decisions the promotion detector reports as overtaken.
fn overtaken(g: &DesignGraph) -> Vec<String> {
    let mut out: Vec<String> = g
        .detect_gaps()
        .unwrap()
        .into_iter()
        .filter(|x| x.gap_source == GapSource::DecisionOvertakenByPromotion)
        .flat_map(|x| x.affected_ids)
        .filter(|id| id.starts_with("dec:idea-"))
        .collect();
    out.sort();
    out.dedup();
    out
}

// ---- C1: a delivered requirement ---------------------------------------------

#[test]
fn a_delivered_requirement_is_followed_through() {
    let mut g = base();
    threaded_requirement(&mut g, "req:target", "passing");
    anticipate(&mut g, "dec:setup", "req:target");
    assert!(
        unresolved(&g).is_empty(),
        "the requirement is delivered — a realized capability with a passing check satisfies \
         it — and nobody had to set `met`: {:?}",
        unresolved(&g)
    );
}

#[test]
fn an_accepted_requirement_not_yet_delivered_still_raises() {
    // The nearest case: the whole thread is there except a passing check.
    let mut g = base();
    threaded_requirement(&mut g, "req:target", "failing");
    anticipate(&mut g, "dec:setup", "req:target");
    assert_eq!(
        unresolved(&g).len(),
        1,
        "accepted is the user's word that the need is real; a failing check means it is not \
         delivered, so the setup is still owed"
    );
}

// ---- C2: a finding already acted on ------------------------------------------

#[test]
fn a_finding_closed_by_a_change_is_followed_through() {
    let mut g = base();
    finding(&mut g, "fact:later");
    change(&mut g, "chg:fix");
    link(&mut g, edge::INVALIDATES, "chg:fix", "fact:later");
    finding(&mut g, "fact:earlier");
    link(&mut g, edge::ANTICIPATES, "fact:earlier", "fact:later");
    assert!(
        unresolved(&g).is_empty(),
        "a change closed the anticipated finding: {:?}",
        unresolved(&g)
    );
}

#[test]
fn a_finding_closed_by_its_valid_to_is_followed_through() {
    let mut g = base();
    finding(&mut g, "fact:later");
    g.upsert_node(
        node::TEMPORAL_FACT,
        "fact:later",
        Props::new().set("valid_to", "2026-10-01"),
    )
    .unwrap();
    anticipate(&mut g, "dec:setup", "fact:later");
    assert!(unresolved(&g).is_empty(), "{:?}", unresolved(&g));
}

#[test]
fn a_finding_answered_by_an_accepted_decision_is_followed_through() {
    let mut g = base();
    finding(&mut g, "fact:later");
    decision(&mut g, "dec:ruling", "accepted", None);
    link(&mut g, edge::MITIGATES, "dec:ruling", "fact:later");
    anticipate(&mut g, "dec:setup", "fact:later");
    assert!(
        unresolved(&g).is_empty(),
        "an accepted decision answered the anticipated finding: {:?}",
        unresolved(&g)
    );
}

#[test]
fn a_finding_only_an_idea_mitigates_is_not_answered() {
    let mut g = base();
    finding(&mut g, "fact:later");
    decision(&mut g, "dec:idea-maybe", "proposed", None);
    link(&mut g, edge::MITIGATES, "dec:idea-maybe", "fact:later");
    anticipate(&mut g, "dec:setup", "fact:later");
    assert_eq!(
        unresolved(&g).len(),
        1,
        "a `proposed` decision is somebody thinking out loud; it answers nothing"
    );
}

#[test]
fn a_finding_that_led_to_a_decision_change_or_requirement_is_followed_through() {
    for (edge_type, next) in [
        (edge::CAUSES, "chg:next"),
        (edge::TRIGGERS, "chg:next"),
        (edge::CAUSES, "dec:next"),
        (edge::CAUSES, "req:next"),
    ] {
        let mut g = base();
        finding(&mut g, "fact:later");
        let ty = type_of(next);
        match ty {
            node::CHANGE_EVENT => change(&mut g, next),
            node::DECISION => decision(&mut g, next, "proposed", None),
            _ => requirement(&mut g, next, "proposed"),
        }
        link(&mut g, edge_type, "fact:later", next);
        anticipate(&mut g, "dec:setup", "fact:later");
        assert!(
            unresolved(&g).is_empty(),
            "the anticipated finding led to a {ty} by {edge_type}: {:?}",
            unresolved(&g)
        );
    }
}

#[test]
fn a_refuted_cause_led_to_nothing() {
    let mut g = base();
    finding(&mut g, "fact:later");
    change(&mut g, "chg:next");
    link_with(
        &mut g,
        edge::CAUSES,
        "fact:later",
        "chg:next",
        Props::new().set("validation_status", "refuted").into(),
    );
    anticipate(&mut g, "dec:setup", "fact:later");
    assert_eq!(
        unresolved(&g).len(),
        1,
        "a CAUSES edge a test refuted asserts no link, so the finding led to nothing"
    );
}

// ---- C3: a ChangeEvent or DesignRule target ----------------------------------

#[test]
fn a_change_event_target_is_followed_through() {
    let mut g = base();
    change(&mut g, "chg:target");
    anticipate(&mut g, "dec:setup", "chg:target");
    assert!(
        unresolved(&g).is_empty(),
        "a recorded change happened: {:?}",
        unresolved(&g)
    );
}

#[test]
fn a_design_rule_target_is_followed_through() {
    let mut g = base();
    rule(&mut g, "rule:target");
    anticipate(&mut g, "dec:setup", "rule:target");
    assert!(
        unresolved(&g).is_empty(),
        "a design rule's existence is its adoption: {:?}",
        unresolved(&g)
    );
}

// ---- C4: unchanged ------------------------------------------------------------

#[test]
fn a_link_to_a_later_finding_still_open_still_raises() {
    // The shape of the 18 held for the owner's ruling: the later finding is
    // recorded against a capability, a file is named as its cause, and an idea
    // mitigates it — none of which acts on it.
    let mut g = base();
    finding(&mut g, "fact:earlier");
    finding(&mut g, "fact:later");
    g.add_capability("cap:subject", "Subject", "is about", Some("realized"))
        .unwrap();
    link(&mut g, edge::HAS_TEMPORAL_FACT, "cap:subject", "fact:later");
    g.add_artifact("art:cause", "cause.rs", Some("code"), Some("src/cause.rs"))
        .unwrap();
    link(&mut g, edge::CAUSES, "art:cause", "fact:later");
    decision(&mut g, "dec:idea-maybe", "proposed", None);
    link(&mut g, edge::MITIGATES, "dec:idea-maybe", "fact:later");
    link(&mut g, edge::ANTICIPATES, "fact:earlier", "fact:later");
    assert_eq!(
        unresolved(&g).len(),
        1,
        "whether a finding-to-finding link promises a follow-through is an open ruling; until \
         it is made the finding stands"
    );
}

// ---- the message names what was computed -------------------------------------

#[test]
fn the_message_names_the_condition_it_computed() {
    let mut g = base();
    finding(&mut g, "fact:later");
    anticipate(&mut g, "dec:setup-f", "fact:later");
    requirement(&mut g, "req:later", "accepted");
    anticipate(&mut g, "dec:setup-r", "req:later");
    let found = unresolved(&g);
    let msg_for = |target: &str| {
        found
            .iter()
            .find(|(a, b, _)| a == target || b == target)
            .map(|(_, _, m)| m.clone())
            .expect("raised")
    };
    let on_finding = msg_for("fact:later");
    for named in ["INVALIDATES", "valid_to", "MITIGATES", "CAUSES", "TRIGGERS"] {
        assert!(
            on_finding.contains(named),
            "the finding's message must name `{named}`, which the rule reads: {on_finding}"
        );
    }
    let on_requirement = msg_for("req:later");
    for named in ["delivered", "`met`"] {
        assert!(
            on_requirement.contains(named),
            "the requirement's message must name {named}: {on_requirement}"
        );
    }
}

// ---- the two rules agree -----------------------------------------------------

/// One target per kind, each anticipated by its own accepted setup AND the
/// promotion of its own idea. `true` means the kind has arrived.
fn agreement_fixture() -> (DesignGraph, Vec<(String, bool)>) {
    let mut g = base();
    let mut kinds: Vec<(String, bool)> = Vec::new();

    threaded_requirement(&mut g, "req:delivered", "passing");
    kinds.push(("req:delivered".into(), true));
    decision(&mut g, "dec:accepted", "accepted", None);
    kinds.push(("dec:accepted".into(), true));
    status_node(&mut g, node::CAPABILITY, "cap:realized", "realized");
    kinds.push(("cap:realized".into(), true));
    status_node(&mut g, node::COMPONENT, "cmp:verified", "verified");
    kinds.push(("cmp:verified".into(), true));
    status_node(&mut g, node::ARTIFACT, "art:realized", "realized");
    kinds.push(("art:realized".into(), true));
    status_node(&mut g, node::DESIGN_EPOCH, "epoch:arrived", "arrived");
    kinds.push(("epoch:arrived".into(), true));
    change(&mut g, "chg:recorded");
    kinds.push(("chg:recorded".into(), true));
    rule(&mut g, "rule:in-force");
    kinds.push(("rule:in-force".into(), true));

    finding(&mut g, "fact:closed");
    change(&mut g, "chg:closer");
    link(&mut g, edge::INVALIDATES, "chg:closer", "fact:closed");
    kinds.push(("fact:closed".into(), true));
    finding(&mut g, "fact:answered");
    decision(&mut g, "dec:answer", "accepted", None);
    link(&mut g, edge::MITIGATES, "dec:answer", "fact:answered");
    kinds.push(("fact:answered".into(), true));
    finding(&mut g, "fact:led-on");
    change(&mut g, "chg:led-to");
    link(&mut g, edge::CAUSES, "fact:led-on", "chg:led-to");
    kinds.push(("fact:led-on".into(), true));

    // And the nearest kinds that have NOT arrived.
    status_node(&mut g, node::CAPABILITY, "cap:planned", "planned");
    kinds.push(("cap:planned".into(), false));
    status_node(&mut g, node::DESIGN_EPOCH, "epoch:planned", "planned");
    kinds.push(("epoch:planned".into(), false));
    requirement(&mut g, "req:proposed", "proposed");
    kinds.push(("req:proposed".into(), false));
    requirement(&mut g, "req:dropped", "dropped");
    kinds.push(("req:dropped".into(), false));
    decision(&mut g, "dec:superseded", "superseded", None);
    kinds.push(("dec:superseded".into(), false));
    decision(&mut g, "dec:deferred", "deferred", None);
    kinds.push(("dec:deferred".into(), false));
    finding(&mut g, "fact:bare");
    kinds.push(("fact:bare".into(), false));
    finding(&mut g, "fact:only-an-idea-mitigates");
    decision(&mut g, "dec:musing", "proposed", None);
    link(
        &mut g,
        edge::MITIGATES,
        "dec:musing",
        "fact:only-an-idea-mitigates",
    );
    kinds.push(("fact:only-an-idea-mitigates".into(), false));

    for (i, (target, _)) in kinds.iter().enumerate() {
        anticipate(&mut g, &format!("dec:setup-{i}"), target);
        let idea = format!("dec:idea-{i}");
        decision(&mut g, &idea, "proposed", None);
        link(&mut g, edge::EVOLVES_INTO, &idea, target);
    }
    (g, kinds)
}

#[test]
fn the_promotion_detector_and_the_follow_through_rule_agree() {
    let (g, kinds) = agreement_fixture();
    let overtaken = overtaken(&g);
    let mut disagreements = Vec::new();
    for (i, (target, arrived)) in kinds.iter().enumerate() {
        let follow_through_says = !raised_for(&g, target);
        let promotion_says = overtaken.contains(&format!("dec:idea-{i}"));
        if follow_through_says != *arrived || promotion_says != *arrived {
            disagreements.push(format!(
                "{target}: arrived={arrived}, follow-through says {follow_through_says}, \
                 promotion says {promotion_says}"
            ));
        }
    }
    assert!(
        disagreements.is_empty(),
        "both rules must read arrival the same way:\n{}",
        disagreements.join("\n")
    );
}

/// The two places the rules differ, each named by that rule's own definition
/// and pinned here so neither difference can grow by accident.
#[test]
fn they_differ_only_where_each_rule_says_so() {
    // 1. Promotion: an idea that became intent the owner ACCEPTED was answered
    //    by that acceptance (`an_idea_promoted_into_an_accepted_requirement_is_asked_about`).
    //    Follow-through: an accepted requirement is a need agreed, not a need
    //    met (`an_accepted_requirement_is_a_need_agreed_not_a_need_met`).
    let mut g = base();
    requirement(&mut g, "req:agreed", "accepted");
    anticipate(&mut g, "dec:setup", "req:agreed");
    decision(&mut g, "dec:idea-agreed", "proposed", None);
    link(&mut g, edge::EVOLVES_INTO, "dec:idea-agreed", "req:agreed");
    assert!(
        raised_for(&g, "req:agreed"),
        "follow-through waits for delivery"
    );
    assert_eq!(
        overtaken(&g),
        vec!["dec:idea-agreed".to_string()],
        "promotion is answered by the acceptance"
    );

    // 2. Follow-through: a target that EVOLVED INTO something had work recorded
    //    against it (the owner's word, 2026-09-15). Promotion: what the idea
    //    became must itself have LANDED; that it became something else again
    //    is a chain of ideas, not an answer.
    let mut g = base();
    decision(&mut g, "dec:choice", "proposed", Some("choice"));
    change(&mut g, "chg:later");
    link(&mut g, edge::EVOLVES_INTO, "dec:choice", "chg:later");
    anticipate(&mut g, "dec:setup", "dec:choice");
    decision(&mut g, "dec:idea-chain", "proposed", None);
    link(&mut g, edge::EVOLVES_INTO, "dec:idea-chain", "dec:choice");
    assert!(
        !raised_for(&g, "dec:choice"),
        "work was recorded against it: {:?}",
        unresolved(&g)
    );
    assert!(
        overtaken(&g).is_empty(),
        "an open choice that evolved has not landed: {:?}",
        overtaken(&g)
    );
}
