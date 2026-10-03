//! A missing required argument is answered with the tool's own published
//! schema, not with a bare serde string.
//!
//! # The class, measured before the fix
//!
//! `unclaimed_findings` called with no arguments answered:
//!
//! ```text
//! failed to deserialize parameters: missing field `change_event_ids`
//! ```
//!
//! That names neither the tool nor what the field wants, and the caller's only
//! route forward is to guess or to go and read the schema. **139 of 180 served
//! tools declare at least one required parameter, across 230 required
//! parameters in total**, and every one of them answered in exactly that shape.
//!
//! This is not 139 oversights. The refusal is produced by the deserialiser
//! BEFORE any handler runs, so no handler could have improved it — which is
//! precisely why the sibling case (`unknown field`) was already intercepted in
//! one place, in `ReflowService::call_tool`. That branch has existed since
//! v0.51.0 and its twin was never written.
//!
//! # It also closes the one-field-per-round-trip complaint
//!
//! The sweep recorded separately that validation "teaches one field per node
//! per round trip": omit three required arguments and you learn about them one
//! refusal at a time. Serde reports only the first missing field, so the fix
//! reads `required` from the published schema and names them ALL — the caller
//! learns the whole obligation from one refusal.
//!
//! # 🛑 AND THE UNIT TESTS BELOW ARE NOT ENOUGH — MEASURED, ON THIS FIX
//!
//! The first wiring of this branch matched `Err(e) if e.message.contains(..)`,
//! copied from the `unknown field` arm beside it. Against a real binary it
//! never fired: **rmcp 3 returns a deserialisation failure as
//! `Ok(CallToolResponse::Complete)` carrying `isError: true`, not as `Err`.**
//!
//! ⭐ WHICH MEANS THE `unknown field` INTERCEPTION HAD BEEN DEAD SINCE v0.51.0.
//! Its reconnect sentence never reached a caller once, and the test behind it
//! (`the_v0510_field_feedback_fixes_land`) calls `stale_client_hint` as a pure
//! function — so it passed every day while the feature did nothing. That is the
//! vacuous-green failure this project keeps finding in other people's code,
//! found in its own, and it was invisible to every test in the tree.
//!
//! **So the binding check for this class is `tools/refusal_speaks.py`**, which
//! asks a real server and is wired into CI. It was run against the UNFIXED
//! binary first and reported **140 failures — 139 tools plus the dead
//! `unknown field` branch** — then zero after. The tests in this file pin the
//! SENTENCE; only the gate proves a caller receives it. Do not let this file
//! stand in for that one.
//!
//! # Since 2026-10-02 the refusal is built from the schema, not from serde
//!
//! The hint this file pinned read the field's description from the schema's
//! TOP-LEVEL `properties` only, so a required field missing inside an item
//! (`related_to[0].evidence`) was refused with "its own schema publishes no
//! description of it" — false for 22 of the 35 nested required fields on the
//! surface (`fact:root-cause-a-missing-nested-field-refusal-says-the-schema-publishes-no-description-when-it-does-2026-10-02`).
//! Arguments are now checked against the published schema before anything
//! deserialises them (`reflow2_mcp::arguments`), at every depth, and every
//! property this file asked of the old hint is asked of the new refusal here.
//! The wire is asked by `every_argument_refusal_names_the_tool_and_the_field_path.rs`
//! and by `tools/refusal_speaks.py`.
//!
//! # What this does NOT claim
//!
//! It checks that the refusal names the tool, the missing field, and what the
//! schema says that field is for. It cannot check that the description is
//! *good* — that obligation belongs to the description-accuracy work, and a
//! field whose published description is thin produces a thin refusal here.
//! Presence is what is derivable; quality is not.

use reflow2_mcp::arguments::{Transport, check, deserializer_refusal, refusal};
use reflow2_mcp::service::ReflowService;
use serde_json::{Map, Value};

/// The refusal a call with `given` arguments receives, built exactly as
/// `call_tool` builds it.
fn refused(tool: &str, given: Map<String, Value>) -> String {
    let schema = schema_for(tool);
    let violations = check(schema.as_object().expect("object schema"), &given);
    assert!(!violations.is_empty(), "`{tool}` {given:?} fits its schema");
    refusal(tool, &violations, Transport::Session)
}

fn schema_for(tool: &str) -> Value {
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
    let t = all
        .into_iter()
        .find(|t| t.name == tool)
        .unwrap_or_else(|| panic!("{tool} is served"));
    serde_json::to_value(&t.input_schema).expect("schema")
}

/// THE MEASURED CASE, in the reporter's own words.
#[test]
fn the_reported_call_is_answered_with_the_field_and_its_purpose() {
    let hint = refused("unclaimed_findings", Map::new());

    assert!(
        hint.contains("`unclaimed_findings`"),
        "the refusal must name the TOOL — the caller may have several in flight:\n{hint}"
    );
    assert!(
        hint.contains("`change_event_ids`"),
        "the refusal must keep naming the missing field:\n{hint}"
    );
    // The schema's own description of that field, not a sentence invented here.
    let desc = schema_for("unclaimed_findings")["properties"]["change_event_ids"]["description"]
        .as_str()
        .expect("change_event_ids publishes a description")
        .to_string();
    let opening: String = desc
        .split_whitespace()
        .take(4)
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        hint.contains(&opening),
        "the refusal must say what the field is FOR, quoting the published schema \
         (looked for {opening:?}):\n{hint}"
    );
}

/// EVERY required field, not just the one serde would have noticed first.
#[test]
fn a_caller_learns_the_whole_obligation_from_one_refusal() {
    // `record_change` declares five required parameters; serde named one.
    let schema = schema_for("record_change");
    let required: Vec<String> = schema["required"]
        .as_array()
        .expect("record_change has required fields")
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    assert!(
        required.len() >= 3,
        "this test needs a tool with several required fields; record_change has {}",
        required.len()
    );

    let hint = refused("record_change", Map::new());

    let unnamed: Vec<&String> = required
        .iter()
        .filter(|f| !hint.contains(&format!("`{f}`")))
        .collect();
    assert!(
        unnamed.is_empty(),
        "a refusal that names one required field at a time costs a round trip per field. \
         {} of {} were not named: {:?}\n{hint}",
        unnamed.len(),
        required.len(),
        unnamed
    );
}

/// The guard on the guard: a message that is NOT a deserialiser refusal must
/// pass through untouched, or the fallback would rewrite unrelated refusals.
#[test]
fn an_unrelated_refusal_is_left_alone() {
    let msg = "the design holds no Contributor with id `nobody`";
    assert_eq!(
        deserializer_refusal("loop_status", msg, Transport::Session),
        None,
        "only a deserialisation refusal may be rewritten"
    );
}

/// With the call's own arguments in hand the refusal names exactly what THIS
/// call lacked, and never a field it passed.
#[test]
fn with_the_arguments_in_hand_the_refusal_names_what_this_call_lacked() {
    let schema = schema_for("record_change");
    let required: Vec<String> = schema["required"]
        .as_array()
        .expect("record_change has required fields")
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    assert!(required.len() >= 3);
    let passed = required
        .iter()
        .find(|f| schema["properties"][f.as_str()]["type"] == "string")
        .expect("record_change has a required string field")
        .clone();
    let mut given = Map::new();
    given.insert(passed.clone(), Value::String("x".into()));
    let hint = refused("record_change", given);
    for f in required.iter().filter(|f| **f != passed) {
        assert!(
            hint.contains(&format!("`{f}` is required")),
            "{f} was not passed and must be named as missing:\n{hint}"
        );
    }
    assert!(
        !hint.contains(&format!("`{passed}` is required")),
        "{passed} was passed and must not be listed as missing:\n{hint}"
    );
    let (_, after) = hint
        .split_once("Already passed")
        .expect("the passed field is set apart");
    assert!(
        after.contains(&format!("`{passed}`")),
        "{passed} was passed and must be set apart:\n{hint}"
    );
}
