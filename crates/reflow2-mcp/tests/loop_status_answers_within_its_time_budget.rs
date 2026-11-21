//! `loop_status` answers within a time budget, names what it did not compute,
//! and stops when its caller has gone — seen the way a client sees it.
//!
//! The class (req:loop-status-answers-within-a-time-budget-and-names-the-sections-it-did-not-compute;
//! fact:root-cause-loop-status-on-reflow2s-design-takes-38-s-on-flo2-io-past-its-30-s-door-gaps-and-defects-each-about-half-2026-11-07):
//! on reflow2's own design the call took 38.1 s on flo2.io's hardware, past
//! the 30 s its gateway waits, and answered nothing. Each earlier crossing had
//! been fixed at one hot spot. These pin the budget itself, never a speed:
//!
//! 1. A budget of nothing (`time_budget_ms: 0`) answers the floor and names
//!    all five whole-design analyses, each with the call that computes it
//!    alone; their counts read null; the answer is not clean; `time_budget`
//!    says it is partial and where the budget came from.
//! 2. A generous budget gives the whole answer: what the call gives with no
//!    budget argument at all, `not_computed` empty.
//! 3. A caller who has gone stops `loop_status`, `detect_gaps` and
//!    `detect_defects` at their next check point.
//!
//! 1 and 2 go over the protocol through an in-process duplex, so the
//! argument is checked against the published schema as a client's is.
//!
//! FAILED FIRST on main at 3256b8188: 1 and 2 were refused there, because
//! `time_budget_ms` was an argument loop_status did not take ("unknown
//! field"); 3 did not compile, because nothing told a handler its caller had
//! gone (`reflow2_mcp::service::with_caller_gone`).

use std::sync::Arc;
use std::time::Duration;

use reflow2_mcp::service::{LoopScopeReq, ReflowService, ScopeReq, with_caller_gone};
use rmcp::ServiceExt;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};

struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "loop-status-answers-within-its-time-budget".to_string();
        cfg.client_info.version = "test".to_string();
        cfg
    }
}

type Client = rmcp::service::RunningService<rmcp::RoleClient, TestClient>;

async fn connect() -> Client {
    connect_to(
        ReflowService::in_memory()
            .expect("in-memory service")
            .without_reaching_out(),
    )
    .await
}

/// A client of `service` (a clone shares its design).
async fn connect_to(service: ReflowService) -> Client {
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

async fn call(c: &Client, tool: &str, args: Value) -> Result<Value, String> {
    let Value::Object(arguments) = args else {
        panic!("arguments for {tool} must be an object");
    };
    let answer = tokio::time::timeout(
        Duration::from_secs(60),
        c.call_tool(CallToolRequestParams::new(tool.to_string()).with_arguments(arguments)),
    )
    .await
    .unwrap_or_else(|_| panic!("{tool} did not answer within 60 s"));
    match answer {
        Ok(r) if r.is_error == Some(true) => Err(format!("{:?}", r.content)),
        Ok(r) => Ok(r.structured_content.unwrap_or(Value::Null)),
        Err(e) => Err(e.to_string()),
    }
}

async fn ok(c: &Client, tool: &str, args: Value) -> Value {
    call(c, tool, args)
        .await
        .unwrap_or_else(|e| panic!("{tool} refused: {e}"))
}

/// Open gaps (a requirement nothing satisfies, a capability nothing owns)
/// and a defect (two components that depend on each other).
async fn seed(c: &Client) {
    ok(c, "add_project", json!({"id": "proj:x", "name": "X"})).await;
    ok(
        c,
        "add_requirement",
        json!({"id": "req:x", "name": "R", "statement": "The system records what it is told."}),
    )
    .await;
    ok(
        c,
        "add_capability",
        json!({"id": "cap:x", "name": "C", "description": "Records."}),
    )
    .await;
    for id in ["cmp:a", "cmp:b"] {
        ok(
            c,
            "add_component",
            json!({"id": id, "name": id, "description": "a part"}),
        )
        .await;
    }
    ok(
        c,
        "depends_on",
        json!({"from_id": "cmp:a", "to_id": "cmp:b"}),
    )
    .await;
    ok(
        c,
        "depends_on",
        json!({"from_id": "cmp:b", "to_id": "cmp:a"}),
    )
    .await;
}

const SECTIONS: [&str; 5] = [
    "confirmation_ledger",
    "reasoning",
    "loop_closure",
    "gaps",
    "defects",
];

fn sections(reply: &Value) -> Vec<String> {
    reply["not_computed"]
        .as_array()
        .unwrap_or_else(|| panic!("not_computed is a list: {reply}"))
        .iter()
        .map(|n| n["section"].as_str().unwrap_or_default().to_string())
        .collect()
}

#[tokio::test]
async fn a_budget_of_nothing_answers_the_floor_and_names_every_analysis() {
    let c = connect().await;
    seed(&c).await;
    let r = call(&c, "loop_status", json!({"time_budget_ms": 0}))
        .await
        .unwrap_or_else(|e| panic!("loop_status refused a time budget: {e}"));

    assert_eq!(sections(&r), SECTIONS, "{r}");
    for n in r["not_computed"].as_array().unwrap() {
        assert_eq!(n["why"], "budget", "{n}");
        assert!(n["call"].as_str().is_some_and(|s| !s.is_empty()), "{n}");
    }
    for field in [
        "unsurfaced_gaps",
        "answered_with_open_gap",
        "structural_defects",
        "unexamined_claims",
    ] {
        assert!(r[field].is_null(), "{field} reads null, never zero: {r}");
    }
    assert!(
        r["unanswered_questions"].is_u64(),
        "the floor is computed: {r}"
    );
    assert_eq!(r["clean"], false, "a partial answer is never clean");
    let next = r["next"].as_array().unwrap();
    assert!(
        next[0]
            .as_str()
            .unwrap()
            .starts_with("NOT COMPUTED THIS CALL"),
        "{next:?}"
    );
    let tb = &r["time_budget"];
    assert_eq!(tb["budget_ms"], 0, "{tb}");
    assert_eq!(tb["set_by"], "argument", "{tb}");
    assert_eq!(tb["complete"], false, "{tb}");
    assert!(
        tb["note"].as_str().is_some_and(|n| n.contains("PARTIAL")),
        "{tb}"
    );
}

#[tokio::test]
async fn a_generous_budget_gives_the_whole_answer() {
    let c = connect().await;
    seed(&c).await;
    let plain = ok(&c, "loop_status", json!({})).await;
    let generous = call(&c, "loop_status", json!({"time_budget_ms": 600_000}))
        .await
        .unwrap_or_else(|e| panic!("loop_status refused a time budget: {e}"));

    for r in [&plain, &generous] {
        assert_eq!(sections(r), Vec::<String>::new(), "{r}");
        assert_eq!(r["time_budget"]["complete"], true, "{r}");
        assert!(r["unsurfaced_gaps"].as_u64().unwrap() > 0, "{r}");
        assert!(r["structural_defects"].as_u64().unwrap() > 0, "{r}");
    }
    assert_eq!(plain["time_budget"]["set_by"], "default");
    assert_eq!(plain["time_budget"]["budget_ms"], 20_000);
    assert_eq!(generous["time_budget"]["set_by"], "argument");
    for field in [
        "unsurfaced_gaps",
        "unanswered_questions",
        "answered_with_open_gap",
        "structural_defects",
        "unproven_capabilities",
        "unexamined_claims",
        "breaches_open",
        "clean",
        "next",
    ] {
        assert_eq!(plain[field], generous[field], "{field}");
    }
}

#[tokio::test]
async fn a_caller_who_has_gone_stops_the_call_at_its_next_check_point() {
    let s = ReflowService::in_memory()
        .expect("in-memory service")
        .without_reaching_out();
    // Seeded over the protocol on a clone, which shares the design; then the
    // handlers are driven directly, as `call_tool` drives them, with the
    // caller-gone question a cancelled request answers `true`.
    seed(&connect_to(s.clone()).await).await;
    let gone: reflow2_mcp::service::CallerGone = Arc::new(|| true);

    let r = with_caller_gone(
        gone.clone(),
        s.loop_status(Parameters(LoopScopeReq {
            time_budget_ms: Some(600_000),
            ..Default::default()
        })),
    )
    .await
    .expect("a stopped loop_status still answers");
    let r = r.structured_content.expect("structured");
    assert_eq!(sections(&r), SECTIONS, "{r}");
    for n in r["not_computed"].as_array().unwrap() {
        assert_eq!(n["why"], "caller_gone", "{n}");
    }
    assert_eq!(r["clean"], false);

    let gaps = with_caller_gone(gone.clone(), s.detect_gaps(Parameters(Default::default()))).await;
    let err = gaps.expect_err("detect_gaps stops when nobody waits for it");
    assert!(err.message.contains("caller_gone"), "{err:?}");
    let defects = with_caller_gone(gone, s.detect_defects(Parameters(ScopeReq::default()))).await;
    let err = defects.expect_err("detect_defects stops when nobody waits for it");
    assert!(err.message.contains("caller_gone"), "{err:?}");

    // And with a caller still there, the same calls answer in full.
    let alive: reflow2_mcp::service::CallerGone = Arc::new(|| false);
    let r = with_caller_gone(alive, s.loop_status(Parameters(LoopScopeReq::default())))
        .await
        .unwrap()
        .structured_content
        .unwrap();
    assert_eq!(sections(&r), Vec::<String>::new(), "{r}");
}

/// THE CLOCK IS THE CALL'S, NOT THE HANDLER'S. A hub's view runs one call on
/// the hub and then on each member in turn, each through its own handler; a
/// budget started per handler would let N designs spend N budgets. So the
/// budget runs from when the call arrived (`call_started`): a call that
/// arrived longer ago than its budget computes no analysis on the next design.
#[tokio::test]
async fn the_budget_runs_from_when_the_call_arrived_not_from_each_design() {
    let s = ReflowService::in_memory()
        .expect("in-memory service")
        .without_reaching_out();
    seed(&connect_to(s.clone()).await).await;
    let arrived = std::time::Instant::now()
        .checked_sub(Duration::from_secs(30))
        .expect("a monotonic clock 30 s back");
    let r = reflow2_mcp::service::with_call_started(
        arrived,
        s.loop_status(Parameters(LoopScopeReq {
            time_budget_ms: Some(10_000),
            ..Default::default()
        })),
    )
    .await
    .unwrap()
    .structured_content
    .unwrap();
    assert_eq!(sections(&r), SECTIONS, "{r}");
    assert!(
        r["time_budget"]["elapsed_ms"].as_u64().unwrap() >= 30_000,
        "elapsed counts from the call's arrival: {}",
        r["time_budget"]
    );
}
