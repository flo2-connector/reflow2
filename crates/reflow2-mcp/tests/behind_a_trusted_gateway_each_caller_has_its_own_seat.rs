//! BEHIND A TRUSTED GATEWAY, EACH CALLER HAS ITS OWN SEAT IN THE USAGE LEDGER —
//! so a cue offered to one caller is followed only by that caller's calls.
//!
//! The symptom (flo2's design 1a97fc1f1de4d5d2,
//! `fact:on-flo2-io-cue-counts-are-per-design-not-per-agent-2026-10-29`): flo2.io
//! keeps ONE engine session per design for every caller, so every call landed
//! on one seat, and a cue offered on the owner's call counted as followed by a
//! member's. The cause, measured on the published 0.94.0 binary
//! (`fact:root-cause-the-usage-ledger-takes-its-seat-from-the-mcp-session-so-a-gateways-callers-share-one-2026-10-30`):
//! the ledger wrote the SESSION's seat on every line, and no per-call input
//! could name another. One engine session per caller would cost a store's
//! worth of open files each; instead the gateway names the caller's seat per
//! call (`_meta["reflow2/seat"]`), as it names the caller.
//!
//! `two_callers_on_one_session_are_counted_on_their_own_seats` FAILED on
//! 0.94.0's engine (the cue read as followed, every line one seat) and passes
//! on this one. It needs a store in a folder: an in-memory design keeps no
//! ledger.

use reflow2_mcp::caller::CallerRule;
use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};

const OWNER: &str = "who:bob";
const MEMBER: &str = "who:ann";
const CUE: &str = "loop_hint/detect_gaps";

struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "flo2-gateway".to_string();
        cfg.client_info.version = "test".to_string();
        cfg
    }
}

type Client = rmcp::service::RunningService<rmcp::RoleClient, TestClient>;

/// ONE session, as the gateway keeps one per design, on a store in a folder.
async fn connect(graph_path: &str, rule: CallerRule) -> Client {
    let service = ReflowService::new(graph_path)
        .expect("service")
        .with_caller_rule(rule);
    let (server_rx, client_tx) = tokio::io::duplex(1 << 22);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 22);
    tokio::spawn(async move {
        if let Ok(running) = service.serve((server_rx, server_tx)).await {
            let _ = running.waiting().await;
        }
    });
    TestClient
        .serve((client_rx, client_tx))
        .await
        .expect("the in-process handshake")
}

async fn call(c: &Client, tool: &str, args: Value, who: &str, seat: &str) -> Value {
    // Seeding names nobody, as flo2 seeds a design: nobody exists to write for yet.
    let meta = if who.is_empty() {
        json!({})
    } else {
        json!({"reflow2/writes_for": who, "reflow2/seat": seat})
    };
    let params: CallToolRequestParams = serde_json::from_value(json!({
        "name": tool, "arguments": args, "_meta": meta,
    }))
    .expect("params");
    let r = c
        .call_tool(params)
        .await
        .unwrap_or_else(|e| panic!("{tool}: {e}"));
    assert!(r.is_error != Some(true), "{tool}: {:?}", r.content);
    r.structured_content.unwrap_or(Value::Null)
}

/// The owner's call offers the cue; the MEMBER then makes the call it names;
/// the owner makes another call that is not it. Returns the cue's counts and
/// the seats on the ledger's call lines, in order.
async fn owner_offered_member_followed(rule: CallerRule) -> (Value, Vec<String>) {
    let dir = tempfile::tempdir().unwrap();
    let gp = dir.path().join("graph").display().to_string();
    let c = connect(&gp, rule).await;
    for id in [OWNER, MEMBER] {
        call(
            &c,
            "add_contributor",
            json!({"id": id, "name": id, "kind": "person"}),
            "",
            "",
        )
        .await;
    }
    let w = call(
        &c,
        "add_requirement",
        json!({"id": "req:warm", "name": "The greenhouse stays above five degrees",
               "statement": "The greenhouse stays above five degrees."}),
        OWNER,
        "seat:owner",
    )
    .await;
    assert!(
        w["loop_hint"]
            .as_str()
            .unwrap_or("")
            .contains("detect_gaps"),
        "the owner's write offers detect_gaps: {w}"
    );
    call(&c, "detect_gaps", json!({}), MEMBER, "seat:member").await;
    call(&c, "graph_report", json!({}), OWNER, "seat:owner").await;
    let u = call(
        &c,
        "usage_report",
        json!({"peek": true}),
        OWNER,
        "seat:reader",
    )
    .await;
    let seats = reflow2_mcp::usage::read_all(&gp)
        .into_iter()
        .filter(|l| l.kind == "call")
        .filter_map(|l| l.seat)
        .collect();
    (u["tally"]["cues"]["by_cue"][CUE].clone(), seats)
}

/// The fault: behind the gateway, the cue the owner was offered is NOT
/// followed by the member's call, and each caller's lines carry their seat.
#[tokio::test]
async fn two_callers_on_one_session_are_counted_on_their_own_seats() {
    let (cue, seats) = owner_offered_member_followed(CallerRule::TrustedGateway {
        name: "flo2.io".into(),
    })
    .await;
    assert_eq!(cue["offered"], json!(1), "{cue}");
    assert_eq!(
        cue["followed_at_all"],
        json!(0),
        "the member's detect_gaps does not follow the owner's cue: {cue}"
    );
    assert_eq!(cue["unattributable"], json!(0), "{cue}");
    assert_eq!(
        seats[2..],
        ["seat:owner", "seat:member", "seat:owner", "seat:reader"].map(String::from),
        "each line carries the seat the gateway named: {seats:?}"
    );
}

/// The seat key is the trusted gateway's word and nobody else's: on a local
/// engine it is ignored, every line carries the session's seat, and the
/// session's later detect_gaps follows the cue, as before.
#[tokio::test]
async fn the_seat_key_is_ignored_when_the_engine_is_not_behind_a_trusted_gateway() {
    let (cue, seats) = owner_offered_member_followed(CallerRule::Local).await;
    assert_eq!(cue["followed_at_all"], json!(1), "{cue}");
    assert!(
        seats
            .iter()
            .all(|s| s == &seats[0] && !s.starts_with("seat:")),
        "one session, one seat of the engine's own: {seats:?}"
    );
}
