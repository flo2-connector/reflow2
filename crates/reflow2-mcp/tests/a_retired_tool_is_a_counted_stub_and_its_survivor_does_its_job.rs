//! A RETIRED TOOL IS A COUNTED REFUSING STUB, AND ITS SURVIVOR DOES ITS JOB.
//!
//! `req:the-2026-10-30-fold-and-removal-candidates-ship-as-counted-refusing-stubs-then-december-decides`
//! (Anthony, 2026-10-30, "Yes, go with B" on
//! `dec:idea-make-the-tool-changes-now-after-a-snapshot-and-restore-what-is-missed-in-december`).
//! Fifteen names stay listed and callable, change nothing, and answer with the
//! call that now does their job; at the re-count (about 2026-12-23) a stub
//! nobody called is deleted and one somebody called is restored. A fold loses
//! no capability, so each survivor is held here to what the folded tool did,
//! read against the folded tool's own kept handler (`<tool>_kept`).
//!
//! That a stub's call lands in the usage ledger under its own name and shows
//! in `usage_report` is held on the real binary, where a ledger exists
//! (`tools/test_feedback_is_a_computed_tally.py`, 5b): an in-memory design
//! keeps none.

use reflow2_mcp::deprecation::{REGISTRY, stub_names};
use reflow2_mcp::service::*;
use rmcp::ServiceExt;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "retired-stub-probe".to_string();
        cfg
    }
}

struct Session {
    client: rmcp::service::RunningService<rmcp::service::RoleClient, Probe>,
}

async fn session(svc: ReflowService) -> Session {
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

/// What a call answered: its structured half, or its whole text, and whether
/// it was refused.
async fn call(s: &Session, tool: &str, args: Value) -> (bool, Value, String) {
    let arguments = args.as_object().cloned().unwrap_or_default();
    match s
        .client
        .call_tool(
            rmcp::model::CallToolRequestParams::new(tool.to_string()).with_arguments(arguments),
        )
        .await
    {
        Ok(r) => {
            let text = r
                .content
                .iter()
                .filter_map(|c| c.as_text().map(|t| t.text.clone()))
                .collect::<Vec<_>>()
                .join("\n");
            (
                !r.is_error.unwrap_or(false),
                r.structured_content.unwrap_or(Value::Null),
                text,
            )
        }
        Err(rmcp::service::ServiceError::McpError(e)) => {
            (false, Value::Null, e.message.to_string())
        }
        Err(e) => panic!("transport failure on {tool}: {e}"),
    }
}

async fn ok(s: &Session, tool: &str, args: Value) -> Value {
    let shown = args.to_string();
    let (answered, v, text) = call(s, tool, args).await;
    assert!(answered, "`{tool}` {shown} was refused: {text}");
    v
}

/// A small design with a change, an artifact, a check and a gap.
async fn seeded() -> (ReflowService, Session) {
    let svc = ReflowService::in_memory().expect("in-memory service");
    let s = session(svc.clone()).await;
    ok(&s, "add_project", json!({"id": "proj:p", "name": "P"})).await;
    ok(
        &s,
        "add_requirement",
        json!({"id": "req:r", "name": "R", "statement": "The pump starts within 2 s."}),
    )
    .await;
    ok(
        &s,
        "add_capability",
        json!({"id": "cap:c", "name": "C", "description": "Starts the pump.", "satisfies": "req:r"}),
    )
    .await;
    ok(
        &s,
        "add_component",
        json!({"id": "cmp:a", "name": "A", "description": "The starter."}),
    )
    .await;
    ok(
        &s,
        "allocate",
        json!({"capability_id": "cap:c", "component_id": "cmp:a"}),
    )
    .await;
    ok(
        &s,
        "add_change_event",
        json!({"id": "chg:c", "name": "Starter reworked", "change_type": "refactor",
               "summary": "The starter was reworked.",
               "affected": [{"node_id": "cmp:a"}, {"node_id": "cap:c"}]}),
    )
    .await;
    (svc, s)
}

/// EVERY STUB: listed with its mark, never offered by find_tools, and its call
/// changes nothing and names what to do.
#[tokio::test]
async fn every_stub_is_listed_refuses_changes_nothing_and_says_what_to_do() {
    let (svc, s) = seeded().await;
    let stubs = stub_names(REGISTRY);
    assert_eq!(
        stubs.len(),
        15,
        "the fifteen 2026-10-30 candidates: {stubs:?}"
    );
    let listed = s.client.list_all_tools().await.expect("tools/list");
    for name in &stubs {
        let t = listed
            .iter()
            .find(|t| t.name == *name)
            .unwrap_or_else(|| panic!("`{name}` stays listed, so a call to it is counted"));
        let d = t.description.as_deref().unwrap_or_default();
        assert!(
            d.starts_with("RETIRED on 2026-10-30"),
            "`{name}`'s listing opens with its mark: {d}"
        );
        assert_eq!(
            t.annotations.as_ref().and_then(|a| a.read_only_hint),
            Some(true),
            "`{name}` changes nothing, so it is read-only"
        );
    }
    // Calls with ordinary arguments, the ones a client of 0.95.0 sent.
    let calls: &[(&str, Value)] = &[
        (
            "set_artifact_checksum",
            json!({"artifact_id": "art:none", "disposition": "baseline_established"}),
        ),
        (
            "acknowledge_gap",
            json!({"gap_id": "gap:none", "reason": "fine"}),
        ),
        (
            "dimension_drift",
            json!({"target_id": "cmp:a", "dimension": "reliability"}),
        ),
        ("propagate_change", json!({"change_event_id": "chg:c"})),
        (
            "verifies",
            json!({"verification_id": "ver:none", "target_id": "cap:c"}),
        ),
        ("graph_report_markdown", json!({})),
        ("evidence_report", json!({})),
        (
            "forecast_readiness",
            json!({"id": "fc:x", "target_id": "cmp:a", "kind": "TRL", "level": 6, "epoch_id": "epoch:none"}),
        ),
        ("frontier", json!({})),
        ("ility_report", json!({})),
        ("granularity_report", json!({})),
        ("consumption_report", json!({})),
        ("surprising_connections", json!({})),
        ("readiness_report", json!({"subject_id": "cap:c"})),
        (
            "gate_on",
            json!({"subject_id": "cap:c", "target_id": "cmp:a", "kind": "TRL", "min_level": 5}),
        ),
    ];
    assert_eq!(calls.len(), stubs.len(), "every stub is called here");
    let before = ok(&s, "export_graph", json!({})).await;
    for (tool, args) in calls {
        let (answered, _, said) = call(&s, tool, args.clone()).await;
        assert!(!answered, "`{tool}` refuses: {said}");
        assert!(
            said.starts_with("REFUSED: `") && said.contains("RETIRED on 2026-10-30"),
            "`{tool}` says it was retired, in words the usage ledger classes as a refusal: {said}"
        );
        assert!(said.contains("changed nothing"), "{said}");
        assert!(said.contains("What to do:"), "{said}");
        assert!(said.contains("2026-12-23"), "it names the re-count: {said}");
    }
    let after = ok(&s, "export_graph", json!({})).await;
    assert_eq!(
        before["nodes"], after["nodes"],
        "no stub call changed a node of the design"
    );
    assert_eq!(before["edges"], after["edges"], "or an edge");

    // A fold's answer is the survivor's call, built from the caller's own words.
    let (_, _, said) = call(
        &s,
        "set_artifact_checksum",
        json!({"artifact_id": "art:a", "disposition": "design_holds", "change_type": "refactor"}),
    )
    .await;
    assert!(
        said.contains(r#"`set_artifact_checksums` with {"accepts":[{"artifact_id":"art:a","change_type":"refactor","disposition":"design_holds"}]}"#),
        "{said}"
    );
    let (_, _, said) = call(
        &s,
        "propagate_change",
        json!({"change_event_id": "chg:c", "full": true}),
    )
    .await;
    assert!(
        said.contains(r#"`propagate_from` with {"full":true,"seed_ids":["chg:c"]}"#),
        "{said}"
    );

    // find_tools never offers one, for the job it now refuses or any other.
    for query in [
        "accept this file's new content as the baseline",
        "what will this change break downstream",
        "give me the design status as readable prose",
        "record that this test checks that capability",
        "when can this release ship given the technology maturity it depends on",
    ] {
        let found = svc
            .find_tools(Parameters(
                serde_json::from_value(json!({"query": query, "limit": 10})).unwrap(),
            ))
            .await
            .expect("find_tools")
            .structured_content
            .expect("structured");
        for item in found["items"].as_array().into_iter().flatten() {
            let tool = item["tool"].as_str().unwrap_or_default();
            assert!(
                !stubs.contains(&tool),
                "find_tools offered the stub `{tool}` for {query:?}"
            );
        }
    }
}

/// `propagate_from` with a ChangeEvent's id as its one seed reads what
/// `propagate_change` read: the same seeds, the same radius, the edge prose.
#[tokio::test]
async fn propagate_from_a_change_reads_what_propagate_change_did() {
    let (svc, s) = seeded().await;
    let survivor = ok(
        &s,
        "propagate_from",
        json!({"seed_ids": ["chg:c"], "full": true}),
    )
    .await;
    let kept = svc
        .propagate_change_kept(
            Parameters(PropagateChangeReq {
                change_event_id: "chg:c".into(),
                max_depth: None,
                full: Some(true),
                budget_chars: None,
                offset: None,
            }),
            "propagate_from",
        )
        .await
        .expect("the kept handler")
        .structured_content
        .expect("structured");
    assert_eq!(survivor["seeded_by_change"], "chg:c", "{survivor}");
    let mut without = survivor.clone();
    without.as_object_mut().unwrap().remove("seeded_by_change");
    assert_eq!(without, kept, "the same reply, field for field");
    let seeds: Vec<&str> = survivor["seeds"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(
        seeds.contains(&"cmp:a") && seeds.contains(&"cap:c"),
        "the walk starts from what the change CHANGED: {seeds:?}"
    );
    // Among several seeds a change is walked as a node, and the reply says so.
    let mixed = ok(
        &s,
        "propagate_from",
        json!({"seed_ids": ["chg:c", "req:r"]}),
    )
    .await;
    assert_eq!(
        mixed["change_seeds_walked_as_nodes"]["ids"],
        json!(["chg:c"]),
        "{mixed}"
    );
    assert!(mixed.get("seeded_by_change").is_none());
}

/// `dimension_drifts` with `target_id` and `dimension` is `dimension_drift`.
#[tokio::test]
async fn dimension_drifts_reads_one_node_as_dimension_drift_did() {
    let (svc, s) = seeded().await;
    for (target, dim) in [("cmp:a", "reliability"), ("cmp:nothing", "security")] {
        let survivor = ok(
            &s,
            "dimension_drifts",
            json!({"target_id": target, "dimension": dim}),
        )
        .await;
        let kept = svc
            .dimension_drift_kept(Parameters(DimensionDriftReq {
                target_id: target.into(),
                dimension: dim.into(),
            }))
            .await
            .expect("the kept handler")
            .structured_content
            .expect("structured");
        assert_eq!(survivor, kept, "{target} on {dim}");
    }
    // Without either it is the sweep it always was.
    let sweep = ok(&s, "dimension_drifts", json!({})).await;
    assert!(
        sweep.get("empty_because").is_some() || sweep.get("items").is_some(),
        "{sweep}"
    );
}

/// `graph_report` with `format: "markdown"` is `graph_report_markdown`'s
/// document; without `format` it is the JSON it always was.
#[tokio::test]
async fn graph_report_in_markdown_is_the_document_graph_report_markdown_was() {
    let (svc, s) = seeded().await;
    let (answered, structured, survivor) =
        call(&s, "graph_report", json!({"format": "markdown"})).await;
    assert!(answered, "{survivor}");
    assert!(structured.is_null(), "a document has no structured half");
    let kept = svc
        .graph_report_markdown_kept()
        .await
        .expect("the kept handler");
    let kept = kept.content[0].as_text().expect("text").text.clone();
    assert_eq!(survivor, kept, "the same document");
    assert!(survivor.starts_with('#'), "{survivor}");
    let json_report = ok(&s, "graph_report", json!({})).await;
    assert!(json_report.get("served_by").is_some(), "{json_report}");
    let (answered, _, said) = call(&s, "graph_report", json!({"format": "yaml"})).await;
    assert!(
        !answered && said.contains("`json`") && said.contains("`markdown`"),
        "{said}"
    );
}

/// `add_verification` with an existing check's id and `verifies` draws the
/// edge `verifies` drew, and every field it is not passed keeps its value.
#[tokio::test]
async fn add_verification_links_an_existing_check_and_keeps_what_it_is_not_passed() {
    let (_svc, s) = seeded().await;
    ok(
        &s,
        "add_verification",
        json!({"id": "ver:v", "name": "Starter test", "method": "test", "level": "unit",
               "description": "Starts the pump on a bench.", "status": "passing",
               "last_run_at": "2026-10-29", "findings": "Started in 1.4 s.",
               "location": "tests/starter.rs"}),
    )
    .await;
    let out = ok(
        &s,
        "add_verification",
        json!({"id": "ver:v", "verifies": [{"target_id": "cap:c"}]}),
    )
    .await;
    assert_eq!(out["verifies"], json!(["cap:c"]), "{out}");
    let node = ok(
        &s,
        "get_node",
        json!({"id": "ver:v", "include_edges": true}),
    )
    .await;
    let p = &node["node"]["properties"];
    for (field, want) in [
        ("name", "Starter test"),
        ("method", "test"),
        ("level", "unit"),
        ("description", "Starts the pump on a bench."),
        ("status", "passing"),
        ("last_run_at", "2026-10-29"),
        ("findings", "Started in 1.4 s."),
        ("location", "tests/starter.rs"),
    ] {
        assert_eq!(p[field], want, "`{field}` survives the revise: {node}");
    }
    assert!(
        node.to_string().contains("VERIFIES"),
        "the VERIFIES edge is drawn: {node}"
    );
}

/// The two bulk forms take one item and do what the one-item tool did.
#[tokio::test]
async fn a_one_item_list_does_what_the_one_item_tool_did() {
    let (_svc, s) = seeded().await;
    ok(
        &s,
        "add_artifact",
        json!({"id": "art:a", "name": "a", "location": "src/a.rs"}),
    )
    .await;
    let out = ok(
        &s,
        "set_artifact_checksums",
        json!({"accepts": [{"artifact_id": "art:a", "checksum": "sha256:aa",
                             "disposition": "baseline_established"}]}),
    )
    .await;
    assert_eq!(out["applied"], true, "{out}");
    let art = ok(&s, "get_node", json!({"id": "art:a"})).await;
    assert_eq!(art["node"]["properties"]["checksum"], "sha256:aa", "{art}");

    let gaps = ok(&s, "detect_gaps", json!({})).await;
    let gap = gaps["items"]
        .as_array()
        .and_then(|g| g.first())
        .cloned()
        .expect("the seeded design raises a gap");
    let gap_id = gap["id"].as_str().unwrap().to_string();
    ok(
        &s,
        "acknowledge_gaps",
        json!({"gaps": [{"gap_id": gap_id, "affected_ids": gap["affected_ids"], "reason": "fine for now"}]}),
    )
    .await;
    let reviewed = ok(&s, "reviewed_gaps", json!({})).await;
    assert!(
        reviewed.to_string().contains(&gap_id),
        "the gap is reviewed: {reviewed}"
    );
}
