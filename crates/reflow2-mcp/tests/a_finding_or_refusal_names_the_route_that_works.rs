//! A finding or a refusal that fires correctly also names the route that works.
//!
//! # The class
//!
//! The dev_reflow2 hub's two-agent exercise (2026-09-28 to 29,
//! `art:dev-reflow2-two-agent-exercise-feedback-2026-09-29`) put eight separate
//! reports into one shape. The check was right and the words stopped one item
//! short of the cure:
//!
//! - I14a: `verifies` refused a Decision with a bare sentence, while
//!   GOVERNED_BY is the modelled route for a check on a ruling.
//! - I17: `budget_report` without `constraint_id` named no design-wide route.
//! - I9: the typed `consumes` refused an Actor that the schema accepts.
//! - I11: `level_spine_disagreement` never named `contains` under the Project,
//!   which its sibling `orphan_level` has named since 2026-09-16.
//! - I12: `unsatisfied_requirement` never named `parks`, the fourth instance.
//! - I13: `unthreaded_cluster` never said which edges thread a cluster.
//! - I14b: `describe_schema` never named the tool that writes a type.
//! - I8: the lens said "N recorded here" and meant people only.
//!
//! Every instance has a root-cause fact in reflow2's design, and each of the
//! 2026-09-29 facts names the same class: an instruction fixed where it was
//! reported while its siblings were never asked. So the pins below are CLASS
//! pins wherever a class can be enumerated. Two walk the schema: every edge tool
//! accepts every pair its schema models, and every node type names a writer.
//! Where only instances exist (a message a person reads), each is pinned against
//! the behaviour it claims — a cure the message names is applied, and the
//! finding must then go away.

use reflow2_core::graph::DesignGraph;
use reflow2_core::vocabulary::EndpointMatch;
use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

// ─── an in-process client, so a refusal arrives exactly as a session sees it ──

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "route-probe".to_string();
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
    let s = Session { client };
    s.ok(
        "add_requirement",
        json!({"id": ANCHOR, "name": "Anchor", "statement": "A node a reference can name."}),
    )
    .await;
    s
}

/// A node every probe session holds, for properties that must name one.
const ANCHOR: &str = "req:anchor";

impl Session {
    /// The reply, or the refusal's words.
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

    /// A node of any declared type, carrying only what its schema requires.
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
                // A declared node reference must resolve; point it at the
                // anchor every probe session holds.
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
}

/// The one map of which served tool writes which part of the vocabulary.
fn writers() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/writers.json");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| {
        panic!("{path} is the map every reader shares, and it is missing: {e}")
    });
    serde_json::from_str(&text).expect("writers.json is JSON")
}

/// Every object anywhere in a reply that satisfies `pick` — replies nest their
/// findings under envelopes whose shape is not what these pins are about.
fn objects<'a>(v: &'a Value, pick: &dyn Fn(&Value) -> bool, out: &mut Vec<&'a Value>) {
    match v {
        Value::Object(m) => {
            if pick(v) {
                out.push(v);
            }
            for x in m.values() {
                objects(x, pick, out);
            }
        }
        Value::Array(a) => {
            for x in a {
                objects(x, pick, out);
            }
        }
        _ => {}
    }
}

fn find<'a>(v: &'a Value, pick: &dyn Fn(&Value) -> bool) -> Vec<&'a Value> {
    let mut out = Vec::new();
    objects(v, pick, &mut out);
    out
}

fn node_type_names() -> Vec<String> {
    let g = DesignGraph::open_in_memory().expect("graph");
    g.describe_vocabulary()
        .node_types
        .into_iter()
        .map(|t| t.node_type)
        .collect()
}

// ─── I14a: an invalid pair names what accepts it, and the tool that draws it ──

#[tokio::test]
async fn a_check_on_a_ruling_is_refused_with_governed_by_and_its_tool() {
    let s = session().await;
    s.ok(
        "add_decision",
        json!({"id": "dec:ruling", "name": "A ruling", "decision": "Do it this way."}),
    )
    .await;
    s.ok(
        "add_verification",
        json!({"id": "ver:check", "name": "The check", "method": "test"}),
    )
    .await;
    let refusal = s
        .call(
            "verifies",
            json!({"verification_id": "ver:check", "target_id": "dec:ruling"}),
        )
        .await
        .expect_err("VERIFIES does not model a Decision target");
    assert!(
        refusal.contains("GOVERNED_BY"),
        "the refusal must name the edge that models a check on a ruling, as create_edge's does; \
         got: {refusal}"
    );
    assert!(
        refusal.contains("`governed_by`"),
        "and the typed tool that draws it — the caller is at a typed helper and is owed the \
         typed route, not only an edge name; got: {refusal}"
    );
}

// ─── I9, as a class: no edge tool is narrower than the pairs its schema models ─

/// Tools that draw an edge but cannot be driven by a bare `from_id`/`to_id`,
/// each with the reason, so the exemption is read rather than assumed.
const NOT_DRIVEN_BY_A_PAIR: &[(&str, &str)] = &[
    (
        "review_relations",
        "draws one of thirteen relations named per link; covered by its own suite",
    ),
    (
        "move_component",
        "MOVES a component on the spine, detaching its parent; `contain_component` draws the same pair",
    ),
    (
        "release_includes_all",
        "takes a release and a SET of artifacts, not one pair",
    ),
    (
        "gate_on",
        "demands a readiness level on the edge; the readiness suite drives it",
    ),
    (
        "authored_by",
        "names a Contributor and a role; the attribution suite drives it",
    ),
    (
        "answers",
        "answers a Question, which only the gap tools mint",
    ),
    ("pin_at_epoch", "pins by epoch sequence, not by pair"),
];

/// Pairs a wildcard admits that no relation the edge stands for uses, each
/// with the measurement behind the exemption.
///
/// CONTAINS is written as ONE edge type for TWO relations: project membership
/// (`Project → *`) and the component spine (`Component → Component`). The
/// union's `from: [Project, Component], to: *` therefore admits a Component
/// containing a Requirement, which neither relation means: 0 of 224 CONTAINS
/// edges in reflow2's own design, 2026-09-29 (96 Component → Component, the rest
/// from the Project). An edge type standing for several relations is recorded
/// on its own (`fact:some-edge-types-are-several-relations-split-by-a-property-value…`).
fn admitted_by_a_union_and_meant_by_nothing(edge: &str, from: &str, to: &str) -> bool {
    edge == "CONTAINS" && from == "Component" && to != "Component"
}

#[tokio::test]
async fn every_edge_tool_accepts_every_pair_its_schema_models() {
    let w = writers();
    let draws = w["draws_edge_type"].as_object().expect("draws_edge_type");
    let skip: BTreeMap<&str, &str> = NOT_DRIVEN_BY_A_PAIR.iter().copied().collect();

    // Which tools may draw each edge type — the FAMILY is what must cover the
    // schema: `contains` takes a Project parent and `contain_component` a
    // Component one, and together they are the CONTAINS surface.
    let mut tools_for: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (tool, edges) in draws {
        if skip.contains_key(tool.as_str()) {
            continue;
        }
        for e in edges.as_array().expect("edge list") {
            tools_for
                .entry(e.as_str().expect("edge name").to_string())
                .or_default()
                .push(tool.clone());
        }
    }

    // The pairs the schema MODELS for each edge: named on both ends, named on
    // one end and open by design on the other, or declared for this pair. A
    // pair accepted only because both ends are `*` is tolerated, not modelled,
    // and is create_edge's business.
    let g = DesignGraph::open_in_memory().expect("graph");
    let types = node_type_names();
    let mut modelled: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for f in &types {
        for t in &types {
            let q = g.edge_types_between(f, t).expect("edge query");
            for m in q.matches {
                let one_end_named =
                    m.from_match == EndpointMatch::Exact || m.to_match == EndpointMatch::Exact;
                if (one_end_named || m.declared_for_this_pair)
                    && tools_for.contains_key(&m.spec.edge_type)
                    && !admitted_by_a_union_and_meant_by_nothing(&m.spec.edge_type, f, t)
                {
                    modelled
                        .entry(m.spec.edge_type.clone())
                        .or_default()
                        .push((f.clone(), t.clone()));
                }
            }
        }
    }

    let s = session().await;
    let mut made: BTreeSet<String> = BTreeSet::new();
    let mut unbuildable: BTreeSet<String> = BTreeSet::new();
    let mut narrower: Vec<String> = Vec::new();
    for (edge, pairs) in &modelled {
        for (f, t) in pairs {
            let (from_id, to_id) = (format!("a:{f}"), format!("b:{t}"));
            for (id, ty) in [(&from_id, f), (&to_id, t)] {
                if made.insert(id.clone())
                    && let Err(e) = s.node(ty, id).await
                {
                    unbuildable.insert(format!("{ty}: {e}"));
                }
            }
            if unbuildable
                .iter()
                .any(|u| u.starts_with(&format!("{f}:")) || u.starts_with(&format!("{t}:")))
            {
                continue;
            }
            let mut refusals = Vec::new();
            let mut drawn = false;
            for tool in &tools_for[edge] {
                match s
                    .call(tool, json!({"from_id": from_id, "to_id": to_id}))
                    .await
                {
                    Ok(_) => {
                        drawn = true;
                        break;
                    }
                    Err(e) => refusals.push(format!(
                        "`{tool}`: {}",
                        e.lines().next().unwrap_or_default()
                    )),
                }
            }
            if !drawn {
                narrower.push(format!(
                    "{edge} {f} -> {t} is modelled by the schema and refused by every tool that \
                     draws it — {}",
                    refusals.join("; ")
                ));
            }
        }
    }
    assert!(
        unbuildable.is_empty() && narrower.is_empty(),
        "UNBUILDABLE (pairs touching these went unmeasured): {unbuildable:#?}\n\n{} modelled \
         pair(s) have no typed route, only create_edge — a typed helper narrower than its schema \
         is the cause behind I9:\n{}",
        narrower.len(),
        narrower.join("\n")
    );
}

// ─── I14b: the schema names who writes each type, from the shared map ─────────

#[tokio::test]
async fn every_node_type_names_the_tool_that_writes_it() {
    let s = session().await;
    let served: BTreeSet<String> = s
        .client
        .list_all_tools()
        .await
        .expect("tools/list")
        .into_iter()
        .map(|t| t.name.to_string())
        .collect();
    let mut silent = Vec::new();
    let mut unserved = Vec::new();
    for t in node_type_names() {
        for required_only in [false, true] {
            let reply = s
                .ok(
                    "describe_schema",
                    json!({"node_type": t, "required_only": required_only}),
                )
                .await;
            let Some(written_by) = reply.get("written_by").and_then(Value::as_array) else {
                silent.push(format!("{t} (required_only: {required_only})"));
                continue;
            };
            if written_by.is_empty()
                && !reply["no_typed_writer"]
                    .as_str()
                    .is_some_and(|n| n.contains("create_node"))
            {
                silent.push(format!(
                    "{t}: empty written_by with no note naming create_node"
                ));
            }
            for tool in written_by.iter().filter_map(Value::as_str) {
                if !served.contains(tool) {
                    unserved.push(format!("{t} names `{tool}`, which is not served"));
                }
            }
        }
    }
    assert!(
        silent.is_empty(),
        "describe_schema must say which served tool writes the type it describes (I14b: the \
         designer learned TemporalFact's fields from an export because nothing said \
         `record_finding` writes one): {silent:#?}"
    );
    assert!(unserved.is_empty(), "{unserved:#?}");
}

#[tokio::test]
async fn the_shared_map_names_only_served_tools_and_declared_types() {
    let s = session().await;
    let served: BTreeSet<String> = s
        .client
        .list_all_tools()
        .await
        .expect("tools/list")
        .into_iter()
        .map(|t| t.name.to_string())
        .collect();
    let g = DesignGraph::open_in_memory().expect("graph");
    let vocab = g.describe_vocabulary();
    let node_types: BTreeSet<String> = vocab.node_types.into_iter().map(|t| t.node_type).collect();
    let edge_types: BTreeSet<String> = vocab.edge_types.into_iter().map(|t| t.edge_type).collect();
    let w = writers();
    let mut wrong = Vec::new();
    for (section, known) in [
        ("writes_node_type", &node_types),
        ("draws_edge_type", &edge_types),
    ] {
        for (tool, names) in w[section].as_object().expect(section) {
            if !served.contains(tool) {
                wrong.push(format!("{section}: `{tool}` is not a served tool"));
            }
            for n in names
                .as_array()
                .expect("list")
                .iter()
                .filter_map(Value::as_str)
            {
                if !known.contains(n) {
                    wrong.push(format!(
                        "{section}: `{tool}` names {n}, which the schema does not declare"
                    ));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

// ─── I17: one budget at a time, and the refusal says where all of them are ────

#[tokio::test]
async fn a_budget_report_without_a_constraint_names_the_design_wide_sweep() {
    let s = session().await;
    let refusal = s
        .call("budget_report", json!({}))
        .await
        .expect_err("constraint_id is required");
    assert!(
        refusal.contains("closure_report"),
        "a caller asking for every budget is owed the design-wide route that exists; got: \
         {refusal}"
    );
}

// ─── I11: a cure a message names must be the cure that clears it ─────────────

#[tokio::test]
async fn a_root_part_is_told_it_can_sit_under_the_project() {
    let s = session().await;
    s.ok("add_project", json!({"id": "proj:p", "name": "P"}))
        .await;
    s.ok(
        "add_component",
        json!({"id": "cmp:engine", "name": "Engine", "description": "The system.", "level": "system"}),
    )
    .await;
    s.ok(
        "add_component",
        json!({"id": "cmp:gateway", "name": "Gateway", "description": "Outside the system.", "level": "component"}),
    )
    .await;
    let is_gateway_spine = |i: &Value| {
        i["kind"] == "level_spine_disagreement"
            && i["components"].to_string().contains("cmp:gateway")
    };
    let issues = s.ok("hierarchy_issues", json!({})).await;
    let spine = find(&issues, &is_gateway_spine);
    let spine = spine
        .first()
        .unwrap_or_else(|| panic!("the fixture must raise level_spine_disagreement: {issues}"));
    let message = spine["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("`contains`") && message.contains("Project"),
        "level_spine_disagreement accepts a Project parent exactly as orphan_level does, and must \
         say so (I11); got: {message}"
    );

    // AND THE CURE IS TRUE: drawn as named, the finding goes away.
    s.ok(
        "contains",
        json!({"project_id": "proj:p", "child_id": "cmp:gateway"}),
    )
    .await;
    let after = s.ok("hierarchy_issues", json!({})).await;
    assert!(
        find(&after, &is_gateway_spine).is_empty(),
        "the named cure must clear the finding: {after}"
    );
}

// ─── I12: a gap that reads parking says so ────────────────────────────────────

#[tokio::test]
async fn an_unsatisfied_requirement_names_parking() {
    let s = session().await;
    s.ok("add_project", json!({"id": "proj:p", "name": "P"}))
        .await;
    s.ok(
        "add_contributor",
        json!({"id": "who:owner", "kind": "person", "name": "The Owner"}),
    )
    .await;
    // The gap is only asked once capabilities exist to satisfy anything.
    s.ok(
        "add_capability",
        json!({"id": "cap:something", "name": "Something", "description": "Does something."}),
    )
    .await;
    s.ok(
        "add_requirement",
        json!({"id": "req:a-person-provisions-it", "name": "A person provisions the hardware",
               "statement": "The owner buys and provisions the hosts."}),
    )
    .await;
    let gaps = s.ok("detect_gaps", json!({})).await;
    let hits = find(&gaps, &|g: &Value| {
        let t = g.to_string();
        g.get("gap_source").is_some()
            && g["gap_source"] == "unsatisfied_requirement"
            && t.contains("req:a-person-provisions-it")
    });
    let gap = hits
        .first()
        .unwrap_or_else(|| panic!("the fixture must raise unsatisfied_requirement: {gaps}"));
    assert_eq!(
        gap["parks"], true,
        "the unsatisfied-requirement gap reads parking (detect.rs is_parked), so its row must say \
         a ruling can park it — the fourth measured time a person stuck here was never told: {gap}"
    );
    let route = gaps["parks_route"].as_str().unwrap_or_default();
    assert!(
        route.contains("ruling:")
            && route.contains("\"parks\"")
            && route.contains("unsatisfied_requirement"),
        "and the reply says HOW, once, naming the findings the ruling is read by: {route:?}"
    );

    // AND THE QUESTION PUT TO THE PERSON CARRIES IT, which is where they are
    // stuck: the words the prompt is phrased from.
    let asked = s.ok("gap_to_prompt", json!({"gap": (*gap).clone()})).await;
    let why = asked["gap"]["why"].as_str().unwrap_or_default();
    assert!(
        why.contains("ruling:") && why.contains("\"parks\""),
        "the prompt's context must name parking too: {why}"
    );

    // AND THE ROUTE IS TRUE: parked under an accepted ruling, the gap is gone.
    s.ok(
        "add_decision",
        json!({"id": "dec:the-owner-provisions", "name": "The owner provisions the hardware",
               "decision": "Buying hosts is the owner's act, outside the design.",
               "status": "accepted", "approver": "who:owner"}),
    )
    .await;
    s.ok(
        "governed_by",
        json!({"from_id": "req:a-person-provisions-it", "to_id": "dec:the-owner-provisions",
               "ruling": "parks"}),
    )
    .await;
    let after = s.ok("detect_gaps", json!({})).await;
    assert!(
        find(&after, &|g: &Value| g["gap_source"]
            == "unsatisfied_requirement"
            && g.to_string().contains("req:a-person-provisions-it"))
        .is_empty(),
        "the named route must clear the gap: {after}"
    );
}

// ─── I13: the cluster finding names the edges that thread ─────────────────────

#[tokio::test]
async fn an_unthreaded_cluster_names_the_edges_that_would_thread_it() {
    let s = session().await;
    s.ok("add_project", json!({"id": "proj:p", "name": "P"}))
        .await;
    for (req, cap, cmp) in [("req:a", "cap:a", "cmp:a"), ("req:b", "cap:b", "cmp:b")] {
        s.ok(
            "add_requirement",
            json!({"id": req, "name": req, "statement": req}),
        )
        .await;
        s.ok(
            "add_capability",
            json!({"id": cap, "name": cap, "description": cap}),
        )
        .await;
        s.ok(
            "add_component",
            json!({"id": cmp, "name": cmp, "description": cmp}),
        )
        .await;
        s.ok("satisfies", json!({"from_id": cap, "to_id": req}))
            .await;
        s.ok("allocate", json!({"from_id": cap, "to_id": cmp}))
            .await;
    }
    // Give the first thread more weight so it is the main body.
    s.ok(
        "add_requirement",
        json!({"id": "req:a2", "name": "a2", "statement": "a2"}),
    )
    .await;
    s.ok("satisfies", json!({"from_id": "cap:a", "to_id": "req:a2"}))
        .await;
    let defects = s.ok("detect_defects", json!({})).await;
    let hits = find(&defects, &|i: &Value| i["category"] == "unthreaded_cluster");
    let cluster = hits
        .first()
        .unwrap_or_else(|| panic!("the fixture must raise unthreaded_cluster: {defects}"));
    let message = cluster["message"].as_str().unwrap_or_default();
    for edge in ["GOVERNED_BY", "SATISFIES", "ALLOCATED_TO", "VERIFIES"] {
        assert!(
            message.contains(edge),
            "the finding must name the edges that DO thread, read from the walk's own set — \
             {edge} missing (I13: a true CAUSES edge was drawn and threaded nothing): {message}"
        );
    }
}

// ─── I8: the lens counts people, and says so ──────────────────────────────────

#[tokio::test]
async fn the_lens_says_it_counts_people_and_names_the_agents_it_left_out() {
    let s = session().await;
    s.ok(
        "add_contributor",
        json!({"id": "who:owner-person", "kind": "person", "name": "A Person"}),
    )
    .await;
    for id in ["who:designer-agent", "who:owner-agent"] {
        s.ok(
            "add_contributor",
            json!({"id": id, "kind": "automated_agent", "name": id,
                   "description": "An agent working the design."}),
        )
        .await;
    }
    let skill = s.ok("get_skill", json!({"name": "jot"})).await;
    let lens = skill["lens"].as_str().unwrap_or_default();
    assert!(
        lens.contains("1 person"),
        "the count is of PEOPLE and must say so — \"1 recorded here\" read as a miscount beside \
         two described contributors (I8): {lens}"
    );
    assert!(
        lens.contains("2 automated agents"),
        "and the agents it deliberately leaves out are named as left out, not silently dropped: \
         {lens}"
    );
}
