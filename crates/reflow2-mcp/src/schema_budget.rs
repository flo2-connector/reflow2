//! `describe_schema` KEEPS TO A REQUESTED BUDGET.
//!
//! `req:the-primes-trials-small-tool-snags-are-fixed`, F3. Measured on 0.85.0
//! with `budget_chars: 4000`: the whole vocabulary answered 99,494 bytes,
//! `node_type: Verification` 45,974, and `from`/`to` 25,558. The prose was
//! trimmed, and the STRUCTURE alone — every edge with its endpoints, matches
//! and properties — was still six to twenty-five times the budget, because
//! the reply refused to shorten a list ("completeness IS the answer").
//!
//! Completeness is kept at the level of NAMES. When a reply is still over the
//! budget after its prose is cut, it is compacted in two steps, each said in
//! the `budget` block:
//!
//! 1. DETAIL WITHHELD: each edge entry keeps `edge_type`, `from` and `to`; each
//!    property keeps its name, type, `required` and legal values; the readings,
//!    hints, match tiers and an edge's own properties are withheld.
//! 2. LISTS SHORTENED, NAMES KEPT: if that is still over, the lists are
//!    shortened (`budget.lists_shortened` says which, shown of total), and
//!    `all_names` lists every edge or node type name each list held, so nothing
//!    is dropped without its name being given.
//!
//! Narrowing the question (`node_type`, or `from` and `to`) or raising
//! `budget_chars` gives the detail back.

use serde_json::{Map, Value, json};

/// Keys that are reading aids, not structure: withheld in step 1.
const DETAIL_KEYS: &[&str] = &[
    "hint",
    "reads_as",
    "reading",
    "from_match",
    "to_match",
    "drawn_by",
    "description",
    "extraction_hint",
    "cues",
    "confused_with",
    "counter_example",
    "mathematics",
];

fn chars_of(v: &Value) -> usize {
    serde_json::to_string(v).map_or(usize::MAX, |s| s.chars().count())
}

fn strip_detail(v: &mut Value, depth: usize) {
    match v {
        Value::Object(m) => {
            if depth > 0 {
                for k in DETAIL_KEYS {
                    m.remove(*k);
                }
                if m.contains_key("edge_type") {
                    m.remove("properties");
                }
            }
            for (k, child) in m.iter_mut() {
                if k != "budget" {
                    strip_detail(child, depth + 1);
                }
            }
        }
        Value::Array(items) => {
            for child in items {
                strip_detail(child, depth + 1);
            }
        }
        _ => {}
    }
}

/// The names each top-level list holds: an entry's `edge_type`, else its
/// `node_type`.
fn names_of_lists(m: &Map<String, Value>) -> Map<String, Value> {
    let mut out = Map::new();
    for (k, v) in m {
        let Value::Array(items) = v else { continue };
        let names: Vec<Value> = items
            .iter()
            .filter_map(|i| {
                i.get("edge_type")
                    .or_else(|| i.get("node_type"))
                    .and_then(Value::as_str)
                    .map(|s| Value::String(s.to_string()))
            })
            .collect();
        if !names.is_empty() && names.len() == items.len() {
            out.insert(k.clone(), Value::Array(names));
        }
    }
    out
}

/// Bring a finished `describe_schema` reply within `budget`, saying how.
/// A reply already within it is returned unchanged.
pub fn keep_to_budget(reply: Value, budget: usize, hint: &str) -> Value {
    if chars_of(&reply) <= budget {
        return reply;
    }
    let mut compact = reply.clone();
    strip_detail(&mut compact, 0);
    let note_detail = "DETAIL WITHHELD TO FIT: each edge keeps `edge_type`, `from` and `to`, each \
                       property its name, type, `required` and legal values; readings, hints, \
                       match tiers and an edge's own properties are withheld. Narrow with \
                       `node_type`, or `from` and `to`, or raise `budget_chars`, to read them.";
    let mark = |v: &mut Value, note: &str| {
        if let Some(m) = v.as_object_mut() {
            let block = m.entry("budget").or_insert_with(|| json!({}));
            if let Some(b) = block.as_object_mut() {
                b.insert("applied".into(), json!(true));
                b.insert("budget_chars".into(), json!(budget));
                b.insert("compacted".into(), json!(note));
            }
        }
    };
    mark(&mut compact, note_detail);
    if chars_of(&compact) <= budget {
        return compact;
    }
    // Step 2: shorten the lists, keeping every name.
    let names = compact.as_object().map(names_of_lists).unwrap_or_default();
    let mut without_budget = compact.clone();
    if let Some(m) = without_budget.as_object_mut() {
        m.remove("budget");
    }
    let names_chars = chars_of(&Value::Object(names.clone()));
    let room = budget.saturating_sub(names_chars + 600);
    let mut shortened = crate::reply_budget::bound_reply_sampling(without_budget, room, hint);
    if let Some(m) = shortened.as_object_mut() {
        if !names.is_empty() {
            m.insert("all_names".into(), Value::Object(names));
        }
        let block = m.entry("budget").or_insert_with(|| json!({}));
        if let Some(b) = block.as_object_mut() {
            b.insert("applied".into(), json!(true));
            b.insert("budget_chars".into(), json!(budget));
            b.insert(
                "compacted".into(),
                json!(format!(
                    "{note_detail} THE LISTS ARE SHORTENED TOO, AND NO NAME IS DROPPED: \
                     `budget.lists_shortened` says which list shows how many of how many, and \
                     `all_names` lists every name each one held."
                )),
            );
        }
    }
    shortened
}

#[cfg(test)]
mod a_budget_is_kept {
    use super::*;

    #[test]
    fn detail_goes_first_and_every_name_survives_a_shortened_list() {
        let edges: Vec<Value> = (0..80)
            .map(|i| {
                json!({"edge_type": format!("EDGE_{i}"), "from": ["A"], "to": ["B"],
                       "hint": "x".repeat(200), "reads_as": "y", "properties": [{"name": "p"}]})
            })
            .collect();
        let reply = json!({"node_type": "A", "incoming": edges});
        let out = keep_to_budget(reply, 3000, "narrow it");
        assert!(chars_of(&out) <= 3000, "{}", chars_of(&out));
        let names = out["all_names"]["incoming"].as_array().unwrap();
        assert_eq!(names.len(), 80, "every name is kept");
        let shown = out["incoming"].as_array().unwrap();
        assert!(shown.len() < 80);
        assert!(shown[0].get("hint").is_none() && shown[0].get("properties").is_none());
    }
}
