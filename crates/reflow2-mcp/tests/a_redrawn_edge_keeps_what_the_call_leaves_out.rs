//! Re-drawing an edge through a typed helper keeps every property the call
//! leaves out — one rule across every helper `draw_edges` runs.
//!
//! # Why
//!
//! The store replaces an edge's properties whole, so a typed helper re-drawn
//! with fewer arguments silently erased the rest. Phase 2 (#771) made
//! `satisfies` keep them (`DesignGraph::draw_edge_keeping`), so a `partial`
//! coverage no longer read as `full` delivery after a re-draw that only added
//! an `evidence`. `depends_on` kept them only when it was passed `argued_by` —
//! the field text says "Re-drawing an existing edge keeps every property the
//! call leaves out", and without `argued_by` it replaced them all, the
//! argument included. Every other helper replaced: an allocation's `weight`, a
//! check's `coverage`, a realization's `completeness`, a governing `note`.
//!
//! # The class
//!
//! For EVERY helper `draw_edges` runs that writes an edge with properties to
//! keep: draw it bare, put a declared property on it the bare call does not
//! name, draw it bare again, and the property must still be there. Excluded,
//! with the reason: `move_component` and `contain_component` MOVE a child
//! rather than re-draw an edge, and the five helpers whose edge declares no
//! property at all (`imposes`, `operates_in`, `performed_in`, `pin_at_epoch`,
//! `precedes`) have nothing to keep — the table below lists them, so a new
//! property on one of those edges fails `the_table_names_every_helper` until it
//! is given a row.
//!
//! The generic `create_edge` is NOT a typed helper and still replaces: it is the
//! write an agent uses to set an edge's whole property bag.

use reflow2_mcp::service::*;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

async fn ok(
    s: &ReflowService,
    what: &str,
    r: Result<rmcp::model::CallToolResult, rmcp::ErrorData>,
) -> Value {
    let _ = s;
    let r = r.unwrap_or_else(|e| panic!("{what}: {}", e.message));
    assert!(!r.is_error.unwrap_or(false), "{what}: {r:?}");
    r.structured_content.unwrap_or(Value::Null)
}

async fn node(s: &ReflowService, t: &str, id: &str, props: Value) {
    let mut p = json!({"name": format!("{t} {id}")});
    for (k, v) in props.as_object().expect("object") {
        p[k] = v.clone();
    }
    let r = s
        .create_node(Parameters(
            serde_json::from_value(json!({"node_type": t, "id": id, "props": p})).expect("req"),
        ))
        .await;
    ok(s, &format!("create {t} {id}"), r).await;
}

async fn seed() -> ReflowService {
    let s = ReflowService::in_memory().expect("in-memory service");
    s.add_contributor(Parameters(
        ContributorReq {
            id: "who:ann".into(),
            name: Some("Ann".into()),
            kind: None,
            handle: None,
            description: None,
        }
        .into(),
    ))
    .await
    .expect("contributor");
    node(&s, "Project", "proj:a", json!({"description": "p"})).await;
    node(&s, "Requirement", "req:a", json!({"statement": "a"})).await;
    node(&s, "Requirement", "req:b", json!({"statement": "b"})).await;
    node(&s, "Capability", "cap:a", json!({"description": "c"})).await;
    node(&s, "Component", "cmp:a", json!({"purpose": "x"})).await;
    node(&s, "Component", "cmp:b", json!({"purpose": "y"})).await;
    node(&s, "Interface", "if:a", json!({"description": "i"})).await;
    node(&s, "Artifact", "art:a", json!({"artifact_type": "code"})).await;
    node(&s, "Verification", "ver:a", json!({"method": "test"})).await;
    node(&s, "Environment", "env:a", json!({})).await;
    node(
        &s,
        "EnvironmentRule",
        "envrule:a",
        json!({"statement": "r"}),
    )
    .await;
    node(&s, "Constraint", "con:a", json!({"statement": "k"})).await;
    node(&s, "Release", "rel:a", json!({"version": "1.0.0"})).await;
    node(&s, "DesignEpoch", "epoch:a", json!({"sequence": 1})).await;
    node(&s, "Flow", "flow:a", json!({"description": "f"})).await;
    node(&s, "Question", "q:a", json!({"question": "why?"})).await;
    node(&s, "Decision", "dec:a", json!({"decision": "d"})).await;
    node(&s, "Resource", "res:a", json!({"description": "r"})).await;
    node(
        &s,
        "TemporalFact",
        "fact:a",
        json!({"statement": "s", "subject_id": "cap:a"}),
    )
    .await;
    s
}

/// (helper, bare arguments, edge type, from, to, property to keep, its value)
type Row = (
    &'static str,
    fn() -> Value,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    fn() -> Value,
);

const ROWS: &[Row] = &[
    (
        "allocate",
        || json!({"from_id": "cap:a", "to_id": "cmp:a"}),
        "ALLOCATED_TO",
        "cap:a",
        "cmp:a",
        "weight",
        || json!(0.5),
    ),
    (
        "answers",
        || json!({"from_id": "dec:a", "question_id": "q:a"}),
        "ANSWERS",
        "dec:a",
        "q:a",
        "note",
        || json!("kept"),
    ),
    (
        "authored_by",
        || json!({"from_id": "req:a", "contributor_id": "who:ann"}),
        "AUTHORED_BY",
        "req:a",
        "who:ann",
        "reviewed_at",
        || json!("2026-10-01"),
    ),
    (
        "calibrated_against",
        || json!({"from_id": "cap:a", "evidence_id": "art:a"}),
        "CALIBRATED_AGAINST",
        "cap:a",
        "art:a",
        "note",
        || json!("kept"),
    ),
    (
        "complies_with",
        || json!({"element_id": "cmp:a", "rule_id": "envrule:a"}),
        "COMPLIES_WITH",
        "cmp:a",
        "envrule:a",
        "evidence",
        || json!("kept"),
    ),
    (
        "constrains",
        || json!({"constraint_id": "con:a", "target_id": "cmp:a"}),
        "CONSTRAINS",
        "con:a",
        "cmp:a",
        "note",
        || json!("kept"),
    ),
    (
        "consumes",
        || json!({"from_id": "cmp:a", "to_id": "if:a"}),
        "CONSUMES",
        "cmp:a",
        "if:a",
        "weight",
        || json!(0.5),
    ),
    (
        "contains",
        || json!({"project_id": "proj:a", "child_id": "cmp:a"}),
        "CONTAINS",
        "proj:a",
        "cmp:a",
        "note",
        || json!("kept"),
    ),
    (
        "decomposes",
        || json!({"from_id": "req:b", "to_id": "req:a"}),
        "DECOMPOSES",
        "req:b",
        "req:a",
        "note",
        || json!("kept"),
    ),
    (
        "depends_on",
        || json!({"from_id": "cmp:a", "to_id": "cmp:b"}),
        "DEPENDS_ON",
        "cmp:a",
        "cmp:b",
        "dependency_type",
        || json!("data_flow"),
    ),
    (
        "deploy_to",
        || json!({"release_id": "rel:a", "environment_id": "env:a"}),
        "DEPLOYED_TO",
        "rel:a",
        "env:a",
        "status",
        || json!("active"),
    ),
    (
        "documents",
        || json!({"artifact_id": "art:a", "target_id": "cap:a"}),
        "DOCUMENTS",
        "art:a",
        "cap:a",
        "doc_kind",
        || json!("readme"),
    ),
    // `gate_on` and `verifies` were typed helpers until 2026-10-30, when each
    // became a counted refusing stub; `draw_edges` runs neither. VERIFIES is
    // drawn by `add_verification`'s `verifies`, through the same core write
    // these rows held (`DesignGraph::verifies`).
    (
        "governed_by",
        || json!({"from_id": "req:a", "to_id": "dec:a"}),
        "GOVERNED_BY",
        "req:a",
        "dec:a",
        "note",
        || json!("kept"),
    ),
    (
        "invalidates",
        || json!({"from_id": "cap:a", "finding_id": "fact:a"}),
        "INVALIDATES",
        "cap:a",
        "fact:a",
        "note",
        || json!("kept"),
    ),
    (
        "owned_by",
        || json!({"from_id": "cap:a", "contributor_id": "who:ann"}),
        "OWNED_BY",
        "cap:a",
        "who:ann",
        "note",
        || json!("kept"),
    ),
    (
        "part_of_flow",
        || json!({"capability_id": "cap:a", "flow_id": "flow:a"}),
        "PART_OF_FLOW",
        "cap:a",
        "flow:a",
        "weight",
        || json!(0.5),
    ),
    (
        "provides",
        || json!({"from_id": "cmp:a", "to_id": "if:a"}),
        "PROVIDES",
        "cmp:a",
        "if:a",
        "weight",
        || json!(0.5),
    ),
    (
        "realizes",
        || json!({"artifact_id": "art:a", "target_id": "cap:a"}),
        "REALIZES",
        "art:a",
        "cap:a",
        "completeness",
        || json!("partial"),
    ),
    (
        "release_includes",
        || json!({"release_id": "rel:a", "target_id": "art:a"}),
        "INCLUDES",
        "rel:a",
        "art:a",
        "as_checksum",
        || json!("abc123"),
    ),
    (
        "require_resource",
        || json!({"from_id": "cmp:a", "resource_id": "res:a"}),
        "REQUIRES_RESOURCE",
        "cmp:a",
        "res:a",
        "criticality",
        || json!("required"),
    ),
    (
        "satisfies",
        || json!({"from_id": "cap:a", "to_id": "req:a"}),
        "SATISFIES",
        "cap:a",
        "req:a",
        "coverage",
        || json!("partial"),
    ),
    (
        "schedule_for",
        || json!({"item_id": "req:a", "target_id": "epoch:a"}),
        "SCHEDULED_FOR",
        "req:a",
        "epoch:a",
        "recorded_at",
        || json!("2026-10-01"),
    ),
    (
        "violates_rule",
        || json!({"element_id": "cmp:a", "rule_id": "envrule:a"}),
        "VIOLATES_RULE",
        "cmp:a",
        "envrule:a",
        "rationale",
        || json!("kept"),
    ),
];

/// Helpers with nothing to keep, or that move rather than re-draw.
const NOT_REDRAWS: &[(&str, &str)] = &[
    ("imposes", "IMPOSES declares no property"),
    ("operates_in", "OPERATES_IN declares no property"),
    ("performed_in", "PERFORMED_IN declares no property"),
    ("pin_at_epoch", "AT_EPOCH declares no property"),
    ("precedes", "PRECEDES declares no property"),
    (
        "move_component",
        "moves a child to a new parent; not a re-draw",
    ),
    (
        "contain_component",
        "moves a child to a new parent; not a re-draw",
    ),
];

async fn draw(s: &ReflowService, tool: &str, args: Value) -> Value {
    let r = s
        .draw_edges(Parameters(
            serde_json::from_value(json!({"edges": [{"tool": tool, "arguments": args}]}))
                .expect("req"),
        ))
        .await;
    ok(s, &format!("draw {tool}"), r).await
}

async fn edge_props(
    s: &ReflowService,
    ty: &str,
    from: &str,
    to: &str,
) -> serde_json::Map<String, Value> {
    let d = s
        .export_graph(Parameters(ExportGraphToReq {
            path: None,
            overwrite: None,
            accept_divergence: None,
            stamps: None,
        }))
        .await
        .expect("export")
        .structured_content
        .expect("structured");
    d["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .find(|e| e["edge_type"] == ty && e["from_id"] == from && e["to_id"] == to)
        .unwrap_or_else(|| panic!("no {from} {ty} {to}"))["properties"]
        .as_object()
        .cloned()
        .unwrap_or_default()
}

#[tokio::test]
async fn every_typed_helper_keeps_what_a_redraw_leaves_out() {
    let mut bad = Vec::new();
    for (tool, bare, ty, from, to, prop, value) in ROWS {
        let s = seed().await;
        draw(&s, tool, bare()).await;
        // Put a declared property on it that the bare call does not name.
        let mut props = edge_props(&s, ty, from, to).await;
        props.insert(prop.to_string(), value());
        let r = s
            .create_edge(Parameters(
                serde_json::from_value(json!({
                    "edge_type": ty, "from_id": from, "to_id": to, "props": props
                }))
                .expect("req"),
            ))
            .await;
        ok(&s, &format!("plant {prop} on {ty}"), r).await;
        assert_eq!(
            edge_props(&s, ty, from, to).await.get(*prop),
            Some(&value())
        );
        // The same bare call again.
        draw(&s, tool, bare()).await;
        let kept = edge_props(&s, ty, from, to).await;
        if kept.get(*prop) != Some(&value()) {
            bad.push(format!(
                "{tool}: a bare re-draw dropped {ty}.{prop} (now {kept:?})"
            ));
        }
    }
    assert!(
        bad.is_empty(),
        "{} helpers replace:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

/// The reported case, named: `depends_on` without `argued_by` keeps the claim
/// an earlier call named.
#[tokio::test]
async fn depends_on_without_an_argument_keeps_the_one_it_had() {
    let s = seed().await;
    node(
        &s,
        "Requirement",
        "req:claim",
        json!({"statement": "cmp:a needs cmp:b"}),
    )
    .await;
    draw(
        &s,
        "depends_on",
        json!({"from_id": "cmp:a", "to_id": "cmp:b", "argued_by": "req:claim"}),
    )
    .await;
    draw(
        &s,
        "depends_on",
        json!({"from_id": "cmp:a", "to_id": "cmp:b"}),
    )
    .await;
    assert_eq!(
        edge_props(&s, "DEPENDS_ON", "cmp:a", "cmp:b")
            .await
            .get("argued_by"),
        Some(&json!("req:claim"))
    );
}

/// What a call DOES name still wins.
#[tokio::test]
async fn a_property_the_call_names_overrides_the_one_kept() {
    let s = seed().await;
    draw(
        &s,
        "deploy_to",
        json!({"release_id": "rel:a", "environment_id": "env:a", "status": "planned"}),
    )
    .await;
    draw(
        &s,
        "deploy_to",
        json!({"release_id": "rel:a", "environment_id": "env:a", "status": "active"}),
    )
    .await;
    assert_eq!(
        edge_props(&s, "DEPLOYED_TO", "rel:a", "env:a")
            .await
            .get("status"),
        Some(&json!("active"))
    );
}

/// Every helper `draw_edges` runs is either a row above or excluded with its
/// reason, so a new helper cannot skip the rule unnoticed.
#[test]
fn the_table_names_every_helper() {
    let mut named: Vec<&str> = ROWS.iter().map(|r| r.0).collect();
    named.extend(NOT_REDRAWS.iter().map(|r| r.0));
    named.sort_unstable();
    let mut served: Vec<&str> = reflow2_mcp::bulk_edges::HELPERS.to_vec();
    served.sort_unstable();
    assert_eq!(named, served);
}
