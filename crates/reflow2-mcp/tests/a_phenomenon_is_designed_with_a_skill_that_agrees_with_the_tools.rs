//! A PHENOMENON IS DESIGNED WITH A SERVED SKILL THAT AGREES WITH THE TOOLS —
//! `req:a-served-skill-designs-a-phenomenon-and-the-experiment-that-tests-it`
//! (Anthony 2026-10-27, option (a) after the sinogram trial (d)), over the MCP
//! surface.
//!
//! Measured in the sinogram trial on 0.92.0
//! (`fact:the-sinogram-trial-laid-out-a-phenomenon-and-names-what-the-skill-must-say-2026-10-27`):
//! no served skill covered a phenomenon, so the trial agent improvised from
//! design-a-proof and design-an-argument; a model's deliberate idealizations
//! read "THE REASONING IS NOT AS STATED" until `held_as` (#806).
//!
//! - `find_skills` finds `design-a-phenomenon` from the ways a scientist asks.
//! - The skill's recipe, run through the tools a client calls, reads as the
//!   skill says: the model and its idealizations are held on purpose (never
//!   not as stated) and every prediction reads conditional on them; a
//!   hypothesis under test is a supposition, discharged into the prediction
//!   stated before the data ("if H, then P"); a number counts through its kept
//!   record; a result pinned to its data supports the claim it bears for
//!   (supported, never proven) and contests the rival it bears against (never
//!   refutes it); and `propagate_from` on one idealization names the
//!   predictions resting on it.

use reflow2_mcp::service::ReflowService;
use reflow2_mcp::skills::SKILLS;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "phenomenon-probe".to_string();
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
    async fn ok(&self, tool: &str, args: Value) -> Value {
        let shown = args.to_string();
        let arguments = args.as_object().cloned().unwrap_or_default();
        match self
            .client
            .call_tool(
                rmcp::model::CallToolRequestParams::new(tool.to_string()).with_arguments(arguments),
            )
            .await
        {
            Ok(r) if r.is_error.unwrap_or(false) => panic!(
                "`{tool}` {shown} was refused: {}",
                r.content
                    .iter()
                    .filter_map(|c| c.as_text().map(|t| t.text.clone()))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            Ok(r) => r.structured_content.unwrap_or(Value::Null),
            Err(e) => panic!("`{tool}` {shown} failed: {e}"),
        }
    }

    async fn statement(&self, tool: &str, id: &str, kind: &str, given: bool) {
        self.ok(
            tool,
            json!({"id": id, "name": id, "statement": format!("FICTIONAL. {id}."),
                   "statement_kind": kind, "taken_as_given": given}),
        )
        .await;
    }

    async fn reading(&self, id: &str) -> Value {
        self.ok("get_node", json!({"id": id})).await["node"].clone()
    }
}

fn skill(name: &str) -> &'static str {
    SKILLS
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no served skill named {name}"))
        .body
}

#[tokio::test]
async fn find_skills_finds_the_phenomenon_skill_from_the_ways_people_ask() {
    let s = session().await;
    for query in [
        "design a phenomenon",
        "design an experiment to test a hypothesis",
        "physical model and its assumptions",
        "predictions an experiment can tell apart",
    ] {
        let found = s
            .ok("find_skills", json!({"query": query, "limit": 3}))
            .await;
        assert_eq!(
            found["items"][0]["name"], "design-a-phenomenon",
            "{query}: {found:#}"
        );
    }
    for (query, name) in [
        ("design a proof", "design-a-proof"),
        ("for and against", "design-an-argument"),
    ] {
        let found = s
            .ok("find_skills", json!({"query": query, "limit": 3}))
            .await;
        assert_eq!(found["items"][0]["name"], name, "{query}: {found:#}");
    }
}

#[test]
fn the_skill_teaches_what_the_trial_named() {
    let body = skill("design-a-phenomenon");
    for taught in [
        // The model and a law used as given are held on purpose.
        "`held_as` `model`",
        "`held_as` `principle`",
        // The hypothesis under test is a supposition, discharged into the
        // prediction stated before the data.
        "`discharges`",
        // One premise of a step held one way, another another: two calls.
        "same `id` and `conclusion`",
        // Each number through its kept record, and nothing faked without one.
        "`computed`",
        "for a record that does not exist",
        // A result supports, contests, or neither; never proven, never refuted.
        "never proven",
        "`still_contested`",
        // The read-back and the debt.
        "`reasoning_report`",
        "`assumptions_held_on_purpose`",
        "Honest limits",
    ] {
        assert!(body.contains(taught), "the skill teaches {taught:?}");
    }
    // The argument and proof skills point to it.
    for other in ["design-an-argument", "design-a-proof"] {
        assert!(
            skill(other).contains("design-a-phenomenon"),
            "{other} points to design-a-phenomenon"
        );
    }
}

/// THE RECIPE THROUGH THE WIRE, the sinogram trial's cupping in miniature: an
/// observation given, a model and two idealizations held, a number through its
/// kept record, two rival causes each taken up as a supposition and discharged
/// into its prediction, a result pinned to its data, and what each reads.
#[tokio::test]
async fn the_recipe_reads_as_the_skill_says() {
    let s = session().await;
    // The observation, given; the model and its idealizations, held.
    s.statement("add_constraint", "con:obs-cupping", "proposition", true)
        .await;
    for id in [
        "con:model-attenuation",
        "con:idl-monochromatic",
        "con:idl-no-scatter",
    ] {
        s.statement("add_constraint", id, "proposition", false)
            .await;
    }
    // A number, through its kept record.
    s.ok(
        "add_artifact",
        json!({"id": "art:calc-depth", "name": "cupping-depth.calc.json", "artifact_type": "data",
               "location": "cupping-depth.calc.json", "checksum": "sha256:c0ffee"}),
    )
    .await;
    s.ok(
        "add_verification",
        json!({"id": "ver:calc-depth", "name": "Cupping depth under a two-line spectrum",
               "method": "analysis", "computed": true, "status": "passing",
               "findings": "7.7% (bounded 7.5% to 7.9%) lower on a 10 mm chord",
               "last_run_at": "2026-10-27",
               "location": "cupping-depth.calc.json (art:calc-depth)"}),
    )
    .await;
    // The model's own prediction, held on purpose.
    s.statement("add_requirement", "req:pred-flat", "proposition", false)
        .await;
    s.ok(
        "add_inference",
        json!({"id": "inf:model-predicts-flat", "premises": ["con:obs-cupping"],
               "assumed": ["con:model-attenuation", "con:idl-monochromatic", "con:idl-no-scatter"],
               "held_as": "model", "conclusion": "req:pred-flat", "rule": "all",
               "rule_name": "the model applied to a uniform object",
               "warrant": "Under the model a uniform object reconstructs flat."}),
    )
    .await;

    // Two rival causes, each a supposition discharged into its prediction.
    // Each drops one idealization and keeps the other.
    for (h, keeps, under, then) in [
        (
            "req:hyp-beam-hardening",
            "con:idl-no-scatter",
            "req:pred-bh-under",
            "req:if-bh-then-filtration",
        ),
        (
            "req:hyp-scatter",
            "con:idl-monochromatic",
            "req:pred-sc-under",
            "req:if-sc-then-distance",
        ),
    ] {
        s.ok(
            "add_requirement",
            json!({"id": h, "name": h, "statement": format!("SUPPOSE {h}."),
                   "statement_kind": "hypothesis", "variables": []}),
        )
        .await;
        s.statement("add_requirement", under, "proposition", false)
            .await;
        s.statement("add_requirement", then, "proposition", false)
            .await;
        let short = h.trim_start_matches("req:hyp-");
        let step = format!("inf:{short}-predicts");
        // First call: the model's statements, held as a model.
        s.ok(
            "add_inference",
            json!({"id": step, "premises": ["con:obs-cupping", "ver:calc-depth"],
                   "assumed": ["con:model-attenuation", keeps], "held_as": "model",
                   "conclusion": under, "rule": "all",
                   "rule_name": "the model with one idealization dropped",
                   "warrant": "Dropping the idealization predicts this signature."}),
        )
        .await;
        // Second call, same id and conclusion: the supposition, held as one.
        s.ok(
            "add_inference",
            json!({"id": step, "premises": ["con:obs-cupping", "ver:calc-depth"], "assumed": [h],
                   "conclusion": under, "rule": "all"}),
        )
        .await;
        s.ok(
            "add_inference",
            json!({"id": format!("inf:if-{short}"), "premises": [under], "discharges": [h],
                   "conclusion": then, "rule": "all", "rule_name": "conditional proof",
                   "warrant": "What follows under the supposition, stated as its prediction."}),
        )
        .await;
    }

    // Before any result: every prediction reads conditional on the held
    // model, and no assumption reads as reasoning not as stated.
    let flat = s.reading("req:pred-flat").await;
    assert_eq!(flat["proof_status"]["status"], "conditional", "{flat:#}");
    let status = s.ok("loop_status", json!({})).await;
    assert_eq!(
        status["reasoning"]["undischarged_assumptions"], 0,
        "a held model and a discharged supposition are not reasoning not as stated: {status:#}"
    );
    assert_eq!(
        status["reasoning"]["assumptions_held_on_purpose"], 3,
        "loop_status counts the model and its two idealizations as held on purpose: {status:#}"
    );
    let then = s.reading("req:if-bh-then-filtration").await;
    assert_eq!(then["proof_status"]["status"], "conditional", "{then:#}");

    // A what-if on one idealization names the predictions resting on it.
    let walk = s
        .ok(
            "propagate_from",
            json!({"seed_ids": ["con:idl-monochromatic"]}),
        )
        .await;
    let walked = walk.to_string();
    assert!(walked.contains("req:pred-flat"), "{walk:#}");
    assert!(walked.contains("req:pred-sc-under"), "{walk:#}");

    // The claims the experiment bears on, and the result, pinned to its data.
    for c in ["req:claim-bh", "req:claim-sc"] {
        s.statement("add_requirement", c, "conjecture", false).await;
    }
    s.ok(
        "add_constraint",
        json!({"id": "con:result-filtration", "name": "Result: cupping fell with filtration",
               "statement": "FICTIONAL. Cupping fell from 7.6% to 2.1% with 2 mm Al added; it did not move with detector distance.",
               "statement_kind": "evidence", "source_reliability": "B",
               "information_credibility": "2", "reported_at": "2026-10-27"}),
    )
    .await;
    s.ok(
        "add_artifact",
        json!({"id": "art:data-filtration", "name": "filtration-series.h5", "artifact_type": "data",
               "location": "data/filtration-series.h5", "checksum": "sha256:da7a"}),
    )
    .await;
    s.ok(
        "documents",
        json!({"artifact_id": "art:data-filtration", "target_id": "con:result-filtration"}),
    )
    .await;
    s.ok(
        "add_inference",
        json!({"id": "inf:result-bears-for-bh",
               "premises": ["con:result-filtration", "req:if-bh-then-filtration", "req:if-sc-then-distance"],
               "conclusion": "req:claim-bh", "rule": "all",
               "rule_name": "the discriminating result",
               "warrant": "Beam hardening predicted the fall with filtration; scatter predicted a change with distance that did not come."}),
    )
    .await;
    s.ok(
        "review_relations",
        json!({"node_id": "con:result-filtration", "links": [{"relation": "CONTRADICTS",
               "other_id": "req:claim-sc",
               "evidence": "scatter predicted a change with detector distance (req:if-sc-then-distance); none was seen"}]}),
    )
    .await;

    // The claim the result bears for: conditional on the held model, resting
    // on the result as evidence. Never proven, and never plain holds.
    let bh = s.reading("req:claim-bh").await;
    assert_eq!(
        bh["statement_reading"]["reads"], "conditional",
        "supported by the result, under the held model: {bh:#}"
    );
    let rep = s
        .ok("reasoning_report", json!({"claim": "req:claim-bh"}))
        .await;
    let claim = rep["claims"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "req:claim-bh")
        .cloned()
        .expect("the claim in its own read-back");
    assert_eq!(
        claim["rests_on_tests"],
        json!(["con:result-filtration"]),
        "{claim:#}"
    );
    assert_eq!(
        claim["assumes"],
        json!([
            "con:idl-monochromatic",
            "con:idl-no-scatter",
            "con:model-attenuation"
        ]),
        "{claim:#}"
    );

    // The rival the result bears against: contested, never refuted. On its
    // own it reads open (no step shows it); the read-back lists it as still
    // contested, with the result against it.
    let sc = s.reading("req:claim-sc").await;
    assert_eq!(sc["statement_reading"]["reads"], "open", "{sc:#}");
    let whole = s.ok("reasoning_report", json!({})).await;
    assert!(
        whole["still_contested"]
            .as_array()
            .is_some_and(|a| a.iter().any(|c| c["claim"] == "req:claim-sc"
                && c["against"] == json!(["con:result-filtration"]))),
        "{whole:#}"
    );

    // The experiment, planned to tell the rivals apart and then run: its
    // passing status moves neither claim. The result does that, as evidence.
    s.ok(
        "add_verification",
        json!({"id": "ver:exp-filtration", "name": "Filtration and distance series",
               "method": "measurement",
               "verifies": [{"target_id": "req:claim-bh"}, {"target_id": "req:claim-sc"}]}),
    )
    .await;
    s.ok(
        "add_verification",
        json!({"id": "ver:exp-filtration", "status": "passing", "last_run_at": "2026-10-27"}),
    )
    .await;
    assert_eq!(
        s.reading("req:claim-sc").await["statement_reading"]["reads"],
        "open"
    );
    assert_eq!(
        s.reading("req:claim-bh").await["statement_reading"]["reads"],
        "conditional"
    );
}
