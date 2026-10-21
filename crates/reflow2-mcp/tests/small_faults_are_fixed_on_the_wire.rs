//! SMALL FAULTS, over the MCP surface — the fifth PR of phase 6. The core's
//! `small_faults_and_one_formula.rs` pins items 1, 2, 9, 11, 12, 14 and the
//! one-formula and evidence decisions; this file pins what a client writes
//! and is told, item by item of
//! `req:small-read-back-faults-from-the-cogito-trial-are-fixed`:
//!
//! - 3: a statement's `source` text is said not to count as citing;
//! - 4: one answer is linked from every record it became;
//! - 5 and 7: reasoning_report spends the budget it is given, and names it;
//! - 6: a self-asked question is withdrawn by its own id;
//! - 8: a principle says it is silent on a case;
//! - 10: a server that has seen only this call does not report a session of
//!   zero writes;
//! - 12: add_constraint takes `source` and a `setpoint`;
//! - 13: get_node says why every foundation holds while no derivation does;
//! - 15: a settled decision's own "PROPOSED…, awaiting the owner" is caught.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

const COGITO: &str = include_str!("../../reflow2-core/tests/fixtures/argument_trial_cogito.json");

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "small-faults-probe".to_string();
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
            Err(e) => Err(e.to_string()),
        }
    }

    async fn ok(&self, tool: &str, args: Value) -> Value {
        let shown = args.to_string();
        self.call(tool, args)
            .await
            .unwrap_or_else(|e| panic!("`{tool}` {shown} was refused: {e}"))
    }
}

#[tokio::test]
async fn a_source_in_text_is_said_not_to_count_as_citing() {
    let s = session().await;
    let r = s
        .ok(
            "add_requirement",
            json!({"id": "req:lemma", "name": "A lemma", "statement": "It holds.",
                   "statement_kind": "lemma", "taken_as_given": true, "source": "Hardy & Wright, 1938"}),
        )
        .await;
    assert!(
        r["source_note"]
            .as_str()
            .is_some_and(|n| n.contains("DOCUMENTS")),
        "{r:#}"
    );
    let r = s
        .ok(
            "add_constraint",
            json!({"id": "con:axiom", "name": "An axiom", "statement": "It is given.",
                   "statement_kind": "axiom", "source": "the course notes"}),
        )
        .await;
    assert!(r.get("source_note").is_some(), "{r:#}");
    let n = s.ok("get_node", json!({"id": "con:axiom"})).await;
    assert_eq!(
        n["node"]["properties"]["source"], "the course notes",
        "{n:#}"
    );
    // An ordinary requirement's `source` (who asked for it) is not a citation
    // and draws no note.
    let r = s
        .ok(
            "add_requirement",
            json!({"id": "req:need", "name": "A need", "statement": "Users need it.",
                   "source": "the customer"}),
        )
        .await;
    assert!(r.get("source_note").is_none(), "{r:#}");
}

#[tokio::test]
async fn one_answer_is_linked_from_every_record_it_became() {
    let s = session().await;
    s.ok(
        "add_contributor",
        json!({"id": "who:owner", "name": "The owner", "kind": "person"}),
    )
    .await;
    for id in ["req:c1", "req:c2", "req:c3"] {
        s.ok(
            "add_requirement",
            json!({"id": id, "name": format!("claim {id}"), "statement": format!("{id}.")}),
        )
        .await;
    }
    let q = s
        .ok(
            "ask_question",
            json!({"question": "Do the three step-claims hold?", "asked_of": "who:owner",
                   "about": ["req:c1", "req:c2", "req:c3"]}),
        )
        .await;
    let qid = q["question_id"]
        .as_str()
        .expect("a question id")
        .to_string();
    let r = s
        .ok(
            "answer_question",
            json!({"question_id": qid, "answer": "Yes, all three.", "record": "req:c1",
                   "also_records": ["req:c2", "req:c3"]}),
        )
        .await;
    assert_eq!(
        r["answers_also_drawn_from"],
        json!(["req:c2", "req:c3"]),
        "{r:#}"
    );
    for id in ["req:c1", "req:c2", "req:c3"] {
        let n = s
            .ok(
                "get_node",
                json!({"id": id, "include_edges": {"edge_types": ["ANSWERS"]}}),
            )
            .await;
        assert_eq!(n["edges"]["by_type"]["ANSWERS"]["out"], 1, "{id}: {n:#}");
    }
}

#[tokio::test]
async fn reasoning_report_spends_the_budget_it_is_given_and_names_it() {
    let s = session().await;
    let doc: Value = serde_json::from_str(COGITO).unwrap();
    s.ok("import_graph", json!({"document": doc})).await;
    let mut trimmed = 0;
    for budget in [2_000usize, 4_000, 6_000, 8_000, 12_000] {
        let r = s
            .ok("reasoning_report", json!({"budget_chars": budget}))
            .await;
        let chars = serde_json::to_string(&r).unwrap().chars().count();
        let Some(b) = r.get("budget") else {
            continue;
        };
        assert_eq!(b["budget_chars"], budget, "{budget}: {b:#}");
        let note = b["note"].as_str().unwrap_or("");
        assert!(!note.contains("a budget of 0"), "{budget}: {note}");
        if b["fields_trimmed"].as_u64().unwrap_or(0) > 0 {
            trimmed += 1;
            assert!(
                chars * 10 >= budget * 7,
                "{budget}: prose was cut while the reply is only {chars} characters: {b:#}"
            );
        }
    }
    assert!(trimmed > 0, "no budget cut prose, so nothing was checked");
}

#[tokio::test]
async fn a_self_asked_question_is_withdrawn_by_its_own_id() {
    let s = session().await;
    s.ok(
        "add_contributor",
        json!({"id": "who:owner", "name": "The owner", "kind": "person"}),
    )
    .await;
    s.ok(
        "add_requirement",
        json!({"id": "req:x", "name": "X", "statement": "X."}),
    )
    .await;
    let q = s
        .ok(
            "ask_question",
            json!({"question": "Is X right?", "asked_of": "who:owner", "about": ["req:x"]}),
        )
        .await;
    let qid = q["question_id"].as_str().unwrap().to_string();
    let r = s.ok("withdraw_question", json!({"question_id": qid})).await;
    assert_eq!(r["withdrawn"], true, "{r:#}");
    let open = s.ok("open_questions", json!({})).await;
    assert!(!open.to_string().contains(&qid), "{open:#}");
    let bad = s
        .call(
            "withdraw_question",
            json!({"question_id": "q:nothing-asked"}),
        )
        .await;
    assert!(
        bad.as_ref().is_err_and(|e| e.contains(&qid)),
        "an unknown id is refused, naming the questions that exist: {bad:?}"
    );
}

#[tokio::test]
async fn a_principle_says_it_is_silent_on_a_case() {
    let s = session().await;
    s.ok(
        "add_constraint",
        json!({"id": "con:case-loop", "name": "The loop case", "statement": "The loop.",
               "statement_kind": "proposition", "taken_as_given": true}),
    )
    .await;
    s.ok(
        "add_constraint",
        json!({"id": "con:pr-doing-allowing", "name": "Doing and allowing",
               "statement": "Doing harm is worse than allowing it.",
               "statement_kind": "proposition", "silent_on": ["con:case-loop"]}),
    )
    .await;
    let r = s
        .ok(
            "reasoning_report",
            json!({"claim": "con:pr-doing-allowing"}),
        )
        .await;
    let c = r["claims"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "con:pr-doing-allowing")
        .cloned()
        .expect("the principle");
    assert_eq!(c["silent_on"], json!(["con:case-loop"]), "{c:#}");
}

#[tokio::test]
async fn a_server_that_has_seen_only_this_call_counts_no_session() {
    let s = session().await;
    let first = s.ok("loop_status", json!({})).await;
    assert_eq!(first["session"]["counted"], false, "{first:#}");
    assert!(
        first["session"]["note"]
            .as_str()
            .is_some_and(|n| n.contains("cannot count")),
        "{first:#}"
    );
    for id in ["req:a", "req:b"] {
        s.ok(
            "add_requirement",
            json!({"id": id, "name": id, "statement": "It."}),
        )
        .await;
    }
    let later = s.ok("loop_status", json!({})).await;
    assert_eq!(later["session"]["writes"], 2, "{later:#}");
    assert_eq!(later["session"]["counted"], true, "{later:#}");
}

#[tokio::test]
async fn a_constraint_takes_a_source_and_a_setpoint() {
    let s = session().await;
    s.ok(
        "add_constraint",
        json!({"id": "con:carrier", "name": "Emulator carrier", "statement": "Transmit at 9.4 GHz.",
               "quantity": "frequency", "unit": "GHz", "limit": 9.4, "direction": "setpoint",
               "source": "the test plan"}),
    )
    .await;
    let b = s
        .ok("budget_report", json!({"constraint_id": "con:carrier"}))
        .await;
    assert_eq!(b["verdict"], "ungated", "{b:#}");
    let n = s.ok("get_node", json!({"id": "con:carrier"})).await;
    assert_eq!(n["node"]["properties"]["source"], "the test plan", "{n:#}");
    assert_eq!(n["node"]["properties"]["direction"], "setpoint", "{n:#}");
}

#[tokio::test]
async fn get_node_says_why_every_foundation_holds_while_no_derivation_does() {
    let s = session().await;
    s.ok(
        "add_requirement",
        json!({"id": "req:a", "name": "A", "statement": "A.", "statement_kind": "axiom",
               "taken_as_given": true}),
    )
    .await;
    s.ok(
        "add_requirement",
        json!({"id": "req:b", "name": "B", "statement": "B.", "statement_kind": "lemma"}),
    )
    .await;
    s.ok(
        "add_inference",
        json!({"id": "inf:a-to-b", "rule": "all", "premises": ["req:a"], "conclusion": "req:b"}),
    )
    .await;
    s.ok(
        "add_requirement",
        json!({"id": "req:obj", "name": "It does not follow", "statement": "It does not follow.",
               "statement_kind": "objection", "taken_as_given": true}),
    )
    .await;
    s.ok(
        "create_edge",
        json!({"edge_type": "CONTRADICTS", "from_id": "req:obj", "to_id": "inf:a-to-b",
               "props": {"evidence": "the step begs the question"}}),
    )
    .await;
    let n = s.ok("get_node", json!({"id": "req:b"})).await;
    assert!(
        n["node"]["foundations_and_proof"]
            .as_str()
            .is_some_and(|t| t.contains("STEP")),
        "{n:#}"
    );
}

#[tokio::test]
async fn a_settled_decisions_own_proposed_text_is_caught() {
    let s = session().await;
    s.ok(
        "add_contributor",
        json!({"id": "who:owner", "name": "The owner", "kind": "person"}),
    )
    .await;
    s.ok(
        "add_decision",
        json!({"id": "dec:idea-x", "name": "Which store?", "kind": "choice",
               "decision": "PROPOSED, awaiting the owner: (a) RocksDB, (b) SQLite."}),
    )
    .await;
    let r = s
        .ok(
            "set_decision_status",
            json!({"decision_id": "dec:idea-x", "status": "accepted", "chose": "(a)",
                   "approver": "who:owner"}),
        )
        .await;
    let governs = r["settled_question_prose"]["governs"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        governs.iter().any(|g| g["node_id"] == "dec:idea-x"),
        "{r:#}"
    );
}
