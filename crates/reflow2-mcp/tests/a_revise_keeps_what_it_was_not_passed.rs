//! A REVISE keeps what it was not passed — asked of every tool that promises
//! it, and the membership is read off the served surface.
//!
//! # The contract, and the class that kept breaking it
//!
//! Twenty constructors say, in their served description, *"CONTENT FIELDS ARE
//! REQUIRED TO CREATE AND OPTIONAL TO REVISE … omitted fields keep their stored
//! value"* (#302, 2026-08-22). The promise is carried by two shared mechanisms
//! — `RequiredFields` for the required fields and `upsert_node` for everything
//! else — and both are correct. What broke it, four times by four different
//! mechanisms, was code in ONE handler that computed from the CALL instead of
//! from the node the merge produces:
//!
//! - `record_finding` wrote its `unwrap_or` defaults into the merge on every
//!   call, so a revise demoted a `defect` to a `finding` and turned a
//!   `forecast` into a `measured` claim (#447, 2026-09-07; three field
//!   sightings, the last on 0.74.0 in the dev_reflow2 hub on 2026-09-29);
//! - `plan_epoch` wrote through a REPLACING core constructor and carried only
//!   `status` forward by hand, so a revise cleared `description` and
//!   `checksum`;
//! - `add_change_event` computed its `undated` note from the call, so a
//!   revise of a dated event said it was undated;
//! - `add_verification`'s "findings need a status" guard read the call and
//!   never named the verdict the node already holds.
//!
//! `fact:root-cause-the-revise-contract-is-implemented-per-handler-and-guarded-by-instances-2026-09-28`
//! walked the class: every instance was added AFTER the contract was stated, in
//! code written for the create case, and **every existing test pinned an
//! INSTANCE** — a hand-picked tool or two. A twenty-first constructor, or a new
//! default in an old one, joined nothing.
//!
//! # ⭐ Why the membership is derived and not hand-listed
//!
//! This test asks the SERVED tool list which tools make the promise, and holds
//! every one of them to it. A tool that starts promising it joins the moment it
//! is served; a tool that stops is dropped the same way. What IS hand-kept is
//! the small [`CONTEXT`] table of values a create cannot be synthesised
//! without (a subject that must already exist, a value a validator reads) —
//! and the first test fails on any promising tool whose create refuses, so the
//! table cannot silently fall behind the surface.
//!
//! # What one case asserts
//!
//! Create with EVERY declared scalar property set to a non-default value;
//! revise with the id, a new `name`, and nothing else the schema does not
//! require; then assert (i) every other stored property is byte-identical, and
//! (ii) the revise reply carries no top-level key the create reply did not,
//! other than the `revision` block a revise is supposed to add. The second half
//! is what catches a note computed from the call.
//!
//! # Counterweights, so "keep" never becomes "never change"
//!
//! A first create with the defaulted fields omitted still lands the documented
//! defaults, and an explicitly passed value still overwrites a stored one —
//! see the last two tests. Without them this file would pass a handler that
//! stopped writing defaults at all.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;

/// The phrase a tool's served description carries when it makes the promise.
/// Matched case-insensitively, because one description spells it mid-sentence.
const PROMISE: &str = "optional to revise";

/// Properties never synthesised, and why. Kept to what a create genuinely
/// cannot be handed a made-up value for — every other scalar is exercised.
const NEVER_SYNTHESISED: &[(&str, &str)] = &[
    ("id", "the case supplies it"),
    (
        "approver",
        "a signature, supplied with a status only where the status needs one",
    ),
    ("acted_at", "rides the approver"),
    (
        "distinct_from",
        "an answer to a near-match refusal, not a property",
    ),
    (
        "node_type",
        "a routing hint for an id's type, not a property of the node",
    ),
];

/// Values a create cannot be synthesised without, per tool: a subject that
/// must already exist, or a value a validator reads. Keyed `(tool, param)`.
/// The seed nodes named here are made in [`seed`].
const CONTEXT: &[(&str, &str, &str)] = &[
    ("record_finding", "subject_id", "req:seed"),
    ("add_readiness", "target_type", "Requirement"),
    ("add_readiness", "target_id", "req:seed"),
    // Validated against a value set the schema does not publish.
    ("add_requirement", "provenance", "imported"),
];

/// Enum values a tool publishes and then refuses on this path, with the reason
/// the refusal gives. Keyed `(tool, param, value, why)`.
const RESERVED_VALUES: &[(&str, &str, &str, &str)] = &[(
    "add_change_event",
    "change_type",
    "baseline_established",
    "written only by set_artifact_checksum's first-checksum disposition",
)];

/// A property that means something only beside another one's value, and is
/// refused otherwise. Keyed `(tool, param, needs_param, needs_value)`: the
/// param is synthesised only when the create also carries that value.
const ONLY_WITH: &[(&str, &str, &str, &str)] = &[(
    "add_change_event",
    "stands_in_for",
    "repair",
    "contained_symptom",
)];

/// Parameters one tool takes that are not properties of the node it writes —
/// a reference to another node, drawn as an edge, or a parameter the tool
/// accepts only to refuse with a pointer. Keyed `(tool, param, why)`. Edges
/// are out of this test's reach by design (see the module doc); the create
/// refusal in the class test is what keeps this table honest, because a new
/// reference parameter makes the synthesised create refuse.
const NOT_A_PROPERTY: &[(&str, &str, &str)] = &[
    (
        "add_capability",
        "allocated_to",
        "a Component reference, drawn as ALLOCATED_TO",
    ),
    (
        "add_capability",
        "satisfies",
        "a Requirement reference, drawn as SATISFIES",
    ),
    (
        "add_change_event",
        "description",
        "accepted only to refuse it and name `summary`/`rationale` instead",
    ),
    (
        "record_finding",
        "caused_by",
        "a node reference, drawn as CAUSES",
    ),
    (
        "record_finding",
        "cause_evidence",
        "rides `caused_by` onto the CAUSES edge",
    ),
];

struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "a-revise-keeps-what-it-was-not-passed".to_string();
        cfg.client_info.version = "test".to_string();
        cfg
    }
}

type Client = rmcp::service::RunningService<rmcp::RoleClient, TestClient>;

/// One in-memory server, reached the way a session reaches it — through
/// `call_tool`, so the refusal hints and the per-client content policy are the
/// ones a caller meets.
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

/// Call a tool by NAME. `Ok(structured reply)` or `Err(refusal text)`.
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

async fn stored(c: &Client, id: &str) -> Map<String, Value> {
    let v = call(c, "get_node", json!({ "id": id }))
        .await
        .unwrap_or_else(|e| panic!("get_node {id}: {e}"));
    v["node"]["properties"]
        .as_object()
        .unwrap_or_else(|| panic!("get_node {id} returned no properties: {v}"))
        .clone()
}

/// The nodes a CONTEXT value may name, made before any case runs.
async fn seed(c: &Client) {
    call(
        c,
        "add_requirement",
        json!({"id":"req:seed","name":"A seed requirement","statement":"exists so a finding has a subject"}),
    )
    .await
    .expect("seed requirement");
    call(
        c,
        "add_contributor",
        json!({"id":"who:seed","name":"Seed person","kind":"person"}),
    )
    .await
    .expect("seed contributor");
}

/// The served tools that make the promise, with their input schemas.
async fn promising_tools(c: &Client) -> Vec<(String, Value, String)> {
    let tools = c.list_all_tools().await.expect("tools/list");
    let mut out: Vec<(String, Value, String)> = tools
        .into_iter()
        .filter(|t| {
            t.description
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .contains(PROMISE)
        })
        .map(|t| {
            (
                t.name.to_string(),
                serde_json::to_value(&t.input_schema).expect("schema"),
                t.description.as_deref().unwrap_or("").to_string(),
            )
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// The id prefix a tool's `id` parameter documents (`` `req:<slug>` ``).
fn id_prefix(tool: &str, schema: &Value) -> String {
    let desc = schema["properties"]["id"]["description"]
        .as_str()
        .unwrap_or_else(|| panic!("{tool}: its `id` parameter publishes no description"));
    desc.split('`')
        .skip(1)
        .step_by(2)
        .find_map(|tok| {
            let (p, _) = tok.split_once(':')?;
            (!p.is_empty() && p.chars().all(|ch| ch.is_ascii_lowercase() || ch == '_'))
                .then(|| p.to_string())
        })
        .unwrap_or_else(|| panic!("{tool}: the `id` description names no `prefix:` ({desc})"))
}

/// The non-null JSON type a schema property takes, and its enum if it has one.
fn shape(prop: &Value) -> (Option<String>, Option<Vec<String>>) {
    let ty = match &prop["type"] {
        Value::String(s) => Some(s.clone()),
        Value::Array(a) => a
            .iter()
            .filter_map(Value::as_str)
            .find(|s| *s != "null")
            .map(String::from),
        _ => None,
    };
    let en = prop["enum"].as_array().map(|a| {
        a.iter()
            .filter_map(Value::as_str)
            .map(String::from)
            .collect::<Vec<_>>()
    });
    (ty, en)
}

/// Text that shares NO word with any other case's text. The cross-type
/// near-match guard compares names and prose across every node in the store,
/// so twenty cases written in the same stock phrase refuse each other — the
/// guard working, and not what this test is about.
fn words(tool: &str, field: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in tool.bytes().chain(*b"/").chain(field.bytes()) {
        h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
    }
    let word = |mut x: u64| -> String {
        (0..9)
            .map(|_| {
                let c = (b'a' + (x % 26) as u8) as char;
                x /= 26;
                c
            })
            .collect()
    };
    format!(
        "{} {} {}",
        word(h),
        word(h.rotate_left(21)),
        word(h.rotate_left(42))
    )
}

/// A value for one property that is NOT what the create path would default
/// it to. `None` means "cannot be synthesised generically — leave it out".
///
/// ⭐ EVERY CASE RUNS TWICE, with a different value each pass. A handler
/// default equal to the synthesised value hides a reset completely — the
/// first run of this test missed `record_finding`'s `fact_type` exactly that
/// way, because the last legal value IS its default. No single choice can be
/// guaranteed to differ from a default nobody declared; two different ones can,
/// since a default equals at most one of them.
fn synthesise(tool: &str, name: &str, prop: &Value, pass: usize) -> Option<Value> {
    let (ty, en) = shape(prop);
    if let Some(values) = en.filter(|v| !v.is_empty()) {
        let values: Vec<&String> = values
            .iter()
            .filter(|v| {
                !RESERVED_VALUES
                    .iter()
                    .any(|(t, p, r, _)| *t == tool && *p == name && r == v)
            })
            .collect();
        let i = values.len() - 1 - (pass % values.len());
        return Some(json!(values[i]));
    }
    let lower = name.to_lowercase();
    match ty.as_deref() {
        Some("string") => {
            if lower.ends_with("_id") {
                return None; // a reference: CONTEXT supplies the required ones
            }
            if lower.ends_with("_at") || lower.starts_with("valid_") || lower.contains("date") {
                return Some(json!("2026-09-01"));
            }
            if lower == "checksum" || lower.ends_with("_hash") {
                return Some(json!(format!("sha256:{}", "ab".repeat(32))));
            }
            if lower == "commits" {
                return Some(json!(["0123abc", "4567def"][pass % 2]));
            }
            if lower == "version" {
                return Some(json!("1.2.3"));
            }
            if lower == "location" {
                return Some(json!("https://example.invalid/revise-case"));
            }
            Some(json!(words(tool, &format!("{name}#{pass}"))))
        }
        Some("number") => Some(json!([0.4, 0.7][pass % 2])),
        Some("integer") => Some(json!([7, 3][pass % 2])),
        Some("boolean") => Some(json!(pass.is_multiple_of(2))),
        _ => None, // arrays and objects are edges or bundles, not properties
    }
}

/// Build the create call for one tool: every synthesisable property, the
/// CONTEXT values, and an approver wherever the schema takes one.
fn create_args(tool: &str, schema: &Value, pass: usize) -> Map<String, Value> {
    let mut args = Map::new();
    args.insert(
        "id".into(),
        json!(format!(
            "{}:revise-case-{tool}-{pass}",
            id_prefix(tool, schema)
        )),
    );
    let props = schema["properties"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    for (name, prop) in &props {
        if NEVER_SYNTHESISED.iter().any(|(n, _)| n == name) {
            continue;
        }
        if NOT_A_PROPERTY
            .iter()
            .any(|(t, p, _)| *t == tool && p == name)
        {
            continue;
        }
        // `x_type` beside an `x_id` (or an `x`) is the routing hint for that
        // reference's type, not a property. A `_type` with no such sibling —
        // `fact_type`, `change_type`, `epoch_type` — IS a property, and the
        // first run of this test skipped `fact_type` by suffix alone, which
        // hid the very reset it was written to catch.
        if let Some(stem) = name.strip_suffix("_type")
            && (stem == "node"
                || props.contains_key(&format!("{stem}_id"))
                || props.contains_key(stem))
        {
            continue;
        }
        if let Some(v) = synthesise(tool, name, prop, pass) {
            args.insert(name.clone(), v);
        }
    }
    for (t, p, v) in CONTEXT {
        if *t == tool {
            args.insert((*p).into(), json!(v));
        }
    }
    for (t, p, needs, value) in ONLY_WITH {
        if *t == tool && args.get(*needs) != Some(&json!(value)) {
            args.remove(*p);
        }
    }
    if props.contains_key("approver") {
        args.insert("approver".into(), json!("who:seed"));
        if props.contains_key("acted_at") {
            args.insert("acted_at".into(), json!("2026-09-01"));
        }
    }
    args
}

/// The one field a revise changes: `name` where the tool takes one, else the
/// first free-text property the create set.
fn revise_field(schema: &Value, create: &Map<String, Value>) -> Option<String> {
    let props = schema["properties"].as_object()?;
    if props.contains_key("name") {
        return Some("name".into());
    }
    create
        .iter()
        .filter(|(k, v)| {
            *k != "id"
                && v.is_string()
                && shape(&props[k.as_str()]).1.is_none()
                && !schema["required"]
                    .as_array()
                    .is_some_and(|r| r.iter().any(|x| x == *k))
        })
        .map(|(k, _)| k.clone())
        .next()
}

/// Build the revise call: the id, a new value for ONE field, and whatever the
/// SCHEMA still requires (taken from the create) — nothing else.
fn revise_args(
    tool: &str,
    schema: &Value,
    create: &Map<String, Value>,
    field: &str,
) -> Map<String, Value> {
    let mut args = Map::new();
    args.insert("id".into(), create["id"].clone());
    args.insert(field.into(), json!(words(tool, "the revise")));
    for r in schema["required"].as_array().into_iter().flatten() {
        let r = r.as_str().unwrap_or_default();
        if let Some(v) = create.get(r) {
            args.insert(r.into(), v.clone());
        }
    }
    args
}

/// Keys a revise reply may carry that its create did not, because a revise is
/// supposed to report them.
///
/// `edges_already_present` (crate::drawn_edges, #623): a revise that re-sends
/// its targets finds the edges its create drew and names them as already
/// there. That is read from the STORE, not computed from the call, and it is
/// what stops a revise reading as having drawn something new.
const REVISE_ONLY_REPLY_KEYS: &[&str] = &["revision", "edges_already_present"];

#[tokio::test]
async fn every_tool_that_promises_optional_to_revise_keeps_what_a_revise_did_not_pass() {
    let c = connect().await;
    seed(&c).await;
    let tools = promising_tools(&c).await;
    assert!(
        tools.len() >= 18,
        "only {} served tools make the promise — the phrase moved, and this test would go \
         vacuous: {:?}",
        tools.len(),
        tools.iter().map(|t| &t.0).collect::<Vec<_>>()
    );

    let mut broken: Vec<String> = Vec::new();
    for pass in 0..2 {
        for (tool, schema, _) in &tools {
            let create = create_args(tool, schema, pass);
            let Some(field) = revise_field(schema, &create) else {
                broken.push(format!(
                    "{tool}: promises optional-to-revise but takes no free-text field this test \
                 can revise it by"
                ));
                continue;
            };
            let id = create["id"].as_str().unwrap().to_string();
            let created = match call(&c, tool, Value::Object(create.clone())).await {
                Ok(v) => v,
                Err(e) => {
                    broken.push(format!(
                    "{tool} (pass {pass}): the create this test synthesised was REFUSED, so the promise could \
                     not be checked. Add what it needs to CONTEXT or NEVER_SYNTHESISED.\n  \
                     args: {}\n  refusal: {e}",
                    Value::Object(create.clone())
                ));
                    continue;
                }
            };
            let before = stored(&c, &id).await;

            let revise = revise_args(tool, schema, &create, &field);
            let revised = match call(&c, tool, Value::Object(revise.clone())).await {
                Ok(v) => v,
                Err(e) => {
                    broken.push(format!(
                        "{tool}: a revise passing only {:?} was REFUSED: {e}",
                        revise.keys().collect::<Vec<_>>()
                    ));
                    continue;
                }
            };
            let after = stored(&c, &id).await;

            // (i) every stored property the revise did not name is untouched.
            for (k, v) in &before {
                if revise.contains_key(k) {
                    continue;
                }
                match after.get(k) {
                Some(w) if w == v => {}
                Some(w) => broken.push(format!("{tool} (pass {pass}): `{k}` changed {v} → {w} on a revise that did not pass it")),
                None => broken.push(format!("{tool} (pass {pass}): `{k}` ({v}) was LOST on a revise that did not pass it")),
            }
            }
            // (ii) no note computed from the call: the revise reply says nothing
            // at top level that the create reply did not, bar the revision block.
            let keys = |v: &Value| -> BTreeSet<String> {
                v.as_object()
                    .map(|o| o.keys().cloned().collect())
                    .unwrap_or_default()
            };
            let extra: Vec<String> = keys(&revised)
                .difference(&keys(&created))
                .filter(|k| !REVISE_ONLY_REPLY_KEYS.contains(&k.as_str()))
                .cloned()
                .collect();
            if !extra.is_empty() {
                broken.push(format!(
                "{tool} (pass {pass}): the revise reply carries {extra:?}, which the create of the same node \
                 did not — a note computed from the call rather than the stored node"
            ));
            }
        }
    }
    assert!(
        broken.is_empty(),
        "{} breach(es) of \"omitted fields keep their stored value\":\n- {}",
        broken.len(),
        broken.join("\n- ")
    );
}

#[tokio::test]
async fn a_create_that_passes_only_an_id_is_refused_or_stores_no_placeholder() {
    // THE OTHER HALF OF THE SAME CONTRACT: "REQUIRED TO CREATE". `RequiredFields`
    // returns an empty placeholder for a field it could not resolve and relies
    // on `finish()` to refuse before anything is written — so a handler that
    // never calls `finish()` STORES the placeholder. Found while writing the
    // test above: `add_verification` created `ver:x` with `name: ""`, because
    // its handler resolves `name` and never asks whether it resolved. Same
    // class — a handler going around the shared mechanism — the mirror image.
    let c = connect().await;
    seed(&c).await;
    let mut broken = Vec::new();
    for (tool, schema, _) in promising_tools(&c).await {
        let id = format!("{}:id-only-{tool}", id_prefix(&tool, &schema));
        let mut args = Map::new();
        args.insert("id".into(), json!(id));
        for (t, p, v) in CONTEXT {
            if *t == tool {
                args.insert((*p).into(), json!(v));
            }
        }
        if call(&c, &tool, Value::Object(args)).await.is_err() {
            continue; // refused: nothing to create it from — the contract held
        }
        let props = stored(&c, &id).await;
        let blanks: Vec<&String> = props
            .iter()
            .filter(|(_, v)| v.as_str() == Some(""))
            .map(|(k, _)| k)
            .collect();
        if !blanks.is_empty() {
            broken.push(format!(
                "{tool}: an id-only create was ACCEPTED and stored {blanks:?} as \"\" — a \
                 required field's placeholder reached the store instead of a refusal"
            ));
        }
    }
    assert!(broken.is_empty(), "{}", broken.join("\n"));
}

#[tokio::test]
async fn a_first_create_still_lands_the_documented_defaults() {
    // COUNTERWEIGHT: the fix applies defaults on CREATE and only there.
    let c = connect().await;
    seed(&c).await;
    call(
        &c,
        "record_finding",
        json!({"id":"fact:defaults","subject_id":"req:seed","name":"n","statement":"s"}),
    )
    .await
    .expect("create");
    let p = stored(&c, "fact:defaults").await;
    assert_eq!(
        p["fact_type"],
        json!("finding"),
        "a new finding defaults to `finding`"
    );
    assert_eq!(p["basis"], json!("measured"), "and to `measured`");
}

#[tokio::test]
async fn an_explicitly_passed_value_still_overwrites_a_stored_one() {
    // COUNTERWEIGHT: "keep what was not passed" must never become "never change".
    let c = connect().await;
    seed(&c).await;
    call(
        &c,
        "record_finding",
        json!({"id":"fact:moves","subject_id":"req:seed","name":"n","statement":"s",
               "fact_type":"defect","basis":"forecast"}),
    )
    .await
    .expect("create");
    call(
        &c,
        "record_finding",
        json!({"id":"fact:moves","subject_id":"req:seed","fact_type":"finding"}),
    )
    .await
    .expect("revise");
    let p = stored(&c, "fact:moves").await;
    assert_eq!(p["fact_type"], json!("finding"), "the passed value wins");
    assert_eq!(
        p["basis"],
        json!("forecast"),
        "and the one not passed is kept"
    );
}

#[tokio::test]
async fn a_findings_revise_without_a_status_names_the_verdict_the_check_already_holds() {
    // THE GUARD, which is deliberately NOT relaxed here: whether findings
    // changed on a revise may inherit the stored verdict (a text correction)
    // or must restate it (a new run) is an open question for the owner
    // (dec:idea-revise-notes-guards-and-defaults-read-the-merged-node). What
    // this fixes is the blind round trip — the refusal now reads the node and
    // says what verdict it holds, so the re-send is informed.
    let c = connect().await;
    call(
        &c,
        "add_verification",
        json!({"id":"ver:run","name":"A check","method":"test","level":"unit",
               "status":"passing","last_run_at":"2026-09-25","findings":"31/35"}),
    )
    .await
    .expect("create");
    let refusal = call(
        &c,
        "add_verification",
        json!({"id":"ver:run","findings":"31/35, 4 expected — corrected wording"}),
    )
    .await
    .expect_err("findings without a status still describe a run whose verdict is unstated");
    assert!(
        refusal.contains("`passing`"),
        "the refusal must name the verdict the check already holds:\n{refusal}"
    );
    // And the create-time refusal, where no verdict exists, still refuses.
    let first = call(
        &c,
        "add_verification",
        json!({"id":"ver:new","name":"n","method":"test","level":"unit","findings":"x"}),
    )
    .await
    .expect_err("a first run with no verdict is still refused");
    assert!(
        !first.contains("already holds"),
        "a new check holds no verdict:\n{first}"
    );
}
