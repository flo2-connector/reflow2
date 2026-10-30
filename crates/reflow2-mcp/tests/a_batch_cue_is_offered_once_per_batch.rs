//! A cue naming the step for when a batch of writes lands is offered once per
//! batch, not on every write; a process tool names its own step as the cue.
//!
//! Increment 7 of `epoch:planned-cognition-plus-memory`,
//! `epoch:planned-the-tool-surface-refined-from-the-counts`. Measured on 0.94.0
//! (increment 3's six writers): `loop_hint/detect_gaps` offered 97 times and
//! followed 0, `loop_hint/get_skill:detect-and-ask` 75 and 0,
//! `loop_hint/governed_by` 31 and 0, `loop_hint/loop_status` 162 offered and
//! followed only at the end of each round. The tools are called where they are
//! needed, so the cue's TIMING changes and its id and words do not
//! (`crate::cues::BATCH_CUES`).
//!
//! Which cues a call withholds is read from the usage ledger in `call_tool`
//! (`crate::cues::withheld`, pinned by its unit tests and, over the wire, by
//! `tools/test_feedback_is_a_computed_tally.py`). What this file pins is the
//! reply each handler builds once that set is known, driven through
//! `cues::with_withheld`, the scope `call_tool` puts around the handler:
//!
//! 1. Nothing withheld (every in-memory design, every direct call): the hint
//!    is exactly what it was.
//! 2. Withheld: a hint that is only a batch step says it was named earlier in
//!    the batch and names no call; a hint with a per-node half keeps that half.
//! 3. The settling cue (`loop_hint/set_decision_status`, the owner-only cue
//!    0.94.0 and 0.95.0 fixed) is never withheld.
//! 4. add_change_event names set_artifact_checksums when, and only when, the
//!    change touched a registered file.

use reflow2_mcp::cues;
use reflow2_mcp::service::*;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn req<T: serde::de::DeserializeOwned>(v: Value) -> Parameters<T> {
    Parameters(serde_json::from_value(v).expect("request shape"))
}

fn every_batch_cue() -> BTreeSet<String> {
    cues::BATCH_CUES.iter().map(|c| c.to_string()).collect()
}

fn hint(v: &rmcp::model::CallToolResult) -> String {
    v.structured_content.as_ref().expect("structured")["loop_hint"]
        .as_str()
        .expect("a loop_hint")
        .to_string()
}

async fn add_requirement(s: &ReflowService, id: &str) -> String {
    hint(
        &s.add_requirement(req(json!({
            "id": id, "name": id, "statement": format!("The pump starts within 5 s ({id}).")
        })))
        .await
        .expect("add_requirement"),
    )
}

#[tokio::test]
async fn nothing_withheld_the_batch_step_is_said_as_before() {
    let s = ReflowService::in_memory().expect("service");
    let h = add_requirement(&s, "req:a").await;
    assert!(
        h.contains("detect_gaps") && h.contains("loop_status"),
        "{h}"
    );
    // An empty withheld set is the same as none.
    let h = cues::with_withheld(BTreeSet::new(), add_requirement(&s, "req:b")).await;
    assert!(
        h.contains("detect_gaps") && h.contains("loop_status"),
        "{h}"
    );
}

#[tokio::test]
async fn a_withheld_batch_step_is_not_repeated_and_the_hint_says_so() {
    let s = ReflowService::in_memory().expect("service");
    let h = cues::with_withheld(every_batch_cue(), add_requirement(&s, "req:a")).await;
    assert_eq!(h, cues::WITHHELD_HINT);
    for name in ["detect_gaps", "detect-and-ask", "loop_status"] {
        assert!(!h.contains(name), "the withheld hint names no call: {h}");
    }
    let h = cues::with_withheld(
        every_batch_cue(),
        async {
            hint(
                &s.create_node(req(json!({
                    "node_type": "Requirement", "id": "req:generic", "props": {"name": "g", "statement": "s"}
                })))
                .await
                .expect("create_node"),
            )
        },
    )
    .await;
    assert_eq!(h, cues::WITHHELD_HINT);
}

/// One withheld id is enough to keep a hint whose ids are all withheld quiet,
/// and one due id brings the whole hint back: the ids stay stable.
#[tokio::test]
async fn a_hint_comes_back_whole_when_any_of_its_steps_is_due() {
    let s = ReflowService::in_memory().expect("service");
    let only_gaps: BTreeSet<String> = [
        "loop_hint/detect_gaps",
        "loop_hint/get_skill:detect-and-ask",
    ]
    .map(String::from)
    .into();
    let h = cues::with_withheld(only_gaps, add_requirement(&s, "req:a")).await;
    assert!(
        h.contains("detect_gaps") && h.contains("loop_status"),
        "loop_status is due, so the hint is said whole: {h}"
    );
}

#[tokio::test]
async fn the_settling_clause_stays_when_the_loop_status_clause_is_withheld() {
    let s = ReflowService::in_memory().expect("service");
    s.add_decision(req(json!({
        "id": "dec:battery", "name": "Which battery bank",
        "decision": "Lithium, because the room is not ventilated for lead-acid."
    })))
    .await
    .expect("add_decision");
    let replace = |text: &'static str| {
        let s = &s;
        async move {
            hint(
                &s.replace_text(req(json!({
                    "node_id": "dec:battery", "field": "decision", "new": text
                })))
                .await
                .expect("replace_text"),
            )
        }
    };
    let full = replace("Lithium, because the room has no extraction fan.").await;
    assert!(
        full.contains("set_decision_status") && full.contains("loop_status"),
        "{full}"
    );
    let h = cues::with_withheld(
        every_batch_cue(),
        replace("Lithium, because the room has no fan."),
    )
    .await;
    assert!(
        h.contains("set_decision_status"),
        "the owner-only settling cue is never withheld: {h}"
    );
    assert!(!h.contains("loop_status"), "{h}");
    assert!(
        !cues::BATCH_CUES.contains(&"loop_hint/set_decision_status"),
        "set_decision_status is not a batch cue"
    );
}

#[tokio::test]
async fn a_process_keeps_naming_its_step_when_the_batch_step_is_withheld() {
    let s = ReflowService::in_memory().expect("service");
    let h = cues::with_withheld(every_batch_cue(), async {
        hint(
            &s.add_capability(req(json!({
                "id": "cap:start", "name": "Start the pump",
                "description": "Starts the pump on demand."
            })))
            .await
            .expect("add_capability"),
        )
    })
    .await;
    assert!(
        h.contains("wire satisfies"),
        "add_capability's own step is said every time it is owed: {h}"
    );
    assert!(!h.contains("detect_gaps"), "{h}");

    // An accepted decision keeps saying where it stands; the governed_by
    // link is the batch half.
    let accepted = |id: &'static str| {
        let s = &s;
        async move {
            hint(
                &s.add_decision(req(json!({
                    "id": id, "name": id, "decision": "Use the duty pump.",
                    "status": "accepted", "approver": "who:ann"
                })))
                .await
                .expect("add_decision"),
            )
        }
    };
    s.add_contributor(req(
        json!({"id": "who:ann", "name": "Ann", "kind": "person"}),
    ))
    .await
    .expect("contributor");
    let full = accepted("dec:duty").await;
    assert!(full.contains("governed_by"), "{full}");
    let h = cues::with_withheld(every_batch_cue(), accepted("dec:standby")).await;
    assert!(
        h.contains("stands `accepted`") && !h.contains("governed_by"),
        "{h}"
    );
}

async fn change(s: &ReflowService, id: &str, affected: Value) -> Value {
    s.add_change_event(req(json!({
        "id": id, "name": id, "change_type": "new_feature", "subject": "system",
        "summary": "The pump starts faster.", "detected_at": "2026-10-30",
        "affected": affected
    })))
    .await
    .expect("add_change_event")
    .structured_content
    .expect("structured")
}

#[tokio::test]
async fn a_change_that_touched_a_registered_file_names_the_checksum_step() {
    let s = ReflowService::in_memory().expect("service");
    s.add_requirement(req(json!({
        "id": "req:a", "name": "a", "statement": "The pump starts within 5 s."
    })))
    .await
    .expect("req");
    s.add_artifact(req(json!({
        "id": "art:pump", "name": "pump.rs", "artifact_type": "code", "location": "src/pump.rs"
    })))
    .await
    .expect("artifact");

    let v = change(
        &s,
        "chg:faster",
        json!([{"node_id": "art:pump"}, {"node_id": "req:a"}]),
    )
    .await;
    let next = v["next_step"].as_str().expect("a next_step");
    assert!(next.contains("set_artifact_checksums"), "{next}");
    assert!(
        next.contains("\"chg:faster\""),
        "it names the change: {next}"
    );
    assert!(next.contains("1 registered file"), "{next}");

    let v = change(&s, "chg:words", json!([{"node_id": "req:a"}])).await;
    assert!(
        v.get("next_step").is_none(),
        "no file touched, no checksum step owed: {v}"
    );
    let v = change(
        &s,
        "chg:gone",
        json!([{"node_id": "art:pump", "action": "removed"}]),
    )
    .await;
    assert!(
        v.get("next_step").is_none(),
        "a removed file has no checksum to accept: {v}"
    );
}
