//! The argument check (`reflow2_mcp::arguments`) agrees with the deserialiser
//! it stands in front of, and understands every keyword the served schemas use.
//!
//! The check refuses a call before serde sees it, so it must be exactly as
//! strict as the request types — never stricter, or it refuses a call that
//! works today; never more lenient, or serde's bare string comes back. Two
//! things could make it disagree, and each has a test here:
//!
//! 1. **A keyword it does not check.** The validator implements the subset of
//!    JSON Schema reflow2's schemas use. A served schema that grows a keyword
//!    outside [`CHECKED`]/[`ANNOTATIONS`], a `format` outside [`FORMATS`], or a
//!    `$ref` that is not a local `$defs` entry fails here by name, so a
//!    construct the check does not understand cannot ship unchecked.
//! 2. **A spelling serde accepts that the schema does not publish.** Aliases
//!    (`id`/`node_id` for `decision_id`, `properties` for `props`) are
//!    deliberately left out of the published schema, so each aliased field
//!    declares them as `x-reflow2-aliases` for the check to read. This asks
//!    SERDE ITSELF, at every object of every tool, which names it accepts —
//!    its unknown-field refusal lists them, aliases included — and fails on any
//!    difference from the schema's names plus declared aliases. A
//!    `#[serde(alias)]` added without its declaration is caught here, before a
//!    caller meets a refusal for a spelling that worked yesterday.

use reflow2_mcp::arguments::{ALIASES, ANNOTATIONS, CHECKED, FORMATS};
use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;

/// Every served tool with its schema AS THE ROUTER HOLDS IT — the aliases
/// declared, before the listing strips them.
fn router_tools() -> Vec<rmcp::model::Tool> {
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

/// Every keyword in `schema`, with where it sits, recursing into every place
/// a subschema can appear in the subset — and only those, so a property NAME
/// or an enum VALUE is never mistaken for a keyword.
fn keywords(schema: &Value, at: &str, out: &mut Vec<(String, String, Value)>) {
    let Some(m) = schema.as_object() else { return };
    for (k, v) in m {
        out.push((at.to_string(), k.clone(), v.clone()));
    }
    for (name, sub) in m
        .get("properties")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        keywords(sub, &format!("{at}.properties.{name}"), out);
    }
    for (name, sub) in m
        .get("$defs")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        keywords(sub, &format!("{at}.$defs.{name}"), out);
    }
    if let Some(items) = m.get("items") {
        keywords(items, &format!("{at}.items"), out);
    }
    if let Some(extra @ Value::Object(_)) = m.get("additionalProperties") {
        keywords(extra, &format!("{at}.additionalProperties"), out);
    }
}

/// THE SUBSET IS ENFORCED: every keyword, format and `$ref` on the served
/// surface is one the check understands.
#[tokio::test]
async fn every_keyword_on_the_served_surface_is_one_the_check_understands() {
    let published = ReflowService::in_memory()
        .expect("service")
        .tools_with_lessons_for_test()
        .await;
    let mut tools = router_tools();
    tools.extend(published);
    assert!(tools.len() >= 380, "{} tools", tools.len());

    let mut unknown = BTreeSet::new();
    let mut seen = BTreeSet::new();
    for t in &tools {
        let root = serde_json::to_value(&*t.input_schema).expect("schema");
        let mut found = Vec::new();
        keywords(&root, "", &mut found);
        for (at, k, v) in found {
            seen.insert(k.clone());
            let where_ = format!("{}{at}", t.name);
            if !CHECKED.contains(&k.as_str()) && !ANNOTATIONS.contains(&k.as_str()) {
                unknown.insert(format!("{where_}: keyword `{k}`"));
            }
            if k == "format" && !v.as_str().is_some_and(|f| FORMATS.contains(&f)) {
                unknown.insert(format!("{where_}: format {v}"));
            }
            if k == "$ref" {
                let target = v
                    .as_str()
                    .and_then(|r| r.strip_prefix("#/$defs/"))
                    .and_then(|n| root["$defs"].get(n));
                if target.is_none() {
                    unknown.insert(format!("{where_}: $ref {v} is not a local $defs entry"));
                }
            }
        }
    }
    assert!(
        unknown.is_empty(),
        "the served schemas use what the argument check does not understand — teach \
         `reflow2_mcp::arguments` the keyword (and a test of it) before serving it, or the check \
         silently passes what serde then refuses:\n  {}",
        unknown.into_iter().collect::<Vec<_>>().join("\n  ")
    );
    // Not vacuous: the walk saw the constructs that matter.
    for k in [
        "type",
        "properties",
        "required",
        "items",
        "enum",
        "$ref",
        ALIASES,
    ] {
        assert!(
            seen.contains(k),
            "the walk never saw `{k}` — is it reaching the schemas?"
        );
    }
}

/// THE PUBLISHED LISTING CARRIES NO ALIAS: the typed spelling is the only one
/// a harness is taught.
#[tokio::test]
async fn the_published_listing_carries_no_alias_declaration() {
    let published = ReflowService::in_memory()
        .expect("service")
        .tools_with_lessons_for_test()
        .await;
    let leaked: Vec<String> = published
        .iter()
        .filter(|t| {
            serde_json::to_string(&*t.input_schema)
                .unwrap()
                .contains(ALIASES)
        })
        .map(|t| t.name.to_string())
        .collect();
    assert!(
        leaked.is_empty(),
        "the listing publishes {ALIASES} on {leaked:?}"
    );
}

struct Quiet;
impl rmcp::ClientHandler for Quiet {}

/// The names serde lists in `unknown field \`…\`, expected one of \`a\`, \`b\``
/// (or `expected \`a\` or \`b\``, `expected \`a\``, `there are no fields`).
fn serde_fields(text: &str, probe: &str) -> Option<BTreeSet<String>> {
    let rest = text.split(&format!("unknown field `{probe}`")).nth(1)?;
    if rest.contains("there are no fields") {
        return Some(BTreeSet::new());
    }
    let listed = rest.split("expected ").nth(1)?;
    Some(
        listed
            .split('`')
            .skip(1)
            .step_by(2)
            .map(String::from)
            .collect(),
    )
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

/// Every object in a schema that names its properties, with the steps to
/// reach it (`None` = item 0 of an array).
fn objects<'a>(
    root: &'a Value,
    schema: &'a Value,
    at: Vec<Option<String>>,
    out: &mut Vec<(Vec<Option<String>>, &'a Value)>,
    depth: usize,
) {
    if depth > 6 {
        return;
    }
    let s = resolve(root, schema);
    if let Some(props) = s.get("properties").and_then(Value::as_object) {
        if !props.is_empty() {
            out.push((at.clone(), s));
        }
        for (name, sub) in props {
            let mut next = at.clone();
            next.push(Some(name.clone()));
            objects(root, sub, next, out, depth + 1);
        }
    }
    if let Some(items) = s.get("items") {
        let mut next = at;
        next.push(None);
        objects(root, items, next, out, depth + 1);
    }
}

fn place(at: &[Option<String>], leaf: Value) -> Value {
    let mut v = leaf;
    for s in at.iter().rev() {
        v = match s {
            Some(k) => json!({ k.as_str(): v }),
            None => json!([v]),
        };
    }
    v
}

/// THE ALIAS DECLARATIONS AGREE WITH SERDE, at every object of every tool.
#[tokio::test]
async fn every_spelling_serde_accepts_is_one_the_check_accepts() {
    let svc = ReflowService::in_memory().expect("service");
    let probe_svc = svc.share();
    let (server_rx, client_tx) = tokio::io::duplex(1 << 20);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 20);
    let (server, client) = tokio::join!(
        svc.serve((server_rx, server_tx)),
        Quiet.serve((client_rx, client_tx))
    );
    let server = server.expect("server handshake");
    let _client = client.expect("client handshake");
    let peer = server.peer().clone();

    const PROBE: &str = "\u{1}probe";
    let mut disagreements = Vec::new();
    let mut compared = 0usize;
    let mut unverifiable = Vec::new();
    for t in router_tools() {
        let root = serde_json::to_value(&*t.input_schema).expect("schema");
        let mut found = Vec::new();
        objects(&root, &root, Vec::new(), &mut found, 0);
        for (at, obj) in found {
            let mut leaf = Map::new();
            leaf.insert(PROBE.into(), json!(0));
            let args = place(&at, Value::Object(leaf));
            let ctx = rmcp::service::RequestContext::new(
                rmcp::model::NumberOrString::Number(1),
                peer.clone(),
            );
            let reply = probe_svc
                .call_unchecked_for_test(
                    &t.name,
                    args.as_object().cloned().unwrap_or_default(),
                    ctx,
                )
                .await;
            let text = match reply {
                Ok(rmcp::model::CallToolResponse::Complete(r)) => r
                    .content
                    .iter()
                    .filter_map(|c| c.as_text().map(|x| x.text.clone()))
                    .collect::<Vec<_>>()
                    .join("\n"),
                Ok(_) => String::new(),
                Err(e) => e.message.to_string(),
            };
            let where_ = format!("{} {}", t.name, args);
            let Some(serde) = serde_fields(&text, PROBE) else {
                unverifiable.push(where_);
                continue;
            };
            compared += 1;
            let props = obj["properties"].as_object().expect("properties");
            let mut schema_names: BTreeSet<String> = props.keys().cloned().collect();
            for p in props.values() {
                for a in p[ALIASES].as_array().into_iter().flatten() {
                    schema_names.insert(a.as_str().unwrap_or_default().to_string());
                }
            }
            if serde != schema_names {
                let only_serde: Vec<_> = serde.difference(&schema_names).collect();
                let only_schema: Vec<_> = schema_names.difference(&serde).collect();
                disagreements.push(format!(
                    "{where_}: serde accepts {only_serde:?} that the schema does not declare; the \
                     schema declares {only_schema:?} that serde refuses"
                ));
            }
        }
    }
    assert!(
        disagreements.is_empty(),
        "the argument check and the deserialiser disagree about which names an object takes. A \
         name serde accepts and the schema does not declare is REFUSED by the check before serde \
         sees it — add it to the field's #[schemars(extend(\"{ALIASES}\" = [...]))] beside its \
         #[serde(alias)]:\n  {}",
        disagreements.join("\n  ")
    );
    assert!(
        compared >= 200,
        "only {compared} objects were compared ({} unverifiable: {:?}) — the probe is not \
         reaching serde",
        unverifiable.len(),
        unverifiable.iter().take(10).collect::<Vec<_>>()
    );
}
