//! A SERVED ITEM IS REMOVED ONLY AFTER A RELEASE THAT ANNOUNCED IT — what a
//! client of the MCP server sees while something is going.
//!
//! The rule, amended on Anthony's word on 2026-10-13
//! (`req:the-mcp-server-keeps-working-as-is-and-door-improvements-are-additive-or-opt-in`):
//! "never removed" becomes "never removed WITHOUT A RELEASE OF NOTICE".
//! Anything a client can call or read is retired in two releases: a
//! DEPRECATING release, in which it still works, says it is going (in its tool
//! description, or in describe_schema for a schema item) and the changelog
//! carries a "What to do" line; then the NEXT release removes it. For a tool,
//! the removal ships as a refusing stub under the old name that names what to
//! do instead
//! (`dec:a-tool-is-never-deleted-without-usage-evidence-across-projects-and-never-without-a-refusing-stub-first`).
//!
//! `crate::deprecation::REGISTRY` is the one place a deprecation is declared;
//! its first real entry, the `Anchor` node type, was announced in 0.85.0 and
//! removed in 0.86.0 (`the_unused_anchor_type_is_removed.rs`), and the node
//! type the entries below deprecate for the test is `Actor`, which this build
//! still serves. Every entry here is
//! test-only, handed to an in-memory service through
//! `with_deprecations_for_test`, and every call goes over the protocol (an
//! in-process duplex), through `call_tool` and `list_tools` as a client's does.
//!
//! What is pinned:
//!
//! 1. A deprecated tool still works, its answer unchanged but for the notice,
//!    and its description opens with the mark.
//! 2. A deprecated argument's description opens with the mark; a deprecated
//!    reply field is named in its tool's description.
//! 3. describe_schema marks a deprecated node type, property and edge type,
//!    and its full answer lists every deprecation with what to do.
//! 4. A call that uses a deprecated item — the tool, an argument it passes, a
//!    node type it names or reads back, a reply field it gets — carries the
//!    notice; a call that uses none carries nothing new.
//! 5. A tool removed after its release of notice answers its old name with a
//!    refusal saying what to do.
//! 6. With the registry as built, tools/list is the reviewed toolsnap goldens,
//!    tool for tool. A node type's entry marks describe_schema, not tools/list,
//!    so a node type's entry would leave every golden as it was.
//! 7. The registry names only what this build serves.

use reflow2_mcp::deprecation::{Deprecation, Item, REGISTRY};
use reflow2_mcp::service::{DescribeSchemaReq, ReflowService};
use rmcp::ServiceExt;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};
use std::time::Duration;

/// The decision every test-only entry names.
const DEC: &str = "dec:test-only-a-retirement";
const MARK: &str = "DEPRECATED in 0.99.0, removed in the next release:";

fn entries() -> Vec<Deprecation> {
    vec![
        Deprecation::tool(
            "get_node",
            "0.99.0",
            "Read the node with scan_nodes instead.",
            DEC,
        ),
        Deprecation::argument(
            "scan_nodes",
            "brief",
            "0.99.0",
            "Pass `limit` and read the fields you need.",
            DEC,
        ),
        Deprecation::reply_field(
            "loop_status",
            "clean",
            "0.99.0",
            "Read `next`: an empty list means nothing is owed.",
            DEC,
        ),
        Deprecation::node_type(
            "Actor",
            "0.99.0",
            "Record a Snapshot, which a cut already takes.",
            DEC,
        ),
        Deprecation::node_property(
            "Requirement",
            "kind",
            "0.99.0",
            "Say the kind in the statement.",
            DEC,
        ),
        Deprecation::edge_type(
            "ANTICIPATES",
            "0.99.0",
            "Draw ENABLES from the earlier work instead.",
            DEC,
        ),
        // A tool removed after its release of notice: nothing serves it.
        Deprecation::tool("add_anchor", "0.99.0", "Call add_snapshot instead.", DEC),
    ]
}

struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "a-served-item-is-removed-only-after-notice".to_string();
        cfg.client_info.version = "test".to_string();
        cfg
    }
}

type Client = rmcp::service::RunningService<rmcp::RoleClient, TestClient>;

/// A client of an in-memory design, served with `deprecations` when given
/// and with the registry as built otherwise.
async fn connect(deprecations: Option<Vec<Deprecation>>) -> Client {
    let mut service = ReflowService::in_memory()
        .expect("in-memory service")
        .without_reaching_out();
    if let Some(d) = deprecations {
        service = service.with_deprecations_for_test(d);
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
        json!({"id": "req:x", "name": "R", "statement": "The system records what it is told.",
               "kind": "functional"}),
    )
    .await;
}

async fn listed(c: &Client) -> Vec<rmcp::model::Tool> {
    c.list_all_tools().await.expect("tools/list")
}

fn description_of(tools: &[rmcp::model::Tool], name: &str) -> String {
    tools
        .iter()
        .find(|t| t.name == name)
        .and_then(|t| t.description.as_deref().map(str::to_string))
        .unwrap_or_else(|| panic!("`{name}` is not listed"))
}

/// The items a reply's notice names, as `kind:name` strings.
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
                    let part = ["path", "property"]
                        .iter()
                        .find_map(|k| item[*k].as_str())
                        .map(|p| format!(".{p}"))
                        .unwrap_or_default();
                    format!("{}:{name}{part}", item["kind"].as_str().unwrap_or("?"))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 1. A DEPRECATED TOOL STILL WORKS, its answer the same but for the notice,
/// and its description opens with the mark.
#[tokio::test]
async fn a_deprecated_tool_still_works_and_its_description_says_it_is_going() {
    let plain = connect(Some(Vec::new())).await;
    let marked = connect(Some(entries())).await;
    seed(&plain).await;
    seed(&marked).await;

    let before = ok(&plain, "get_node", json!({"id": "proj:x"})).await;
    let mut after = ok(&marked, "get_node", json!({"id": "proj:x"})).await;
    assert_eq!(
        noticed(&after),
        vec!["tool:get_node".to_string()],
        "the call carries the notice: {after}"
    );
    let notice = after["deprecation_notice"][0].clone();
    assert!(
        notice["says"].as_str().unwrap_or("").contains(MARK)
            && notice["what_to_do"] == "Read the node with scan_nodes instead."
            && notice["decision"] == DEC
            && notice["deprecated_in"] == "0.99.0",
        "the notice says what is going, when, what to do, and who decided: {notice}"
    );
    after.as_object_mut().unwrap().remove("deprecation_notice");
    assert_eq!(
        after, before,
        "a deprecated tool answers exactly as before, the notice aside"
    );

    let tools = listed(&marked).await;
    let get_node = description_of(&tools, "get_node");
    assert!(
        get_node.starts_with(&format!("{MARK} Read the node with scan_nodes instead.")),
        "the mark opens the description: {get_node}"
    );
    let unmarked = description_of(&listed(&plain).await, "get_node");
    assert!(
        get_node.ends_with(&unmarked),
        "the tool's own description follows the mark, unchanged"
    );
    assert!(
        !description_of(&tools, "add_project").contains("DEPRECATED"),
        "a tool nothing deprecates is listed as before"
    );
}

/// 2. A deprecated ARGUMENT says so in its own description, and a deprecated
/// REPLY FIELD is named in its tool's description.
#[tokio::test]
async fn a_deprecated_argument_and_reply_field_are_marked_where_a_client_reads_them() {
    let c = connect(Some(entries())).await;
    let tools = listed(&c).await;
    let scan = tools.iter().find(|t| t.name == "scan_nodes").unwrap();
    let brief = scan.input_schema["properties"]["brief"]["description"]
        .as_str()
        .unwrap_or("");
    assert!(
        brief.starts_with(&format!(
            "{MARK} Pass `limit` and read the fields you need."
        )),
        "the argument's description opens with the mark: {brief}"
    );
    let limit = scan.input_schema["properties"]["limit"]["description"]
        .as_str()
        .unwrap_or("");
    assert!(
        !limit.contains("DEPRECATED"),
        "its sibling is untouched: {limit}"
    );
    let loop_status = description_of(&tools, "loop_status");
    assert!(
        loop_status.contains("`clean`") && loop_status.contains(MARK),
        "the reply field is named, with the mark, in its tool's description: {loop_status}"
    );
}

/// 3. DESCRIBE_SCHEMA MARKS a deprecated node type, property and edge type,
/// and its full answer lists every deprecation with what to do.
#[tokio::test]
async fn describe_schema_marks_a_deprecated_node_type_property_and_edge_type() {
    let c = connect(Some(entries())).await;
    seed(&c).await;
    let all = ok(&c, "describe_schema", json!({"budget_chars": 10_000_000})).await;
    let node = |t: &str| {
        all["node_types"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["node_type"] == t)
            .cloned()
            .unwrap_or_else(|| panic!("{t} is not in the vocabulary"))
    };
    let actor = node("Actor");
    assert!(
        actor["deprecated"]
            .as_str()
            .is_some_and(|s| s.starts_with(MARK) && s.contains("Record a Snapshot")),
        "the node type is marked: {actor}"
    );
    assert!(
        node("Snapshot").get("deprecated").is_none(),
        "a type nothing deprecates is not"
    );
    let req = node("Requirement");
    let prop = |name: &str| {
        req["properties"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == name)
            .cloned()
            .unwrap()
    };
    assert!(
        prop("kind")["deprecated"]
            .as_str()
            .is_some_and(|s| s.starts_with(MARK)),
        "the property is marked: {}",
        prop("kind")
    );
    assert!(prop("statement").get("deprecated").is_none());
    let edge = all["edge_types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["edge_type"] == "ANTICIPATES")
        .cloned()
        .unwrap();
    assert!(
        edge["deprecated"]
            .as_str()
            .is_some_and(|s| s.starts_with(MARK)),
        "the edge type is marked: {edge}"
    );
    let listing = all["deprecations"]
        .as_array()
        .expect("the full answer lists them");
    assert_eq!(listing.len(), entries().len());
    assert!(
        listing.iter().all(|d| d["what_to_do"].as_str().is_some()
            && d["deprecated_in"] == "0.99.0"
            && d["decision"] == DEC
            && d["item"]["kind"].as_str().is_some()),
        "each listed with its item, version, what to do and decision: {listing:?}"
    );

    let one = ok(&c, "describe_schema", json!({"node_type": "Actor"})).await;
    assert!(
        one["deprecated"]
            .as_str()
            .is_some_and(|s| s.starts_with(MARK)),
        "focused on the type, the answer is marked at the top: {one}"
    );

    // With nothing deprecated, the answer carries no new key at all.
    let plain = connect(Some(Vec::new())).await;
    seed(&plain).await;
    let bare = ok(
        &plain,
        "describe_schema",
        json!({"budget_chars": 10_000_000}),
    )
    .await;
    assert!(bare.get("deprecations").is_none());
    assert!(
        !bare.to_string().contains("\"deprecated\""),
        "an empty registry marks nothing"
    );
}

/// 4. A CALL THAT USES A DEPRECATED ITEM CARRIES THE NOTICE: the argument it
/// passes, the node type it names or reads back, the property it writes, the
/// edge type it draws, the reply field it gets. A call that uses none carries
/// nothing new.
#[tokio::test]
async fn a_call_that_uses_a_deprecated_item_gets_the_notice() {
    let c = connect(Some(entries())).await;
    seed(&c).await;

    let r = ok(
        &c,
        "scan_nodes",
        json!({"node_type": "Project", "brief": true}),
    )
    .await;
    assert_eq!(noticed(&r), vec!["argument:scan_nodes.brief".to_string()]);
    let r = ok(&c, "scan_nodes", json!({"node_type": "Project"})).await;
    assert!(
        r.get("deprecation_notice").is_none(),
        "not passing the argument is not using it: {r}"
    );

    let r = ok(
        &c,
        "create_node",
        json!({"node_type": "Actor", "id": "actor:a", "props": {"name": "A"}}),
    )
    .await;
    assert_eq!(noticed(&r), vec!["node_type:Actor".to_string()]);
    let r = ok(&c, "scan_nodes", json!({"node_type": "Actor"})).await;
    assert_eq!(noticed(&r), vec!["node_type:Actor".to_string()]);

    // A property is used by writing it through the type's own tool, and by
    // reading it back on a node of that type.
    let r = ok(
        &c,
        "add_requirement",
        json!({"id": "req:y", "name": "Y", "statement": "S", "kind": "functional"}),
    )
    .await;
    assert!(
        noticed(&r).contains(&"node_property:Requirement.kind".to_string()),
        "{r}"
    );
    let r = ok(&c, "scan_nodes", json!({"node_type": "Requirement"})).await;
    assert!(
        noticed(&r).contains(&"node_property:Requirement.kind".to_string()),
        "{r}"
    );

    ok(
        &c,
        "add_capability",
        json!({"id": "cap:x", "name": "C", "description": "Records."}),
    )
    .await;
    let r = ok(
        &c,
        "create_edge",
        json!({"edge_type": "ANTICIPATES", "from_id": "req:x", "to_id": "cap:x"}),
    )
    .await;
    assert_eq!(noticed(&r), vec!["edge_type:ANTICIPATES".to_string()]);

    let r = ok(&c, "loop_status", json!({})).await;
    assert_eq!(
        noticed(&r),
        vec!["reply_field:loop_status.clean".to_string()]
    );

    let r = ok(&c, "add_project", json!({"id": "proj:y", "name": "Y"})).await;
    assert!(
        r.get("deprecation_notice").is_none(),
        "a call that uses nothing deprecated carries nothing new: {r}"
    );
}

/// 5. A TOOL REMOVED AFTER ITS RELEASE OF NOTICE answers its old name with a
/// refusal that says what to do — never a silent absence.
#[tokio::test]
async fn a_removed_tool_answers_its_old_name_with_what_to_do() {
    let c = connect(Some(entries())).await;
    let why = call(&c, "add_anchor", json!({}))
        .await
        .expect_err("a removed tool is refused");
    assert!(
        why.contains("tool not found")
            && why.contains("DEPRECATED in 0.99.0")
            && why.contains("Call add_snapshot instead."),
        "the refusal keeps rmcp's words first and says what to do: {why}"
    );
    assert!(
        !listed(&c).await.iter().any(|t| t.name == "add_anchor"),
        "the stub is a refusal, not a listed tool"
    );
    let plain = connect(Some(Vec::new())).await;
    let unknown = call(&plain, "add_anchor", json!({})).await.unwrap_err();
    assert!(
        !unknown.contains("DEPRECATED"),
        "a name nobody deprecated is refused as before: {unknown}"
    );
}

/// 6. THE REGISTRY AS BUILT leaves tools/list exactly as reviewed: every tool
/// equals its committed toolsnap golden. In the change that built the rule the
/// registry is empty and the goldens are main's, so this is main's surface,
/// byte for byte (`tools/toolsnap.py` checks the same over a binary's stdio).
#[tokio::test]
async fn the_registry_as_built_leaves_tools_list_as_reviewed() {
    let c = connect(None).await;
    let tools = listed(&c).await;
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/toolsnaps");
    let goldens = std::fs::read_dir(dir)
        .expect("the toolsnap goldens")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .count();
    assert_eq!(tools.len(), goldens, "one golden per listed tool");
    let mut differ = Vec::new();
    for t in &tools {
        let path = format!("{dir}/{}.json", t.name);
        let golden: Value = serde_json::from_str(
            &std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{path} is committed")),
        )
        .expect("a golden is JSON");
        if serde_json::to_value(t).unwrap() != golden {
            differ.push(t.name.to_string());
        }
    }
    assert!(
        differ.is_empty(),
        "tools/list differs from the reviewed goldens for: {differ:?}"
    );
    let r = {
        seed(&c).await;
        ok(&c, "loop_status", json!({})).await
    };
    if REGISTRY.is_empty() {
        assert!(
            r.get("deprecation_notice").is_none(),
            "an empty registry adds nothing to a reply"
        );
    }
}

/// 7. THE REGISTRY NAMES ONLY WHAT THIS BUILD SERVES — a typo would mark
/// nothing and announce something that is not there.
#[tokio::test]
async fn the_registry_names_only_what_this_build_serves() {
    let s = ReflowService::in_memory().expect("service");
    let vocabulary = s
        .describe_schema(Parameters(DescribeSchemaReq {
            budget_chars: Some(10_000_000),
            node_type: None,
            from: None,
            to: None,
            required_only: false,
            tool: None,
            full: false,
        }))
        .await
        .expect("describe_schema")
        .structured_content
        .expect("structured");
    let served = ReflowService::served_tools();
    let problems = reflow2_mcp::deprecation::validate(REGISTRY, &served, &vocabulary);
    assert!(problems.is_empty(), "the registry: {problems:#?}");
    assert!(
        reflow2_mcp::deprecation::validate(&entries(), &served, &vocabulary).is_empty(),
        "the test entries are valid: {:#?}",
        reflow2_mcp::deprecation::validate(&entries(), &served, &vocabulary)
    );

    let bad = vec![
        Deprecation::argument("scan_nodes", "brif", "0.99.0", "Do X.", DEC),
        Deprecation::node_type("Anchr", "0.99.0", "Do X.", DEC),
        Deprecation::node_property("Requirement", "knd", "0.99.0", "Do X.", DEC),
        Deprecation::edge_type("ANTICIPATE", "0.99.0", "Do X.", DEC),
        Deprecation::argument("add_anchor", "x", "0.99.0", "Do X.", DEC),
        Deprecation::tool("get_node", "next", "Do X.", DEC),
        Deprecation::tool("scan_nodes", "0.99.0", "", DEC),
        Deprecation::tool("loop_status", "0.99.0", "Do X.", "req:not-a-decision"),
    ];
    let problems = reflow2_mcp::deprecation::validate(&bad, &served, &vocabulary);
    for (i, d) in bad.iter().enumerate() {
        let named = match &d.item {
            Item::Tool { tool } => tool.to_string(),
            Item::Argument { path, .. } | Item::ReplyField { path, .. } => path.to_string(),
            Item::NodeType { node_type } => node_type.to_string(),
            Item::EdgeType { edge_type } => edge_type.to_string(),
            Item::NodeProperty { property, .. } => property.to_string(),
        };
        assert!(
            problems.iter().any(|p| p.contains(&format!("entry {i}"))),
            "entry {i} (`{named}`) is invalid and was not caught: {problems:#?}"
        );
    }
}
