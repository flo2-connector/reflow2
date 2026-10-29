//! A cue that asks for the owner's word is offered as the next call to whoever
//! can give it, and to nobody known not to be able to.
//!
//! `cap:a-settling-cue-goes-to-the-owner-not-to-an-agent-who-cannot-give-the-word`
//! (increment 2 of `epoch:planned-cognition-plus-memory`). In the recall
//! pilot the commonest cue was `set_decision_status`, offered 101 times by
//! add_decision's and replace_text's loop hints and followed 5 times: an agent
//! recording someone else's project cannot give the owner's word, so the cue
//! named a call that was never its to make
//! (`fact:in-the-recall-pilot-agents-followed-4-percent-of-reflow2s-cues-within-five-calls-2026-10-29`).
//!
//! THE RULE (Anthony, 2026-10-29: "Go with your recommendation and fix them
//! first"): offer `set_decision_status` UNLESS the caller is known — named by
//! `writes_for`, or by the gateway that serves the engine for others — and is
//! not the owner. The first build read a call made for nobody as "not the
//! owner", so the owner working alone on their own machine, and every `--call`,
//! was told settling was a question for the owner
//! (`fact:root-cause-the-settling-cue-read-an-unnamed-local-caller-as-not-the-owner-2026-10-29`).
//! `a_local_call_made_for_nobody_is_offered_the_settling_call` failed on that
//! build and passes on this one.
//!
//! Driven through `serving_for`, the step `call_tool` takes around every
//! handler to say who the call is for (the `--call` door names nobody: it is
//! the local, unnamed case).

use reflow2_mcp::caller::CallerRule;
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

/// The same design, served for others behind a trusted gateway (flo2.io's
/// shape), which names the caller on every call.
async fn served_behind_a_gateway(owner: Option<&str>) -> ReflowService {
    let s = design(owner)
        .await
        .with_caller_rule(CallerRule::TrustedGateway {
            name: "flo2.io".into(),
        });
    assert!(s.caller_rule().served_for_others());
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

fn offers_the_settling_call(h: &str) {
    assert!(h.contains("set_decision_status"), "{h}");
    assert!(
        !h.contains("question for the owner"),
        "the caller who can give the word is not told to ask for it: {h}"
    );
}

fn says_it_is_a_question_for_the_owner(h: &str) {
    assert!(h.contains("question for the owner"), "{h}");
    assert!(h.contains("ask_question"), "{h}");
    assert!(
        !h.contains("set_decision_status"),
        "the settling call is not offered as this caller's next call: {h}"
    );
}

/// The fault this file pins: locally, a call made for nobody — the owner
/// working alone, a `--call` — comes from the person running reflow2, who is
/// the one who can give the word. Whether or not the design names an owner.
#[tokio::test]
async fn a_local_call_made_for_nobody_is_offered_the_settling_call() {
    for owner in [None, Some("who:bob")] {
        let s = design(owner).await;
        assert!(!s.caller_rule().served_for_others());
        let h = decision_hint(&s, None).await;
        assert!(
            h.contains("lands `proposed`"),
            "where it landed is still said: {h}"
        );
        offers_the_settling_call(&h);
        offers_the_settling_call(&replace_hint(&s, None).await);
    }
}

/// A design that names no owner takes a named caller at their word.
#[tokio::test]
async fn a_named_caller_on_a_design_with_no_owner_is_offered_the_settling_call() {
    let s = design(None).await;
    offers_the_settling_call(&decision_hint(&s, Some("who:ann")).await);
    offers_the_settling_call(&replace_hint(&s, Some("who:ann")).await);
}

/// Where the design names its owner, a named caller who is not the owner is
/// told it is a question for the owner; the owner is offered the call.
#[tokio::test]
async fn a_named_caller_who_is_not_the_owner_is_told_it_is_a_question_for_the_owner() {
    let s = design(Some("who:bob")).await;
    let h = decision_hint(&s, Some("who:ann")).await;
    assert!(h.contains("lands `proposed`"), "{h}");
    says_it_is_a_question_for_the_owner(&h);
    says_it_is_a_question_for_the_owner(&replace_hint(&s, Some("who:ann")).await);

    offers_the_settling_call(&decision_hint(&s, Some("who:bob")).await);
    offers_the_settling_call(&replace_hint(&s, Some("who:bob")).await);
}

/// Behind the trusted gateway nothing changed: the gateway names every
/// caller, a member who is not the owner is told it is a question for the
/// owner, and the owner is offered the call. A call there that names nobody
/// cannot sign at all, so it is not offered the call either.
#[tokio::test]
async fn behind_a_trusted_gateway_only_the_named_owner_is_offered_the_settling_call() {
    let s = served_behind_a_gateway(Some("who:bob")).await;
    says_it_is_a_question_for_the_owner(&decision_hint(&s, Some("who:ann")).await);
    says_it_is_a_question_for_the_owner(&replace_hint(&s, Some("who:ann")).await);
    offers_the_settling_call(&decision_hint(&s, Some("who:bob")).await);
    offers_the_settling_call(&replace_hint(&s, Some("who:bob")).await);
    says_it_is_a_question_for_the_owner(&replace_hint(&s, None).await);

    // A design there that names no owner takes the named caller at their
    // word, as it always did.
    let s = served_behind_a_gateway(None).await;
    offers_the_settling_call(&decision_hint(&s, Some("who:ann")).await);
}
