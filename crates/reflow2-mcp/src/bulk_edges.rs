//! `draw_edges` — the bulk form of EVERY typed edge helper, running each
//! helper's own body per item.
//!
//! # The finding this answers
//!
//! `fact:root-cause-a-bulk-edge-form-was-offered-and-no-typed-helper-names-it-2026-09-29`
//! (I24 of `art:dev-reflow2-two-agent-exercise-feedback-2026-09-29`). A designer
//! agent built one design through the flo2 connector in about 270 single-edge
//! calls. `create_edges` was reachable for the thin helpers and NO helper named
//! it. For the helpers that carry checks of their own — `constrains`
//! (contribution, unit, basis, source), `governed_by` (its ruling), `authored_by` (the role set) — there was no bulk route
//! that kept those checks: `create_edges` takes free `props` and runs none of
//! them.
//!
//! # The shape, and why the other options lost
//!
//! Each item names a typed helper and carries THAT helper's arguments. The item
//! runs the helper's own body (`<helper>_on`, the same function the tool itself
//! calls after taking the lock), so its argument parsing, its checks, its
//! refusal words and its reply are the helper's by construction — there is no
//! second copy of any check to drift. All items run inside one atomic batch
//! ([`reflow2_core::DesignGraph::atomically`]): all or nothing, every failure
//! named, `check_only` writes nothing, and a later item reads an earlier one's
//! write exactly as a second call would.
//!
//! Option (b) of `dec:idea-every-typed-edge-helper-has-a-bulk-form-that-keeps-its-checks`
//! as posed — `create_edges` routing each edge TYPE to a helper — needs a map
//! from each edge type's free `props` to each helper's argument names, kept by
//! hand, and an edge type drawn by several helpers (CONTAINS by `contains`,
//! `contain_component`, `move_component`) has no single helper to route to.
//! (c), list-taking helpers, multiplies ~32 schemas. (d), a component written
//! with its structural edges, is a larger capture change and is left to
//! `req:the-mcp-surface-is-sized-for-tokens-per-task`. (a) alone, naming
//! `create_edges`, fixes discoverability for the thin helpers and nothing for
//! the rest. So: one tool, per-item helper names, the helper's own body — and
//! (a) is done too, generated: every helper's LISTED description names this
//! form ([`name_the_bulk_route`]).
//!
//! # Honest limits
//!
//! - [`HELPERS`] is a list kept here, because a match needs arms. It is not
//!   the source of truth: `a_bulk_edge_write_keeps_each_helpers_checks`
//!   enumerates the helpers from `writers.json` and the served tool list, and
//!   fails when a helper neither names this form nor takes a list itself.
//! - `review_relations` and `release_includes_all` are absent on purpose: each
//!   already takes a list, so each is its own bulk form.

use reflow2_core::DesignGraph;
use reflow2_core::bulk::BulkReport;
use rmcp::ErrorData as McpError;
use rmcp::model::{CallToolResult, Tool};
use schemars::JsonSchema;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value as JsonValue, json};

use crate::service::ReflowService;

/// Every typed edge helper `draw_edges` runs, by served tool name.
pub(crate) const HELPERS: &[&str] = &[
    "allocate",
    "answers",
    "authored_by",
    "calibrated_against",
    "complies_with",
    "constrains",
    "consumes",
    "contain_component",
    "contains",
    "decomposes",
    "depends_on",
    "deploy_to",
    "documents",
    "gate_on",
    "governed_by",
    "imposes",
    "invalidates",
    "move_component",
    "operates_in",
    "owned_by",
    "part_of_flow",
    "performed_in",
    "pin_at_epoch",
    "precedes",
    "provides",
    "realizes",
    "release_includes",
    "require_resource",
    "satisfies",
    "schedule_for",
    "verifies",
    "violates_rule",
];

/// Append the bulk route to every helper's LISTED description — generated
/// here from [`HELPERS`], so no description carries a hand-written copy.
///
/// Applied at list time rather than on the router, because `find_tools` ranks
/// the router's descriptions: one shared sentence on 32 tools measurably moved
/// five of them off first place for their own job.
pub(crate) fn name_the_bulk_route(mut tools: Vec<Tool>) -> Vec<Tool> {
    for t in &mut tools {
        if HELPERS.contains(&t.name.as_ref()) {
            let base = t.description.as_deref().unwrap_or_default();
            t.description = Some(
                format!(
                    "{base} MANY AT ONCE: `draw_edges`, items {{\"tool\": \"{}\", \
                     \"arguments\": {{…}}}}, same checks, all or nothing.",
                    t.name
                )
                .into(),
            );
        }
    }
    tools
}

/// One item of a `draw_edges` call.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DrawEdgeItem {
    /// The typed edge helper to run, by its tool name (`allocate`,
    /// `constrains`, `governed_by`, …).
    pub tool: String,
    /// That helper's own arguments, exactly as the helper takes them.
    #[serde(default)]
    pub arguments: JsonValue,
}

/// `draw_edges` arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DrawEdgesReq {
    /// The edges to draw, one typed helper call per item, applied together.
    pub edges: Vec<DrawEdgeItem>,
    /// Run every item's checks and WRITE NOTHING.
    #[serde(default)]
    pub check_only: bool,
    /// Largest reply, in characters; the default budget otherwise.
    #[serde(default)]
    pub budget_chars: Option<usize>,
}

fn parse<T: DeserializeOwned>(tool: &str, args: JsonValue) -> Result<T, McpError> {
    serde_json::from_value(args).map_err(|e| {
        McpError::invalid_params(
            format!("`{tool}`: failed to deserialize parameters: {e}"),
            None,
        )
    })
}

/// Run ONE typed helper's own body on a graph the caller holds.
fn run_helper(
    g: &mut DesignGraph,
    tool: &str,
    args: JsonValue,
) -> Result<CallToolResult, McpError> {
    macro_rules! helpers {
        ($($name:literal => $on:ident),* $(,)?) => {
            match tool {
                $($name => ReflowService::$on(g, parse(tool, args)?),)*
                other => Err(McpError::invalid_params(
                    format!(
                        "`{other}` is not a typed edge helper `draw_edges` runs. It runs: {}. \
                         For an edge no typed helper draws, use `create_edges`.",
                        HELPERS.join(", ")
                    ),
                    None,
                )),
            }
        };
    }
    helpers! {
        "allocate" => allocate_on,
        "answers" => answers_on,
        "authored_by" => authored_by_on,
        "calibrated_against" => calibrated_against_on,
        "complies_with" => complies_with_on,
        "constrains" => constrains_on,
        "consumes" => consumes_on,
        "contain_component" => contain_component_on,
        "contains" => contains_on,
        "decomposes" => decomposes_on,
        "depends_on" => depends_on_on,
        "deploy_to" => deploy_to_on,
        "documents" => documents_on,
        "gate_on" => gate_on_on,
        "governed_by" => governed_by_on,
        "imposes" => imposes_on,
        "invalidates" => invalidates_on,
        "move_component" => move_component_on,
        "operates_in" => operates_in_on,
        "owned_by" => owned_by_on,
        "part_of_flow" => part_of_flow_on,
        "performed_in" => performed_in_on,
        "pin_at_epoch" => pin_at_epoch_on,
        "precedes" => precedes_on,
        "provides" => provides_on,
        "realizes" => realizes_on,
        "release_includes" => release_includes_on,
        "require_resource" => require_resource_on,
        "satisfies" => satisfies_on,
        "schedule_for" => schedule_for_on,
        "verifies" => verifies_on,
        "violates_rule" => violates_rule_on,
    }
}

/// One item, run: the helper's reply as the helper would send it, or its
/// refusal in the helper's own words.
pub(crate) fn draw_one(g: &mut DesignGraph, item: &DrawEdgeItem) -> Result<JsonValue, String> {
    let r = run_helper(g, &item.tool, item.arguments.clone()).map_err(|e| e.message.to_string())?;
    let text = || {
        r.content
            .iter()
            .filter_map(|c| c.as_text().map(|t| t.text.clone()))
            .collect::<Vec<_>>()
            .join("\n")
    };
    if r.is_error.unwrap_or(false) {
        return Err(text());
    }
    Ok(r.structured_content
        .clone()
        .unwrap_or_else(|| json!({ "value": text() })))
}

/// The edge a helper's reply describes, as a sentence with its subject first —
/// the shape every constructor's reply now uses (`crate::drawn_edges`).
fn sentence_of(reply: &JsonValue) -> Option<String> {
    let s = |k: &str| reply.get(k).and_then(JsonValue::as_str);
    Some(crate::drawn_edges::sentence(
        s("from_id")?,
        s("edge_type")?,
        s("to_id")?,
    ))
}

/// The `draw_edges` reply for a batch report over `(index, item)` pairs.
pub(crate) fn reply(
    report: BulkReport<(usize, String, JsonValue)>,
    budget: Option<usize>,
) -> JsonValue {
    let failures: Vec<JsonValue> = report
        .failures
        .iter()
        .map(|f| json!({ "index": f.index, "tool": f.id, "error": f.error }))
        .collect();
    let sentences: Vec<String> = report
        .written
        .iter()
        .filter_map(|(_, _, r)| sentence_of(r))
        .collect();
    let written: Vec<JsonValue> = report
        .written
        .into_iter()
        .map(|(index, tool, reply)| json!({ "index": index, "tool": tool, "reply": reply }))
        .collect();
    let v = if report.check_only {
        json!({
            "check_only": true,
            "applied": false,
            "would_apply": failures.is_empty(),
            "would_draw": sentences,
            "failures": failures,
            "note": "every item ran its helper's own checks and nothing was written",
        })
    } else if report.applied {
        json!({
            "applied": true,
            "drawn": sentences,
            "written": written,
            "failures": failures,
        })
    } else {
        json!({
            "applied": false,
            "failures": failures,
            "note": "NOTHING was written: fix the listed items and send the whole batch again — \
                     a bulk write is all or nothing",
        })
    };
    crate::reply_budget::bound_reply(
        v,
        budget.unwrap_or(crate::reply_budget::DEFAULT_REPLY_BUDGET_CHARS),
        "Each item's own reply is what the helper would have sent; ask for a smaller batch to \
         read them in full.",
    )
}
