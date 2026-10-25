//! A NULL UNSETS at every typed constructor, and a field left out keeps its
//! value — asked of every constructor the server serves, over every optional
//! property it takes (`dec:null-unsets-on-every-typed-constructor`, Anthony
//! 2026-10-24: "null unsets everywhere").
//!
//! # The failure
//!
//! The Shop sample trial (2026-10-24) sent `limit: null` to `add_constraint`
//! to clear a numeric limit, and the limit stayed: a typed request reads an
//! explicit `null` and an absent key as the same `None`, so the constructor's
//! merge kept the stored value. The 2026-09-07 rule (v0.51.0, "a null UNSETS
//! rather than being refused") had been built into `create_node`'s props bag
//! only — pinned there by `the_v0510_field_feedback_fixes_land.rs` — and never
//! reached the typed doors.
//!
//! # ⭐ Why the membership is read off the served surface
//!
//! The constructors are every served `add_*` tool and every tool that promises
//! "optional to revise" (`record_finding`, `plan_epoch`). A constructor added
//! tomorrow joins the moment it is served, and fails here until it unsets
//! what it is sent as null — so a new door cannot skip the rule the way the
//! typed doors skipped it for seven weeks. The properties come from the
//! schema the server itself loads: every field a constructor takes that its
//! node type declares as a property is asked, so a new optional field joins
//! the same way.
//!
//! The create each case starts from is the one
//! `a_revise_keeps_what_it_was_not_passed.rs` synthesises, plus the list
//! properties and the two constructors that file does not reach
//! (`add_environment_rule`, `add_inference`), whose values are below.
//!
//! # What one case asserts
//!
//! 1. A revise passing only the id (and what the tool's schema requires on
//!    every call) changes no stored property: left out keeps.
//! 2. For each optional property the create stored, a revise sending it as
//!    `null` removes it — or puts back its schema default, exactly as
//!    `create_node`'s null does — names it under `unset` in the reply, and
//!    leaves every other stored property as it was.
//! 3. For each REQUIRED property the create sent — the schema's, or one the
//!    constructor needs to create the node, which a create without it is
//!    refused for — a `null` is refused naming the field, and nothing is
//!    written.
//! 4. After each null, the node can still be revised by its id alone.
//! 5. Anthony's ruling, 2026-10-25 ("Go with your recommendation on null
//!    unsets"): a null on a property the schema calls ADDITIVE history
//!    (`distinct_from`) or on a `status` is REFUSED by name and writes
//!    nothing; a status refusal names the `set_*_status` tool that moves it.

use crate::a_revise_keeps_what_it_was_not_passed as harness;
use serde_json::{Map, Value, json};

/// List-valued properties the generic create leaves out, with a value each
/// one's validator accepts. Keyed `(param, value)`; applied wherever a
/// constructor takes the param as a declared list property.
fn list_values() -> Vec<(&'static str, Value)> {
    vec![
        ("variables", json!(["n"])),
        ("fixes", json!(["n=40"])),
        ("silent_on", json!(["req:seed"])),
        ("distinct_from", json!(["req:seed"])),
        ("steps", json!(["add_requirement"])),
        ("units", json!(["mass=kg"])),
        ("decomposition_levels", json!(["component", "system"])),
    ]
}

/// Per-tool values the generic create cannot synthesise: the two constructors
/// the revise-keeps harness does not reach, which need nodes that exist.
fn extra_context(tool: &str) -> Vec<(&'static str, Value)> {
    match tool {
        "add_inference" => vec![
            ("rule", json!("all")),
            ("premises", json!(["req:seed"])),
            ("conclusion", json!("req:null-case-conclusion")),
        ],
        _ => Vec::new(),
    }
}

/// Properties a constructor takes that this test does not ask, and why. Each
/// is a structural field of a proof step whose value must name parts of a
/// proof (an assumed premise, a sibling step, a case split) that a generic
/// create cannot build; the rule reaches them through the same
/// `unset_nulled` as every other property of `add_inference`, which `k`,
/// `name`, `warrant`, `rule_name` and the confirmation fields pin here.
const NOT_ASKED: &[(&str, &str, &str)] = &[
    ("add_inference", "cases", "names assumed case premises"),
    ("add_inference", "discharges", "names an assumed premise"),
    ("add_inference", "independent_of", "names another step"),
    (
        "add_inference",
        "independent_except",
        "names lemmas of another step",
    ),
    (
        "add_inference",
        "generalizes_over",
        "needs a variable the conclusion mentions",
    ),
    (
        "add_inference",
        "cases_exhaustive",
        "names the step that shows the split",
    ),
    ("add_inference", "rule_node", "names a rule node of its own"),
    (
        "add_inference",
        "establishes",
        "names a node the step establishes",
    ),
    (
        "add_inference",
        "conclusion_form",
        "rides a proof's conclusion",
    ),
];

/// Properties whose default is COMPUTED rather than declared in the schema:
/// a null puts back the computed value, so the case asserts only that the
/// value the create stored is gone and something stands in its place.
const COMPUTED_DEFAULT: &[(&str, &str, &str)] = &[(
    "add_inference",
    "name",
    "a step with no name is named from its rule and conclusion",
)];

/// The served tool that moves a status on `node_type`, when there is one:
/// `set_<type>_status` (`set_epoch_status` for a DesignEpoch).
fn status_setter(node_type: &str, served: &[String]) -> Option<String> {
    let mut snake = String::new();
    for (i, ch) in node_type.chars().enumerate() {
        if ch.is_ascii_uppercase() && i > 0 {
            snake.push('_');
        }
        snake.push(ch.to_ascii_lowercase());
    }
    let snake = snake.strip_prefix("design_").unwrap_or(&snake).to_string();
    let name = format!("set_{snake}_status");
    served.contains(&name).then_some(name)
}

/// The served constructors, with their input schemas.
async fn constructors(c: &harness::Client) -> Vec<(String, Value)> {
    let tools = c.list_all_tools().await.expect("tools/list");
    let mut out: Vec<(String, Value)> = tools
        .into_iter()
        .filter(|t| {
            t.name.starts_with("add_")
                || t.description
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains("optional to revise")
        })
        .map(|t| {
            (
                t.name.to_string(),
                serde_json::to_value(&t.input_schema).expect("schema"),
            )
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// The `unset` list a reply carries — on the node itself, or on the node a
/// reply nests (`event`, `finding`).
fn reply_unset(v: &Value) -> Vec<String> {
    let pick = |x: &Value| -> Option<Vec<String>> {
        x.get("unset")?.as_array().map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
    };
    if let Some(u) = pick(v) {
        return u;
    }
    v.as_object()
        .into_iter()
        .flat_map(|o| o.values())
        .find_map(pick)
        .unwrap_or_default()
}

fn without(m: &Map<String, Value>, k: &str) -> Map<String, Value> {
    let mut m = m.clone();
    m.remove(k);
    m
}

#[tokio::test]
async fn every_typed_constructor_unsets_a_property_sent_as_null_and_keeps_one_left_out() {
    let c = harness::connect().await;
    harness::seed(&c).await;
    harness::call(
        &c,
        "add_requirement",
        json!({"id": "req:null-case-conclusion", "name": "Qzv conclusion wbx",
               "statement": "a conclusion an inference case can reach"}),
    )
    .await
    .expect("seed conclusion");
    let schema = reflow2_core::DesignGraph::open_in_memory()
        .expect("in-memory graph")
        .schema()
        .clone();

    let tools = constructors(&c).await;
    let served: Vec<String> = c
        .list_all_tools()
        .await
        .expect("tools/list")
        .into_iter()
        .map(|t| t.name.to_string())
        .collect();
    assert!(
        tools.len() >= 23,
        "only {} constructors served — the membership moved and this test would go vacuous: \
         {:?}",
        tools.len(),
        tools.iter().map(|t| &t.0).collect::<Vec<_>>()
    );

    let mut broken: Vec<String> = Vec::new();
    for (tool, tool_schema) in &tools {
        let params = tool_schema["properties"]
            .as_object()
            .cloned()
            .unwrap_or_default();
        let required_params: Vec<String> = tool_schema["required"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        let mut create = harness::create_args(tool, tool_schema, 0);
        for (p, v) in list_values() {
            if params.contains_key(p) {
                create.insert(p.into(), v);
            }
        }
        for (p, v) in extra_context(tool) {
            create.insert(p.into(), v);
        }
        if tool == "add_inference" {
            create.remove("k");
        }
        for (t, f, _) in NOT_ASKED {
            if t == tool {
                create.remove(*f);
            }
        }
        let id = create["id"].as_str().unwrap().to_string();
        if let Err(e) = harness::call(&c, tool, Value::Object(create.clone())).await {
            broken.push(format!(
                "{tool}: the create this test synthesised was REFUSED, so the rule could not be \
                 asked. Add what it needs above.\n  args: {}\n  refusal: {e}",
                Value::Object(create.clone())
            ));
            continue;
        }
        let node = harness::call(&c, "get_node", json!({ "id": id }))
            .await
            .unwrap_or_else(|e| panic!("get_node {id}: {e}"));
        let node_type = node["node"]["node_type"].as_str().unwrap().to_string();
        let def = schema
            .node_types
            .get(&node_type)
            .unwrap_or_else(|| panic!("{tool}: writes {node_type}, which the schema lacks"));
        let base = |extra: (&str, Value)| -> Value {
            let mut args = Map::new();
            args.insert("id".into(), json!(id));
            for r in &required_params {
                if let Some(v) = create.get(r) {
                    args.insert(r.clone(), v.clone());
                }
            }
            args.insert(extra.0.into(), extra.1);
            Value::Object(args)
        };

        // 1. Left out keeps.
        let before = harness::stored(&c, &id).await;
        let keep = base(("id", json!(id)));
        if let Err(e) = harness::call(&c, tool, keep.clone()).await {
            broken.push(format!(
                "{tool}: a revise passing only {keep} was refused: {e}"
            ));
            continue;
        }
        let after = harness::stored(&c, &id).await;
        if before != after {
            broken.push(format!(
                "{tool}: a revise that left every optional field out CHANGED the node\n  \
                 before: {}\n  after:  {}",
                Value::Object(before),
                Value::Object(after)
            ));
        }

        // 2. Null unsets, field by field. 3. A required property refuses.
        let mut asked = 0usize;
        for k in create.keys() {
            if k == "id" || required_params.contains(k) {
                continue;
            }
            let Some(p) = def.properties.get(k.as_str()) else {
                continue; // an edge, a signature, a hint: not a stored value
            };
            if NOT_ASKED.iter().any(|(t, f, _)| t == tool && f == k) {
                continue;
            }
            let before = harness::stored(&c, &id).await;
            if p.required {
                match harness::call(&c, tool, base((k, Value::Null))).await {
                    Ok(v) => broken.push(format!(
                        "{tool}: `{k}: null` on a REQUIRED property was accepted: {v}"
                    )),
                    Err(e) if !(e.contains(&format!("`{k}`")) && e.contains("REQUIRED")) => broken
                        .push(format!(
                            "{tool}: `{k}: null` on a required property was refused without \
                             naming it as required: {e}"
                        )),
                    Err(_) => {}
                }
                if harness::stored(&c, &id).await != before {
                    broken.push(format!(
                        "{tool}: the refused `{k}: null` still wrote to the node"
                    ));
                }
                continue;
            }
            if !before.contains_key(k) {
                continue; // the create did not store it, so there is nothing to unset
            }
            asked += 1;
            // Anthony's ruling, 2026-10-25: a null never erases history and
            // never silently resets a status. Both are refused by name and
            // write nothing; a status refusal names the setter that moves it.
            let additive = p
                .description
                .as_deref()
                .is_some_and(|d| d.contains("ADDITIVE"));
            if additive || k == "status" {
                let setter = status_setter(&node_type, &served);
                match harness::call(&c, tool, base((k, Value::Null))).await {
                    Ok(v) => broken.push(format!(
                        "{tool}: `{k}: null` was accepted; a null on {} is refused: {v}",
                        if additive {
                            "additive history"
                        } else {
                            "a status"
                        }
                    )),
                    Err(e) => {
                        if !e.contains(&format!("`{k}`")) {
                            broken.push(format!(
                                "{tool}: `{k}: null` refused without naming it: {e}"
                            ));
                        }
                        if additive && !e.contains("history") {
                            broken.push(format!(
                                "{tool}: `{k}: null` refused without saying it is history: {e}"
                            ));
                        }
                        if k == "status"
                            && let Some(set) = &setter
                            && !e.contains(set.as_str())
                        {
                            broken.push(format!(
                                "{tool}: `status: null` refused without naming `{set}`: {e}"
                            ));
                        }
                    }
                }
                if harness::stored(&c, &id).await != before {
                    broken.push(format!(
                        "{tool}: the refused `{k}: null` still wrote to the node"
                    ));
                }
                continue;
            }
            let reply = match harness::call(&c, tool, base((k, Value::Null))).await {
                Ok(v) => v,
                // A field the CONSTRUCTOR needs to create the node, though the
                // schema does not mark it required (`DesignEpoch.sequence`,
                // `TemporalFact.statement`): a null on it is refused like a
                // required property's, or the node would be left one its own
                // constructor cannot revise. Accepted only when a create
                // without the field is refused naming it too.
                Err(e) if e.contains(&format!("`{k}`")) && e.contains("REQUIRED") => {
                    let mut probe = create.clone();
                    probe.remove(k);
                    probe.insert(
                        "id".into(),
                        json!(id.replacen("revise-case", &format!("null-probe-{k}"), 1)),
                    );
                    match harness::call(&c, tool, Value::Object(probe)).await {
                        Err(p) if p.contains(&format!("`{k}`")) => {}
                        other => broken.push(format!(
                            "{tool}: `{k}: null` was refused as REQUIRED, but a create without \
                             it is not refused for it ({other:?}): {e}"
                        )),
                    }
                    if harness::stored(&c, &id).await != before {
                        broken.push(format!(
                            "{tool}: the refused `{k}: null` still wrote to the node"
                        ));
                    }
                    continue;
                }
                Err(e) => {
                    broken.push(format!("{tool}: `{k}: null` was refused: {e}"));
                    continue;
                }
            };
            let after = harness::stored(&c, &id).await;
            let expected = p.default.as_ref().map(|d| serde_json::to_value(d).unwrap());
            let got = after.get(k).cloned();
            if COMPUTED_DEFAULT.iter().any(|(t, f, _)| t == tool && f == k) {
                if got.is_none() || got == before.get(k).cloned() {
                    broken.push(format!(
                        "{tool}: `{k}: null` should put back its computed default; it holds {got:?}"
                    ));
                }
            } else if got != expected {
                broken.push(format!(
                    "{tool}: `{k}: null` left {k} = {} (expected {})",
                    got.map_or("absent".into(), |v| v.to_string()),
                    expected.map_or("absent".into(), |v| format!("the default {v}")),
                ));
            }
            if !reply_unset(&reply).contains(k) {
                broken.push(format!(
                    "{tool}: `{k}: null` — the reply does not name it under `unset`: {reply}"
                ));
            }
            if let Err(e) = harness::call(&c, tool, keep.clone()).await {
                broken.push(format!(
                    "{tool}: after `{k}: null` the node can no longer be revised by id: {e}"
                ));
            }
            if without(&before, k) != without(&after, k) {
                broken.push(format!(
                    "{tool}: `{k}: null` changed OTHER properties\n  before: {}\n  after:  {}",
                    Value::Object(without(&before, k)),
                    Value::Object(without(&after, k))
                ));
            }
        }
        if asked == 0 {
            broken.push(format!(
                "{tool}: no optional property was asked — the create stored none, so this \
                 constructor is not held to the rule"
            ));
        }
    }
    assert!(
        broken.is_empty(),
        "{} problem(s):\n{}",
        broken.len(),
        broken.join("\n")
    );
}
