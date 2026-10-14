//! What awaits the owner's word is LISTED: built work whose record is still
//! open, each item with what delivers it, so an owner can close it in one look.
//!
//! Measured on reflow2's own design 2026-10-13 (main c58305ba8,
//! `fact:root-cause-built-work-stays-open-because-its-record-is-kept-by-hand-and-the-ship-moves-only-the-capability-2026-10-13`):
//! 33 of 82 proposed requirements were already delivered, and no gap kind asks
//! about a proposed requirement whose work shipped; the promotion detector
//! found 13 open ideas whose promotion had landed (#743). Closing either is the
//! owner's word — so the fix is not a write, it is a list the owner reads.
//!
//! Two families, both computed by code that already existed:
//! - a `proposed` Requirement the shared arrival predicate (`arrival.rs`) reads
//!   as DELIVERED, with the capabilities (or delivered children) that deliver it;
//! - a `proposed` Decision whose `EVOLVES_INTO` target has landed — the promotion
//!   detector's own population, with what the idea became.
//!
//! It rides `loop_status`'s owner-scoped answer, beside the decisions a person
//! was asked to settle and the questions put to them by name, so the approval
//! page (`dec:idea-an-owner-reads-what-they-are-asked-to-approve-in-one-addressed-digest`)
//! can build on one call. It never moves a status.

use std::collections::HashMap;

use reflow2_core::awaiting::AwaitingKind;
use reflow2_core::detect::GapSource;
use reflow2_core::nodes::{Props, edge, node};
use reflow2_core::{DesignGraph, Value};

fn base() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.add_project("prj:p", "P").unwrap();
    g.add_contributor("who:owner", "The Owner", None, None, None)
        .unwrap();
    g
}

fn requirement(g: &mut DesignGraph, id: &str, status: &str) {
    g.add_requirement(id, id, "it must hold").unwrap();
    g.set_requirement_status(id, status).unwrap();
}

fn decision(g: &mut DesignGraph, id: &str, status: &str) {
    g.create_node(
        node::DECISION,
        id,
        Props::new()
            .set("name", id)
            .set("decision", "should we?")
            .set("kind", "exploratory")
            .set("status", status),
    )
    .unwrap();
}

/// A requirement at `status` with a complete thread: a capability satisfies
/// it, a file realizes the capability, and a check on it carries
/// `check_status`.
fn threaded_requirement(g: &mut DesignGraph, id: &str, status: &str, check_status: &str) {
    requirement(g, id, status);
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

fn evolves(g: &mut DesignGraph, idea: &str, target_type: &str, target: &str) {
    g.create_edge(
        edge::EVOLVES_INTO,
        node::DECISION,
        idea,
        target_type,
        target,
        HashMap::<String, Value>::new(),
    )
    .unwrap();
}

fn status(g: &DesignGraph, ty: &str, id: &str) -> Option<String> {
    g.get_node(ty, id).unwrap().and_then(|n| {
        n.properties
            .get("status")
            .and_then(Value::as_str)
            .map(str::to_string)
    })
}

/// The fixture every test reads: one of each kind that IS awaiting, and the
/// nearest case of each that is NOT.
fn fixture() -> DesignGraph {
    let mut g = base();
    // Awaiting: proposed, and delivered by its thread.
    threaded_requirement(&mut g, "req:shipped", "proposed", "passing");
    // Not awaiting: the owner already accepted it.
    threaded_requirement(&mut g, "req:agreed", "accepted", "passing");
    // Not awaiting: proposed, but its check fails, so it is not delivered.
    threaded_requirement(&mut g, "req:red", "proposed", "failing");

    // Awaiting: a proposed idea that became a requirement the owner accepted.
    decision(&mut g, "dec:idea-promoted", "proposed");
    evolves(&mut g, "dec:idea-promoted", node::REQUIREMENT, "req:agreed");
    // Not awaiting: the owner already accepted the idea itself.
    decision(&mut g, "dec:idea-settled", "accepted");
    evolves(&mut g, "dec:idea-settled", node::REQUIREMENT, "req:agreed");
    // Not awaiting: what the idea became has not landed.
    requirement(&mut g, "req:wish", "proposed");
    decision(&mut g, "dec:idea-pending", "proposed");
    evolves(&mut g, "dec:idea-pending", node::REQUIREMENT, "req:wish");
    g
}

#[test]
fn a_delivered_proposed_requirement_is_listed_with_what_delivers_it() {
    let g = fixture();
    let list = g.awaiting_word().unwrap();
    let item = list
        .iter()
        .find(|i| i.node_id == "req:shipped")
        .unwrap_or_else(|| panic!("a delivered proposed requirement is listed: {list:?}"));
    assert_eq!(item.kind, AwaitingKind::DeliveredRequirement);
    assert_eq!(item.status, "proposed");
    assert_eq!(
        item.evidence_ids,
        ["cap:for-req:shipped"],
        "it carries what delivers it, by id"
    );
    assert!(
        item.evidence.contains("cap:for-req:shipped"),
        "and says so in words: {}",
        item.evidence
    );
    assert!(
        item.closed_by.contains("set_requirement_status"),
        "it names the call that closes it: {}",
        item.closed_by
    );
    let wire = serde_json::to_value(item).unwrap();
    assert_eq!(wire["kind"], "delivered_requirement", "{wire}");
}

#[test]
fn a_proposed_idea_whose_promotion_landed_is_listed_with_what_it_became() {
    let g = fixture();
    let list = g.awaiting_word().unwrap();
    let item = list
        .iter()
        .find(|i| i.node_id == "dec:idea-promoted")
        .unwrap_or_else(|| panic!("a promoted idea is listed: {list:?}"));
    assert_eq!(item.kind, AwaitingKind::PromotedIdea);
    assert_eq!(item.status, "proposed");
    assert_eq!(item.evidence_ids, ["req:agreed"]);
    assert!(
        item.closed_by.contains("set_decision_status"),
        "{}",
        item.closed_by
    );

    // The SAME population the promotion detector reports: one reading, two
    // readers.
    let mut detected: Vec<String> = g
        .detect_gaps()
        .unwrap()
        .into_iter()
        .filter(|x| x.gap_source == GapSource::DecisionOvertakenByPromotion)
        .flat_map(|x| x.affected_ids)
        .filter(|id| id.starts_with("dec:"))
        .collect();
    detected.sort();
    let mut listed: Vec<String> = list
        .iter()
        .filter(|i| i.kind == AwaitingKind::PromotedIdea)
        .map(|i| i.node_id.clone())
        .collect();
    listed.sort();
    assert_eq!(listed, detected);
}

#[test]
fn an_accepted_one_is_not_listed_and_neither_is_work_not_yet_delivered() {
    let g = fixture();
    let ids: Vec<String> = g
        .awaiting_word()
        .unwrap()
        .into_iter()
        .map(|i| i.node_id)
        .collect();
    for absent in [
        "req:agreed",
        "dec:idea-settled",
        "req:red",
        "req:wish",
        "dec:idea-pending",
    ] {
        assert!(
            !ids.iter().any(|i| i == absent),
            "{absent} does not await the owner's word: {ids:?}"
        );
    }
    assert_eq!(
        ids,
        ["req:shipped", "dec:idea-promoted"],
        "requirements first"
    );
}

#[test]
fn loop_status_lists_it_for_a_named_owner_and_moves_no_status() {
    let g = fixture();
    let before: Vec<Option<String>> = [
        (node::REQUIREMENT, "req:shipped"),
        (node::DECISION, "dec:idea-promoted"),
    ]
    .iter()
    .map(|(t, id)| status(&g, t, id))
    .collect();

    let scoped = g.loop_status_for(Some("who:owner")).unwrap();
    let listed: Vec<&str> = scoped
        .awaiting_word
        .iter()
        .map(|i| i.node_id.as_str())
        .collect();
    assert_eq!(listed, ["req:shipped", "dec:idea-promoted"]);
    assert!(
        scoped.next.iter().any(|n| n.contains("`awaiting_word`")),
        "the scoped `next` says where they are: {:?}",
        scoped.next
    );
    assert!(
        scoped.clean,
        "nothing records whose word closes them, so they are not counted as this person's: \
         {:?}",
        scoped.next
    );

    let unscoped = g.loop_status_for(None).unwrap();
    assert!(
        unscoped.awaiting_word.is_empty(),
        "the every-session answer stays as cheap as it was; the list rides the scoped one"
    );

    let after: Vec<Option<String>> = [
        (node::REQUIREMENT, "req:shipped"),
        (node::DECISION, "dec:idea-promoted"),
    ]
    .iter()
    .map(|(t, id)| status(&g, t, id))
    .collect();
    assert_eq!(before, after, "listing moves no status");
    assert_eq!(after[0].as_deref(), Some("proposed"));
}
