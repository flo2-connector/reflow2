//! A WRITE replies with a RECEIPT, not the whole stored node — one contract,
//! applied at the one place every tool call passes through.
//!
//! # The failure this ends
//!
//! Until 2026-09-29 every write answered with the node as stored, whatever the
//! call changed, and `replace_text` added each replaced field's prior value in
//! full. Measured on 0.74.0 with no gateway in the way
//! (`fact:root-cause-a-write-replies-with-the-whole-stored-node-and-replace-text-adds-each-prior-field-2026-09-29`):
//! - a 12-character append to a 1-character rationale returned 7,501
//!   characters, because the node's untouched 6 KB decision came back with it;
//! - `set_decision_status` spent 6,414 characters to move one status;
//! - `add_decision` spent 6,330 characters echoing its own input;
//! - a designer agent echoed about 290 KB for about 40 KB of edits over three
//!   rounds.
//!
//! No reply shape for a write had ever been stated. Each handler returned the
//! stored node by convention, so fixing one handler would have left the others.
//!
//! # The contract (`req:a-write-replies-with-a-receipt-not-the-whole-node`)
//!
//! Shaped HERE, at `call_tool`, for every tool the served surface marks as a
//! write (`read_only_hint` false — the predicate `--call` and a read-only
//! surface already use), so a new write tool joins by being served:
//! - a stored value of at most [`SHORT`] characters is echoed as stored: a
//!   status, a kind, a short name. That is what a writer checks;
//! - a longer one is given by size under `elided`. The writer sent it and can
//!   read it back with `get_node`;
//! - a revise names each replaced field's `prior_chars` and `after_chars`, and
//!   `prior_in`, the snapshot that keeps the prior value;
//! - every warning, note, drawn edge and removal report (`shortened`,
//!   `fields_at_risk`) is left exactly as the handler wrote it;
//! - `echo: "node"` returns the reply exactly as the handler built it.
//!
//! # What is never elided, and why
//!
//! A prior value that NOTHING else holds (`fields_at_risk`) is echoed in full
//! whatever its size. Then the reply is its only copy, and the reason the prior
//! block exists at all is that a lost paragraph could not be put back
//! (`ReplacedField`'s own doc). A receipt that dropped it would reproduce the
//! failure `req:a-revising-write-says-what-it-removed` was built to end.
//!
//! # Why a shaping layer and not a smaller reply in each handler
//!
//! - Per handler is how the class arose: about 150 write handlers each built
//!   their own reply. A 151st would join nothing.
//! - Handlers keep building the full reply, so `echo: "node"` costs nothing and
//!   tests that call a handler directly still read everything it wrote.
//! - The shape is recognised structurally: a node is an object carrying
//!   `node_id`, `node_type` and `properties`, and an edge carries `edge_type`,
//!   `from_id`, `to_id` and `properties`. Those are the dto shapes every write
//!   serialises through (`crate::dto`).

use std::sync::Arc;

use rmcp::model::{JsonObject, Tool};
use serde_json::{Map, Value};

/// A stored value at most this many characters long is echoed; a longer one
/// is given by size. Chosen to keep what a writer checks — a status, a kind, a
/// short name, a reference — and to drop what it wrote at length and already
/// holds.
pub const SHORT: usize = 200;

/// The argument every write tool takes.
pub(crate) const ECHO: &str = "echo";

/// What a write's reply carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Echo {
    /// The default: a receipt.
    Receipt,
    /// The reply exactly as the handler built it — the whole stored node and
    /// every prior value.
    Node,
}

const ECHO_VALUES: [&str; 2] = ["receipt", "node"];

const ECHO_DESCRIPTION: &str = "What the reply carries. `receipt` (the default) names the node, \
    echoes each stored value of at most 200 characters, gives longer ones by size, and names the \
    snapshot that keeps anything a revise replaced. `node` returns the whole stored node and \
    every prior value.";

/// Shown once in a reply that elided anything, so the size-only fields read as
/// a choice the caller can reverse rather than as missing data.
const NOTE: &str = "A write replies with a receipt: stored values over 200 characters are given \
    by size (`elided`, `prior_chars`), and `prior_in` names the snapshot that keeps a replaced \
    value. Pass echo: \"node\" for the whole stored node and every prior value.";

/// Is this served tool a write? Read off its annotation, as every other
/// "does it write?" question on this server is.
pub(crate) fn is_write(tool: &Tool) -> bool {
    !tool
        .annotations
        .as_ref()
        .and_then(|a| a.read_only_hint)
        .unwrap_or(false)
}

/// Take `echo` out of a write call's arguments, so the handler never sees an
/// argument its request type does not declare. Absent means a receipt.
/// Anything but the two values is refused, before the handler runs, naming
/// both.
pub(crate) fn take_echo(args: Option<&mut JsonObject>) -> Result<Echo, String> {
    let Some(raw) = args.and_then(|a| a.remove(ECHO)) else {
        return Ok(Echo::Receipt);
    };
    match raw.as_str() {
        Some("receipt") => Ok(Echo::Receipt),
        Some("node") => Ok(Echo::Node),
        _ => Err(format!(
            "`echo` takes \"receipt\" (the default: the node's id, its short values, and the size \
             of each long one) or \"node\" (the whole stored node and every prior value); got \
             {raw}. Nothing was written."
        )),
    }
}

/// Declare `echo` on every write tool's input schema, so a caller can find it
/// and a client that validates arguments against the schema lets it through.
pub(crate) fn declare_echo(tools: Vec<Tool>) -> Vec<Tool> {
    tools
        .into_iter()
        .map(|mut t| {
            t.input_schema = published_input_schema(&t);
            t
        })
        .collect()
}

/// One tool's input schema as it is PUBLISHED: a write's carries `echo`. The
/// argument check (`crate::arguments`) reads this, so what is checked is what
/// a caller was shown — a read's schema is shared, not copied.
pub(crate) fn published_input_schema(t: &Tool) -> Arc<JsonObject> {
    if !is_write(t) {
        return Arc::clone(&t.input_schema);
    }
    let mut schema: JsonObject = (*t.input_schema).clone();
    let props = schema
        .entry("properties")
        .or_insert_with(|| Value::Object(Map::new()));
    if let Value::Object(p) = props {
        p.insert(
            ECHO.into(),
            serde_json::json!({
                "type": "string",
                "enum": ECHO_VALUES,
                "default": "receipt",
                "description": ECHO_DESCRIPTION,
            }),
        );
    }
    Arc::new(schema)
}

/// A write's reply, as a receipt. `v` is the structured reply the handler
/// built; the result is the same object with long stored values given by size.
/// Public so a test can measure one call's reply both ways — the reply as built
/// (`echo: "node"`) and as sent by default — without making the write twice.
pub fn receipt(mut v: Value) -> Value {
    let mut elided = false;
    shape(&mut v, &mut elided);
    if elided && let Value::Object(o) = &mut v {
        o.insert("receipt_note".into(), Value::String(NOTE.into()));
    }
    v
}

/// Characters a value occupies: a string's own length, anything else as
/// serialised.
fn size(v: &Value) -> usize {
    match v {
        Value::String(s) => s.chars().count(),
        other => other.to_string().chars().count(),
    }
}

/// A node or an edge as `crate::dto` serialises it.
fn is_record(o: &Map<String, Value>) -> bool {
    o.get("properties").is_some_and(Value::is_object)
        && ((o.contains_key("node_id") && o.contains_key("node_type"))
            || (o.contains_key("edge_type")
                && o.contains_key("from_id")
                && o.contains_key("to_id")))
}

/// The stored sizes of the node a revision belongs to: the object itself when
/// it is the node, else its `node` member (`replace_text`'s shape).
fn node_sizes(o: &Map<String, Value>) -> Option<Map<String, Value>> {
    let node = if is_record(o) {
        o
    } else {
        o.get("node")
            .and_then(Value::as_object)
            .filter(|n| is_record(n))?
    };
    let props = node.get("properties")?.as_object()?;
    Some(
        props
            .iter()
            .map(|(k, v)| (k.clone(), Value::from(size(v))))
            .collect(),
    )
}

fn shape(v: &mut Value, elided: &mut bool) {
    match v {
        Value::Array(items) => {
            for x in items {
                shape(x, elided);
            }
        }
        Value::Object(o) => {
            // The revision first: its `after_chars` are read from the node
            // BEFORE the node's own long values are elided.
            let after = node_sizes(o);
            if let Some(Value::Object(rev)) = o.get_mut("revision") {
                shape_revision(rev, after.as_ref(), elided);
            }
            if is_record(o) {
                elide_properties(o, elided);
            }
            for (k, x) in o.iter_mut() {
                if k != "revision" && k != "properties" {
                    shape(x, elided);
                }
            }
        }
        _ => {}
    }
}

/// Move each stored value over [`SHORT`] characters out of `properties` and
/// into `elided`, keyed by field, valued by size.
fn elide_properties(o: &mut Map<String, Value>, elided: &mut bool) {
    let Some(Value::Object(props)) = o.get_mut("properties") else {
        return;
    };
    let long: Vec<String> = props
        .iter()
        .filter(|(_, v)| size(v) > SHORT)
        .map(|(k, _)| k.clone())
        .collect();
    if long.is_empty() {
        return;
    }
    let mut sizes = Map::new();
    for k in long {
        if let Some(v) = props.remove(&k) {
            sizes.insert(k, Value::from(size(&v)));
        }
    }
    o.insert("elided".into(), Value::Object(sizes));
    *elided = true;
}

/// Each replaced field: its size before and after, and — when something else
/// keeps the prior value — where, instead of the value itself.
fn shape_revision(
    rev: &mut Map<String, Value>,
    after: Option<&Map<String, Value>>,
    elided: &mut bool,
) {
    let at_risk: Vec<String> = rev
        .get("fields_at_risk")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();
    let per_field: Map<String, Value> = rev
        .get("fields_preserved_in")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let whole = rev
        .get("prior_state_preserved_in")
        .and_then(Value::as_str)
        .map(String::from);
    let Some(Value::Array(replaced)) = rev.get_mut("replaced") else {
        return;
    };
    for entry in replaced.iter_mut() {
        let Value::Object(e) = entry else { continue };
        let Some(field) = e.get("field").and_then(Value::as_str).map(String::from) else {
            continue;
        };
        let Some(prior_chars) = e.get("prior").map(size) else {
            continue;
        };
        e.insert("prior_chars".into(), Value::from(prior_chars));
        if let Some(n) = after.and_then(|a| a.get(&field)) {
            e.insert("after_chars".into(), n.clone());
        }
        // THE ONLY COPY IS NEVER DROPPED: a prior value nothing else holds is
        // echoed in full whatever its size.
        if prior_chars <= SHORT || at_risk.contains(&field) {
            continue;
        }
        let kept_in = per_field
            .get(&field)
            .and_then(Value::as_str)
            .map(String::from)
            .or_else(|| whole.clone());
        if let Some(snap) = kept_in {
            e.remove("prior");
            e.insert("prior_in".into(), Value::String(snap));
            *elided = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn long(n: usize) -> String {
        "x".repeat(n)
    }

    #[test]
    fn a_node_keeps_its_short_values_and_sizes_its_long_ones() {
        let r = receipt(json!({
            "graph_id": "g", "node_id": "dec:a", "node_type": "Decision",
            "properties": {"status": "accepted", "decision": long(6_000), "name": long(SHORT)},
            "loop_hint": "loop: a hint the handler wrote"
        }));
        assert_eq!(r["properties"]["status"], "accepted");
        assert_eq!(r["properties"]["name"].as_str().map(str::len), Some(SHORT));
        assert!(r["properties"].get("decision").is_none());
        assert_eq!(r["elided"]["decision"], 6_000);
        assert_eq!(r["loop_hint"], "loop: a hint the handler wrote");
        assert!(r["receipt_note"].is_string());
    }

    #[test]
    fn a_reply_with_nothing_long_is_unchanged() {
        let v = json!({"graph_id":"g","node_id":"req:a","node_type":"Requirement",
            "properties":{"name":"short","status":"proposed"}});
        assert_eq!(receipt(v.clone()), v);
    }

    #[test]
    fn a_preserved_prior_value_is_named_by_where_it_is_kept() {
        let r = receipt(json!({
            "edit": {"field": "decision", "mode": "append"},
            "node": {"graph_id":"g","node_id":"dec:a","node_type":"Decision",
                     "properties":{"decision": long(6_170)}},
            "revision": {"replaced":[{"field":"decision","prior": long(6_000)}],
                         "added": [], "changed": true, "prior_content_hash":"sha256:x",
                         "prior_state_preserved_in":"snap:one", "note":"kept"}
        }));
        let e = &r["revision"]["replaced"][0];
        assert!(e.get("prior").is_none());
        assert_eq!(e["prior_chars"], 6_000);
        assert_eq!(e["after_chars"], 6_170);
        assert_eq!(e["prior_in"], "snap:one");
        assert_eq!(r["node"]["elided"]["decision"], 6_170);
        assert_eq!(r["revision"]["note"], "kept");
    }

    #[test]
    fn a_prior_value_nothing_else_holds_is_echoed_in_full() {
        let r = receipt(json!({
            "graph_id":"g","node_id":"dec:a","node_type":"Decision",
            "properties":{"decision": long(10)},
            "revision": {"replaced":[{"field":"decision","prior": long(6_000)}],
                         "added": [], "changed": true, "prior_content_hash":"sha256:x",
                         "fields_at_risk": ["decision"], "note":"at risk"}
        }));
        let e = &r["revision"]["replaced"][0];
        assert_eq!(e["prior"].as_str().map(str::len), Some(6_000));
        assert_eq!(e["prior_chars"], 6_000);
        assert_eq!(e["after_chars"], 10);
    }

    #[test]
    fn a_prior_value_with_no_named_keeper_is_kept_rather_than_lost() {
        let r = receipt(json!({
            "graph_id":"g","node_id":"dec:a","node_type":"Decision",
            "properties":{"decision": "short now"},
            "revision": {"replaced":[{"field":"decision","prior": long(6_000)}],
                         "added": [], "changed": true, "prior_content_hash":"sha256:x", "note":"n"}
        }));
        assert!(r["revision"]["replaced"][0]["prior"].is_string());
    }

    #[test]
    fn an_edge_and_a_nested_node_are_shaped_too() {
        let r = receipt(json!({
            "finding": {"graph_id":"g","node_id":"fact:a","node_type":"TemporalFact",
                        "properties":{"statement": long(900)}},
            "edges": [{"edge_type":"CAUSES","from_id":"a","to_id":"b","graph_id":"g",
                       "properties":{"evidence": long(700)}}]
        }));
        assert_eq!(r["finding"]["elided"]["statement"], 900);
        assert_eq!(r["edges"][0]["elided"]["evidence"], 700);
    }

    #[test]
    fn echo_takes_two_values_and_refuses_the_rest() {
        let mut a = JsonObject::new();
        assert_eq!(take_echo(Some(&mut a)), Ok(Echo::Receipt));
        a.insert(ECHO.into(), json!("node"));
        assert_eq!(take_echo(Some(&mut a)), Ok(Echo::Node));
        assert!(
            !a.contains_key(ECHO),
            "taken out before the handler sees it"
        );
        a.insert(ECHO.into(), json!("all"));
        let e = take_echo(Some(&mut a)).unwrap_err();
        assert!(e.contains("receipt") && e.contains("node"), "{e}");
    }
}
