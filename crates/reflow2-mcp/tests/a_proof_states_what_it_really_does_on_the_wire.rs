//! A PROOF STATES WHAT IT REALLY DOES, over the MCP surface — phase 3 of the
//! build list (req:a-proof-can-state-a-contradiction-cases-suppositions-and-cited-theorems,
//! Anthony 2026-10-18). The core's `a_proof_states_what_it_really_does.rs`
//! pins the readings on the trial fixtures; this file pins what a client
//! calls and reads:
//!
//! - `add_requirement` takes `statement_kind` `hypothesis` and `falsum`, and
//!   `get_node` reads them (`assumed` / `discharged`, `false`);
//! - `add_inference` takes `cases` (refused unless each is assumed beneath)
//!   and `cases_exhaustive` (refused unless it names a node, and drawn as a
//!   premise), and the step reads whether its cases are declared exhaustive;
//! - a generalization over F under a still-open hypothesis about F does not
//!   hold (the Arrow trial's N19), and the reply says which assumption;
//! - a theorem taken as given reads `cited_unverified`, and what rests on it
//!   reads conditional, until a pinned source documents it;
//! - `loop_status` counts what each leaves owed.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "proof-probe".to_string();
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
            Ok(v) => panic!("`{tool}` {shown} should have been refused, and answered {v:#}"),
            Err(e) => e,
        }
    }

    async fn definition(&self, id: &str) {
        self.ok(
            "add_constraint",
            json!({"id": id, "name": format!("definition {id}"), "statement": "A definition.",
                   "statement_kind": "definition"}),
        )
        .await;
    }

    async fn statement(&self, id: &str, kind: Option<&str>, variables: Option<&[&str]>) {
        let mut args = json!({"id": id, "name": format!("statement {id}"),
                              "statement": format!("The statement {id}.")});
        if let Some(k) = kind {
            args["statement_kind"] = json!(k);
        }
        if let Some(v) = variables {
            args["variables"] = json!(v);
        }
        self.ok("add_requirement", args).await;
    }

    async fn reads(&self, id: &str) -> Value {
        self.ok("get_node", json!({"id": id})).await["node"]["statement_reading"]["reads"].clone()
    }

    async fn reasoning(&self) -> Value {
        self.ok("loop_status", json!({})).await["reasoning"].clone()
    }
}

#[tokio::test]
async fn a_case_split_is_written_with_its_cases_and_what_makes_them_exhaustive() {
    let s = session().await;
    s.definition("con:def-even").await;
    s.definition("con:every-n-even-or-odd").await;
    for (id, kind) in [
        ("req:case-even", Some("hypothesis")),
        ("req:case-odd", Some("hypothesis")),
        ("req:when-even", None),
        ("req:when-odd", None),
        ("req:n2-plus-n-even", Some("theorem")),
    ] {
        s.statement(id, kind, Some(&["n"])).await;
    }
    for (step, case, out) in [
        ("inf:case-even", "req:case-even", "req:when-even"),
        ("inf:case-odd", "req:case-odd", "req:when-odd"),
    ] {
        s.ok(
            "add_inference",
            json!({"id": step, "rule": "all", "premises": ["con:def-even"],
                   "assumed": [case], "conclusion": out}),
        )
        .await;
    }
    // A case nothing beneath assumes is refused, naming what is.
    let why = s
        .refused(
            "add_inference",
            json!({"id": "inf:split", "rule": "all",
                   "premises": ["req:when-even", "req:when-odd"],
                   "cases": ["req:case-even", "con:def-even"],
                   "conclusion": "req:n2-plus-n-even"}),
        )
        .await;
    assert!(
        why.contains("`cases`") && why.contains("req:case-odd"),
        "{why}"
    );
    // An exhaustiveness record that is not a node is refused.
    let why = s
        .refused(
            "add_inference",
            json!({"id": "inf:split", "rule": "all",
                   "premises": ["req:when-even", "req:when-odd"],
                   "cases": ["req:case-even", "req:case-odd"],
                   "cases_exhaustive": "con:no-such-record",
                   "conclusion": "req:n2-plus-n-even"}),
        )
        .await;
    assert!(why.contains("con:no-such-record"), "{why}");

    // Named, but not declared exhaustive: the step does not hold.
    let split = s
        .ok(
            "add_inference",
            json!({"id": "inf:split", "rule": "all",
                   "premises": ["req:when-even", "req:when-odd"],
                   "cases": ["req:case-even", "req:case-odd"],
                   "conclusion": "req:n2-plus-n-even"}),
        )
        .await;
    let r = &split["inference_reading"];
    assert_eq!(r["cases"]["declared"], false, "{split:#}");
    assert_eq!(r["verdict"], "unknown", "{split:#}");
    assert_eq!(
        s.reasoning().await["cases_not_declared_exhaustive"],
        1,
        "the loop counts it"
    );

    // Declared, by the owner's word: it holds, and the record is a premise.
    let split = s
        .ok(
            "add_inference",
            json!({"id": "inf:split", "rule": "all", "premises": [],
                   "cases": ["req:case-even", "req:case-odd"],
                   "cases_exhaustive": "con:every-n-even-or-odd",
                   "conclusion": "req:n2-plus-n-even"}),
        )
        .await;
    let r = &split["inference_reading"];
    assert_eq!(r["cases"]["declared"], true, "{split:#}");
    assert_eq!(r["cases"]["exhaustive_by"], "con:every-n-even-or-odd");
    assert_eq!(r["verdict"], "holds", "{split:#}");
    assert!(
        r["premises"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == "con:every-n-even-or-odd"),
        "the exhaustiveness record is drawn as a premise: {split:#}"
    );
    assert!(
        r.get("assumes").is_none(),
        "the cases are discharged: {split:#}"
    );
    assert_eq!(s.reads("req:case-even").await, "discharged");
    let thm = s.ok("get_node", json!({"id": "req:n2-plus-n-even"})).await;
    assert_eq!(thm["node"]["proof_status"]["status"], "shown", "{thm:#}");
}

#[tokio::test]
async fn a_reductio_rests_on_a_falsum_that_reads_false() {
    let s = session().await;
    s.definition("con:def-finite").await;
    s.statement("req:lemma", Some("lemma"), None).await;
    s.statement("req:suppose-finite", Some("hypothesis"), Some(&[]))
        .await;
    s.statement("req:bottom", Some("falsum"), None).await;
    s.statement("req:infinitely-many", Some("theorem"), None)
        .await;
    s.ok(
        "add_inference",
        json!({"id": "inf:lemma", "rule": "all", "premises": ["con:def-finite"],
               "conclusion": "req:lemma"}),
    )
    .await;
    s.ok(
        "add_inference",
        json!({"id": "inf:absurd", "rule": "all", "premises": ["req:lemma", "con:def-finite"],
               "assumed": ["req:suppose-finite"], "conclusion": "req:bottom"}),
    )
    .await;
    assert_eq!(s.reads("req:suppose-finite").await, "assumed");
    let reductio = s
        .ok(
            "add_inference",
            json!({"id": "inf:reductio", "rule": "all", "premises": ["req:bottom"],
                   "discharges": ["req:suppose-finite"],
                   "conclusion": "req:infinitely-many", "rule_name": "reductio"}),
        )
        .await;
    assert_eq!(
        reductio["inference_reading"]["verdict"], "holds",
        "{reductio:#}"
    );
    let bottom = s.ok("get_node", json!({"id": "req:bottom"})).await;
    assert_eq!(
        bottom["node"]["statement_reading"]["reads"], "false",
        "{bottom:#}"
    );
    assert_eq!(s.reads("req:suppose-finite").await, "discharged");
    let thm = s.ok("get_node", json!({"id": "req:infinitely-many"})).await;
    assert_eq!(thm["node"]["proof_status"]["status"], "shown", "{thm:#}");
}

#[tokio::test]
async fn a_generalization_over_f_under_an_open_hypothesis_about_f_does_not_hold() {
    let s = session().await;
    s.definition("con:def-unanimity").await;
    s.statement("req:hyp-f", Some("hypothesis"), Some(&["F"]))
        .await;
    s.statement("req:f-dictatorial", None, Some(&["F"])).await;
    s.statement("req:every-f", None, None).await;
    s.ok(
        "add_inference",
        json!({"id": "inf:under", "rule": "all", "premises": ["con:def-unanimity"],
               "assumed": ["req:hyp-f"], "conclusion": "req:f-dictatorial"}),
    )
    .await;
    let g = s
        .ok(
            "add_inference",
            json!({"id": "inf:gen", "rule": "all", "premises": ["req:f-dictatorial"],
                   "generalizes_over": ["F"], "conclusion": "req:every-f"}),
        )
        .await;
    let r = &g["inference_reading"];
    assert_eq!(r["verdict"], "unknown", "{g:#}");
    assert_eq!(
        r["generalization"]["named_by_open_assumptions"],
        json!([{"id": "req:hyp-f", "fixes": ["F"]}]),
        "{g:#}"
    );
    assert!(r["note"].as_str().unwrap().contains("req:hyp-f"), "{g:#}");
    assert_eq!(
        s.reasoning().await["generalization_not_holding_ids"],
        json!(["inf:gen"])
    );
    // Discharged on the generalizing step itself: valid.
    let g = s
        .ok(
            "add_inference",
            json!({"id": "inf:gen", "rule": "all", "premises": [],
                   "discharges": ["req:hyp-f"], "generalizes_over": ["F"],
                   "conclusion": "req:every-f"}),
        )
        .await;
    assert_eq!(g["inference_reading"]["verdict"], "holds", "{g:#}");
}

#[tokio::test]
async fn a_cited_theorem_reads_unverified_until_a_pinned_source_documents_it() {
    let s = session().await;
    s.ok(
        "add_requirement",
        json!({"id": "req:lindemann", "name": "Lindemann: π is transcendental",
               "statement": "π is transcendental.", "statement_kind": "theorem",
               "taken_as_given": true,
               "source": "F. Lindemann, Math. Ann. 20 (1882) 213–225, recalled"}),
    )
    .await;
    s.statement("req:pi-not-algebraic", Some("corollary"), None)
        .await;
    s.ok(
        "add_inference",
        json!({"id": "inf:corollary", "rule": "all", "premises": ["req:lindemann"],
               "conclusion": "req:pi-not-algebraic"}),
    )
    .await;
    assert_eq!(s.reads("req:lindemann").await, "cited_unverified");
    let cor = s
        .ok("reasoning_report", json!({"claim": "req:pi-not-algebraic"}))
        .await;
    let claim = cor["claims"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "req:pi-not-algebraic")
        .cloned()
        .unwrap();
    assert_eq!(claim["truth"], "conditional", "{claim:#}");
    assert_eq!(claim["rests_on_unverified"], json!(["req:lindemann"]));
    assert_eq!(s.reasoning().await["unverified_citations"], 1);

    s.ok(
        "add_artifact",
        json!({"id": "art:lindemann-1882", "name": "Lindemann 1882, as read",
               "location": "https://doi.org/10.1007/BF01446522",
               "checksum": "sha256:00ff"}),
    )
    .await;
    s.ok(
        "documents",
        json!({"artifact_id": "art:lindemann-1882", "target_id": "req:lindemann"}),
    )
    .await;
    assert_eq!(s.reads("req:lindemann").await, "cited");
    let cor = s
        .ok("get_node", json!({"id": "req:pi-not-algebraic"}))
        .await;
    assert_eq!(cor["node"]["proof_status"]["status"], "shown", "{cor:#}");
    assert_eq!(s.reasoning().await["unverified_citations"], 0);
}
