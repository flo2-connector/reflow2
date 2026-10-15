//! THE UNUSED `Anchor` NODE TYPE IS ANNOUNCED FOR REMOVAL — the first real
//! entry in `deprecation::REGISTRY`, seen the way a client of the MCP server
//! sees it.
//!
//! Anthony's ruling, `dec:anchor-retire-or-implement` (accepted 2026-10-13):
//! "Remove the unused Anchor type, but only after a rule for retiring things
//! exists, because it would be the first thing ever taken off what Alex's MCP
//! sees." The rule exists (`cap:a-served-item-is-removed-only-after-a-release-that-announced-it`,
//! #751): a DEPRECATING release in which the item still works and says it is
//! going, then the NEXT release removes it. This is the deprecating half.
//!
//! What is pinned, every call over the protocol through an in-process duplex,
//! with the registry AS BUILT (no test-only entries):
//!
//! 1. describe_schema marks `Anchor` deprecated with what to do, lists it
//!    under `deprecations` with the decision that retires it (the listed item
//!    as the registry holds it, not marked as if it were the vocabulary), and
//!    the what-to-do names tools this build serves.
//! 2. Creating an Anchor still works, exactly as before but for the notice
//!    its reply carries — and reading it back carries the notice too.
//! 3. The two extraction hints the decision says must move have moved:
//!    DriftEvent's no longer claims it references an Anchor, and says what it
//!    does reference — which is what drift detection draws; Anchor's own no
//!    longer claims DriftEvents reference it.
//! 4. A call that never touches Anchor carries nothing new, so the notice is
//!    news and not noise.
//!
//! OBSERVED FAILING FIRST, 2026-10-14: this file, run against main at
//! 959436d80 (registry empty, hints unchanged), failed all four tests — no
//! `deprecated` on Anchor in describe_schema, no notice on its create
//! (`left: []`), DriftEvent's hint still saying it "References the Anchor it
//! diverged from via DEPENDS_ON", and the noise check, which asks its question
//! of the registry as built, finding no Anchor entry to ask it of.
//!
//! AND ONE THING THE FIRST REAL ENTRY SHOWED, which #751's test-only entries
//! did not: describe_schema's full answer marked its own `deprecations`
//! listing, because the listing went in before the mark was applied to the
//! whole reply, and the listed `item` holds a `node_type`. So
//! `deprecations[0].item` carried a `deprecated` copy of the mark. The
//! assertion on the listed item failed on 3a24c4daa and passes once the
//! listing goes in after the mark (tools/query.rs).

use reflow2_mcp::deprecation::{Item, REGISTRY};
use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};
use std::time::Duration;

/// The decision that retires the type.
const DECISION: &str = "dec:anchor-retire-or-implement";

/// The release that deprecates it: the next after 0.84.0, as #751's own
/// tests and the mark's documentation number it.
const DEPRECATED_IN: &str = "0.85.0";

struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "the-unused-anchor-type-is-announced-for-removal".to_string();
        cfg.client_info.version = "test".to_string();
        cfg
    }
}

type Client = rmcp::service::RunningService<rmcp::RoleClient, TestClient>;

/// A client of an in-memory design, served with the registry as built — or,
/// with `plain`, with nothing deprecated, which is what the last release
/// served.
async fn connect(plain: bool) -> Client {
    let mut service = ReflowService::in_memory()
        .expect("in-memory service")
        .without_reaching_out();
    if plain {
        service = service.with_deprecations_for_test(Vec::new());
    }
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

/// The registry's own entry for the type, which everything below compares
/// against so the sentence is written once.
fn the_entry() -> &'static reflow2_mcp::deprecation::Deprecation {
    REGISTRY
        .iter()
        .find(|d| matches!(&d.item, Item::NodeType { node_type } if node_type == "Anchor"))
        .expect("deprecation::REGISTRY deprecates the Anchor node type")
}

/// The items a reply's notice names, as `kind:name`.
fn noticed(reply: &Value) -> Vec<String> {
    reply["deprecation_notice"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|n| {
                    let item = &n["item"];
                    let name = ["tool", "node_type", "edge_type"]
                        .iter()
                        .find_map(|k| item[*k].as_str())
                        .unwrap_or("?");
                    format!("{}:{name}", item["kind"].as_str().unwrap_or("?"))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 1. DESCRIBE_SCHEMA MARKS ANCHOR DEPRECATED, with what to do, and lists it
/// under `deprecations` with the decision that retires it.
#[tokio::test]
async fn describe_schema_marks_anchor_deprecated_with_what_to_do() {
    let c = connect(false).await;
    let all = ok(&c, "describe_schema", json!({"budget_chars": 10_000_000})).await;
    let anchor = all["node_types"]
        .as_array()
        .expect("node_types")
        .iter()
        .find(|n| n["node_type"] == "Anchor")
        .cloned()
        .expect("Anchor is still in the vocabulary: deprecating is not removing");
    let mark = anchor["deprecated"].as_str().unwrap_or("");
    let head = format!("DEPRECATED in {DEPRECATED_IN}, removed in the next release: ");
    assert!(
        mark.starts_with(&head),
        "the node type carries the mark: {anchor}"
    );

    let entry = the_entry();
    assert_eq!(entry.deprecated_in, DEPRECATED_IN);
    assert_eq!(entry.decision, DECISION);
    assert_eq!(mark, format!("{head}{}", entry.what_to_do));
    // The what-to-do names the tools that do the job today. A sentence naming
    // a tool this build does not serve would send a client nowhere.
    let served: Vec<String> = c
        .list_all_tools()
        .await
        .expect("tools/list")
        .iter()
        .map(|t| t.name.to_string())
        .collect();
    for tool in ["add_epoch", "snapshot_before_change"] {
        assert!(
            entry.what_to_do.contains(&format!("`{tool}`")),
            "what to do names `{tool}`: {}",
            entry.what_to_do
        );
        assert!(served.contains(&tool.to_string()), "`{tool}` is served");
    }

    let listed = all["deprecations"]
        .as_array()
        .expect("the full answer lists what is going");
    let it = listed
        .iter()
        .find(|d| d["item"]["kind"] == "node_type" && d["item"]["node_type"] == "Anchor")
        .unwrap_or_else(|| panic!("Anchor is listed under `deprecations`: {listed:?}"));
    assert_eq!(it["deprecated_in"], DEPRECATED_IN);
    assert_eq!(it["decision"], DECISION);
    assert_eq!(it["what_to_do"], entry.what_to_do.as_ref());
    assert_eq!(it["what"], "the node type `Anchor`");
    // The listing names the item and nothing else. Its `item` holds a
    // `node_type`, so a mark applied to the whole answer after the listing
    // went in would mark the listing's own entry as if it were the vocabulary.
    assert_eq!(
        it["item"],
        json!({"kind": "node_type", "node_type": "Anchor"}),
        "the listed item is the registry's item, unmarked: {it}"
    );

    let one = ok(&c, "describe_schema", json!({"node_type": "Anchor"})).await;
    assert_eq!(
        one["deprecated"].as_str(),
        Some(mark),
        "asked about the type alone, the answer opens marked: {one}"
    );
    assert!(
        all["node_types"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| n.get("deprecated").is_some())
            .count()
            == 1,
        "Anchor is the only node type marked"
    );
}

/// 2. CREATING AN ANCHOR STILL WORKS, answered exactly as the last release
/// answered it but for the notice — and reading it back carries the notice.
#[tokio::test]
async fn creating_an_anchor_still_works_and_its_reply_says_it_is_going() {
    let marked = connect(false).await;
    let plain = connect(true).await;
    let args = json!({"node_type": "Anchor", "id": "anchor:p2-to-p3",
                      "props": {"name": "Leaving P2", "checksum": "sha256:0123",
                                "design_intent": "What the build round is for."}});
    let before = ok(&plain, "create_node", args.clone()).await;
    let mut after = ok(&marked, "create_node", args).await;
    assert_eq!(
        noticed(&after),
        vec!["node_type:Anchor".to_string()],
        "the create carries the notice: {after}"
    );
    let notice = &after["deprecation_notice"][0];
    let entry = the_entry();
    assert_eq!(notice["what_to_do"], entry.what_to_do.as_ref());
    assert_eq!(notice["decision"], DECISION);
    assert_eq!(notice["deprecated_in"], DEPRECATED_IN);
    assert_eq!(
        notice["says"],
        format!(
            "the node type `Anchor` is DEPRECATED in {DEPRECATED_IN}, removed in the next \
             release: {}",
            entry.what_to_do
        )
    );
    after.as_object_mut().unwrap().remove("deprecation_notice");
    assert_eq!(
        after, before,
        "a deprecated type is written exactly as before, the notice aside"
    );

    let read = ok(&marked, "get_node", json!({"id": "anchor:p2-to-p3"})).await;
    assert_eq!(read["node"]["properties"]["checksum"], "sha256:0123");
    assert_eq!(noticed(&read), vec!["node_type:Anchor".to_string()]);
    let listed = ok(&marked, "scan_nodes", json!({"node_type": "Anchor"})).await;
    assert_eq!(listed["total"], 1);
    assert_eq!(noticed(&listed), vec!["node_type:Anchor".to_string()]);
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

/// 3. THE TWO HINTS HAVE MOVED. DriftEvent's says what a DriftEvent
/// references — and drift detection draws exactly that — and neither hint
/// describes a link between the two types, which no code makes.
#[tokio::test]
async fn the_two_hints_no_longer_describe_a_link_nothing_makes() {
    let c = connect(false).await;
    let drift = hint(&c, "DriftEvent").await;
    assert!(
        !drift.contains("Anchor"),
        "DriftEvent's hint no longer names the Anchor: {drift}"
    );
    assert!(
        drift.contains("DEPENDS_ON") && drift.contains("Artifact"),
        "it says what a DriftEvent does reference: {drift}"
    );
    let anchor = hint(&c, "Anchor").await;
    assert!(
        !anchor.contains("DriftEvent"),
        "Anchor's hint no longer claims a DriftEvent references it: {anchor}"
    );

    // What the new hint says is what drift detection draws: a reconcile that
    // records its finding links the event to the drifted Artifact.
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
               "record_events": true, "detected_at": "2026-10-14"}),
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

/// 4. A CALL THAT NEVER TOUCHES ANCHOR CARRIES NOTHING NEW — the reads a
/// session makes most, on an ordinary design.
#[tokio::test]
async fn a_call_that_never_touches_anchor_carries_no_notice() {
    let entry = the_entry();
    assert_eq!(entry.decision, DECISION);
    let c = connect(false).await;
    seed(&c).await;
    for (tool, args) in [
        ("loop_status", json!({})),
        ("graph_report", json!({})),
        ("detect_gaps", json!({})),
        ("scan_nodes", json!({"node_type": "Requirement"})),
        ("get_node", json!({"id": "cap:x", "include_edges": true})),
        ("describe_schema", json!({"node_type": "Requirement"})),
        ("describe_schema", json!({})),
        ("search_design", json!({"query": "records"})),
        (
            "find_tools",
            json!({"query": "record a baseline of the design"}),
        ),
    ] {
        let r = ok(&c, tool, args.clone()).await;
        assert!(
            r.get("deprecation_notice").is_none(),
            "{tool} {args} uses nothing that is going, and says nothing about it: {}",
            r["deprecation_notice"]
        );
    }
}
