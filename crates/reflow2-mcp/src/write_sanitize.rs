//! EVERY WRITE IS CLEANED OF HIDDEN CHARACTERS, and says so
//! (`req:every-write-and-every-import-is-cleaned-of-hidden-characters-and-says-what-was-stripped`,
//! `dec:idea-bl-41s-mechanical-half-is-built-now-for-text-a-hosted-design-receives`).
//!
//! `reflow2_core::sanitize_text` strips the characters whose only purpose is to
//! make text read one way to a person and another to a machine: Unicode tag
//! characters, bidirectional overrides, and zero-width or hidden formatting.
//! Until 2026-11-20 it ran in exactly one place, INGEST extraction
//! (`ingest.rs::sanitize_extracted`). Every other write stored what it was sent.
//! On a design several people write to, that is someone else's text reaching
//! your agent. An outside review of v0.101.0 found the gap
//! (`fact:root-cause-bl-41s-trigger-for-the-mechanical-half-is-met-on-shared-hosted-designs-and-nothing-re-raised-it-2026-11-20`).
//!
//! So the `call_tool` choke point runs every write tool's arguments through
//! [`clean_arguments`] before the handler sees them. A new write tool joins by
//! being served, the same way the receipt does.
//!
//! THREE RULES, each the requirement's:
//!
//! 1. **Text is cleaned and the reply says so.** What was removed, by kind, and
//!    from which fields, rides the reply as `sanitized`. Nothing is stripped
//!    silently (rule 6).
//! 2. **An id is never cleaned; it is refused.** The store refuses only NUL in
//!    a key segment, so `req:a\u{200B}b` and `req:ab` are two ids that read the
//!    same. Cleaning the first would quietly address the second. A string that
//!    reads as an id once its hidden characters are gone is therefore refused,
//!    naming the field, and the caller sends it again without them.
//! 3. **An import is never rewritten.** `import_graph` stores its document as
//!    given, because an export's content is its identity (the rule in
//!    `reflow2_core::sanitize`). Its reply reports every field holding hidden
//!    characters instead ([`scan_export`]). Anthony's word, 2026-11-20:
//!    "Detect and report".
//!
//! ZERO-WIDTH JOINER IS KEPT, as `sanitize_text` keeps it, because emoji
//! sequences need it.

use reflow2_core::{SanitizeReport, sanitize_text};
use serde_json::{Value, json};

/// Write tools whose arguments are NOT cleaned here, each with its reason.
/// `import_graph` carries a whole export, which is stored as given and
/// reported on by [`scan_export`].
pub const NOT_CLEANED: &[&str] = &["import_graph"];

/// At most this many field paths are named in a reply; the rest are counted.
const FIELDS_SHOWN: usize = 20;

#[derive(Default)]
struct Found {
    report: SanitizeReport,
    fields: Vec<String>,
}

/// Does `s` read as a node or contributor id (`type:slug`, `who:ajs`,
/// `<graph_id>:req:x`)? No whitespace, a `:` after a short prefix of letters,
/// digits, `_`, `-` or `.`.
fn reads_as_id(s: &str) -> bool {
    if s.is_empty() || s.chars().any(char::is_whitespace) {
        return false;
    }
    match s.split_once(':') {
        Some((prefix, rest)) => {
            !prefix.is_empty()
                && !rest.is_empty()
                && prefix
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        }
        None => false,
    }
}

fn child(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{path}.{key}")
    }
}

fn walk(value: &mut Value, path: &str, found: &mut Found) -> Result<(), String> {
    match value {
        Value::String(s) => {
            let (clean, report) = sanitize_text(s);
            if report.is_clean() {
                return Ok(());
            }
            if reads_as_id(&clean) {
                return Err(format!(
                    "`{path}` holds {} inside what reads as an id ({:?} once they are removed). \
                     An id is never cleaned, because the cleaned id would name a different node \
                     from the one written, and the two read the same. Send it again without them.",
                    report.describe(),
                    clean
                ));
            }
            let clean = clean.into_owned();
            *s = clean;
            found.report.merge(&report);
            found.fields.push(path.to_string());
        }
        Value::Array(items) => {
            for (i, item) in items.iter_mut().enumerate() {
                walk(item, &format!("{path}[{i}]"), found)?;
            }
        }
        Value::Object(map) => {
            for (key, item) in map.iter_mut() {
                walk(item, &child(path, key), found)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn block(found: &Found, kinds_key: &str, note: &str) -> Value {
    let shown: Vec<&String> = found.fields.iter().take(FIELDS_SHOWN).collect();
    let mut out = json!({ "fields": shown, "note": note });
    out[kinds_key] = json!(found.report);
    if found.fields.len() > FIELDS_SHOWN {
        out["fields_not_shown"] = json!(found.fields.len() - FIELDS_SHOWN);
    }
    out
}

/// Clean a write tool's arguments in place. `Ok(None)`: nothing was hidden.
/// `Ok(Some(block))`: the `sanitized` block the reply carries. `Err(message)`:
/// an id held hidden characters, and the call is refused with this message.
pub fn clean_arguments(
    arguments: &mut serde_json::Map<String, Value>,
) -> Result<Option<Value>, String> {
    let mut found = Found::default();
    for (key, value) in arguments.iter_mut() {
        walk(value, key, &mut found)?;
    }
    if found.report.is_clean() {
        return Ok(None);
    }
    Ok(Some(block(
        &found,
        "stripped",
        &format!(
            "Hidden characters were removed before this was stored: {}. They make text read \
             differently to a person than to a machine, so reflow2 stores none of them in \
             what a write sends; the zero-width joiner emoji use is kept. The text as stored \
             is the text you sent without them.",
            found.report.describe()
        ),
    )))
}

/// What an import's document holds in hidden characters, by node and edge
/// field, WITHOUT changing it. `None` when it holds none.
pub fn scan_export(doc: &reflow2_core::export::GraphExport) -> Option<Value> {
    let mut found = Found::default();
    // An id in a document is reported like any other field: nothing here
    // stores what it reads, so nothing is refused.
    for n in &doc.nodes {
        let node_id = Value::String(n.node_id.clone());
        report_only(&node_id, &format!("node {}.node_id", n.node_id), &mut found);
        let props = serde_json::to_value(&n.properties).unwrap_or(Value::Null);
        report_only(&props, &format!("node {}", n.node_id), &mut found);
    }
    for e in &doc.edges {
        let props = serde_json::to_value(&e.properties).unwrap_or(Value::Null);
        let path = format!("edge {} {} {}", e.from_id, e.edge_type, e.to_id);
        report_only(&props, &path, &mut found);
    }
    if found.report.is_clean() {
        return None;
    }
    Some(block(
        &found,
        "found",
        &format!(
            "This document holds hidden characters: {}. They make text read differently to a \
             person than to a machine. The import stored the document exactly as given, \
             because an export's content is its identity; clean them in the source and import \
             it again if they should not be there.",
            found.report.describe()
        ),
    ))
}

/// [`walk`] for a document that is only read: every string with hidden
/// characters is counted and named, an id included, and nothing is changed or
/// refused.
fn report_only(value: &Value, path: &str, found: &mut Found) {
    match value {
        Value::String(s) => {
            let (_, report) = sanitize_text(s);
            if !report.is_clean() {
                found.report.merge(&report);
                found.fields.push(path.to_string());
            }
        }
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                report_only(item, &format!("{path}[{i}]"), found);
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                report_only(item, &child(path, key), found);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_reads_as_one_and_prose_does_not() {
        assert!(reads_as_id("req:ab"));
        assert!(reads_as_id("who:ajs"));
        assert!(reads_as_id("1a97fc1f1de4d5d2:req:x"));
        assert!(!reads_as_id("a sentence: with a colon"));
        assert!(!reads_as_id("plain"));
        assert!(!reads_as_id(":x"));
    }

    #[test]
    fn text_is_cleaned_and_named_by_path() {
        let mut args = json!({
            "statement": "safe\u{200B} text\u{E0041}",
            "related_to": [{"evidence": "\u{202E}reversed"}],
            "emoji": "👨\u{200D}👩",
        });
        let block = clean_arguments(args.as_object_mut().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(args["statement"], "safe text");
        assert_eq!(args["related_to"][0]["evidence"], "reversed");
        assert_eq!(args["emoji"], "👨\u{200D}👩", "ZWJ is kept");
        let fields: Vec<&str> = block["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(fields.contains(&"statement") && fields.contains(&"related_to[0].evidence"));
        assert_eq!(block["stripped"]["unicode_tag"], 1);
        assert_eq!(block["stripped"]["bidi_control"], 1);
        assert_eq!(block["stripped"]["hidden_formatting"], 1);
    }

    #[test]
    fn an_id_with_hidden_characters_is_refused_not_cleaned() {
        let mut args = json!({"id": "req:a\u{200B}b", "statement": "fine"});
        let err = clean_arguments(args.as_object_mut().unwrap()).unwrap_err();
        assert!(
            err.contains("`id`") && err.contains("never cleaned"),
            "{err}"
        );
    }

    #[test]
    fn clean_arguments_answer_nothing() {
        let mut args = json!({"id": "req:ab", "statement": "plain"});
        assert!(
            clean_arguments(args.as_object_mut().unwrap())
                .unwrap()
                .is_none()
        );
    }
}
