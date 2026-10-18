//! A near-match at a PRESCRIBED LAYER of the capture loop is reported, not refused.
//!
//! # The count that reopened this
//!
//! `chg:a-cross-type-near-match-is-named-as-a-layer` (2026-08-29) shipped the
//! conservative half of this repair — reword the refusal when the types differ
//! — and REJECTED the behavioural half for exactly one stated reason:
//!
//! > Nobody has counted how often a cross-type hit IS a real duplicate, so
//! > suppressing the check would act on an unmeasured assumption.
//!
//! `fact:twelve-near-match-refusals-across-two-designs-and-not-one-was-a-duplicate`
//! is that count, taken 2026-08-31 from two field reports written by sessions
//! that could not see each other, on designs with nothing in common (a
//! shipping-container aquaponics build and a children's music game):
//!
//! * **12 refusals in the field. 12 judged distinct. 0 real duplicates.**
//! * **3 more incurred by the triage session itself**, writing the very
//!   decision that acts on the count — and those three are the only ones not
//!   self-reported by an agent grading its own work.
//!
//! Both reports independently named the same pairings. Anthony chose the
//! behavioural half on 2026-08-31.
//!
//! # Why the false-positive rate is not the whole argument
//!
//! It would be, if the guard were still doing its job. It is not. bhome, at its
//! eighth refusal:
//!
//! > I have stopped treating each as noteworthy and started passing
//! > `distinct_from` pre-emptively, which is itself the concerning outcome: a
//! > guard that is routinely pre-empted has stopped being a check.
//!
//! ⭐ SO THE SILENT MISS THAT OPTION ① WAS REJECTED TO AVOID HAD ALREADY
//! ARRIVED BY ANOTHER ROUTE — an agent that pre-declares `distinct_from` before
//! every write is not being checked at all, and unlike a suppression it leaves
//! no record of the trade.
//!
//! # What is suppressed, and what is emphatically not
//!
//! 🛑 THE REFUSAL IS SUPPRESSED. THE REPORT IS NOT. A prescribed-layer match
//! still comes back in `search_first.near_matches` exactly as before, so the
//! agent still sees what it matched and a later reader can still find it. This
//! is the difference between narrowing a guard and blinding it, and the first
//! test below pins it — without that assertion this change would be
//! indistinguishable from deleting the check.
//!
//! The suppressed set was the pairs with EVIDENCE behind them until
//! 2026-10-17, when the list gave way to a rule: every cross-type match is a
//! note (`dec:idea-a-cross-type-near-match-is-a-note-and-only-a-same-type-one-refuses`).
//! A same-type duplicate still refuses, which is what `a_same_type_duplicate_is_still_refused`
//! holds.

use reflow2_mcp::service::*;
use rmcp::handler::server::wrapper::Parameters;

macro_rules! j {
    ($call:expr) => {
        $call
            .await
            .expect("tool ok")
            .structured_content
            .expect("structured content present")
    };
}

async fn svc() -> ReflowService {
    ReflowService::in_memory().expect("in-memory service")
}

/// One idea in one vocabulary, used verbatim on both sides on purpose: the
/// layered capture is precisely the case where the words genuinely DO match.
const IDEA: &str = "A lost reading heals itself because the outdoor unit sends cumulative totals \
                    rather than deltas, so a dropped packet costs nothing.";

fn requirement(id: &str, name: &str) -> RequirementReq {
    RequirementReq {
        source: None,
        provenance: None,
        id: id.into(),
        name: Some(name.into()),
        statement: Some(IDEA.into()),
        distinct_from: None,
        replaces: None,
        status: None,
        approver: None,
        acted_at: None,
        priority: None,
        concern: None,
        kind: None,
        taken_as_given: None,
        statement_kind: None,
        variables: None,
        fixes: None,
    }
}

fn capability(id: &str, name: &str) -> CapabilityReq {
    CapabilityReq {
        satisfies: None,
        allocated_to: None,
        id: id.into(),
        name: Some(name.into()),
        description: Some(IDEA.into()),
        status: None,
        distinct_from: None,
        replaces: None,
        tier: None,
    }
}

fn decision(id: &str, name: &str, kind: Option<&str>) -> DecisionReq {
    DecisionReq {
        id: id.into(),
        name: Some(name.into()),
        decision: Some(IDEA.into()),
        rationale: None,
        distinct_from: None,
        replaces: None,
        kind: kind.map(Into::into),
        related_to: None,
        no_relation_note: Some("no honest relation; this is a test fixture".into()),
        status: None,
        approver: None,
        acted_at: None,
    }
}

/// ⭐ THE MOST-MEASURED PAIRING: 4 of bhome's 8 refusals and 1 of musicjug's 4.
/// A Capability and the Requirement it satisfies are the golden thread, and the
/// served instructions ask for both.
#[tokio::test]
async fn a_capability_and_the_requirement_it_satisfies_is_not_refused() {
    let s = svc().await;
    s.add_requirement(Parameters(requirement(
        "req:a-lost-reading-heals-itself",
        "A lost reading heals itself",
    )))
    .await
    .expect("the requirement lands");

    let out = j!(s.add_capability(Parameters(capability(
        "cap:cumulative-totals",
        "Send cumulative totals",
    ))));

    // 🛑 AND THE MATCH IS STILL REPORTED. Suppressing the refusal must not
    // suppress the evidence — without this assertion, narrowing the guard and
    // deleting it would look identical from outside.
    let reported = serde_json::to_string(&out).expect("serialisable");
    assert!(
        reported.contains("req:a-lost-reading-heals-itself"),
        "a prescribed-layer match must still be REPORTED in search_first even though it no \
         longer refuses: {reported}"
    );
}

/// The promotion path, and the only measured SAME-TYPE false positive: step 5
/// of the brainstorm skill turns an `exploratory` Decision into a settled one,
/// so the idea and its answer are supposed to read alike. 1 of bhome's 8 and 2
/// of musicjug's 4.
#[tokio::test]
async fn an_idea_and_the_decision_that_settles_it_is_not_refused() {
    let s = svc().await;
    s.add_decision(Parameters(decision(
        "dec:idea-cumulative-or-deltas",
        "OPEN — cumulative totals or deltas?",
        Some("exploratory"),
    )))
    .await
    .expect("the idea lands");

    s.add_decision(Parameters(decision(
        "dec:cumulative-totals",
        "DECIDED — cumulative totals",
        Some("choice"),
    )))
    .await
    .expect("the decision that settles an idea is not a duplicate of the idea");
}

/// The pairing this triage session incurred three times, writing the decision
/// that acts on the count. A measurement and the Decision it informs are
/// supposed to read alike.
#[tokio::test]
async fn a_decision_and_the_measurement_it_acts_on_is_not_refused() {
    let s = svc().await;
    s.add_project(Parameters(
        serde_json::from_value(serde_json::json!({ "id": "proj:test", "name": "Test" }))
            .expect("project request"),
    ))
    .await
    .expect("the project lands");

    s.create_node(Parameters(CreateNodeReq {
        node_type: "TemporalFact".into(),
        id: "fact:deltas-lose-a-dropped-packet".into(),
        props: Some(
            serde_json::json!({
                "statement": IDEA,
                "subject_id": "proj:test",
                "basis": "measured",
                "valid_from": "2026-08-31",
            })
            .as_object()
            .expect("object")
            .clone(),
        ),
        expected_content_hash: None,
        approver: None,
        acted_at: None,
    }))
    .await
    .expect("the measurement lands");

    s.add_decision(Parameters(decision(
        "dec:send-cumulative-totals",
        "DECIDED — send cumulative totals",
        Some("choice"),
    )))
    .await
    .expect("a decision is not a duplicate of the measurement that motivates it");
}

/// 🛑 THE COUNTERWEIGHT THAT MATTERS MOST. The guard's whole purpose is the
/// same-type duplicate — the same record written twice in different words. If
/// this ever passes, the change above stopped being a narrowing and became a
/// deletion.
#[tokio::test]
async fn a_same_type_duplicate_is_still_refused() {
    let s = svc().await;
    s.add_requirement(Parameters(requirement(
        "req:a-lost-reading-heals-itself",
        "A lost reading heals itself",
    )))
    .await
    .expect("the first requirement lands");

    let err = s
        .add_requirement(Parameters(requirement(
            "req:cumulative-not-deltas",
            "Cumulative totals, not deltas",
        )))
        .await
        .expect_err("a same-type near duplicate is still refused");

    assert!(
        err.message.to_string().contains("SHARPEN"),
        "the same-type route must still be offered: {}",
        err.message
    );
}

/// ⭐ SINCE 2026-10-17 THERE IS NO UNMEASURED PAIR LEFT TO REFUSE. The list
/// of measured pairs gave way to a rule
/// (`dec:idea-a-cross-type-near-match-is-a-note-and-only-a-same-type-one-refuses`):
/// every cross-type match is a note on the successful write. Component↔DesignRule,
/// the pair this test held as still refusing, is now written, and the match is
/// still reported. The rule's own class test is
/// `a_near_match_across_types_or_within_reasoning_is_a_note.rs`.
#[tokio::test]
async fn a_once_unmeasured_cross_type_pair_is_written_with_the_match_reported() {
    let s = svc().await;
    s.add_component(Parameters(ComponentReq {
        id: "cmp:outdoor-unit".into(),
        name: Some("Outdoor unit".into()),
        description: Some(IDEA.into()),
        kind: None,
        level: None,
        distinct_from: None,
        replaces: None,
        tier: None,
        status: None,
    }))
    .await
    .expect("the component lands");

    let reply = j!(s.add_design_rule(Parameters(DesignRuleReq {
        id: "rule:cumulative-totals".into(),
        name: Some("Cumulative totals, not deltas".into()),
        statement: Some(IDEA.into()),
        category: None,
        enforced: None,
        units: None,
        distinct_from: None,
        replaces: None,
        approver: None,
        acted_at: None,
        steps: None,
    })));
    let named = reply["search_first"]["near_matches"]
        .as_array()
        .expect("the match is still reported on the write")
        .iter()
        .any(|m| m["node_id"] == "cmp:outdoor-unit" && m["node_type"] == "Component");
    assert!(named, "the report names the match and its type: {reply}");
}

// ---- Two more prescribed pairs, from dev_storyflow 2026-09-05 (9 fires, 0 duplicates) ----
// fact:defect-two-prescribed-layer-pairs-are-missing-so-a-check-beside-its-ruling-is-refused-as-a-duplicate

#[tokio::test]
async fn a_check_beside_the_ruling_it_tests_is_not_refused() {
    // add_verification runs no near-match guard (it has no distinct_from either —
    // dev_storyflow noted that), so the reporter's fire was this direction: a
    // ruling written beside the check that already measures its subject.
    let s = svc().await;
    j!(s.add_verification(Parameters(serde_json::from_value(serde_json::json!({
        "id": "ver:hydrate-under-fifty-ms", "name": "The hydrate path answers in under fifty milliseconds for a story node",
        "description": "Measures the hydrate path at p50 on the reference machine and asserts under fifty milliseconds."
    })).unwrap())));
    let out = j!(s.add_decision(Parameters(serde_json::from_value(serde_json::json!({
        "id": "dec:hydrate-under-fifty-ms", "name": "The hydrate path answers in under fifty milliseconds for a story node",
        "decision": "The hydrate path answers in under fifty milliseconds for a story node, measured at p50 on the reference machine.",
        "rationale": "A budget written before the code."
    })).unwrap())));
    let reported = serde_json::to_string(&out).expect("serialisable");
    assert!(
        reported.contains("ver:hydrate-under-fifty-ms"),
        "a Decision-Verification match is REPORTED, not refused: {reported}"
    );
}

#[tokio::test]
async fn a_requirement_promoted_from_the_idea_that_spawned_it_is_not_refused() {
    let s = svc().await;
    j!(s.add_decision(Parameters(serde_json::from_value(serde_json::json!({
        "id": "dec:idea-a-campaign-is-a-story", "kind": "exploratory",
        "name": "OPEN — a campaign is just another story with a longer arc",
        "decision": "Recorded as brainstorming: a campaign is just another story with a longer arc and several authors.",
        "rationale": "Counter: campaigns have participants stories do not."
    })).unwrap())));
    let out = j!(s.add_requirement(Parameters(RequirementReq {
        source: None,
        provenance: None,
        id: "req:a-campaign-is-a-story".into(),
        name: Some("A campaign is just another story with a longer arc".into()),
        statement: Some("A campaign is just another story with a longer arc and several authors, and is modelled as one.".into()),
        distinct_from: None,
        replaces: None,
    status: None,
    approver: None,
    acted_at: None,
    priority: None,
    concern: None,
    kind: None,
        taken_as_given: None,
        statement_kind: None,
        variables: None,
        fixes: None,
    })));
    let reported = serde_json::to_string(&out).expect("serialisable");
    assert!(
        reported.contains("dec:idea-a-campaign-is-a-story"),
        "a Requirement-Decision match (the promotion path) is REPORTED, not refused: {reported}"
    );
}
