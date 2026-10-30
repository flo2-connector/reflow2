//! A design states how its premises support a conclusion, and reflow2 checks
//! that the conclusion's status follows — over the MCP surface
//! (req:a-design-states-how-premises-support-a-conclusion-and-reflow2-checks-it-follows,
//! dec:an-inference-is-a-record-between-premises-and-conclusion).
//!
//! The typed constructor `add_inference` writes the step in one call — its
//! rule, its premises, its one conclusion and the optional `k`, `rule_name`
//! and `warrant` — and refuses a bad rule or a bad `k` naming the legal values.
//! The check is read where a client already reads: the change walk
//! (`propagate_change` / `propagate_from`, `inferences`), `get_node` on the
//! Inference (`inference_reading`), and `detect_gaps`
//! (`conclusion_does_not_follow`).
//!
//! Driven over the MCP surface, the way a caller meets it, so every test here
//! can be observed failing on a build that predates the tool: `add_inference`
//! is not served there, and the first call is refused.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "inference-probe".to_string();
        cfg
    }
}

struct Session {
    client: rmcp::service::RunningService<rmcp::service::RoleClient, Probe>,
}

async fn session() -> Session {
    let svc = ReflowService::in_memory().expect("in-memory service");
    let (server_rx, client_tx) = tokio::io::duplex(1 << 22);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 22);
    tokio::spawn(async move {
        if let Ok(running) = svc.serve((server_rx, server_tx)).await {
            let _ = running.waiting().await;
        }
    });
    let client = Probe
        .serve((client_rx, client_tx))
        .await
        .expect("in-process handshake");
    Session { client }
}

impl Session {
    async fn call(&self, tool: &str, args: Value) -> Result<Value, String> {
        let arguments = args.as_object().cloned().unwrap_or_default();
        match self
            .client
            .call_tool(
                rmcp::model::CallToolRequestParams::new(tool.to_string()).with_arguments(arguments),
            )
            .await
        {
            Ok(r) if r.is_error.unwrap_or(false) => Err(r
                .content
                .iter()
                .filter_map(|c| c.as_text().map(|t| t.text.clone()))
                .collect::<Vec<_>>()
                .join("\n")),
            Ok(r) => Ok(r.structured_content.unwrap_or(Value::Null)),
            Err(rmcp::service::ServiceError::McpError(e)) => Err(e.message.to_string()),
            Err(e) => Err(format!("transport failure: {e}")),
        }
    }

    async fn ok(&self, tool: &str, args: Value) -> Value {
        let shown = args.to_string();
        self.call(tool, args)
            .await
            .unwrap_or_else(|e| panic!("`{tool}` {shown} was refused: {e}"))
    }

    async fn refused(&self, tool: &str, args: Value) -> String {
        let shown = args.to_string();
        match self.call(tool, args).await {
            Ok(v) => panic!("`{tool}` {shown} was accepted: {v:#}"),
            Err(e) => e,
        }
    }

    async fn requirement(&self, id: &str) {
        self.ok(
            "add_requirement",
            json!({"id": id, "name": format!("claim {id}"), "statement": "A claim its premises support."}),
        )
        .await;
    }

    async fn check(&self, id: &str, status: &str) {
        self.ok(
            "add_verification",
            json!({"id": id, "name": format!("check {id}"), "description": "A check."}),
        )
        .await;
        self.ok(
            "set_verification_status",
            json!({"verification_id": id, "status": status, "last_run_at": "2026-10-14"}),
        )
        .await;
    }

    /// The walk's row for one Inference, from a propagate reply.
    fn row(reply: &Value, inference: &str) -> Value {
        reply["inferences"]
            .as_array()
            .and_then(|rows| rows.iter().find(|r| r["inference"] == inference))
            .cloned()
            .unwrap_or_else(|| panic!("no walk row for {inference}: {reply:#}"))
    }
}

#[tokio::test]
async fn add_inference_writes_the_step_in_one_call() {
    let s = session().await;
    s.requirement("req:the-pitch-is-wet").await;
    s.check("ver:it-rained", "passing").await;
    s.ok(
        "add_contributor",
        json!({"id": "who:owner", "name": "The owner", "kind": "person"}),
    )
    .await;
    s.ok(
        "add_decision",
        json!({
            "id": "dec:rain-wets-the-pitch",
            "name": "If it rains, the pitch is wet",
            "decision": "Rain on the pitch makes it wet.",
            "status": "accepted",
            "approver": "who:owner",
        }),
    )
    .await;

    let reply = s
        .ok(
            "add_inference",
            json!({
                "id": "inf:modus-ponens",
                "rule": "all",
                "rule_name": "modus ponens",
                "warrant": "P, and P → Q, so Q.",
                "premises": ["ver:it-rained", "dec:rain-wets-the-pitch"],
                "conclusion": "req:the-pitch-is-wet",
            }),
        )
        .await;
    let text = reply.to_string();
    assert!(
        text.contains("PREMISE_OF"),
        "the reply names the edges it drew: {reply:#}"
    );
    assert!(text.contains("CONCLUDES"), "{reply:#}");

    // get_node on the step carries its verdict, computed on the read.
    let node = s.ok("get_node", json!({"id": "inf:modus-ponens"})).await;
    let reading = &node["node"]["inference_reading"];
    assert_eq!(reading["verdict"], "holds", "{node:#}");
    assert_eq!(reading["rule"], "all", "{node:#}");
    assert_eq!(reading["rule_name"], "modus ponens", "{node:#}");
    assert_eq!(reading["conclusion"], "req:the-pitch-is-wet", "{node:#}");

    // P fails: the step no longer holds, and the walk says BROKEN.
    s.ok(
        "set_verification_status",
        json!({"verification_id": "ver:it-rained", "status": "failing", "last_run_at": "2026-10-14"}),
    )
    .await;
    let node = s.ok("get_node", json!({"id": "inf:modus-ponens"})).await;
    assert_eq!(
        node["node"]["inference_reading"]["verdict"], "fails",
        "{node:#}"
    );
    s.ok(
        "add_change_event",
        json!({
            "id": "chg:it-did-not-rain",
            "name": "The rain check failed",
            "change_type": "scope_change",
            "summary": "The measurement that said it rained now fails.",
            "affected": [{"node_id": "ver:it-rained"}],
        }),
    )
    .await;
    let radius = s
        .ok(
            "propagate_from",
            json!({"seed_ids": ["chg:it-did-not-rain"]}),
        )
        .await;
    let r = Session::row(&radius, "inf:modus-ponens");
    assert_eq!(r["impact"], "broken", "{r:#}");
    assert_eq!(r["conclusion"], "req:the-pitch-is-wet", "{r:#}");
}

#[tokio::test]
async fn a_bad_rule_or_k_is_refused_with_its_legal_values() {
    let s = session().await;
    s.requirement("req:quorum").await;
    for v in ["ver:1", "ver:2", "ver:3"] {
        s.check(v, "passing").await;
    }
    let base = |rule: &str| {
        json!({
            "id": "inf:q",
            "rule": rule,
            "premises": ["ver:1", "ver:2", "ver:3"],
            "conclusion": "req:quorum",
        })
    };

    let err = s.refused("add_inference", base("most")).await;
    for legal in ["all", "any", "k_of_n"] {
        assert!(err.contains(legal), "a bad rule names `{legal}`: {err}");
    }

    let err = s.refused("add_inference", base("k_of_n")).await;
    assert!(
        err.contains('k'),
        "k_of_n with no k is refused naming k: {err}"
    );

    let mut zero = base("k_of_n");
    zero["k"] = json!(0);
    let err = s.refused("add_inference", zero).await;
    assert!(
        err.contains("1 ≤ k ≤ 3"),
        "k = 0 names the legal range: {err}"
    );

    let mut four = base("k_of_n");
    four["k"] = json!(4);
    let err = s.refused("add_inference", four).await;
    assert!(
        err.contains("1 ≤ k ≤ 3"),
        "k past the premises names the range: {err}"
    );

    let mut stray = base("all");
    stray["k"] = json!(2);
    let err = s.refused("add_inference", stray).await;
    assert!(
        err.contains("k_of_n"),
        "k on another rule is refused: {err}"
    );

    let mut ghost = base("all");
    ghost["premises"] = json!(["ver:1", "ver:missing"]);
    let err = s.refused("add_inference", ghost).await;
    assert!(
        err.contains("ver:missing"),
        "a premise that does not exist is named: {err}"
    );

    // Nothing was written by any refusal.
    let node = s.ok("get_node", json!({"id": "inf:q"})).await;
    assert!(node["node"].is_null(), "{node:#}");

    // And the good call lands.
    let mut good = base("k_of_n");
    good["k"] = json!(2);
    s.ok("add_inference", good).await;
    let node = s.ok("get_node", json!({"id": "inf:q"})).await;
    assert_eq!(
        node["node"]["inference_reading"]["verdict"], "holds",
        "{node:#}"
    );
}

/// The requirement's SHOWN WHEN, through the surface: a claim with redundant
/// support is marked `any`; remove one support and it is WEAKENED and still
/// standing; remove the last and it is UNSUPPORTED.
#[tokio::test]
async fn redundant_support_is_weakened_then_broken_over_the_surface() {
    let s = session().await;
    s.requirement("req:altitude-known").await;
    s.check("ver:gps", "passing").await;
    s.check("ver:baro", "passing").await;
    s.ok(
        "add_inference",
        json!({
            "id": "inf:redundant-altitude",
            "rule": "any",
            "rule_name": "redundant sensors",
            "premises": ["ver:gps", "ver:baro"],
            "conclusion": "req:altitude-known",
        }),
    )
    .await;

    let radius = s
        .ok("propagate_from", json!({"seed_ids": ["ver:gps"]}))
        .await;
    let r = Session::row(&radius, "inf:redundant-altitude");
    assert_eq!(r["impact"], "weakened", "{r:#}");
    assert_eq!(r["conclusion_after"], "holds", "still standing: {r:#}");

    s.ok(
        "delete_edge",
        json!({"edge_type": "PREMISE_OF", "from_id": "ver:gps", "to_id": "inf:redundant-altitude"}),
    )
    .await;
    let radius = s
        .ok("propagate_from", json!({"seed_ids": ["ver:baro"]}))
        .await;
    let r = Session::row(&radius, "inf:redundant-altitude");
    // Losing the last support leaves the claim UNSUPPORTED, never false
    // (req:unsupported-is-not-refuted, 2026-10-17).
    assert_eq!(r["impact"], "unsupported", "{r:#}");
    assert_eq!(r["conclusion_after"], "unknown", "{r:#}");
    assert_eq!(r["conclusion_reads"], "unsupported", "{r:#}");
}

#[tokio::test]
async fn a_claimed_met_past_its_inference_is_a_gap_and_a_cycle_is_refused() {
    let s = session().await;
    s.requirement("req:a").await;
    s.requirement("req:b").await;
    s.check("ver:bad", "failing").await;
    s.ok(
        "add_inference",
        json!({"id": "inf:a-to-b", "rule": "all", "premises": ["req:a", "ver:bad"], "conclusion": "req:b"}),
    )
    .await;
    s.ok(
        "set_requirement_status",
        json!({"requirement_id": "req:b", "status": "met"}),
    )
    .await;
    let gaps = s.ok("detect_gaps", json!({})).await;
    assert!(
        gaps.to_string().contains("conclusion_does_not_follow"),
        "a `met` whose Inference fails is reported: {gaps:#}"
    );

    // b supporting a closes the circle: refused, nothing written.
    let err = s
        .refused(
            "add_inference",
            json!({"id": "inf:b-to-a", "rule": "all", "premises": ["req:b"], "conclusion": "req:a"}),
        )
        .await;
    assert!(err.contains("circular"), "{err}");
    let node = s.ok("get_node", json!({"id": "inf:b-to-a"})).await;
    assert!(
        node["node"].is_null(),
        "a refused step stores nothing: {node:#}"
    );
}

#[tokio::test]
async fn confirmation_is_recorded_and_read_on_the_step() {
    let s = session().await;
    s.requirement("req:p").await;
    s.check("ver:passing", "passing").await;

    // An unconfirmed step reads confirmed_how: unconfirmed.
    let reply = s
        .ok(
            "add_inference",
            json!({
                "id": "inf:unconfirmed",
                "rule": "all",
                "premises": ["ver:passing"],
                "conclusion": "req:p"
            }),
        )
        .await;
    let reading = &reply["inference_reading"];
    assert_eq!(reading["confirmation"]["confirmed_how"], "unconfirmed");
    assert!(reading["confirmation"]["confirmed_by"].is_null());

    // A confirmed step records who, how, and when.
    s.requirement("req:q").await;
    let reply = s
        .ok(
            "add_inference",
            json!({
                "id": "inf:confirmed",
                "rule": "all",
                "premises": ["ver:passing"],
                "conclusion": "req:q",
                "confirmed_by": "who:ajs",
                "confirmed_how": "person",
                "confirmed_at": "2026-10-16T12:00:00Z"
            }),
        )
        .await;
    let reading = &reply["inference_reading"];
    assert_eq!(reading["confirmation"]["confirmed_how"], "person");
    assert_eq!(reading["confirmation"]["confirmed_by"], "who:ajs");
    assert_eq!(
        reading["confirmation"]["confirmed_at"],
        "2026-10-16T12:00:00Z"
    );

    // get_node on the step also carries confirmation.
    let node = s.ok("get_node", json!({"id": "inf:confirmed"})).await;
    assert_eq!(
        node["node"]["inference_reading"]["confirmation"]["confirmed_how"],
        "person"
    );
}

#[tokio::test]
async fn proof_status_is_shown_when_unconfirmed_and_proven_when_confirmed() {
    let s = session().await;
    s.requirement("req:theorem").await;
    s.check("ver:base-case", "passing").await;

    // One unconfirmed step: conclusion is shown, not proven.
    s.ok(
        "add_inference",
        json!({
            "id": "inf:step-1",
            "rule": "all",
            "premises": ["ver:base-case"],
            "conclusion": "req:theorem"
        }),
    )
    .await;
    let node = s.ok("get_node", json!({"id": "req:theorem"})).await;
    let ps = &node["node"]["proof_status"];
    assert_eq!(ps["status"], "shown", "unconfirmed step → shown: {ps:#}");
    assert!(
        ps["unconfirmed_steps"].as_u64().unwrap() > 0,
        "at least one unconfirmed: {ps:#}"
    );

    // Confirm the step: conclusion becomes proven.
    s.ok(
        "add_inference",
        json!({
            "id": "inf:step-1",
            "rule": "all",
            "premises": [],
            "conclusion": "req:theorem",
            "confirmed_by": "who:ajs",
            "confirmed_how": "person",
            "confirmed_at": "2026-10-16T12:00:00Z"
        }),
    )
    .await;
    let node = s.ok("get_node", json!({"id": "req:theorem"})).await;
    let ps = &node["node"]["proof_status"];
    assert_eq!(ps["status"], "proven", "all confirmed → proven: {ps:#}");
    assert_eq!(ps["unconfirmed_steps"], 0);
}

#[tokio::test]
async fn a_counterexample_contradicts_and_breaks_downstream_steps() {
    let s = session().await;
    // req:false-claim is concluded by a step resting on ver:evidence.
    s.requirement("req:false-claim").await;
    s.check("ver:evidence", "passing").await;
    s.ok(
        "add_inference",
        json!({
            "id": "inf:deduction",
            "rule": "all",
            "premises": ["ver:evidence"],
            "conclusion": "req:false-claim"
        }),
    )
    .await;

    // Before the counterexample, the claim holds.
    let node = s.ok("get_node", json!({"id": "req:false-claim"})).await;
    assert_eq!(
        node["node"]["proof_status"]["status"], "shown",
        "before: {:#}",
        node["node"]["proof_status"]
    );

    // Record a counterexample that contradicts it: a check that HOLDS. An
    // objection refutes only once it is established (requirement A); an
    // unestablished one contests, which a_line_of_reasoning_reads_back covers.
    s.check("ver:counterexample", "passing").await;
    s.ok(
        "create_edge",
        json!({
            "edge_type": "CONTRADICTS",
            "from_type": "Verification",
            "from_id": "ver:counterexample",
            "to_type": "Requirement",
            "to_id": "req:false-claim"
        }),
    )
    .await;

    // Now the claim reads as refuted and the proof is unproven.
    let node = s.ok("get_node", json!({"id": "req:false-claim"})).await;
    assert_eq!(
        node["node"]["proof_status"]["status"], "unproven",
        "contradicted → unproven: {:#}",
        node["node"]["proof_status"]
    );

    // req:downstream rests on req:false-claim; the blast radius reports it.
    s.requirement("req:downstream").await;
    s.ok(
        "add_inference",
        json!({
            "id": "inf:uses-claim",
            "rule": "all",
            "premises": ["req:false-claim"],
            "conclusion": "req:downstream"
        }),
    )
    .await;
    let reading = s.ok("get_node", json!({"id": "inf:uses-claim"})).await;
    assert_eq!(
        reading["node"]["inference_reading"]["verdict"], "fails",
        "downstream step fails: {:#}",
        reading["node"]["inference_reading"]
    );
}

#[tokio::test]
async fn a_bad_confirmed_how_is_refused() {
    let s = session().await;
    s.requirement("req:p").await;
    s.check("ver:c", "passing").await;
    let err = s
        .refused(
            "add_inference",
            json!({
                "id": "inf:bad",
                "rule": "all",
                "premises": ["ver:c"],
                "conclusion": "req:p",
                "confirmed_how": "robot"
            }),
        )
        .await;
    assert!(
        err.contains("person") && err.contains("checker"),
        "refused with legal values: {err}"
    );
}
