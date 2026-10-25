//! THE UNUSED `Anchor` NODE TYPE IS REMOVED — the first item ever taken off
//! what a client of the MCP server sees, seen the way that client sees it.
//!
//! Anthony's ruling, `dec:anchor-retire-or-implement` (accepted 2026-10-13):
//! "Remove the unused Anchor type, but only after a rule for retiring things
//! exists, because it would be the first thing ever taken off what Alex's MCP
//! sees." The rule (`cap:a-served-item-is-removed-only-after-a-release-that-announced-it`)
//! is two releases: 0.85.0 announced it (still served, marked `deprecated`, a
//! `deprecation_notice` on every call that used it), and 0.86.0 removes it.
//! Anthony, 2026-10-17: "Remove it before the tag".
//!
//! What is pinned, every call over the protocol through an in-process duplex,
//! with the registry AS BUILT:
//!
//! 1. The vocabulary no longer holds it: describe_schema lists no `Anchor`,
//!    answers a question about it as about any type it does not know, and
//!    lists no `deprecations` (the registry is empty, because a node type's
//!    entry leaves with the item).
//! 2. NO SERVED TEXT SAYS IT IS STILL COMING, OR GOING: not tools/list, not
//!    describe_schema's whole answer. 0.86.0 as first cut kept the type and
//!    went on serving "DEPRECATED in 0.85.0, removed in the next release" —
//!    a sentence the release serving it made false.
//! 3. Writing one is refused naming the type, and nothing is written.
//! 4. An export that holds one is refused by the import, naming the type as
//!    RETIRED with the decision and the way through, and nothing is written:
//!    the import is all or nothing.
//! 5. DriftEvent's hint (moved in 0.85.0 with the announcement) still says
//!    what a DriftEvent references, and drift detection still draws exactly
//!    that.
//!
//! A STORE that holds an Anchor is the provenance guard's, in the core: it
//! opens when it holds none and is refused by name when it holds any
//! (`reflow2-core/tests/provenance.rs`, and docs/upgrading-to-v0.86.0.md for
//! the measurement against the published 0.85.0 binary).
//!
//! NOT RUN FIRST AGAINST THE BUILD THAT STILL SERVED IT: on that build (the
//! cut at e9ab4eaa4) 1-4 cannot pass by construction — `Anchor` was in the
//! schema and the registry, so describe_schema listed and marked it, the
//! create and the import were accepted. Said rather than claimed as observed.

use reflow2_mcp::deprecation::REGISTRY;
use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};
use std::time::Duration;

struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "the-unused-anchor-type-is-removed".to_string();
        cfg.client_info.version = "test".to_string();
        cfg
    }
}

type Client = rmcp::service::RunningService<rmcp::RoleClient, TestClient>;

/// A client of an in-memory design, served with the registry as built.
async fn connect() -> Client {
    let service = ReflowService::in_memory()
        .expect("in-memory service")
        .without_reaching_out();
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

/// The structured reply, or the refusal's words.
async fn call(c: &Client, tool: &str, args: Value) -> Result<Value, String> {
    let Value::Object(arguments) = args else {
        panic!("arguments for {tool} must be an object");
    };
    let answer = tokio::time::timeout(
        Duration::from_secs(30),
        c.call_tool(CallToolRequestParams::new(tool.to_string()).with_arguments(arguments)),
    )
    .await
    .unwrap_or_else(|_| panic!("{tool} did not answer within 30 s"));
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
        json!({"id": "cap:x", "name": "C", "description": "Records.", "satisfies": "req:x"}),
    )
    .await;
}

/// How many design nodes the design holds, by asking it.
async fn node_count(c: &Client) -> u64 {
    let report = ok(c, "graph_report", json!({})).await;
    report["design_nodes"]
        .as_u64()
        .unwrap_or_else(|| panic!("graph_report carries `design_nodes`: {report}"))
}

/// Whether `text` names the type `Anchor` as a word ("Anchored" is not it).
fn names_anchor(text: &str) -> bool {
    text.match_indices("Anchor").any(|(i, _)| {
        !text[i + "Anchor".len()..]
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
    })
}

/// 1. THE VOCABULARY NO LONGER HOLDS IT, and nothing is listed as going.
#[tokio::test]
async fn describe_schema_no_longer_holds_anchor_and_lists_nothing_going() {
    // The registry may hold later retirements (the two renamed tool stubs,
    // 2026-10-25); what this holds is that Anchor's entry left with it.
    assert!(
        !REGISTRY.iter().any(|d| d.item.names().contains(&"Anchor")),
        "a node type's registry entry leaves with the item: {REGISTRY:?}"
    );
    let c = connect().await;
    let all = ok(&c, "describe_schema", json!({"budget_chars": 10_000_000})).await;
    let types: Vec<&str> = all["node_types"]
        .as_array()
        .expect("node_types")
        .iter()
        .filter_map(|n| n["node_type"].as_str())
        .collect();
    assert_eq!(types.len(), 28, "29 node types less Anchor: {types:?}");
    assert!(!types.contains(&"Anchor"), "Anchor is gone: {types:?}");
    assert!(
        !all["deprecations"].to_string().contains("Anchor"),
        "Anchor is not listed as going: {}",
        all["deprecations"]
    );
    // Asked about it alone, it is a type this build does not know — the
    // answer every other unknown name gets, not a special case.
    let anchor = call(&c, "describe_schema", json!({"node_type": "Anchor"})).await;
    let unknown = call(&c, "describe_schema", json!({"node_type": "NoSuchType"})).await;
    assert_eq!(
        anchor.is_err(),
        unknown.is_err(),
        "Anchor is answered as an unknown type is: {anchor:?} / {unknown:?}"
    );
    if let Ok(a) = &anchor {
        assert!(a.get("deprecated").is_none(), "and carries no mark: {a}");
    }
}

/// 2. NO SERVED TEXT SAYS IT IS STILL COMING OR GOING.
#[tokio::test]
async fn no_served_text_says_anything_is_removed_in_the_next_release() {
    let c = connect().await;
    let tools = c.list_all_tools().await.expect("tools/list");
    let listed = serde_json::to_string(&tools).expect("tools serialize");
    let mut all = ok(&c, "describe_schema", json!({"budget_chars": 10_000_000})).await;
    // The registry may list TOOLS already removed (their stub release, the two
    // renamed stubs since 2026-10-25): those entries say "removed in the next
    // release" of a tool nothing serves. Every entry must be such a tool, and
    // the rest of the served text holds to the rule below.
    if let Some(going) = all.as_object_mut().and_then(|o| o.remove("deprecations")) {
        for d in going.as_array().into_iter().flatten() {
            let tool = d["item"]["tool"].as_str().unwrap_or_default();
            assert!(
                !tool.is_empty() && !tools.iter().any(|t| t.name == tool),
                "a deprecation listed beside the Anchor removal names a tool that is gone: {d}"
            );
        }
    }
    for (what, text) in [("tools/list", listed), ("describe_schema", all.to_string())] {
        assert!(
            !text.contains("removed in the next release"),
            "{what} says something is removed in the next release, and this release \
             removes nothing it still serves"
        );
        assert!(
            !text.contains("DEPRECATED in"),
            "{what} marks something deprecated with an empty registry"
        );
        assert!(
            !names_anchor(&text),
            "{what} still names the removed type `Anchor`"
        );
    }
}

/// 3. WRITING ONE IS REFUSED NAMING THE TYPE, and nothing is written.
#[tokio::test]
async fn creating_an_anchor_is_refused_naming_the_type() {
    let c = connect().await;
    seed(&c).await;
    let before = node_count(&c).await;
    let refused = call(
        &c,
        "create_node",
        json!({"node_type": "Anchor", "id": "anchor:p2-to-p3",
               "props": {"name": "Leaving P2", "checksum": "sha256:0123"}}),
    )
    .await
    .expect_err("an Anchor is no longer writable");
    assert!(
        refused.contains("Anchor"),
        "the refusal names the type: {refused}"
    );
    assert_eq!(node_count(&c).await, before, "nothing was written");
    assert!(
        call(&c, "get_node", json!({"id": "anchor:p2-to-p3"}))
            .await
            .map(|v| v["node"].is_null())
            .unwrap_or(true),
        "no Anchor is in the design"
    );
}

/// 4. AN EXPORT THAT HOLDS ONE IS REFUSED BY THE IMPORT, naming the type, and
/// nothing is written.
#[tokio::test]
async fn an_export_holding_an_anchor_is_refused_by_the_import_naming_it() {
    let c = connect().await;
    seed(&c).await;
    let mut doc = ok(&c, "export_graph", json!({})).await;
    if let Some(inner) = doc.get("document").cloned() {
        doc = inner;
    }
    assert!(doc["nodes"].is_array(), "an export carries `nodes`: {doc}");
    let nodes = doc["nodes"].as_array_mut().expect("checked above");
    let mut anchor = nodes[0].clone();
    anchor["node_type"] = json!("Anchor");
    anchor["node_id"] = json!("anchor:p2-to-p3");
    anchor["properties"] = json!({"name": "Leaving P2", "checksum": "sha256:0123"});
    nodes.push(anchor);

    let fresh = connect().await;
    let refused = call(&fresh, "import_graph", json!({"document": doc}))
        .await
        .expect_err("an export holding an Anchor does not import");
    assert!(
        refused.contains("Anchor"),
        "the import's refusal names the type: {refused}"
    );
    assert!(
        refused.contains("was retired from the schema")
            && refused.contains("dec:anchor-retire-or-implement"),
        "and says it was retired, by which decision, rather than leaving it an unknown \
         name: {refused}"
    );
    let scanned = ok(&fresh, "scan_nodes", json!({"node_type": "Requirement"})).await;
    assert_eq!(
        scanned["total"], 0,
        "the import is all or nothing: not even the export's other nodes were written"
    );
}

/// The hint describe_schema serves for one node type, whole: the budget is
/// raised because a type every `*` edge reaches (DriftEvent) answers past the
/// default one, and its prose is the first thing withheld.
async fn hint(c: &Client, node_type: &str) -> String {
    let one = ok(
        c,
        "describe_schema",
        json!({"node_type": node_type, "budget_chars": 10_000_000}),
    )
    .await;
    one["hint"]
        .as_str()
        .unwrap_or_else(|| panic!("{node_type} has a hint: {one}"))
        .to_string()
}

/// 5. DRIFTEVENT'S HINT says what a DriftEvent references, and drift detection
/// draws exactly that.
#[tokio::test]
async fn driftevents_hint_says_what_drift_detection_draws() {
    let c = connect().await;
    let drift = hint(&c, "DriftEvent").await;
    assert!(
        !drift.contains("Anchor"),
        "DriftEvent's hint names no Anchor: {drift}"
    );
    assert!(
        drift.contains("DEPENDS_ON") && drift.contains("Artifact"),
        "it says what a DriftEvent does reference: {drift}"
    );

    seed(&c).await;
    ok(
        &c,
        "add_artifact",
        json!({"id": "art:a", "name": "a", "location": "src/a.rs", "checksum": "abc"}),
    )
    .await;
    ok(
        &c,
        "reconcile_artifacts",
        json!({"observed": [{"artifact_id": "art:a", "present": true, "checksum": "abd"}],
               "record_events": true, "detected_at": "2026-10-17"}),
    )
    .await;
    let events = ok(&c, "scan_nodes", json!({"node_type": "DriftEvent"})).await;
    let id = events["items"][0]["node_id"]
        .as_str()
        .unwrap_or_else(|| panic!("the reconcile recorded a DriftEvent: {events}"))
        .to_string();
    let event = ok(&c, "get_node", json!({"id": id, "include_edges": true})).await;
    let edges: Vec<(String, String)> = event["edges"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e["edge_type"].as_str().unwrap_or("").to_string(),
                e["other"]["node_type"].as_str().unwrap_or("").to_string(),
            )
        })
        .collect();
    assert!(
        edges.contains(&("DEPENDS_ON".to_string(), "Artifact".to_string())),
        "the event DEPENDS_ON the drifted Artifact: {edges:?}"
    );
}
