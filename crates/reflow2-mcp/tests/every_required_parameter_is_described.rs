//! Every REQUIRED parameter on the served surface carries a description a
//! caller can act on.
//!
//! # The class, measured
//!
//! The 2026-09-10/11 sweep of all 180 tools found that **76 of 230 required
//! parameters, across 68 tools, published no description at all** (the
//! remediation tracker's NEW-02). It surfaced only because the missing-argument
//! refusal had just started quoting the schema — and found nothing to quote.
//! 21 of the 76 were a constructor's own `id`; the rest were reference ids and
//! payloads (`release_id` ×4, `verification_id` ×4, `gaps`, `edges`, `accepts`,
//! …) that a caller has to work out from the tool's prose, or guess.
//!
//! Increment 434 (`epoch:planned-the-tool-surface-predicts-itself`) wrote all
//! 76. This test is what stops a 77th: it fails on any required parameter that
//! ships without one.
//!
//! # ⭐ Why the membership is derived and not hand-listed
//!
//! The same argument as `every_reference_parameter_names_its_type`: whether a
//! required parameter carries a description is decidable from the served
//! schema, so the class computes its own membership and a 181st tool joins it
//! the moment it is served. A hand-kept list would need someone to remember
//! this test exists, which is the failure `every_declared_enum_is_wired_or_exempt`
//! documents for the table it is forced to keep.
//!
//! # What this does NOT claim
//!
//! Presence, not quality. A description that says "the id" passes here and
//! fails a user. The bar for QUALITY is the blind prediction test the sweep
//! ran — predict the tool's behaviour from its served text, then call it —
//! and that is a judgement, not a check. What this pins is that the judgement
//! has something to judge.
//!
//! # `EXEMPT`
//!
//! Empty on purpose, and kept as a list rather than removed: the day a
//! parameter genuinely cannot be described (none has been found), its entry
//! and its reason go here, and the second test refuses an entry that names a
//! parameter which is no longer required or already described — so the
//! exemption list cannot rot into silence.

use reflow2_mcp::service::ReflowService;
use serde_json::Value;
use std::collections::BTreeSet;

/// Required parameters that legitimately carry no description, with the reason.
/// Keyed `(tool, parameter)`. See the module doc for why this is empty.
const EXEMPT: &[(&str, &str, &str)] = &[];

fn tools() -> Vec<rmcp::model::Tool> {
    let mut all = ReflowService::capture_router().list_all();
    for r in [
        ReflowService::assure_router(),
        ReflowService::exchange_router(),
        ReflowService::temporal_tools_router(),
        ReflowService::ask_router(),
        ReflowService::built_router(),
        ReflowService::coherence_router(),
        ReflowService::ingest_tools_router(),
        ReflowService::operate_tools_router(),
        ReflowService::query_router(),
        ReflowService::claims_tools_router(),
        ReflowService::skills_router(),
    ] {
        all.extend(r.list_all());
    }
    all
}

/// Every `(tool, required parameter)` on the surface — AT EVERY DEPTH — with
/// whether it carries a non-empty description. A nested parameter is named by
/// its path (`related_to[].evidence`), and its description may sit on the
/// property itself or on the `$defs` entry its `$ref` names.
///
/// ⚠️ THIS WALKED ONLY THE TOP LEVEL UNTIL 2026-10-02, the same blind spot as
/// the refusal it guards: the missing-argument hint read `properties` and
/// never followed `$ref` into `$defs`, so it told callers 22 described nested
/// fields had no description, and this test never saw the 13 that really had
/// none (`fact:root-cause-a-missing-nested-field-refusal-says-the-schema-publishes-no-description-when-it-does-2026-10-02`).
/// The check and the instrument shared one walk, so they were wrong together.
fn required_parameters(tools: &[rmcp::model::Tool]) -> Vec<(String, String, bool)> {
    let mut out = Vec::new();
    for t in tools {
        let root: Value = serde_json::to_value(&t.input_schema).expect("schema");
        walk(&root, &root, "", &mut out, t.name.as_ref(), 0);
    }
    out
}

fn resolve<'a>(root: &'a Value, mut s: &'a Value) -> &'a Value {
    for _ in 0..16 {
        match s
            .get("$ref")
            .and_then(Value::as_str)
            .and_then(|r| r.strip_prefix("#/$defs/"))
            .and_then(|n| root["$defs"].get(n))
        {
            Some(t) => s = t,
            None => break,
        }
    }
    s
}

fn described(root: &Value, prop: &Value) -> bool {
    let own = prop["description"].as_str();
    let via_ref = resolve(root, prop)["description"].as_str();
    own.or(via_ref).is_some_and(|d| !d.trim().is_empty())
}

fn walk(
    root: &Value,
    schema: &Value,
    at: &str,
    out: &mut Vec<(String, String, bool)>,
    tool: &str,
    depth: usize,
) {
    if depth > 8 {
        return;
    }
    let s = resolve(root, schema);
    let props = s["properties"].as_object().cloned().unwrap_or_default();
    for r in s["required"].as_array().into_iter().flatten() {
        let Some(r) = r.as_str() else { continue };
        let path = if at.is_empty() {
            r.to_string()
        } else {
            format!("{at}.{r}")
        };
        let ok = props.get(r).is_some_and(|p| described(root, p));
        out.push((tool.to_string(), path, ok));
    }
    for (name, sub) in &props {
        let here = if at.is_empty() {
            name.clone()
        } else {
            format!("{at}.{name}")
        };
        walk(root, sub, &here, out, tool, depth + 1);
    }
    if let Some(items) = s.get("items") {
        walk(root, items, &format!("{at}[]"), out, tool, depth + 1);
    }
}

/// THE WALK REACHES THE NESTED FIELDS: measured 2026-10-02, 35 required
/// nested fields across 16 tools. A walk that finds none is the old blind spot
/// back, and would pass every day.
#[test]
fn the_walk_reaches_required_fields_inside_items() {
    let nested: Vec<String> = required_parameters(&tools())
        .into_iter()
        .filter(|(_, p, _)| p.contains('.'))
        .map(|(t, p, _)| format!("{t}.{p}"))
        .collect();
    assert!(
        nested.len() >= 35,
        "the walk found {} required nested fields; 35 were measured on 2026-10-02:\n  {}",
        nested.len(),
        nested.join("\n  ")
    );
    assert!(
        nested
            .iter()
            .any(|n| n == "add_decision.related_to[].evidence"),
        "add_decision's related_to[].evidence — the measured case — is not reached"
    );
}

/// THE CLASS CONTRACT: a required parameter says what it is.
#[test]
fn every_required_parameter_carries_a_description() {
    let tools = tools();
    assert!(tools.len() >= 180, "the surface shrank to {}", tools.len());
    let exempt: BTreeSet<(&str, &str)> = EXEMPT.iter().map(|(t, p, _)| (*t, *p)).collect();
    let mut missing: Vec<String> = required_parameters(&tools)
        .into_iter()
        .filter(|(t, p, described)| !described && !exempt.contains(&(t.as_str(), p.as_str())))
        .map(|(t, p, _)| format!("{t}.{p}"))
        .collect();
    missing.sort();
    assert!(
        missing.is_empty(),
        "{} required parameter(s) publish no description:\n  {}\n\nA caller meets these as a \
         bare `{{\"type\": \"string\"}}` and has to guess, and the missing-argument refusal that \
         quotes the schema has nothing to quote. Give each a doc comment on its request-struct \
         field saying what it is and where a caller gets one — or, if it genuinely cannot be \
         described, add it to EXEMPT with the reason. Measured 2026-09-11: 76 of 230 were bare.",
        missing.len(),
        missing.join("\n  ")
    );
}

/// The other direction: an EXEMPT entry must still name a required parameter
/// that is actually undescribed, or the list is carrying a stale claim.
#[test]
fn every_exemption_still_names_a_bare_required_parameter() {
    let tools = tools();
    let bare: BTreeSet<(String, String)> = required_parameters(&tools)
        .into_iter()
        .filter(|(_, _, described)| !described)
        .map(|(t, p, _)| (t, p))
        .collect();
    let stale: Vec<String> = EXEMPT
        .iter()
        .filter(|(t, p, _)| !bare.contains(&((*t).to_string(), (*p).to_string())))
        .map(|(t, p, _)| format!("{t}.{p}"))
        .collect();
    assert!(
        stale.is_empty(),
        "EXEMPT names {} parameter(s) that are no longer bare required parameters — remove them, \
         or the list is asserting an ambiguity that no longer exists:\n  {}",
        stale.len(),
        stale.join("\n  ")
    );
}
