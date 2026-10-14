//! Every edge property the schema gives a DEFAULT is sayable by each typed tool
//! that draws that edge, or is named here with the reason it is not.
//!
//! # The class, not the instance
//!
//! A declared default materialises on create. A tool that draws an edge
//! without a field for one of its defaulted properties therefore makes the
//! default speak for every writer, whatever they meant, and nothing in the
//! reply says so. `review_relations` drew every CONTRADICTS `alignment:
//! opposing`, and 10 edges in reflow2's own design said in their evidence that
//! they were corroborations (`fact:root-cause-review-relations-cannot-mark-a-
//! contradiction-supporting-and-the-default-says-opposing-2026-10-13`). The same
//! shape had been found on CAUSES' `basis` on 2026-09-22
//! (`fact:the-causes-edge-carries-a-full-rca-vocabulary-that-no-served-tool-can-set`)
//! and fixed nowhere. This test is the standing check.
//!
//! # What is covered, and what is not
//!
//! COVERED: every tool `writers.json` says draws an edge type (`draws_edge_type`:
//! `review_relations` and the typed edge helpers), and `add_decision`, whose
//! `related_to` draws the review relations through the same per-link shape.
//! The map is the one the server serves `describe_schema`'s `drawn_by` from,
//! and `a_finding_or_refusal_names_the_route_that_works.rs` checks it against
//! what each tool really draws — so a second hand-kept list is not made here.
//!
//! NOT COVERED, on purpose:
//! - `create_edge` / `create_edges` take a free `props` map: every declared
//!   property is already sayable there, and the schema validates it.
//! - `draw_edges` runs the typed helpers above, each item with that helper's
//!   own arguments, so it can say exactly what they can.
//! - Constructors and operations that draw an edge as a side effect of what
//!   they record (`link_artifact`'s IMPLEMENTS, `record_change`'s CHANGED,
//!   `claim_region`'s CLAIMS …). Their edges are bookkeeping the operation
//!   writes, not a relation a caller chooses, and nothing maps which edges
//!   each draws.
//!
//! A NEW TYPED EDGE TOOL is caught by the third test: a write tool named after
//! an edge type must be in the map, so it cannot join the surface unchecked.

use reflow2_mcp::service::*;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// `(tool, edge type, property, why the tool does not expose it)`. One line of
/// reason each. An entry that stops being true — the tool exposes it, or the
/// tool no longer draws the edge, or the default is gone — fails the test as
/// stale, so this list can only shrink honestly.
const EXEMPT: &[(&str, &str, &str, &str)] = &[
    (
        "allocate",
        "ALLOCATED_TO",
        "weight_basis",
        "allocate sets no `weight`, so there is no weight whose basis to state; create_edge sets the two together.",
    ),
    (
        "consumes",
        "CONSUMES",
        "weight_basis",
        "consumes sets no `weight`, so there is no weight whose basis to state; create_edge sets the two together.",
    ),
    (
        "provides",
        "PROVIDES",
        "weight_basis",
        "provides sets no `weight`, so there is no weight whose basis to state; create_edge sets the two together.",
    ),
    (
        "part_of_flow",
        "PART_OF_FLOW",
        "weight_basis",
        "part_of_flow sets no `weight`, so there is no weight whose basis to state; create_edge sets the two together.",
    ),
    (
        "depends_on",
        "DEPENDS_ON",
        "weight_basis",
        "depends_on sets no `weight`, so there is no weight whose basis to state; create_edge sets the two together.",
    ),
    (
        "depends_on",
        "DEPENDS_ON",
        "optional",
        "OPEN, this test's own class: depends_on stores every dependency optional=false; review_relations can say `optional` since 2026-10-13, and this helper waits for its own change.",
    ),
    (
        "violates_rule",
        "VIOLATES_RULE",
        "status",
        "A violation is born `proposed` on purpose; confirming or rejecting it is set_violation_status's ruling, not the proposer's.",
    ),
];

/// Tools whose edge properties are taken inside each item of a list
/// parameter, not at the top level: `(tool, list parameter)`.
const PER_ITEM: &[(&str, &str)] = &[
    ("review_relations", "links"),
    ("add_decision", "related_to"),
];

fn writers() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/writers.json");
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{path} is the map every reader shares: {e}"));
    serde_json::from_str(&text).expect("writers.json is JSON")
}

/// Each covered tool and the edge types it draws.
fn covered() -> BTreeMap<String, Vec<String>> {
    let w = writers();
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (tool, edges) in w["draws_edge_type"].as_object().expect("draws_edge_type") {
        out.insert(
            tool.clone(),
            edges
                .as_array()
                .expect("edge list")
                .iter()
                .map(|e| e.as_str().expect("edge name").to_string())
                .collect(),
        );
    }
    let review = out
        .get("review_relations")
        .cloned()
        .expect("review_relations draws edges");
    out.insert("add_decision".to_string(), review);
    out
}

/// Every property the schema declares WITH A DEFAULT, per edge type.
fn defaulted() -> BTreeMap<String, BTreeMap<String, reflow2_core::foundation::core::PropertyDef>> {
    let schema = reflow2_core::schema::load_schema().expect("schema loads");
    let mut out = BTreeMap::new();
    for (edge, def) in &schema.edge_types {
        let props: BTreeMap<_, _> = def
            .properties
            .iter()
            .filter(|(_, p)| p.default.is_some())
            .map(|(k, p)| (k.clone(), p.clone()))
            .collect();
        if !props.is_empty() {
            out.insert(edge.clone(), props);
        }
    }
    out
}

/// The fields a tool takes for the edge it draws, by name, with their
/// published schemas.
fn fields(tools: &[rmcp::model::Tool], tool: &str) -> BTreeMap<String, Value> {
    let t = tools
        .iter()
        .find(|t| t.name == tool)
        .unwrap_or_else(|| panic!("`{tool}` is not served"));
    let schema = serde_json::to_value(&t.input_schema).expect("schema");
    let props = |v: &Value| -> BTreeMap<String, Value> {
        v["properties"]
            .as_object()
            .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default()
    };
    match PER_ITEM.iter().find(|(t, _)| *t == tool) {
        None => props(&schema),
        Some((_, list)) => {
            let reference = schema["properties"][list]["items"]["$ref"]
                .as_str()
                .unwrap_or_else(|| panic!("{tool}.{list} is a list of a named shape: {schema}"));
            let def = reference.trim_start_matches("#/$defs/");
            props(&schema["$defs"][def])
        }
    }
}

/// Whether a published field can carry every legal value of the property —
/// a field that exists by name but cannot say the value is not exposure.
fn can_say(field: &Value, prop: &reflow2_core::foundation::core::PropertyDef) -> bool {
    use reflow2_core::foundation::core::PropertyType;
    let types: Vec<&str> = match &field["type"] {
        Value::String(s) => vec![s.as_str()],
        Value::Array(a) => a.iter().filter_map(Value::as_str).collect(),
        _ => vec![],
    };
    match prop.prop_type {
        PropertyType::Bool => types.contains(&"boolean"),
        PropertyType::Enum => {
            let published: BTreeSet<&str> = field["enum"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            prop.values
                .iter()
                .flatten()
                .all(|v| published.contains(v.as_str()))
        }
        _ => !types.is_empty(),
    }
}

/// ⭐ THE CLASS GUARD.
#[test]
fn every_defaulted_property_an_edge_tool_draws_is_exposed_or_exempt() {
    let tools = ReflowService::served_tools();
    let defaults = defaulted();
    let exempt: BTreeMap<(&str, &str, &str), &str> = EXEMPT
        .iter()
        .map(|(t, e, p, why)| ((*t, *e, *p), *why))
        .collect();

    let mut gaps = Vec::new();
    let mut exposed_count = 0;
    let mut considered: BTreeSet<(String, String, String)> = BTreeSet::new();
    for (tool, edges) in covered() {
        let fields = fields(&tools, &tool);
        for edge in edges {
            for (prop, def) in defaults.get(&edge).into_iter().flatten() {
                considered.insert((tool.clone(), edge.clone(), prop.clone()));
                let exposed = fields.get(prop).is_some_and(|f| can_say(f, def));
                if exposed {
                    exposed_count += 1;
                    continue;
                }
                if !exempt.contains_key(&(tool.as_str(), edge.as_str(), prop.as_str())) {
                    gaps.push(format!(
                        "{tool} draws {edge}, whose `{prop}` defaults to {:?}, and has no field \
                         that can say otherwise",
                        def.default
                    ));
                }
            }
        }
    }
    assert!(
        gaps.is_empty(),
        "{} defaulted edge propert(ies) a typed tool cannot set, so the default speaks for \
         every writer:\n  {}\n\nGive the tool an optional field for each (validated against \
         the schema's values), or add it to EXEMPT with the one-line reason it must not.",
        gaps.len(),
        gaps.join("\n  ")
    );
    assert!(
        exposed_count >= 10,
        "the sweep found only {exposed_count} exposed propert(ies) — it is reading nothing"
    );

    let stale: Vec<String> = EXEMPT
        .iter()
        .filter(|(t, e, p, _)| {
            let key = (t.to_string(), e.to_string(), p.to_string());
            !considered.contains(&key)
                || fields(&tools, t)
                    .get(*p)
                    .is_some_and(|f| can_say(f, &defaults[*e][*p]))
        })
        .map(|(t, e, p, _)| format!("{t} / {e}.{p}"))
        .collect();
    assert!(
        stale.is_empty(),
        "EXEMPT names {} entr(ies) that are no longer gaps — exposed now, or no longer drawn \
         or defaulted. Remove them: {}",
        stale.len(),
        stale.join(", ")
    );
}

/// The name check above cannot see a field that is accepted and then dropped.
/// For the tool this test was written for, every exposed property is drawn
/// with a NON-default value and read back.
#[tokio::test]
async fn review_relations_stores_every_defaulted_property_it_takes() {
    let s = ReflowService::in_memory_as("defaults").unwrap();
    let ok = |r: Result<rmcp::model::CallToolResult, rmcp::ErrorData>| {
        r.expect("tool ok").structured_content.expect("structured")
    };
    ok(s.add_project(Parameters(
        serde_json::from_value(json!({"id": "proj:p", "name": "P"})).unwrap(),
    ))
    .await);
    let req = |id: &str| -> Value {
        json!({"id": id, "name": id, "statement": format!("The need called {id}.")})
    };
    ok(
        s.add_requirement(Parameters(serde_json::from_value(req("req:from")).unwrap()))
            .await,
    );

    let defaults = defaulted();
    let relations = covered()["review_relations"].clone();
    let mut checked = 0;
    for relation in &relations {
        for (prop, def) in defaults.get(relation).into_iter().flatten() {
            let other: Value = match (&def.values, &def.default) {
                (Some(values), Some(d)) => json!(
                    values
                        .iter()
                        .find(|v| Some(v.as_str()) != d.as_str())
                        .expect("an enum with a second value")
                ),
                (None, Some(reflow2_core::foundation::core::Value::Bool(b))) => json!(!b),
                _ => panic!("{relation}.{prop}: no non-default value to draw"),
            };
            let to = format!("req:to-{}-{prop}", relation.to_lowercase());
            ok(
                s.add_requirement(Parameters(serde_json::from_value(req(&to)).unwrap()))
                    .await,
            );
            let mut link = json!({"relation": relation, "other_id": to,
                                  "evidence": "drawn by the conformance sweep"});
            link[prop] = other.clone();
            let args: ReviewRelationsReq =
                serde_json::from_value(json!({"node_id": "req:from", "links": [link]}))
                    .unwrap_or_else(|e| panic!("review_relations takes `{prop}`: {e}"));
            ok(s.review_relations(Parameters(args)).await);

            let got = ok(s
                .get_node(Parameters(
                    serde_json::from_value(json!({"id": "req:from", "include_edges":
                        {"direction": "out", "edge_types": [relation]}}))
                    .unwrap(),
                ))
                .await);
            let stored = got["edges"]["items"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|e| e["to_id"] == json!(to))
                .map(|e| e["properties"][prop].clone())
                .unwrap_or(Value::Null);
            assert_eq!(
                stored, other,
                "{relation}.{prop} drawn as {other} was stored as {stored}"
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 6,
        "only {checked} defaulted review-relation properties were drawn"
    );
}

/// A write tool named after an edge type is a typed edge helper, and must be
/// in the map the first test covers — otherwise it joins the surface unchecked.
#[test]
fn a_tool_named_after_an_edge_type_is_in_the_map() {
    let schema = reflow2_core::schema::load_schema().expect("schema loads");
    let map = covered();
    let missing: Vec<String> = ReflowService::served_tools()
        .iter()
        .filter(|t| {
            t.annotations
                .as_ref()
                .and_then(|a| a.read_only_hint)
                .is_some_and(|ro| !ro)
        })
        .map(|t| t.name.to_string())
        .filter(|n| schema.edge_types.contains_key(&n.to_uppercase()) && !map.contains_key(n))
        .collect();
    assert!(
        missing.is_empty(),
        "write tool(s) named after an edge type are not in writers.json's draws_edge_type: {}",
        missing.join(", ")
    );
}
