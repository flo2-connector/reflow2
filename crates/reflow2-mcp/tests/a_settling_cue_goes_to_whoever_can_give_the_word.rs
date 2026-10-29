//! A cue that asks for the owner's word is offered as the next call only to a
//! call made for someone who can give it.
//!
//! `cap:a-settling-cue-goes-to-the-owner-not-to-an-agent-who-cannot-give-the-word`
//! (increment 2 of `epoch:planned-cognition-plus-memory`). In the recall
//! pilot the commonest cue was `set_decision_status`, offered 101 times by
//! add_decision's and replace_text's loop hints and followed 5 times: a solo
//! agent recording someone else's project cannot give the owner's word, so
//! the cue named a call that was never its to make
//! (`fact:in-the-recall-pilot-agents-followed-4-percent-of-reflow2s-cues-within-five-calls-2026-10-29`).
//!
//! Driven through `serving_for`, the step `call_tool` takes around every
//! handler to say who the call is for.

use reflow2_mcp::service::*;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

fn req<T: serde::de::DeserializeOwned>(v: Value) -> Parameters<T> {
    Parameters(serde_json::from_value(v).expect("request shape"))
}

async fn design(owner: Option<&str>) -> ReflowService {
    let s = ReflowService::in_memory().expect("service");
    for id in ["who:ann", "who:bob"] {
        s.add_contributor(req(json!({"id": id, "name": id, "kind": "person"})))
            .await
            .expect("contributor");
    }
    s.add_project(req(json!({"id": "proj:p", "name": "Pump house"})))
        .await
        .expect("project");
    if let Some(owner) = owner {
        s.owned_by(req(json!({"from_id": "proj:p", "contributor_id": owner})))
            .await
            .expect("owned_by");
    }
    s
}

async fn decision_hint(s: &ReflowService, who: Option<&str>) -> String {
    let v = s
        .serving_for(
            who.map(String::from),
            s.add_decision(req(json!({
                "id": "dec:battery", "name": "Which battery bank",
                "decision": "Lithium, because the room is not ventilated for lead-acid."
            }))),
        )
        .await
        .expect("add_decision")
        .structured_content
        .expect("structured");
    v["loop_hint"].as_str().expect("a loop_hint").to_string()
}

async fn replace_hint(s: &ReflowService, who: Option<&str>) -> String {
    let v = s
        .serving_for(
            who.map(String::from),
            s.replace_text(req(json!({
                "node_id": "dec:battery", "field": "decision",
                "new": "Lithium, because the room has no extraction fan."
            }))),
        )
        .await
        .expect("replace_text")
        .structured_content
        .expect("structured");
    v["loop_hint"].as_str().expect("a loop_hint").to_string()
}

/// The pilot's case: nobody is named, so the cue says what it is — a question
/// for the owner — and names the call that puts it to them, not the settler.
#[tokio::test]
async fn a_call_made_for_nobody_is_told_settling_is_a_question_for_the_owner() {
    let s = design(None).await;
    let h = decision_hint(&s, None).await;
    assert!(
        h.contains("lands `proposed`"),
        "where it landed is still said: {h}"
    );
    assert!(h.contains("question for the owner"), "{h}");
    assert!(h.contains("ask_question"), "{h}");
    assert!(
        !h.contains("set_decision_status"),
        "the settling call is not offered as this caller's next call: {h}"
    );
    let r = replace_hint(&s, None).await;
    assert!(r.contains("question for the owner"), "{r}");
    assert!(!r.contains("set_decision_status"), "{r}");
}

/// A design that names no owner takes a named caller at their word: the
/// cue is offered as before.
#[tokio::test]
async fn a_named_caller_on_a_design_with_no_owner_is_offered_the_settling_call() {
    let s = design(None).await;
    let h = decision_hint(&s, Some("who:ann")).await;
    assert!(h.contains("set_decision_status"), "{h}");
    let r = replace_hint(&s, Some("who:ann")).await;
    assert!(r.contains("set_decision_status"), "{r}");
}

/// Where the design names its owner, only the owner is offered it.
#[tokio::test]
async fn where_the_design_names_its_owner_only_the_owner_is_offered_it() {
    let s = design(Some("who:bob")).await;
    let not_owner = decision_hint(&s, Some("who:ann")).await;
    assert!(not_owner.contains("question for the owner"), "{not_owner}");
    assert!(!not_owner.contains("set_decision_status"), "{not_owner}");
    let owner = decision_hint(&s, Some("who:bob")).await;
    assert!(owner.contains("set_decision_status"), "{owner}");
}
