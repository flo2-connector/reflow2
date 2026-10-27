//! AN ASSUMPTION SAYS HOW IT IS HELD, over the MCP surface.
//!
//! Seen twice on flo2.io (reflow2 0.92.0):
//! `fact:a-chosen-principle-taken-up-as-assumed-raises-a-false-never-discharged-gap-2026-10-25`
//! (the legal-case trial: a governing standard, assumed as the argument skill
//! says) and
//! `fact:a-physical-models-stated-assumptions-read-as-reasoning-not-as-stated-2026-10-27`
//! (the sinogram trial: nine idealizations of a physical model). loop_status
//! said "THE REASONING IS NOT AS STATED: 9 assumption(s) never discharged",
//! because the detector read every undischarged `assumed` premise as a
//! proof's supposition. One property carried three meanings and only the
//! first had a reader.
//!
//! Now an assumed premise says HOW it is held (`held_as` on add_inference,
//! stored on the PREMISE_OF link): `supposition` (a proof's hypothesis, to be
//! discharged), `principle` (an argument's chosen starting point) or `model`
//! (an idealization held on purpose). Only a supposition left undischarged is
//! NOT AS STATED; the others are counted as held on purpose, and what rests on
//! them still reads conditional on them. An assumption nobody marked reads as
//! it always did, so an old design is unchanged.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "held-as-probe".to_string();
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

    async fn requirement(&self, id: &str, extra: Value) {
        let mut args = json!({"id": id, "name": format!("statement {id}"),
                              "statement": "A statement."});
        if let (Some(a), Some(e)) = (args.as_object_mut(), extra.as_object()) {
            for (k, v) in e {
                a.insert(k.clone(), v.clone());
            }
        }
        self.ok("add_requirement", args).await;
    }

    async fn not_as_stated(&self) -> Vec<Value> {
        let r = self
            .ok("detect_gaps", json!({"budget_chars": 200_000}))
            .await;
        r["items"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|g| g["gap_source"] == "reasoning_not_as_stated")
            .collect()
    }

    async fn loop_lines(&self) -> (Value, Vec<String>) {
        let r = self.ok("loop_status", json!({})).await;
        let next = r["next"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|l| l.as_str().map(str::to_string))
            .collect();
        (r["reasoning"].clone(), next)
    }
}

/// A physical model: two idealizations, held on purpose, and the prediction
/// that rests on them.
async fn model(s: &Session) {
    s.requirement("req:assume-monochromatic", json!({})).await;
    s.requirement("req:assume-parallel-beam", json!({})).await;
    s.requirement("req:pred-arcs", json!({})).await;
    let step = s
        .ok(
            "add_inference",
            json!({"id": "inf:pred-arcs", "rule": "all", "premises": [],
                   "assumed": ["req:assume-monochromatic", "req:assume-parallel-beam"],
                   "held_as": "model",
                   "conclusion": "req:pred-arcs"}),
        )
        .await;
    assert_eq!(
        step["inference_reading"]["assumes"],
        json!(["req:assume-monochromatic", "req:assume-parallel-beam"]),
        "{step:#}"
    );
}

/// ⭐ The trial's shape: a model with two idealizations reads conditional on
/// them, and loop_status has no NOT AS STATED line for them; it counts them as
/// held on purpose.
#[tokio::test]
async fn a_models_idealizations_are_held_on_purpose_not_as_stated() {
    let s = session().await;
    model(&s).await;

    let found = s.not_as_stated().await;
    assert!(
        found.is_empty(),
        "a model's idealizations raised reasoning_not_as_stated: {found:#?}"
    );
    let (debt, next) = s.loop_lines().await;
    assert!(
        !next.iter().any(|l| l.contains("NOT AS STATED")),
        "loop_status still reads the model as not as stated: {next:#?}"
    );
    assert_eq!(debt["undischarged_assumptions"], 0, "{debt:#}");
    assert_eq!(debt["assumptions_held_on_purpose"], 2, "{debt:#}");
    assert_eq!(
        debt["assumptions_held_on_purpose_ids"],
        json!(["req:assume-monochromatic", "req:assume-parallel-beam"]),
        "{debt:#}"
    );

    // The reading stays conditional on its named assumptions.
    let r = s
        .ok(
            "reasoning_report",
            json!({"claim": "req:pred-arcs", "budget_chars": 200_000}),
        )
        .await;
    let a = r["assumptions"].as_array().expect("assumptions listed");
    assert_eq!(a.len(), 2, "{r:#}");
    for x in a {
        assert_eq!(x["held_as"], "model", "{x:#}");
        assert!(
            x["reads"].as_str().unwrap().contains("held on purpose"),
            "{x:#}"
        );
    }
    let claim = &r["claims"][0];
    assert_eq!(claim["id"], "req:pred-arcs", "{r:#}");
    assert_eq!(claim["proof_status"], "conditional", "{claim:#}");
}

/// An argument's chosen principle reads the same way (the legal-case shape).
#[tokio::test]
async fn a_chosen_principle_is_held_on_purpose() {
    let s = session().await;
    s.requirement("req:rule-of-law", json!({})).await;
    s.requirement("req:fact-found", json!({})).await;
    s.requirement("req:holding", json!({})).await;
    s.ok(
        "add_inference",
        json!({"id": "inf:apply", "rule": "all", "premises": ["req:fact-found"],
               "assumed": ["req:rule-of-law"], "held_as": "principle",
               "conclusion": "req:holding"}),
    )
    .await;
    assert!(s.not_as_stated().await.is_empty());
    let (debt, _) = s.loop_lines().await;
    assert_eq!(debt["assumptions_held_on_purpose"], 1, "{debt:#}");
}

/// ⭐ ADDITIVE: an assumption nobody said how it is held reads exactly as
/// before (a supposition never discharged), and the gap now says the field
/// exists.
#[tokio::test]
async fn an_unmarked_assumption_reads_as_before_and_the_gap_names_held_as() {
    let s = session().await;
    s.requirement("req:suppose", json!({})).await;
    s.requirement("req:claim", json!({})).await;
    s.ok(
        "add_inference",
        json!({"id": "inf:under", "rule": "all", "premises": [],
               "assumed": ["req:suppose"], "conclusion": "req:claim"}),
    )
    .await;
    let found = s.not_as_stated().await;
    assert_eq!(found.len(), 1, "{found:#?}");
    let said = found[0].to_string();
    assert!(
        said.contains("held_as"),
        "the gap does not name held_as: {said}"
    );
    let (debt, next) = s.loop_lines().await;
    assert_eq!(debt["undischarged_assumptions"], 1, "{debt:#}");
    assert!(
        debt.get("assumptions_held_on_purpose").is_none(),
        "an old design grew a field: {debt:#}"
    );
    assert!(
        next.iter().any(|l| l.contains("NOT AS STATED")),
        "{next:#?}"
    );

    // `supposition`, said, reads the same as unsaid.
    s.ok(
        "add_inference",
        json!({"id": "inf:under", "rule": "all", "premises": [],
               "assumed": ["req:suppose"], "held_as": "supposition",
               "conclusion": "req:claim"}),
    )
    .await;
    assert_eq!(s.not_as_stated().await.len(), 1);

    // And saying `model` on a revise moves it, on the same link.
    s.ok(
        "add_inference",
        json!({"id": "inf:under", "rule": "all", "premises": [],
               "assumed": ["req:suppose"], "held_as": "model",
               "conclusion": "req:claim"}),
    )
    .await;
    assert!(s.not_as_stated().await.is_empty());
}

/// A statement of kind `hypothesis` IS a supposition: saying it is held as a
/// principle or a model is refused, naming the kind. `held_as` with nothing
/// assumed is refused too.
#[tokio::test]
async fn held_as_is_refused_where_it_cannot_mean_anything() {
    let s = session().await;
    s.requirement("req:h", json!({"statement_kind": "hypothesis"}))
        .await;
    s.requirement("req:p", json!({})).await;
    s.requirement("req:c", json!({})).await;
    let e = s
        .call(
            "add_inference",
            json!({"id": "inf:h", "rule": "all", "premises": ["req:p"],
                   "assumed": ["req:h"], "held_as": "model", "conclusion": "req:c"}),
        )
        .await
        .expect_err("a hypothesis held as a model should be refused");
    assert!(e.contains("hypothesis"), "{e}");
    let e = s
        .call(
            "add_inference",
            json!({"id": "inf:h", "rule": "all", "premises": ["req:p"],
                   "held_as": "model", "conclusion": "req:c"}),
        )
        .await
        .expect_err("held_as with nothing assumed should be refused");
    assert!(e.contains("assumed"), "{e}");
}
