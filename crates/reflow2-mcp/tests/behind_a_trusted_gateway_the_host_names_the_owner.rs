//! BEHIND A TRUSTED GATEWAY, THE HOST NAMES WHO OWNS THE DESIGN — and the
//! settling cue goes by that word, with nothing written into the design.
//!
//! The symptom (flo2's design 1a97fc1f1de4d5d2,
//! `fact:on-flo2-io-the-settle-hint-reaches-members-only-where-a-design-records-its-owner-2026-10-29`):
//! on flo2.io a member a design is shared with was still offered
//! `set_decision_status`, because the engine knew an owner only from the
//! design's Project `OWNED_BY` edge, and flo2 keeps ownership in its own
//! records and writes none. The cause, measured on the published 0.94.0
//! binary (`fact:root-cause-behind-a-trusted-gateway-the-engine-can-learn-a-designs-owner-only-from-owned-by-2026-10-30`):
//! the engine took no other word on ownership. Writing `OWNED_BY` into every
//! stored design would write into people's designs; instead the gateway, which
//! already names the caller on each call (`_meta["reflow2/writes_for"]`), names
//! the owner the same way (`_meta["reflow2/design_owners"]`).
//!
//! Driven through rmcp's real `call_tool`, so the request's `_meta` is read
//! where every call reads it. `a_named_member_on_a_design_with_no_owned_by_is_told_to_ask_the_owner`
//! FAILED on 0.94.0's engine (the member was offered set_decision_status) and
//! passes on this one.

use reflow2_mcp::caller::CallerRule;
use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};

const OWNER: &str = "who:bob";
const MEMBER: &str = "who:ann";

struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "owner-word-test".to_string();
        cfg.client_info.version = "test".to_string();
        cfg
    }
}

type Client = rmcp::service::RunningService<rmcp::RoleClient, TestClient>;

/// A design with two contributors and a Project that records NO owner — as
/// flo2.io makes them — served under `rule`.
async fn connect(rule: CallerRule) -> Client {
    let service = ReflowService::in_memory()
        .expect("in-memory service")
        .with_caller_rule(rule);
    let (server_rx, client_tx) = tokio::io::duplex(1 << 22);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 22);
    tokio::spawn(async move {
        if let Ok(running) = service.serve((server_rx, server_tx)).await {
            let _ = running.waiting().await;
        }
    });
    let c = TestClient
        .serve((client_rx, client_tx))
        .await
        .expect("the in-process handshake");
    // Seeded naming nobody, as flo2 seeds a design: nobody exists to write for yet.
    for id in [OWNER, MEMBER] {
        call(
            &c,
            "add_contributor",
            json!({"id": id, "name": id, "kind": "person"}),
            json!({}),
        )
        .await;
    }
    call(
        &c,
        "add_project",
        json!({"id": "proj:p", "name": "Pump house"}),
        json!({"reflow2/writes_for": OWNER}),
    )
    .await;
    c
}

async fn call(c: &Client, tool: &str, args: Value, meta: Value) -> Value {
    let params: CallToolRequestParams =
        serde_json::from_value(json!({"name": tool, "arguments": args, "_meta": meta}))
            .expect("params");
    let r = c
        .call_tool(params)
        .await
        .unwrap_or_else(|e| panic!("{tool}: {e}"));
    assert!(r.is_error != Some(true), "{tool}: {:?}", r.content);
    r.structured_content.unwrap_or(Value::Null)
}

/// The loop_hint add_decision gives a caller named `who`, the gateway naming
/// `owners` as the design's owners (or saying nothing).
async fn decision_hint(c: &Client, id: &str, who: &str, owners: Option<&[&str]>) -> String {
    let mut meta = json!({"reflow2/writes_for": who});
    if let Some(owners) = owners {
        meta["reflow2/design_owners"] = json!(owners);
    }
    let v = call(
        c,
        "add_decision",
        json!({"id": id, "name": "Which battery bank", "decision": "Lithium."}),
        meta,
    )
    .await;
    v["loop_hint"].as_str().expect("a loop_hint").to_string()
}

async fn replace_hint(c: &Client, id: &str, who: &str, owners: &[&str]) -> String {
    let v = call(
        c,
        "replace_text",
        json!({"node_id": id, "field": "decision", "new": "Lithium, no extraction fan."}),
        json!({"reflow2/writes_for": who, "reflow2/design_owners": owners}),
    )
    .await;
    v["loop_hint"].as_str().expect("a loop_hint").to_string()
}

fn offers_the_settling_call(h: &str) {
    assert!(h.contains("set_decision_status"), "{h}");
    assert!(!h.contains("question for the owner"), "{h}");
}

fn says_it_is_a_question_for_the_owner(h: &str) {
    assert!(h.contains("question for the owner"), "{h}");
    assert!(h.contains("ask_question"), "{h}");
    assert!(!h.contains("set_decision_status"), "{h}");
}

fn gateway() -> CallerRule {
    CallerRule::TrustedGateway {
        name: "flo2.io".into(),
    }
}

/// The fault: a design that records no owner, behind the gateway. The owner
/// the gateway names is offered the settling call; a member it names is told
/// it is a question for the owner — and nothing about ownership is written.
#[tokio::test]
async fn a_named_member_on_a_design_with_no_owned_by_is_told_to_ask_the_owner() {
    let c = connect(gateway()).await;
    says_it_is_a_question_for_the_owner(
        &decision_hint(&c, "dec:a", MEMBER, Some(&[OWNER])).await,
    );
    offers_the_settling_call(&decision_hint(&c, "dec:b", OWNER, Some(&[OWNER])).await);
    says_it_is_a_question_for_the_owner(&replace_hint(&c, "dec:a", MEMBER, &[OWNER]).await);
    offers_the_settling_call(&replace_hint(&c, "dec:b", OWNER, &[OWNER]).await);

    // Several owners: any of them can give the word.
    offers_the_settling_call(&decision_hint(&c, "dec:c", MEMBER, Some(&[OWNER, MEMBER])).await);

    // Nothing was written about ownership: the Project still records no owner.
    let p = call(
        &c,
        "get_node",
        json!({"id": "proj:p", "include_edges": {"edge_types": ["OWNED_BY"]}}),
        json!({"reflow2/writes_for": OWNER}),
    )
    .await;
    assert_eq!(p["edges"]["by_type"].get("OWNED_BY"), None, "{p}");
    assert_eq!(p["edges"]["matched"], json!(0), "{p}");
}

/// Without the host's word the design's own record decides, as before: a
/// design that records no owner takes a named caller at their word.
#[tokio::test]
async fn behind_the_gateway_with_no_owner_named_the_design_record_decides() {
    let c = connect(gateway()).await;
    offers_the_settling_call(&decision_hint(&c, "dec:a", MEMBER, None).await);
    // An empty set is no word on ownership.
    offers_the_settling_call(&decision_hint(&c, "dec:b", MEMBER, Some(&[])).await);
}

/// The owners key is the trusted gateway's word and nobody else's. On a local
/// engine, and on one served for others with no gateway declared, it is
/// ignored: a caller cannot make a member "not the owner" — or themselves the
/// owner — by sending it.
#[tokio::test]
async fn the_owner_key_is_ignored_when_the_engine_is_not_behind_a_trusted_gateway() {
    for rule in [
        CallerRule::Local,
        CallerRule::for_serving(true, &[], None),
    ] {
        let c = connect(rule).await;
        offers_the_settling_call(&decision_hint(&c, "dec:a", MEMBER, Some(&[OWNER])).await);
        // And where the design DOES record its owner there, the key cannot
        // make a member the owner.
        call(
            &c,
            "owned_by",
            json!({"from_id": "proj:p", "contributor_id": OWNER}),
            json!({"reflow2/writes_for": OWNER}),
        )
        .await;
        says_it_is_a_question_for_the_owner(
            &decision_hint(&c, "dec:b", MEMBER, Some(&[MEMBER])).await,
        );
    }
}
