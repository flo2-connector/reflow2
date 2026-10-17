//! A PROOF SAYS WHAT IT ASSUMES, AND AN AGENT ASKS, over the MCP surface —
//! phase 2 of the fixes Anthony accepted on 2026-10-17 after the primes trial
//! on flo2.io
//! (fact:the-primes-trial-rung-1-ran-on-flo2-io-and-its-friction-names-what-counts-2026-10-17).
//! The core's `a_proof_says_what_it_assumes.rs` pins the readings; this file
//! pins what a client calls and reads:
//!
//! - `add_inference` takes `assumed`, `discharges` (refused unless assumed
//!   beneath), `generalizes_over`, `independent_of` and `independent_except`
//!   (req:a-proof-can-discharge-an-assumption-and-generalize-over-a-variable);
//! - `add_requirement` / `add_constraint` take `statement_kind`, `variables`
//!   and `fixes`, a read carries `statement_reading`, and a definition carries
//!   no budget direction (req:a-proof-is-designed-with-a-skill-and-the-tools-fit-mathematics);
//! - `ask_question` puts the agent's own question to a named person, and
//!   `open_questions`, `loop_status` and `answer_question` treat it like any
//!   waiting question (req:an-agent-asks-the-owner-its-own-question-about-a-node);
//! - `find_skills` finds the proof skill from "design a proof" and "prove a
//!   theorem".

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

    async fn axiom(&self, id: &str) {
        self.ok(
            "add_constraint",
            json!({"id": id, "name": format!("axiom {id}"), "statement": "A definition.",
                   "statement_kind": "definition"}),
        )
        .await;
    }

    async fn statement(&self, id: &str) {
        self.ok(
            "add_requirement",
            json!({"id": id, "name": format!("statement {id}"), "statement": "A statement."}),
        )
        .await;
    }
}

/// Euclid's reductio, written the way the skill teaches it.
async fn reductio(s: &Session) {
    s.axiom("con:def-finite").await;
    s.axiom("con:def-prime").await;
    for id in [
        "req:lemma-euclid",
        "req:suppose-finite",
        "req:contradiction",
        "req:theorem",
    ] {
        s.statement(id).await;
    }
    s.ok(
        "add_inference",
        json!({"id": "inf:euclid", "rule": "all", "premises": ["con:def-prime"],
               "conclusion": "req:lemma-euclid"}),
    )
    .await;
    let absurd = s
        .ok(
            "add_inference",
            json!({"id": "inf:absurd", "rule": "all",
                   "premises": ["req:lemma-euclid", "con:def-finite"],
                   "assumed": ["req:suppose-finite"],
                   "conclusion": "req:contradiction"}),
        )
        .await;
    assert_eq!(
        absurd["inference_reading"]["assumes"],
        json!(["req:suppose-finite"]),
        "{absurd:#}"
    );
}

#[tokio::test]
async fn a_reductio_is_recorded_with_its_supposition_assumed_and_discharged() {
    let s = session().await;
    reductio(&s).await;
    // A discharge of something nothing beneath assumes is refused, naming
    // what IS assumed.
    let why = s
        .refused(
            "add_inference",
            json!({"id": "inf:theorem", "rule": "all", "premises": ["req:contradiction"],
                   "discharges": ["con:def-prime"], "conclusion": "req:theorem"}),
        )
        .await;
    assert!(why.contains("is not assumed anywhere"), "{why}");
    assert!(why.contains("req:suppose-finite"), "{why}");
    let nothing = s.ok("get_node", json!({"id": "inf:theorem"})).await;
    assert!(
        nothing["node"].is_null(),
        "refused means nothing written: {nothing:#}"
    );

    s.ok(
        "add_inference",
        json!({"id": "inf:theorem", "rule": "all",
               "premises": ["req:contradiction", "con:def-finite"],
               "discharges": ["req:suppose-finite"], "conclusion": "req:theorem",
               "rule_name": "proof by contradiction"}),
    )
    .await;
    let theorem = s.ok("get_node", json!({"id": "req:theorem"})).await;
    assert_eq!(
        theorem["node"]["proof_status"]["status"], "shown",
        "{theorem:#}"
    );
    let contradiction = s.ok("get_node", json!({"id": "req:contradiction"})).await;
    assert_eq!(
        contradiction["node"]["proof_status"]["status"], "conditional",
        "{contradiction:#}"
    );
    let r = s
        .ok("reasoning_report", json!({"claim": "req:theorem"}))
        .await;
    assert_eq!(
        r["assumptions"][0]["reads"], "assumed, discharged by inf:theorem",
        "{r:#}"
    );
}

#[tokio::test]
async fn a_generalization_is_declared_on_the_step_and_a_fixed_variable_stops_it() {
    let s = session().await;
    s.axiom("con:def-prime").await;
    s.ok(
        "add_requirement",
        json!({"id": "req:e5", "name": "q is not on the list L", "statement": "…",
               "variables": ["L"]}),
    )
    .await;
    s.statement("req:lemma-euclid").await;
    s.ok(
        "add_inference",
        json!({"id": "inf:e5", "rule": "all", "premises": ["con:def-prime"],
               "conclusion": "req:e5"}),
    )
    .await;
    let step = s
        .ok(
            "add_inference",
            json!({"id": "inf:euclid", "rule": "all", "premises": ["req:e5"],
                   "generalizes_over": ["L"], "conclusion": "req:lemma-euclid"}),
        )
        .await;
    let reading = &step["inference_reading"];
    assert_eq!(reading["verdict"], "holds", "{step:#}");
    assert_eq!(reading["generalization"]["mentioned_by"], json!(["req:e5"]));

    s.ok(
        "add_requirement",
        json!({"id": "req:e5", "fixes": ["L=2,3,5"]}),
    )
    .await;
    let node = s.ok("get_node", json!({"id": "inf:euclid"})).await;
    let reading = &node["node"]["inference_reading"];
    assert_eq!(reading["verdict"], "unknown", "{node:#}");
    assert!(
        reading["note"]
            .as_str()
            .unwrap()
            .contains("req:e5 fixes L=2,3,5"),
        "{node:#}"
    );
}

#[tokio::test]
async fn two_proofs_are_stated_independent_and_the_statement_is_checked() {
    let s = session().await;
    s.axiom("con:def-prime").await;
    for id in ["req:l5", "req:a", "req:b", "req:theorem"] {
        s.statement(id).await;
    }
    for (id, premises, conclusion) in [
        ("inf:l5", vec!["con:def-prime"], "req:l5"),
        ("inf:a", vec!["req:l5"], "req:a"),
        ("inf:b", vec!["req:l5"], "req:b"),
        ("inf:by-a", vec!["req:a"], "req:theorem"),
    ] {
        s.ok(
            "add_inference",
            json!({"id": id, "rule": "all", "premises": premises, "conclusion": conclusion}),
        )
        .await;
    }
    let why = s
        .refused(
            "add_inference",
            json!({"id": "inf:by-b", "rule": "all", "premises": ["req:b"],
                   "independent_of": ["inf:nowhere"], "conclusion": "req:theorem"}),
        )
        .await;
    assert!(why.contains("not another step"), "{why}");
    let step = s
        .ok(
            "add_inference",
            json!({"id": "inf:by-b", "rule": "all", "premises": ["req:b"],
                   "independent_of": ["inf:by-a"], "conclusion": "req:theorem"}),
        )
        .await;
    let ind = &step["inference_reading"]["independence"][0];
    assert_eq!(ind["holds"], false, "{step:#}");
    assert_eq!(ind["shared_steps"], json!(["inf:l5"]));
    let step = s
        .ok(
            "add_inference",
            json!({"id": "inf:by-b", "rule": "all", "premises": [],
                   "independent_except": ["req:l5"], "conclusion": "req:theorem"}),
        )
        .await;
    assert_eq!(
        step["inference_reading"]["independence"][0]["holds"], true,
        "{step:#}"
    );
}

#[tokio::test]
async fn a_statement_says_its_kind_and_a_definition_carries_no_budget_direction() {
    let s = session().await;
    let def = s
        .ok(
            "add_constraint",
            json!({"id": "con:def-prime", "name": "Definition: prime",
                   "statement": "A prime is an integer p > 1 whose only positive divisors are 1 and p.",
                   "statement_kind": "definition", "echo": "node"}),
        )
        .await;
    assert!(
        def["properties"].get("direction").is_none(),
        "a definition carries no budget words: {def:#}"
    );
    let budget = s
        .ok(
            "add_constraint",
            json!({"id": "con:mass", "name": "Mass", "statement": "Under 40 kg.",
                   "quantity": "mass_kg", "limit": 40.0, "echo": "node"}),
        )
        .await;
    assert_eq!(budget["properties"]["direction"], "maximum", "{budget:#}");
    s.ok(
        "add_requirement",
        json!({"id": "req:conjecture", "name": "n² + n + 41 is always prime",
               "statement": "For every n ≥ 0, n² + n + 41 is prime.",
               "statement_kind": "conjecture"}),
    )
    .await;
    let c = s.ok("get_node", json!({"id": "req:conjecture"})).await;
    assert_eq!(
        c["node"]["statement_reading"]["kind"], "conjecture",
        "{c:#}"
    );
    assert_eq!(c["node"]["statement_reading"]["reads"], "open", "{c:#}");
    // A corollary and a proposition are statement kinds, and read `stated`
    // until a step concludes them; neither is taken as given.
    for (id, kind) in [
        ("req:corollary", "corollary"),
        ("con:proposition", "proposition"),
    ] {
        let tool = if id.starts_with("con:") {
            "add_constraint"
        } else {
            "add_requirement"
        };
        s.ok(
            tool,
            json!({"id": id, "name": kind, "statement": kind, "statement_kind": kind}),
        )
        .await;
        let n = s.ok("get_node", json!({"id": id})).await;
        assert_eq!(n["node"]["statement_reading"]["kind"], kind, "{n:#}");
        assert_eq!(n["node"]["statement_reading"]["reads"], "stated", "{n:#}");
        assert!(
            n["node"]["properties"].get("taken_as_given").is_none(),
            "{n:#}"
        );
    }
    let why = s
        .refused(
            "add_requirement",
            json!({"id": "req:x", "name": "x", "statement": "x", "statement_kind": "remark"}),
        )
        .await;
    assert!(why.contains("statement_kind"), "{why}");
}

#[tokio::test]
async fn an_agent_puts_its_own_question_to_the_owner_and_it_waits_like_any_other() {
    let s = session().await;
    s.ok("add_contributor", json!({"id": "who:jane", "name": "Jane"}))
        .await;
    s.axiom("con:def-finite").await;
    s.statement("req:theorem").await;

    // Refused before anything is written: nobody named, nothing it is about,
    // a node that is not there.
    let why = s
        .refused(
            "ask_question",
            json!({"question": "Which foundations do you take as given?",
                   "asked_of": "who:nobody", "about": ["con:def-finite"]}),
        )
        .await;
    assert!(why.contains("who:nobody"), "{why}");
    let why = s
        .refused(
            "ask_question",
            json!({"question": "Which foundations do you take as given?",
                   "asked_of": "who:jane", "about": []}),
        )
        .await;
    assert!(why.contains("about"), "{why}");
    let why = s
        .refused(
            "ask_question",
            json!({"question": "Which foundations do you take as given?",
                   "asked_of": "who:jane", "about": ["req:nowhere"]}),
        )
        .await;
    assert!(why.contains("req:nowhere"), "{why}");

    let asked = s
        .ok(
            "ask_question",
            json!({"question": "Do you take the definition of a finite set as given?",
                   "asked_of": "who:jane", "about": ["con:def-finite", "req:theorem"],
                   "batch": "round-1"}),
        )
        .await;
    let qid = asked["question_id"].as_str().unwrap().to_string();
    assert!(qid.starts_with("question:asked-"), "{asked:#}");
    assert_eq!(asked["status"], "asked");
    assert_eq!(asked["batch_position"], 1);

    let open = s.ok("open_questions", json!({})).await;
    let row = open["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|q| q["question_id"] == qid.as_str())
        .unwrap_or_else(|| panic!("open_questions lists it: {open:#}"))
        .clone();
    assert_eq!(row["origin"], "asker", "{row:#}");
    assert_eq!(row["asked_of"], "who:jane");
    assert!(
        row.get("gap_id").is_none(),
        "no gap stands behind it: {row:#}"
    );
    let evidence: Vec<&str> = row["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap())
        .collect();
    assert_eq!(evidence, vec!["con:def-finite", "req:theorem"]);

    let ls = s.ok("loop_status", json!({})).await;
    assert_eq!(ls["unanswered_questions"], 1, "{ls:#}");
    let theirs = s
        .ok("loop_status", json!({"contributor_id": "who:jane"}))
        .await;
    assert_eq!(
        theirs["questions_put_to_them"][0]["question_id"],
        qid.as_str(),
        "{theirs:#}"
    );

    // The wording never changes once asked.
    let why = s
        .refused(
            "ask_question",
            json!({"id": qid, "question": "Something else?", "asked_of": "who:jane",
                   "about": ["req:theorem"]}),
        )
        .await;
    assert!(why.contains("wording never changes"), "{why}");

    let answered = s
        .ok(
            "answer_question",
            json!({"question_id": qid, "answer": "Yes, take it as given.",
                   "answered_by": "who:jane"}),
        )
        .await;
    assert_eq!(answered["answered_by"], "who:jane", "{answered:#}");
    let q = s.ok("get_node", json!({"id": qid})).await;
    assert_eq!(q["node"]["properties"]["answered_by"], "who:jane", "{q:#}");
    let ls = s.ok("loop_status", json!({})).await;
    assert_eq!(ls["unanswered_questions"], 0, "{ls:#}");
}

#[tokio::test]
async fn find_skills_finds_the_proof_skill_from_the_ways_people_ask() {
    let s = session().await;
    for query in ["design a proof", "prove a theorem"] {
        let found = s
            .ok("find_skills", json!({"query": query, "limit": 3}))
            .await;
        assert_eq!(
            found["items"][0]["name"], "design-a-proof",
            "{query}: {found:#}"
        );
    }
}
