//! A served tool described in full: its input schema and the lessons the OPENED
//! design holds for it — what `tools/list` gives an MCP client of that design,
//! readable through a door that never lists.
//!
//! `req:the-cli-describes-any-tool-with-its-full-schema-and-lessons`, step 4 of
//! `epoch:planned-the-call-door-works-for-an-agent-that-cannot-use-mcp`
//! (`dec:idea-the-cli-describes-a-tool-with-its-schema-and-lessons`).
//!
//! # The class this closes
//!
//! The `--call` door reaches the CALL path and not the LISTING path, so
//! everything reflow2 delivers at `tools/list` was invisible through it. Measured
//! on 0.77.0 (2026-10-02):
//!
//! · argument shapes were learned one refusal at a time — the door held every
//!   tool's input schema and printed none of it, `find_tools` returned top-level
//!   parameter NAMES, and no served tool took a tool's name
//!   (`fact:root-cause-argument-shapes-are-learned-by-refusal-because-the-door-holds-the-full-schema-and-prints-none-of-it-2026-10-02`);
//! · the lessons a design hangs on a tool (`steps`) rode only `tools/list` —
//!   218 lessons on 80 of 195 tools on reflow2's own design, and no reply the
//!   door could reach carried one
//!   (`fact:root-cause-tool-lessons-reach-only-tools-list-and-no-door-reply-carries-one-2026-10-02`);
//! · and the one listing the door did build was built on an EMPTY in-memory
//!   design, so even printing it would have dropped every lesson
//!   (`fact:the-call-doors-tool-list-is-built-on-an-empty-design-so-it-holds-no-lessons-2026-10-02`).
//!
//! # One listing, three readers
//!
//! [`ToolListing`] is built by `ReflowService::tool_listing` from ONE read of the
//! design, by the same function `tools/list` uses — so what is described here
//! cannot disagree with what a session on that design is offered. Three readers
//! render it, all through this module: `reflow2-mcp --describe <tool>`,
//! `reflow2-mcp --list-tools`, and the served `describe_schema` with `tool`,
//! which reaches every door a served tool reaches (`--call`, a gateway, a client
//! that defers or truncates tool schemas).
//!
//! # Two forms
//!
//! FULL is the `tools/list` entry itself, unaltered. BRIEF, the default, keeps
//! EVERY structural fact of the input schema — each property, type, `required`
//! list, enum value, `$defs` shape and `$ref` — and cuts only prose: each field
//! description to its first sentence. The lessons ride whole in both (in the
//! full form, inside the description, where `tools/list` puts them). A brief
//! reply is held to the reply budget by `reply_budget::bound_reply`, which
//! withholds prose before anything else and says so.

use std::collections::BTreeMap;

use rmcp::model::Tool;
use serde_json::{Map, Value, json};

use crate::lessons::Lesson;

/// The served tool list of one design, and the lessons it was built with.
#[derive(Debug, Clone)]
pub struct ToolListing {
    /// The list exactly as `tools/list` serves it to a client of this design.
    pub served: Vec<Tool>,
    /// The same list before this design's lessons were appended — the brief
    /// form's description, so the lessons are carried once, as records.
    pub bare: Vec<Tool>,
    /// This design's lessons, by the step (tool or skill) they name.
    pub lessons: BTreeMap<String, Vec<Lesson>>,
}

/// Where the listing came from, for the sentence that says why a tool has no
/// lessons.
#[derive(Debug, Clone)]
pub enum Source {
    /// A design was opened; the label names it (a path, or "this design").
    Design(String),
    /// No design was opened: the reason, e.g. "there is no design at …".
    NoDesign(String),
}

/// Which door is asking, so "how do I get the rest?" is answered in the words
/// that door can act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Door {
    /// `reflow2-mcp --describe` / `--list-tools`.
    Cli,
    /// The served `describe_schema` with `tool`.
    Served,
}

/// How the whole of one tool is asked for, through each door.
fn full_route(door: Door, tool: &str) -> String {
    match door {
        Door::Cli => format!(
            "`reflow2-mcp --describe {tool} --full` prints the whole description and input \
             schema, exactly as tools/list serves them on this design."
        ),
        Door::Served => format!(
            "`describe_schema` with `tool: \"{tool}\"` and `full: true` returns the whole \
             description and input schema, exactly as tools/list serves them on this design."
        ),
    }
}

/// The sentence a `find_tools` reply carries, so a caller who has found a tool
/// by its job can read how to call it — through whichever door it came in.
pub const ROUTE_FROM_FIND_TOOLS: &str = "To read a tool's whole input schema — nested shapes, \
     allowed values, required fields — and the lessons this design holds for it, call \
     `describe_schema` with `tool` (through the shell door: `reflow2-mcp --call describe_schema \
     --args '{\"tool\":\"<name>\"}'`), or run `reflow2-mcp --describe <name>`. Each is the \
     tools/list entry a session on this design is given, briefly by default; ask for `full` to \
     have it unaltered.";

/// The lessons block for one tool, in both forms: how many, and either the
/// lessons themselves or the reason there are none.
fn lessons_block(tool: &str, lessons: &[Lesson], source: &Source, with_items: bool) -> Value {
    if lessons.is_empty() {
        let none_because = match source {
            Source::Design(label) => format!(
                "{label} holds no lesson for `{tool}`: no DesignRule and no current TemporalFact \
                 names it in `steps`. So tools/list serves its description unchanged. A lesson \
                 is hung on a tool with record_finding or add_design_rule (`steps`)."
            ),
            Source::NoDesign(why) => format!(
                "{why}, so there are no lessons to serve: this is the surface every design \
                 serves before it holds a lesson. Point --graph-path at a design to read the \
                 lessons it holds for `{tool}`."
            ),
        };
        return json!({ "count": 0, "items": [], "none_because": none_because });
    }
    let note = format!(
        "{} lesson(s) THIS DESIGN holds for `{tool}`, recorded by earlier sessions on it. \
         tools/list appends them to the tool's description — the moment before the call — \
         because a lesson filed elsewhere was measured not to change the next call. Read them \
         before you call it; get_node reads any one in full.",
        lessons.len()
    );
    if with_items {
        json!({ "count": lessons.len(), "note": note, "items": lessons })
    } else {
        json!({
            "count": lessons.len(),
            "note": note,
            "where": "appended to `served.description`, exactly as tools/list carries them",
        })
    }
}

/// The served names closest to one that is not served, and the route to find
/// one by its job.
fn not_served(listing: &ToolListing, name: &str) -> String {
    let served: std::collections::BTreeSet<String> =
        listing.served.iter().map(|t| t.name.to_string()).collect();
    let near = crate::lessons::nearest(name, &served);
    let nearest = if near.is_empty() {
        String::new()
    } else {
        format!(" The nearest served names: {}.", near.join(", "))
    };
    format!(
        "no tool named `{name}` is served, so there is nothing to describe.{nearest} \
         find_tools finds a tool from a sentence in your own words."
    )
}

/// The first sentence of a description, or the whole of a short one. A cut
/// says it was cut.
fn first_sentence(text: &str) -> String {
    const AT_LEAST: usize = 40;
    const AT_MOST: usize = 240;
    let t = text.trim();
    let chars: Vec<char> = t.chars().collect();
    let mut end = None;
    for i in 0..chars.len() {
        if i + 1 >= AT_MOST {
            break;
        }
        let ends = matches!(chars[i], '.' | '!' | '?')
            && chars.get(i + 1).is_some_and(|c| c.is_whitespace());
        let paragraph = chars[i] == '\n' && chars.get(i + 1) == Some(&'\n');
        if (ends || paragraph) && i + 1 >= AT_LEAST {
            end = Some(i + 1);
            break;
        }
    }
    match end {
        Some(e) if e < chars.len() => {
            let head: String = chars[..e].iter().collect();
            format!("{} …", head.trim_end())
        }
        Some(_) => t.to_string(),
        None if chars.len() <= AT_MOST => t.to_string(),
        None => {
            let head: String = chars[..AT_MOST].iter().collect();
            format!("{head}…")
        }
    }
}

/// The brief form of an input schema: every key kept, every value kept, and
/// each `description` cut to its first sentence. Values that are DATA rather
/// than schema (`default`, `enum`, `const`, `examples`) are never walked, so a
/// default that happens to hold a `description` key is never rewritten.
pub fn brief_schema(schema: &Value) -> Value {
    match schema {
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                let kept = match (k.as_str(), v) {
                    ("description", Value::String(s)) => Value::String(first_sentence(s)),
                    ("default" | "enum" | "const" | "examples", _) => v.clone(),
                    _ => brief_schema(v),
                };
                out.insert(k.clone(), kept);
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(brief_schema).collect()),
        other => other.clone(),
    }
}

fn find<'a>(tools: &'a [Tool], name: &str) -> Option<&'a Tool> {
    tools.iter().find(|t| t.name == name)
}

fn lessons_for<'a>(listing: &'a ToolListing, name: &str) -> &'a [Lesson] {
    listing.lessons.get(name).map(Vec::as_slice).unwrap_or(&[])
}

/// ONE tool described. `full` is the `tools/list` entry unaltered (under
/// `served`); otherwise the brief form, held to `budget` characters. `Err` is
/// the refusal for a name nothing serves.
pub fn describe_one(
    listing: &ToolListing,
    name: &str,
    full: bool,
    door: Door,
    source: &Source,
    budget: usize,
) -> Result<Value, String> {
    let Some(served) = find(&listing.served, name) else {
        return Err(not_served(listing, name));
    };
    let lessons = lessons_for(listing, name);
    if full {
        let entry = serde_json::to_value(served).map_err(|e| e.to_string())?;
        return Ok(json!({
            "tool": name,
            "form": "full",
            "served": entry,
            "lessons": lessons_block(name, lessons, source, false),
        }));
    }
    let bare = find(&listing.bare, name).unwrap_or(served);
    let brief = json!({
        "tool": name,
        "form": "brief",
        "annotations": served.annotations,
        "description": bare.description,
        "input_schema": brief_schema(&Value::Object((*served.input_schema).clone())),
        "lessons": lessons_block(name, lessons, source, true),
        "full_form": full_route(door, name),
    });
    Ok(crate::reply_budget::bound_reply(
        brief,
        budget,
        &format!(
            "Every property, type, required list, allowed value and lesson id is kept; only \
             prose was cut. {}",
            full_route(door, name)
        ),
    ))
}

/// The lesson tally a whole listing carries: how many lessons ride on served
/// tools, and on how many tools.
fn tally(listing: &ToolListing, source: &Source) -> Value {
    let mut count = 0usize;
    let mut tools = 0usize;
    for t in &listing.served {
        let n = lessons_for(listing, t.name.as_ref()).len();
        count += n;
        tools += usize::from(n > 0);
    }
    if count == 0 {
        let none_because = match source {
            Source::Design(label) => format!(
                "{label} holds no lesson for any served tool, so tools/list serves every \
                 description unchanged."
            ),
            Source::NoDesign(why) => format!(
                "{why}, so there are no lessons to serve: this is the surface every design \
                 serves before it holds a lesson."
            ),
        };
        return json!({ "count": 0, "tools_with_lessons": 0, "none_because": none_because });
    }
    json!({
        "count": count,
        "tools_with_lessons": tools,
        "note": "Each tool's lessons are appended to its description, as tools/list carries \
                 them. `--describe <tool>` lists one tool's lessons as records.",
    })
}

/// EVERY served tool. `full` is the `tools/list` array unaltered (under
/// `tools`), for a generator rendering a reference; otherwise an index — each
/// tool's name, whether it only reads, its required arguments and its lesson
/// count — held to `budget` characters. The index carries no sentences on
/// purpose: 195 first sentences alone overrun the budget, and finding a tool by
/// its job is `find_tools`' work.
pub fn describe_all(listing: &ToolListing, full: bool, source: &Source, budget: usize) -> Value {
    if full {
        return json!({
            "form": "full",
            "count": listing.served.len(),
            "tools": listing.served,
            "lessons": tally(listing, source),
        });
    }
    let tools: Vec<Value> = listing
        .bare
        .iter()
        .map(|t| {
            let required = t
                .input_schema
                .get("required")
                .cloned()
                .unwrap_or_else(|| json!([]));
            json!({
                "tool": t.name,
                "read_only": t.annotations.as_ref().and_then(|a| a.read_only_hint),
                "required": required,
                "lessons": lessons_for(listing, t.name.as_ref()).len(),
            })
        })
        .collect();
    crate::reply_budget::bound_reply(
        json!({
            "form": "brief",
            "count": tools.len(),
            "tools": tools,
            "lessons": tally(listing, source),
            "describe": "`reflow2-mcp --describe <tool>` gives one tool's whole input schema and \
                         the lessons this design holds for it; `--list-tools --full` gives every \
                         tool exactly as tools/list serves it; `find_tools` finds a tool by its \
                         job.",
        }),
        budget,
        "Every tool, its required arguments and its lesson count are kept. `--describe <tool>` \
         reads one tool in full.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_first_sentence_is_cut_where_it_ends_and_says_so() {
        let s = "The node at the other end of the relation. The edge runs TO it unless told.";
        assert_eq!(
            first_sentence(s),
            "The node at the other end of the relation. …"
        );
        assert_eq!(first_sentence("Short."), "Short.");
        // "e.g. x" this early is not a sentence end.
        let eg = "e.g. a path. Then a much longer explanation follows here, past forty.";
        assert!(
            first_sentence(eg).starts_with("e.g. a path. Then"),
            "{}",
            first_sentence(eg)
        );
    }

    /// The brief form keeps every structural fact: with every description
    /// removed from both, the two schemas are identical.
    #[test]
    fn the_brief_schema_differs_from_the_served_one_only_in_prose() {
        let served = json!({
            "type": "object",
            "description": "One. Two.",
            "properties": {
                "description": { "type": "string", "description": "A property named description. More." },
                "related_to": { "type": "array", "items": { "$ref": "#/$defs/Link" } }
            },
            "required": ["description"],
            "$defs": { "Link": {
                "type": "object",
                "properties": { "relation": { "type": "string", "enum": ["CAUSES", "BLOCKS"],
                    "description": "Which relation, of many more words than forty characters. And more." } },
                "required": ["relation"]
            } },
            "default": { "description": "data, never cut. Not a schema." }
        });
        let brief = brief_schema(&served);
        fn strip(v: &Value) -> Value {
            match v {
                Value::Object(m) => Value::Object(
                    m.iter()
                        .filter(|(k, v)| !(k.as_str() == "description" && v.is_string()))
                        .map(|(k, v)| {
                            let kept = if k == "default" { v.clone() } else { strip(v) };
                            (k.clone(), kept)
                        })
                        .collect(),
                ),
                Value::Array(a) => Value::Array(a.iter().map(strip).collect()),
                o => o.clone(),
            }
        }
        assert_eq!(strip(&brief), strip(&served));
        assert_eq!(
            brief["$defs"]["Link"]["properties"]["relation"]["enum"],
            json!(["CAUSES", "BLOCKS"])
        );
        assert_eq!(
            brief["default"]["description"],
            json!("data, never cut. Not a schema.")
        );
    }
}
