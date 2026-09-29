//! A constructor's loop hint describes what THIS call left behind, never the
//! common case it was written for.
//!
//! # The finding this pins
//!
//! `fact:root-cause-add-decisions-lands-proposed-hint-recurred-on-the-one-call-settle-path-2026-09-29`.
//! `add_decision` replied `properties.status: "accepted"` AND
//! `loop_hint: "a Decision lands \`proposed\` — only the owner's word moves
//! it"` in the same reply. The hint was a fixed string. It was first reported
//! on 2026-08-14 on the merge path (an accepted node revised), was never fixed,
//! and since 2026-09-06 — when constructors began taking `status` and
//! `approver` in one call — it has been false on the NORMAL shape of a one-call
//! settle. A designer agent read it on 2026-09-29 as the gate contradicting
//! itself.
//!
//! # The class
//!
//! A reply sentence hand-written for the common case and never conditioned on
//! the result it describes. Swept on the constructors whose hints make a
//! conditional claim: `add_decision` (where it landed), `add_design_rule`
//! (asked for `enforced` even when it was stated) and `add_capability` (told a
//! caller who had just passed `satisfies` to "wire satisfies").

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

fn hint(v: &Value) -> String {
    v["loop_hint"].as_str().expect("a loop_hint").to_string()
}

const CLAIMS_PROPOSED: &str = "lands `proposed`";

#[tokio::test]
async fn a_decision_settled_in_the_creating_call_is_not_told_it_landed_proposed() {
    let s = svc().await;
    let v = j!(s.add_decision(Parameters(
        serde_json::from_value(json!({
            "id": "dec:settled", "name": "We host it ourselves",
            "decision": "Self-hosting, because the data may not leave the site.",
            "kind": "choice", "status": "accepted",
            "approver": "who:ann", "acted_at": "2026-09-29"
        }))
        .unwrap()
    )));
    assert_eq!(v["properties"]["status"], "accepted", "{v}");
    assert!(
        !hint(&v).contains(CLAIMS_PROPOSED),
        "the reply said accepted and the hint said proposed: {v}"
    );
    assert!(
        hint(&v).contains("accepted"),
        "the hint names where it stands: {v}"
    );
}

/// The 2026-08-14 shape: a merge over a node that was already accepted.
#[tokio::test]
async fn a_merge_over_an_accepted_decision_is_not_told_it_landed_proposed() {
    let s = svc().await;
    j!(s.add_decision(Parameters(
        serde_json::from_value(json!({
            "id": "dec:settled", "name": "We host it ourselves",
            "decision": "Self-hosting, because the data may not leave the site.",
            "status": "accepted", "approver": "who:ann"
        }))
        .unwrap()
    )));
    let v = j!(s.add_decision(Parameters(
        serde_json::from_value(json!({
            "id": "dec:settled",
            "rationale": "The site's data-residency rule is the whole reason."
        }))
        .unwrap()
    )));
    assert_eq!(v["properties"]["status"], "accepted", "{v}");
    assert!(!hint(&v).contains(CLAIMS_PROPOSED), "{v}");
}

/// Regression cover: where the claim IS true, it is still made.
#[tokio::test]
async fn a_decision_that_did_land_proposed_is_still_told_so() {
    let s = svc().await;
    let v = j!(s.add_decision(Parameters(
        serde_json::from_value(json!({
            "id": "dec:open", "name": "Do we host it ourselves?",
            "decision": "Options: host, buy, or wait."
        }))
        .unwrap()
    )));
    assert_eq!(v["properties"]["status"], "proposed", "{v}");
    assert!(hint(&v).contains(CLAIMS_PROPOSED), "{v}");
}

#[tokio::test]
async fn a_rule_whose_power_was_stated_is_not_asked_for_it() {
    let s = svc().await;
    let stated = j!(s.add_design_rule(Parameters(
        serde_json::from_value(json!({
            "id": "rule:units", "name": "Units",
            "statement": "Time in ms, sizes in bytes.",
            "enforced": true, "approver": "who:ann"
        }))
        .unwrap()
    )));
    assert!(
        !hint(&stated).contains("`enforced`"),
        "the call stated `enforced` and was asked to state it: {stated}"
    );
    let unstated = j!(s.add_design_rule(Parameters(
        serde_json::from_value(json!({
            "id": "rule:naming", "name": "Naming",
            "statement": "Ids are kebab-case slugs with a type prefix."
        }))
        .unwrap()
    )));
    assert!(
        hint(&unstated).contains("`enforced`"),
        "regression: a rule with no stated power is still asked: {unstated}"
    );
}

#[tokio::test]
async fn a_capability_that_passed_satisfies_is_not_told_to_wire_it() {
    let s = svc().await;
    j!(s.add_requirement(Parameters(
        serde_json::from_value(json!({
            "id": "req:a-need", "name": "A need", "statement": "Something must hold."
        }))
        .unwrap()
    )));
    let wired = j!(s.add_capability(Parameters(
        serde_json::from_value(json!({
            "id": "cap:wired", "name": "Holds it",
            "description": "Makes the thing hold.", "satisfies": "req:a-need"
        }))
        .unwrap()
    )));
    assert!(
        !hint(&wired).contains("wire satisfies"),
        "the call passed satisfies and was told to wire it: {wired}"
    );
    let unwired = j!(s.add_capability(Parameters(
        serde_json::from_value(json!({
            "id": "cap:unwired", "name": "Reports status",
            "description": "Tells the operator how things stand."
        }))
        .unwrap()
    )));
    assert!(
        hint(&unwired).contains("wire satisfies"),
        "regression: an unthreaded capability is still told: {unwired}"
    );
}
