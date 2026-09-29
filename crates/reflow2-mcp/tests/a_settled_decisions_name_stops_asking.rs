//! A settled decision's NAME stops asking the question it settled — or the
//! design says so.
//!
//! # The finding this pins
//!
//! `fact:root-cause-a-settled-decisions-name-still-reads-open-because-the-09-19-fix-reached-the-body-and-no-check-reads-names-2026-09-29`.
//! The brainstorm skill names an idea as its open question, "OPEN — does X…?",
//! which COPIES the status into the name at birth. flo2 F12 (2026-09-19)
//! reported settled decisions still named that way; the fix gave
//! `set_decision_status` a `chose` field for the body and nothing for the name,
//! and no check read `name`. It recurred on 2026-09-29: six decisions settled,
//! all six still named "OPEN —", ~30 KB of whole-node replace_text to fix them.
//! reflow2's own design held 44 of 266 accepted decisions so named.
//!
//! # The class
//!
//! Status copied into a prose field with no act that moves both and no check
//! that reads the copy. So three things are pinned, one per leg:
//!
//! - the settle can move the name in the same call (`name`);
//! - the settle reply says when it did not (`name_still_reads_open`);
//! - a sweep finds the ones already standing (`settled_decision_named_open`),
//!   which no reply will ever reach again.
//!
//! And the governed-prose check (`settled_question_prose`) now reads a
//! governed node's name as well as its body.

use reflow2_mcp::service::*;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

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
    let s = ReflowService::in_memory().expect("in-memory service");
    j!(s.add_contributor(Parameters(ContributorReq {
        id: "who:ann".into(),
        name: Some("Ann".into()),
        kind: None,
        handle: None,
        description: None,
    })));
    s
}

async fn idea(s: &ReflowService, id: &str, name: &str, text: &str) {
    j!(s.add_decision(Parameters(
        serde_json::from_value(json!({
            "id": id, "name": name, "decision": text, "kind": "exploratory",
            "no_relation_note": "fixture"
        }))
        .unwrap()
    )));
}

async fn settle(s: &ReflowService, args: Value) -> Result<Value, String> {
    let req: SetDecisionStatusReq = serde_json::from_value(args).map_err(|e| e.to_string())?;
    s.set_decision_status(Parameters(req))
        .await
        .map(|r| r.structured_content.expect("structured"))
        .map_err(|e| e.to_string())
}

#[tokio::test]
async fn the_settle_can_retitle_the_decision_in_the_same_call() {
    let s = svc().await;
    idea(
        &s,
        "dec:hosting",
        "OPEN — do we host it ourselves?",
        "Host it, buy it, or wait.",
    )
    .await;
    let v = settle(
        &s,
        json!({
            "decision_id": "dec:hosting", "status": "accepted", "approver": "who:ann",
            "chose": "Host it: the data may not leave the site.",
            "name": "We host it ourselves"
        }),
    )
    .await
    .expect("set_decision_status takes `name`");
    assert_eq!(v["properties"]["name"], "We host it ourselves", "{v}");
    assert_eq!(v["properties"]["status"], "accepted", "{v}");
    assert!(
        v.get("name_still_reads_open").is_none(),
        "a retitled decision is not stale: {v}"
    );
}

#[tokio::test]
async fn an_empty_name_is_refused_before_the_status_moves() {
    let s = svc().await;
    idea(
        &s,
        "dec:hosting",
        "OPEN — do we host it?",
        "Host, buy, or wait.",
    )
    .await;
    let err = settle(
        &s,
        json!({"decision_id": "dec:hosting", "status": "accepted", "approver": "who:ann",
               "name": "   "}),
    )
    .await
    .expect_err("an empty name is refused");
    assert!(err.contains("`name`"), "{err}");
    let still = settle(
        &s,
        json!({"decision_id": "dec:hosting", "status": "proposed"}),
    )
    .await
    .unwrap();
    assert_eq!(
        still["properties"]["name"], "OPEN — do we host it?",
        "nothing was written by the refused call: {still}"
    );
}

#[tokio::test]
async fn a_settle_that_leaves_the_name_asking_says_so() {
    let s = svc().await;
    idea(
        &s,
        "dec:hosting",
        "OPEN — do we host it ourselves?",
        "Host it, buy it, or wait.",
    )
    .await;
    let v = settle(
        &s,
        json!({"decision_id": "dec:hosting", "status": "accepted", "approver": "who:ann",
               "chose": "Host it."}),
    )
    .await
    .unwrap();
    let block = &v["name_still_reads_open"];
    assert_eq!(
        block["name"], "OPEN — do we host it ourselves?",
        "the stale name is quoted back: {v}"
    );
    assert!(
        block["note"].as_str().unwrap_or("").contains("`name`"),
        "the block names the parameter that fixes it: {v}"
    );
}

/// Counterweights: silent where the name tells the truth.
#[tokio::test]
async fn silent_on_an_open_or_deferred_decision_and_on_a_decision_about_opening() {
    let s = svc().await;
    idea(
        &s,
        "dec:open",
        "OPEN — which sensor?",
        "Capacitive or resistive.",
    )
    .await;
    let deferred = settle(
        &s,
        json!({"decision_id": "dec:open", "status": "deferred", "approver": "who:ann"}),
    )
    .await
    .unwrap();
    assert!(
        deferred.get("name_still_reads_open").is_none(),
        "deferred leaves the question open: {deferred}"
    );

    idea(
        &s,
        "dec:api",
        "Open the API to partners",
        "Partners get read access to the catalogue.",
    )
    .await;
    let v = settle(
        &s,
        json!({"decision_id": "dec:api", "status": "accepted", "approver": "who:ann"}),
    )
    .await
    .unwrap();
    assert!(
        v.get("name_still_reads_open").is_none(),
        "a decision ABOUT opening something is not a stale status word: {v}"
    );
}

/// The standing ones: a sweep finds every accepted decision still named open,
/// and only those. This is the leg that surfaces the 44 already in reflow2's
/// own design, which no settle reply will ever reach.
#[tokio::test]
async fn the_sweep_names_every_accepted_decision_still_named_open() {
    let s = svc().await;
    for (id, name, text) in [
        ("dec:a", "OPEN — do we host it?", "Host, buy, or wait."),
        (
            "dec:b",
            "Open question: which sensor?",
            "Capacitive or resistive.",
        ),
        ("dec:c", "OPEN — which pump?", "Diaphragm or peristaltic."),
        (
            "dec:d",
            "Open the API to partners",
            "Read access to the catalogue.",
        ),
    ] {
        idea(&s, id, name, text).await;
    }
    for id in ["dec:a", "dec:b", "dec:d"] {
        settle(
            &s,
            json!({"decision_id": id, "status": "accepted", "approver": "who:ann"}),
        )
        .await
        .unwrap();
    }
    // dec:c stays proposed — named OPEN, and truthfully so.
    let gaps = j!(s.detect_gaps(Parameters(GapScopeReq {
        scope: None,
        depth: None,
        budget_chars: None,
    })));
    let found: Vec<&Value> = gaps["items"]
        .as_array()
        .expect("items")
        .iter()
        .filter(|g| g["gap_source"] == "settled_decision_named_open")
        .collect();
    assert_eq!(found.len(), 1, "one rollup: {gaps}");
    let mut named: Vec<&str> = found[0]["affected_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    named.sort_unstable();
    assert_eq!(
        named,
        vec!["dec:a", "dec:b"],
        "exactly the accepted decisions still named open: {}",
        found[0]
    );
}

/// The governed-prose check reads a governed node's NAME, not only its body.
#[tokio::test]
async fn a_governed_node_whose_name_still_says_open_is_named_at_the_settle() {
    let s = svc().await;
    j!(s.add_requirement(Parameters(
        serde_json::from_value(json!({
            "id": "req:residency",
            "name": "Where the data lives (not yet decided)",
            "statement": "The data lives somewhere the site's rules allow."
        }))
        .unwrap()
    )));
    idea(
        &s,
        "dec:hosting",
        "OPEN — do we host it?",
        "Host, buy, or wait.",
    )
    .await;
    j!(s.governed_by(Parameters(
        serde_json::from_value(json!({
            "from_id": "req:residency", "to_id": "dec:hosting"
        }))
        .unwrap()
    )));
    let v = settle(
        &s,
        json!({"decision_id": "dec:hosting", "status": "accepted", "approver": "who:ann"}),
    )
    .await
    .unwrap();
    let governs = v["settled_question_prose"]["governs"]
        .as_array()
        .unwrap_or_else(|| panic!("the governed node's name still reads open: {v}"));
    assert_eq!(governs[0]["node_id"], "req:residency", "{v}");
    assert_eq!(governs[0]["field"], "name", "{v}");
}
