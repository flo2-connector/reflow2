//! EVERY WRITER THAT CAN SET A NODE'S SETTLING VALUE GOES THROUGH ONE RULE.
//!
//! #628 declared what settles intent on each TYPED tool, and the generic
//! writers went round it: `create_node`, `create_nodes`, `import_graph` and
//! `apply_merge` wrote an accepted Decision, an accepted Requirement or an
//! enforced DesignRule with nobody's name and no note
//! (fact:root-cause-the-settle-rule-guards-the-typed-doors-and-the-generic-writers-go-around-it-2026-09-29).
//!
//! Membership is read off the SERVED surface: a write tool that takes a node
//! type with a property bag, a list of such items, a whole document, or a
//! merge's `theirs` is a node writer and must be driven here — an unknown one
//! fails, so a fifth door cannot open unnoticed. Each is driven four ways:
//! an unsigned settle (refused by a constructor, written and NAMED by an
//! import or merge), a signed one (written, signature drawn, nothing said), a
//! non-settling write (silent), and a re-write of a value already settled
//! (not a new settle).

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};

const BOSS: &str = "who:boss";

struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "every-writer-settles-through-one-rule".to_string();
        cfg.client_info.version = "test".to_string();
        cfg
    }
}

type Client = rmcp::service::RunningService<rmcp::RoleClient, TestClient>;

async fn connect() -> Client {
    let service = ReflowService::in_memory().expect("in-memory service");
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

/// `Ok(structured reply)` or `Err(refusal text)`.
async fn call(c: &Client, tool: &str, args: Value) -> Result<Value, String> {
    let Value::Object(arguments) = args else {
        panic!("arguments for {tool} must be an object");
    };
    match c
        .call_tool(CallToolRequestParams::new(tool.to_string()).with_arguments(arguments))
        .await
    {
        Ok(r) if r.is_error == Some(true) => Err(format!("{:?}", r.content)),
        Ok(r) => Ok(r.structured_content.unwrap_or(Value::Null)),
        Err(e) => Err(e.to_string()),
    }
}

async fn writers(c: &Client) -> Vec<String> {
    let tools = c.list_all_tools().await.expect("tools/list");
    let mut out = Vec::new();
    for t in tools {
        let read_only = t
            .annotations
            .as_ref()
            .and_then(|a| a.read_only_hint)
            .unwrap_or(false);
        if read_only {
            continue;
        }
        let schema = serde_json::to_value(&t.input_schema).expect("schema");
        let props = &schema["properties"];
        let item_takes_node = |p: &Value| {
            let items = &p["items"];
            let ip = items["properties"].as_object().or_else(|| {
                let key = items["$ref"].as_str()?.rsplit('/').next()?;
                schema["$defs"][key]["properties"]
                    .as_object()
                    .or_else(|| schema["definitions"][key]["properties"].as_object())
            });
            ip.is_some_and(|ip| ip.contains_key("node_type") && ip.contains_key("props"))
        };
        let node_bag = !props["node_type"].is_null() && !props["props"].is_null();
        let node_list = props
            .as_object()
            .is_some_and(|m| m.values().any(item_takes_node));
        let document = !props["document"].is_null();
        let merge = !props["theirs_path"].is_null();
        // A text editor that can reach any string field — a Requirement's
        // `status` is one (found on 0.74.0 by the flo2 hotfix agent).
        let text_edit =
            !props["node_id"].is_null() && !props["field"].is_null() && !props["new"].is_null();
        if node_bag || node_list || document || merge || text_edit {
            out.push(t.name.to_string());
        }
    }
    out.sort();
    out
}

/// Does `id` carry an AUTHORED_BY whose roles hold `approver`? Read from the
/// export, the record every gate reads.
async fn signed_by(c: &Client, id: &str) -> bool {
    let export = call(c, "export_graph", json!({}))
        .await
        .expect("export_graph");
    export["edges"].as_array().is_some_and(|edges| {
        edges.iter().any(|e| {
            e["edge_type"] == "AUTHORED_BY"
                && e["from_id"] == id
                && e["properties"].to_string().contains("approver")
        })
    })
}

fn doc(nodes: Value, edges: Value) -> Value {
    json!({"nodes": nodes, "edges": edges})
}

fn decision(id: &str, status: &str) -> Value {
    json!({"node_type": "Decision", "node_id": id, "properties": {"name": format!("Decide {id}"), "decision": "a fixture choice", "kind": "choice", "status": status}})
}

/// Drive one writer; returns the problems found.
async fn drive(c: &Client, tool: &str, n: &mut usize, dir: &std::path::Path) -> Vec<String> {
    let mut p = Vec::new();
    let mut id = |stem: &str| {
        *n += 1;
        format!("dec:{stem}-{}", *n)
    };
    match tool {
        "create_node" | "create_nodes" => {
            let one = |id: &str, status: &str, approver: Option<&str>| {
                let mut item = json!({"node_type": "Decision", "id": id, "props": {"name": format!("Decide {id}"), "decision": "a fixture choice", "kind": "choice", "status": status}});
                if let Some(a) = approver {
                    item["approver"] = json!(a);
                }
                if tool == "create_node" {
                    item
                } else {
                    json!({"nodes": [item]})
                }
            };
            let unsigned = id("unsigned");
            if let Ok(r) = call(c, tool, one(&unsigned, "accepted", None)).await {
                p.push(format!("{tool} RECORDED an unsigned settle: {r}"));
            }
            if call(c, "get_node", json!({"id": unsigned}))
                .await
                .is_ok_and(|r| !r["node"].is_null())
            {
                p.push(format!("{tool}: a refused settle still left a node behind"));
            }
            let signed = id("signed");
            match call(c, tool, one(&signed, "accepted", Some(BOSS))).await {
                Err(e) => p.push(format!("{tool} refused a SIGNED settle: {e}")),
                Ok(_) => {
                    if !signed_by(c, &signed).await {
                        p.push(format!("{tool}: a signed settle drew no approver edge"));
                    }
                    let rename = if tool == "create_node" {
                        json!({"node_type": "Decision", "id": signed, "props": {"name": "Renamed, still accepted", "status": "accepted"}})
                    } else {
                        json!({"nodes": [{"node_type": "Decision", "id": signed, "props": {"name": "Renamed, still accepted", "status": "accepted"}}]})
                    };
                    if let Err(e) = call(c, tool, rename).await {
                        p.push(format!("{tool}: re-writing the value a node already holds was refused as a new settle: {e}"));
                    }
                }
            }
            let quiet = id("quiet");
            if let Err(e) = call(c, tool, one(&quiet, "proposed", None)).await {
                p.push(format!("{tool} refused a write that settles nothing: {e}"));
            }
        }
        "import_graph" | "apply_merge" => {
            let unsigned = id("imported-unsigned");
            let signed = id("imported-signed");
            let quiet = id("imported-quiet");
            let theirs = doc(
                json!([decision(&unsigned, "accepted"), decision(&signed, "accepted"), decision(&quiet, "proposed"),
                       {"node_type": "Contributor", "node_id": BOSS, "properties": {"name": "The owner", "kind": "person"}}]),
                json!([{"edge_type": "AUTHORED_BY", "from_id": signed, "to_id": BOSS, "properties": {"roles": ["approver"]}}]),
            );
            let reply = if tool == "import_graph" {
                call(c, tool, json!({"document": theirs})).await
            } else {
                let base = dir.join(format!("base-{n}.json"));
                let their = dir.join(format!("theirs-{n}.json"));
                std::fs::write(&base, doc(json!([]), json!([])).to_string()).expect("base");
                std::fs::write(&their, theirs.to_string()).expect("theirs");
                call(
                    c,
                    tool,
                    json!({"base_path": base, "theirs_path": their, "resolutions": {}}),
                )
                .await
            };
            match reply {
                Err(e) => p.push(format!(
                    "{tool} refused a document it should write and report: {e}"
                )),
                Ok(r) => {
                    let named = r["settled_without_approver"]["nodes"].to_string();
                    if !named.contains(&unsigned) {
                        p.push(format!(
                            "{tool} wrote an unsigned settle and did not name it: {r}"
                        ));
                    }
                    if named.contains(&signed) {
                        p.push(format!("{tool} named a settle the document SIGNED: {r}"));
                    }
                    if named.contains(&quiet) {
                        p.push(format!("{tool} named a node that settles nothing: {r}"));
                    }
                }
            }
        }
        "replace_text" => {
            let rid = id("text").replace("dec:", "req:");
            call(
                c,
                "add_requirement",
                json!({"id": rid, "name": format!("A need {rid}"), "statement": "a fixture need"}),
            )
            .await
            .expect("fixture requirement");
            if let Ok(r) = call(
                c,
                tool,
                json!({"node_id": rid, "field": "status", "old": "proposed", "new": "accepted"}),
            )
            .await
            {
                p.push(format!(
                    "{tool} SETTLED a Requirement through a text edit, unsigned: {r}"
                ));
            }
            if let Err(e) = call(
                c,
                tool,
                json!({"node_id": rid, "field": "statement", "new": "an appended note"}),
            )
            .await
            {
                p.push(format!(
                    "{tool} refused a prose edit that settles nothing: {e}"
                ));
            }
        }
        "mirror_surface" => {
            // Another design's published surface, carrying an accepted
            // Requirement: its owner signed it THERE.
            let mirrored = id("mirrored").replace("dec:", "req:");
            let surface = json!({
                "graph_id": "another-design",
                "nodes": [{"node_type": "Requirement", "node_id": mirrored, "properties": {"name": "A published need of theirs", "statement": "their accepted, published need", "status": "accepted", "designation": "published"}}],
                "edges": []
            });
            match call(c, tool, json!({"document": surface})).await {
                Err(e) => p.push(format!("{tool} refused a surface fixture: {e}")),
                Ok(r)
                    if !r["settled_without_approver"]["nodes"]
                        .to_string()
                        .contains(&mirrored) =>
                {
                    p.push(format!(
                        "{tool} mirrored settled intent here and did not name it: {r}"
                    ))
                }
                Ok(_) => {}
            }
        }
        other => p.push(format!(
            "`{other}` writes a node's properties and this test has no fixture for it: a writer \
             that can set a settling value without being driven here"
        )),
    }
    p
}

#[tokio::test]
async fn every_node_writer_holds_a_settle_to_the_owners_word() {
    let c = connect().await;
    call(
        &c,
        "add_contributor",
        json!({"id": BOSS, "name": "The owner", "kind": "person"}),
    )
    .await
    .expect("seed contributor");
    let dir = std::env::temp_dir().join(format!("reflow2-settle-writers-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let found = writers(&c).await;
    for must in [
        "apply_merge",
        "create_node",
        "create_nodes",
        "import_graph",
        "mirror_surface",
        "replace_text",
    ] {
        assert!(
            found.iter().any(|t| t == must),
            "{must} was not recognised as a node writer: {found:?}"
        );
    }
    let mut n = 0;
    let mut problems = Vec::new();
    for tool in &found {
        problems.extend(drive(&c, tool, &mut n, &dir).await);
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// The typed doors are held to the same table: a Decision recorded already
/// `rejected` retires an option and settles nothing, so it needs no signature
/// — `add_decision` refused it unsigned until 2026-09-29 while the CI gate
/// never asked about it (two answers to one question).
#[tokio::test]
async fn a_typed_door_settles_exactly_what_the_core_table_says() {
    let c = connect().await;
    let rejected = call(&c, "add_decision", json!({"id": "dec:retired-option", "name": "An option we turned down", "decision": "not this road", "kind": "choice", "status": "rejected"})).await;
    assert!(
        rejected.is_ok(),
        "a Decision recorded as rejected settles nothing and was refused unsigned: {rejected:?}"
    );
    let accepted = call(&c, "add_decision", json!({"id": "dec:unsigned-choice", "name": "A choice nobody signed", "decision": "this road", "kind": "choice", "status": "accepted"})).await;
    assert!(
        accepted.is_err(),
        "an unsigned accepted Decision was recorded: {accepted:?}"
    );
}
