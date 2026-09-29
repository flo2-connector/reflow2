//! A bulk typed-edge write runs each typed helper's OWN checks, and every
//! typed helper names its bulk form.
//!
//! # The finding this pins
//!
//! `fact:root-cause-a-bulk-edge-form-was-offered-and-no-typed-helper-names-it-2026-09-29`
//! (item I24 of `art:dev-reflow2-two-agent-exercise-feedback-2026-09-29`). A
//! designer agent built one design through the flo2 connector in about 270
//! single-edge calls. `create_edges` was reachable for the thin helpers and no
//! helper named it; for the helpers with semantics of their own — `constrains`
//! (contribution, unit, basis, source), `governed_by` (its ruling),
//! `authored_by` (the role set) — no bulk route that keeps
//! those checks existed at all, because `create_edges` takes free `props` and
//! runs none of them.
//!
//! # What is pinned, as a CLASS
//!
//! - Every typed edge helper — enumerated from the one shared map
//!   (`writers.json`) and the served tool list, never from a list kept here —
//!   either names the bulk form in its served description, or takes a list
//!   itself (its own bulk form).
//! - For every helper a single pair can drive, one item through `draw_edges`
//!   and one call to the helper give the SAME reply and store the SAME edge,
//!   and refuse the same bad input with the SAME words.
//! - The semantic checks survive bulk: a `constrains` contribution with its
//!   unit and basis, a `governed_by` ruling (carried, and an unknown one
//!   answered in the helper's words), and two `authored_by` roles on one pair within one call (the role set).
//! - The bulk form is all-or-nothing with every failure named, and `check_only`
//!   writes nothing.

use reflow2_core::graph::DesignGraph;
use reflow2_core::vocabulary::EndpointMatch;
use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "bulk-probe".to_string();
        cfg
    }
}

struct Session {
    client: rmcp::service::RunningService<rmcp::service::RoleClient, Probe>,
}

const ANCHOR: &str = "req:anchor";

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
    let s = Session { client };
    s.ok(
        "add_requirement",
        json!({"id": ANCHOR, "name": "Anchor", "statement": "A node a reference can name."}),
    )
    .await;
    s
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

    async fn node(&self, node_type: &str, id: &str) -> Result<(), String> {
        let spec = self
            .call(
                "describe_schema",
                json!({"node_type": node_type, "required_only": true}),
            )
            .await?;
        let mut props = serde_json::Map::new();
        for p in spec["properties"].as_array().cloned().unwrap_or_default() {
            let name = p["name"].as_str().unwrap_or_default().to_string();
            let value = match p["prop_type"].as_str().unwrap_or("string") {
                "enum" => p["values"][0].clone(),
                "bool" | "boolean" => json!(false),
                "int" | "integer" | "float" | "number" => json!(0),
                _ if name.ends_with("_id") => json!(ANCHOR),
                _ => json!(format!("{node_type} {id}")),
            };
            props.insert(name, value);
        }
        self.call(
            "create_node",
            json!({"node_type": node_type, "id": id, "props": props}),
        )
        .await
        .map(|_| ())
    }

    /// One item through the bulk form: `Ok(reply)` exactly when the bulk call
    /// applied, else the item's failure text.
    async fn bulk_one(&self, tool: &str, arguments: Value) -> Result<Value, String> {
        let r = self
            .call(
                "draw_edges",
                json!({"edges": [{"tool": tool, "arguments": arguments}]}),
            )
            .await?;
        if r["applied"] == json!(true) {
            Ok(r["written"][0]["reply"].clone())
        } else {
            Err(r["failures"][0]["error"]
                .as_str()
                .unwrap_or("<no failure text>")
                .to_string())
        }
    }

    /// Every edge the design holds touching `id`, graph id stripped, as a set.
    async fn edges_touching(&self, id: &str) -> BTreeSet<String> {
        let export = self.ok("export_graph", json!({})).await;
        let mut out = BTreeSet::new();
        for e in export["edges"].as_array().cloned().unwrap_or_default() {
            let from = e["from_id"].as_str().or(e["from"].as_str()).unwrap_or("");
            let to = e["to_id"].as_str().or(e["to"].as_str()).unwrap_or("");
            if from == id || to == id {
                out.insert(strip(e).to_string());
            }
        }
        out
    }
}

/// The per-graph id differs between two sessions and says nothing about the
/// edge; everything else must match.
fn strip(mut v: Value) -> Value {
    match &mut v {
        Value::Object(m) => {
            m.remove("graph_id");
            for x in m.values_mut() {
                *x = strip(x.take());
            }
        }
        Value::Array(a) => {
            for x in a.iter_mut() {
                *x = strip(x.take());
            }
        }
        _ => {}
    }
    v
}

fn writers() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/writers.json");
    let text = std::fs::read_to_string(path).expect("writers.json is the shared map");
    serde_json::from_str(&text).expect("writers.json is JSON")
}

/// Every typed edge helper: the tools the shared map says draw an edge type.
fn typed_edge_helpers() -> BTreeMap<String, Vec<String>> {
    let w = writers();
    w["draws_edge_type"]
        .as_object()
        .expect("draws_edge_type")
        .iter()
        .map(|(tool, edges)| {
            (
                tool.clone(),
                edges
                    .as_array()
                    .expect("edge list")
                    .iter()
                    .map(|e| e.as_str().expect("edge name").to_string())
                    .collect(),
            )
        })
        .collect()
}

/// A helper whose own arguments already take a list is its own bulk form.
fn takes_a_list(schema: &serde_json::Map<String, Value>) -> bool {
    // `Vec<T>` is `"type": "array"`; `Option<Vec<T>>` is `"type": ["array", "null"]`.
    let is_array = |p: &Value| match p.get("type") {
        Some(Value::String(t)) => t == "array",
        Some(Value::Array(ts)) => ts.iter().any(|t| t == "array"),
        _ => false,
    };
    schema
        .get("properties")
        .and_then(Value::as_object)
        .is_some_and(|props| props.values().any(is_array))
}

// ─── every helper names its bulk form, or takes a list itself ────────────────

#[tokio::test]
async fn every_typed_edge_helper_names_its_bulk_form_or_is_one() {
    let s = session().await;
    let listed: BTreeMap<String, rmcp::model::Tool> = s
        .client
        .list_all_tools()
        .await
        .expect("tools/list")
        .into_iter()
        .map(|t| (t.name.to_string(), t))
        .collect();
    assert!(
        listed.contains_key("draw_edges"),
        "no `draw_edges` is served: the typed helpers have no bulk form that keeps their checks"
    );
    let mut silent = Vec::new();
    for tool in typed_edge_helpers().keys() {
        let t = listed
            .get(tool)
            .unwrap_or_else(|| panic!("`{tool}` is in writers.json and not served"));
        let names_it = t
            .description
            .as_deref()
            .is_some_and(|d| d.contains("draw_edges"));
        if !names_it && !takes_a_list(&t.input_schema) {
            silent.push(tool.clone());
        }
    }
    assert!(
        silent.is_empty(),
        "{} typed edge helper(s) neither name `draw_edges` nor take a list, so a caller \
         meeting them learns no bulk route (I24): {silent:?}",
        silent.len()
    );
}

// ─── one item through the bulk form == one call to the helper ────────────────

/// Helpers a single `(from_id, to_id)` pair cannot drive; each is exercised by
/// its own case below or is a list-taking form.
const NOT_DRIVEN_BY_A_PAIR: &[&str] = &[
    "review_relations",
    "move_component",
    "release_includes_all",
    "gate_on",
    "authored_by",
    "answers",
    "pin_at_epoch",
];

/// The first pair the schema MODELS for `edge` (named on one end at least, or
/// declared for the pair) — enough to drive the helper once.
fn a_modelled_pair(g: &DesignGraph, types: &[String], edge: &str) -> Option<(String, String)> {
    for f in types {
        for t in types {
            let Ok(q) = g.edge_types_between(f, t) else {
                continue;
            };
            for m in q.matches {
                let one_end_named =
                    m.from_match == EndpointMatch::Exact || m.to_match == EndpointMatch::Exact;
                let union_only = edge == "CONTAINS" && f == "Component" && t != "Component";
                if m.spec.edge_type == edge
                    && (one_end_named || m.declared_for_this_pair)
                    && !union_only
                {
                    return Some((f.clone(), t.clone()));
                }
            }
        }
    }
    None
}

#[tokio::test]
async fn one_bulk_item_and_one_helper_call_answer_alike_for_every_helper() {
    let graph = DesignGraph::open_in_memory().expect("graph");
    let types: Vec<String> = graph
        .describe_vocabulary()
        .node_types
        .into_iter()
        .map(|t| t.node_type)
        .collect();
    let single = session().await;
    let bulk = session().await;
    let mut compared = 0usize;
    let mut differ: Vec<String> = Vec::new();
    for (tool, edges) in typed_edge_helpers() {
        if NOT_DRIVEN_BY_A_PAIR.contains(&tool.as_str()) {
            continue;
        }
        let Some((f, t)) = edges
            .iter()
            .find_map(|e| a_modelled_pair(&graph, &types, e))
        else {
            continue;
        };
        let from_id = format!("a:{tool}");
        let to_id = format!("b:{tool}");
        let mut built = true;
        for sess in [&single, &bulk] {
            built &= sess.node(&f, &from_id).await.is_ok() && sess.node(&t, &to_id).await.is_ok();
        }
        if !built {
            continue;
        }
        let args = json!({"from_id": from_id, "to_id": to_id});
        let one = single.call(&tool, args.clone()).await;
        let many = bulk.bulk_one(&tool, args).await;
        match (&one, &many) {
            (Ok(a), Ok(b)) if strip(a.clone()) == strip(b.clone()) => {}
            (Err(a), Err(b)) if a == b => {}
            _ => differ.push(format!(
                "`{tool}` {f} -> {t}: helper {one:?} / bulk {many:?}"
            )),
        }
        if single.edges_touching(&from_id).await != bulk.edges_touching(&from_id).await {
            differ.push(format!("`{tool}`: the two designs store different edges"));
        }
        // The same bad input is refused with the same words.
        let bad = json!({"from_id": from_id, "to_id": "nothing:here"});
        let one = single.call(&tool, bad.clone()).await;
        let many = bulk.bulk_one(&tool, bad).await;
        match (&one, &many) {
            (Err(a), Err(b)) if a == b => {}
            _ => differ.push(format!("`{tool}` refusal: helper {one:?} / bulk {many:?}")),
        }
        compared += 1;
    }
    assert!(
        compared >= 20,
        "only {compared} helper(s) were driven — the harness, not the surface, is broken"
    );
    assert!(
        differ.is_empty(),
        "{} disagreement(s) between a helper and its bulk form:\n{}",
        differ.len(),
        differ.join("\n")
    );
}

// ─── the semantic checks survive the bulk form ──────────────────────────────

#[tokio::test]
async fn a_contribution_keeps_its_unit_basis_and_source_in_bulk() {
    let single = session().await;
    let bulk = session().await;
    for s in [&single, &bulk] {
        s.node("Constraint", "con:latency")
            .await
            .expect("a constraint");
        s.node("Component", "cmp:router")
            .await
            .expect("a component");
    }
    let args = json!({"constraint_id": "con:latency", "target_id": "cmp:router", "contribution": 12.5, "unit": "ms", "basis": "estimated", "source": "who:someone", "note": "measured by nobody yet"});
    let one = single.ok("constrains", args.clone()).await;
    let many = bulk
        .bulk_one("constrains", args)
        .await
        .expect("bulk constrains");
    assert_eq!(
        strip(one),
        strip(many),
        "a CONSTRAINS contribution differs in bulk"
    );
    assert_eq!(
        single.edges_touching("con:latency").await,
        bulk.edges_touching("con:latency").await
    );
}

#[tokio::test]
async fn a_ruling_is_checked_and_kept_alike_in_bulk() {
    let single = session().await;
    let bulk = session().await;
    for s in [&single, &bulk] {
        s.node("Decision", "dec:musing").await.expect("a decision");
    }
    // A `parks` ruling is carried on the edge, exactly as the helper carries it.
    let parks = json!({"from_id": ANCHOR, "to_id": "dec:musing", "ruling": "parks"});
    let one = single.call("governed_by", parks.clone()).await;
    let many = bulk.bulk_one("governed_by", parks).await;
    assert_eq!(
        one, many,
        "a ruling differs between the helper and its bulk form"
    );
    assert_eq!(
        many.as_ref()
            .ok()
            .map(|r| r["properties"]["ruling"].clone()),
        Some(json!("parks")),
        "the ruling must travel: {many:?}"
    );
    // A ruling the helper does not know is answered alike, in its own words.
    let bogus = json!({"from_id": ANCHOR, "to_id": "dec:musing", "ruling": "not-a-ruling"});
    let one = single.call("governed_by", bogus.clone()).await;
    let many = bulk.bulk_one("governed_by", bogus).await;
    assert_eq!(
        one, many,
        "an unknown ruling is answered differently in bulk"
    );
    assert_eq!(
        single.edges_touching("dec:musing").await,
        bulk.edges_touching("dec:musing").await
    );
}

#[tokio::test]
async fn two_roles_on_one_pair_in_one_bulk_call_keep_the_role_set() {
    let single = session().await;
    let bulk = session().await;
    for s in [&single, &bulk] {
        s.node("Contributor", "who:pat")
            .await
            .expect("a contributor");
    }
    let author = json!({"from_id": ANCHOR, "contributor_id": "who:pat", "role": "author"});
    let approver = json!({"from_id": ANCHOR, "contributor_id": "who:pat", "role": "approver", "acted_at": "2026-09-29"});
    single.ok("authored_by", author.clone()).await;
    let second = single.ok("authored_by", approver.clone()).await;
    let r = bulk
        .ok(
            "draw_edges",
            json!({"edges": [
                {"tool": "authored_by", "arguments": author},
                {"tool": "authored_by", "arguments": approver}
            ]}),
        )
        .await;
    assert_eq!(r["applied"], json!(true), "{r}");
    assert_eq!(
        strip(second),
        strip(r["written"][1]["reply"].clone()),
        "the second role must read the first, inside one bulk call"
    );
    assert_eq!(
        single.edges_touching("who:pat").await,
        bulk.edges_touching("who:pat").await
    );
}

// ─── all or nothing, every failure named, and a check writes nothing ────────

#[tokio::test]
async fn a_bulk_write_is_all_or_nothing_and_names_every_failure() {
    let s = session().await;
    s.node("Capability", "cap:route")
        .await
        .expect("a capability");
    s.node("Component", "cmp:router")
        .await
        .expect("a component");
    let good =
        json!({"tool": "allocate", "arguments": {"from_id": "cap:route", "to_id": "cmp:router"}});
    let r = s
        .ok(
            "draw_edges",
            json!({"edges": [
                good.clone(),
                {"tool": "allocate", "arguments": {"from_id": "cap:route", "to_id": "cmp:missing"}},
                {"tool": "no_such_helper", "arguments": {}}
            ]}),
        )
        .await;
    assert_eq!(r["applied"], json!(false), "{r}");
    let failed: Vec<u64> = r["failures"]
        .as_array()
        .expect("failures")
        .iter()
        .filter_map(|f| f["index"].as_u64())
        .collect();
    assert_eq!(failed, vec![1, 2], "every failure, by position: {r}");
    assert!(
        s.edges_touching("cmp:router").await.is_empty(),
        "a rejected batch wrote something"
    );

    let checked = s
        .ok(
            "draw_edges",
            json!({"edges": [good.clone()], "check_only": true}),
        )
        .await;
    assert_eq!(checked["check_only"], json!(true), "{checked}");
    assert!(
        checked["failures"].as_array().expect("failures").is_empty(),
        "{checked}"
    );
    assert!(
        s.edges_touching("cmp:router").await.is_empty(),
        "check_only wrote something"
    );

    let applied = s.ok("draw_edges", json!({"edges": [good]})).await;
    assert_eq!(applied["applied"], json!(true), "{applied}");
    assert_eq!(
        applied["drawn"],
        json!(["cap:route ALLOCATED_TO cmp:router"]),
        "each edge drawn is named, subject first: {applied}"
    );
    assert_eq!(s.edges_touching("cmp:router").await.len(), 1);
}
