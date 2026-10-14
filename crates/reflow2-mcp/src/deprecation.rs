//! THE RETIREMENT RULE: anything a client can call or read is removed only
//! after a release that announced it.
//!
//! # The rule, on the owner's word
//!
//! `req:the-mcp-server-keeps-working-as-is-and-door-improvements-are-additive-or-opt-in`,
//! amended on Anthony's word on 2026-10-13: "never removed" becomes "never
//! removed WITHOUT A RELEASE OF NOTICE". Anything a client can call or read (a
//! tool, a field, a node type, a reply shape) is retired in two releases:
//!
//! 1. A DEPRECATING release, in which it still works, says it is going (in its
//!    tool description, or in describe_schema for a schema item), and the
//!    changelog carries a "What to do" line.
//! 2. The NEXT release removes it.
//!
//! "The check that the served surface only grows becomes a check that it
//! shrinks only through a deprecating release" — that check is
//! `tests/the_mcp_surface_only_grows.rs`, which reads the deprecations a
//! release announced out of that release's own describe_schema.
//!
//! For a TOOL, the removal in step 2 ships as a stub under the old name that
//! refuses and names what to do, for that one release, so a removed door is
//! never a silent absence
//! (`dec:a-tool-is-never-deleted-without-usage-evidence-across-projects-and-never-without-a-refusing-stub-first`).
//! Here that stub is the registry entry itself: while an entry names a tool
//! the build no longer serves, a call to the old name is refused with what to
//! do ([`removed_tool`]). The entry comes out in the release after. The same
//! decision says a tool is removed only on usage evidence from more than one
//! project; the entry's `decision` is where that evidence is recorded, and
//! nothing here can check it.
//!
//! # One place declares a deprecation: [`REGISTRY`]
//!
//! Each entry names the item ([`Item`]: a tool, a tool argument, a reply field
//! at a path, a node type, an edge type or a node property), the version that
//! deprecates it, what to do instead in one sentence, and the design id of the
//! decision that retires it. Everything a client sees is generated from it:
//!
//! - THE MARK, where a client reads the item: a deprecated tool's description
//!   and a deprecated argument's description open with "DEPRECATED in
//!   <version>, removed in the next release: <what to do>" ([`mark_tools`]); a
//!   deprecated reply field is named, with the mark, in its tool's description;
//!   describe_schema carries the mark as `deprecated` on a node type, a
//!   property and an edge type, and its full answer lists every entry under
//!   `deprecations` ([`mark_vocabulary`], [`listing`]).
//! - THE NOTICE, on a call that uses one ([`used_by`], [`attach_notice`]).
//! - The stub refusal for a tool that went ([`removed_tool`]).
//!
//! A deprecated item still works exactly as before. With the registry empty,
//! nothing here changes a byte of tools/list or of any reply — every function
//! below returns its input untouched for an empty slice, and the toolsnap
//! goldens and the surface record pin it.
//!
//! # The notice, and why it has this shape
//!
//! A call that uses a deprecated item gets one more top-level field in its
//! structured reply, [`NOTICE_KEY`] (`deprecation_notice`): a list with one
//! entry per item used — the item, what it is in words, the version, what to
//! do, the decision, and the whole sentence (`says`). A field rather than any
//! other channel because it is the one place every client population reads:
//!
//! - A field ADDED to a reply object is the change the surface rule always
//!   allowed; no client of the last release loses anything.
//! - A client that reads `structuredContent` reads it there. A client that
//!   hands its model only the text block (Grok, Alex's client) reads it too,
//!   because `content_policy` builds that block from the structured reply
//!   after the call, notice included.
//! - `_meta` was the alternative and is the wrong one: it is protocol
//!   metadata a model is not shown, so the person the rule exists for would
//!   never hear it.
//!
//! A reply with no structured half (a Markdown document, a refusal) gets one
//! more text block instead, after the ones it had, so a client showing the
//! first block shows what it always did.
//!
//! What counts as USING an item: calling the tool; passing the argument (a
//! value at its path that is not null); getting the reply field; naming the
//! node or edge type in the arguments or reading it back in the reply (a
//! string equal to it under a `node_type`, `edge_type`, `relation` or `*_type`
//! key, or in a `*_types` list); writing or reading the property on a node of
//! its type, or passing it to a tool `writers.json` says writes the type.
//! describe_schema is where schema items are MARKED, so asking it about one is
//! not using it. The same function decides it for a served call and for the
//! surface check, so the two cannot disagree.
//!
//! # The changelog and the cut
//!
//! Every entry is announced under `### Deprecated` with a "What to do" line —
//! in a `changelog.d/` fragment until its release is cut, then in that
//! release's section of CHANGELOG.md ([`changelog_problems`], run on the real
//! files by `tests/every_deprecation_is_announced_in_the_changelog.rs`). The
//! release cut lists every deprecation and every removal in the release notes,
//! and telling the people who rely on the MCP server is owed before the
//! release ships (AGENTS.md, `flow:release-cut`). Who tells Alex, and how, is
//! Anthony's open decision (D3); nothing here assumes an answer.

use std::borrow::Cow;

use rmcp::model::{CallToolResult, ContentBlock, Tool};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Text that is `&'static str` in the registry and owned when read back from a
/// record or a reply.
pub type Text = Cow<'static, str>;

/// The structured reply's field that carries the notice.
pub const NOTICE_KEY: &str = "deprecation_notice";

/// The tool where schema items are marked: asking it about one is not using it.
const DESCRIBE_SCHEMA: &str = "describe_schema";

/// What a client can call or read, and so what can be retired.
///
/// A `path` is the surface test's own: dot-separated field names, `[]` after
/// a name whose value is a list — `accepts[].disposition`, `items[].node_id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Item {
    /// A served tool.
    Tool { tool: Text },
    /// An argument of a tool, at a path in its input schema.
    Argument { tool: Text, path: Text },
    /// A field of a tool's structured reply, at a path.
    ReplyField { tool: Text, path: Text },
    /// A node type of the schema.
    NodeType { node_type: Text },
    /// An edge type of the schema.
    EdgeType { edge_type: Text },
    /// A property of a node type.
    NodeProperty { node_type: Text, property: Text },
}

impl Item {
    /// The item in words: "the tool `get_node`".
    pub fn describe(&self) -> String {
        match self {
            Item::Tool { tool } => format!("the tool `{tool}`"),
            Item::Argument { tool, path } => format!("the argument `{path}` of `{tool}`"),
            Item::ReplyField { tool, path } => format!("the reply field `{path}` of `{tool}`"),
            Item::NodeType { node_type } => format!("the node type `{node_type}`"),
            Item::EdgeType { edge_type } => format!("the edge type `{edge_type}`"),
            Item::NodeProperty {
                node_type,
                property,
            } => format!("the property `{property}` of `{node_type}`"),
        }
    }

    /// The names a changelog entry carries, each in backticks, to name it.
    pub fn names(&self) -> Vec<&str> {
        match self {
            Item::Tool { tool } => vec![tool],
            Item::Argument { tool, path } | Item::ReplyField { tool, path } => vec![tool, path],
            Item::NodeType { node_type } => vec![node_type],
            Item::EdgeType { edge_type } => vec![edge_type],
            Item::NodeProperty {
                node_type,
                property,
            } => vec![node_type, property],
        }
    }
}

/// One deprecation: what is going, since when, what to do instead, and who
/// decided.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Deprecation {
    /// What is going.
    pub item: Item,
    /// The release that deprecates it (`x.y.z`). The next release removes it.
    pub deprecated_in: Text,
    /// What to do instead, in one sentence.
    pub what_to_do: Text,
    /// The design id of the Decision that retires it (`dec:…`).
    pub decision: Text,
}

impl Deprecation {
    const fn new(
        item: Item,
        deprecated_in: &'static str,
        what_to_do: &'static str,
        decision: &'static str,
    ) -> Self {
        Self {
            item,
            deprecated_in: Cow::Borrowed(deprecated_in),
            what_to_do: Cow::Borrowed(what_to_do),
            decision: Cow::Borrowed(decision),
        }
    }

    /// A served tool.
    pub const fn tool(
        tool: &'static str,
        deprecated_in: &'static str,
        what_to_do: &'static str,
        decision: &'static str,
    ) -> Self {
        Self::new(
            Item::Tool {
                tool: Cow::Borrowed(tool),
            },
            deprecated_in,
            what_to_do,
            decision,
        )
    }

    /// An argument of a tool, at `path` in its input schema.
    pub const fn argument(
        tool: &'static str,
        path: &'static str,
        deprecated_in: &'static str,
        what_to_do: &'static str,
        decision: &'static str,
    ) -> Self {
        Self::new(
            Item::Argument {
                tool: Cow::Borrowed(tool),
                path: Cow::Borrowed(path),
            },
            deprecated_in,
            what_to_do,
            decision,
        )
    }

    /// A field of a tool's reply, at `path`.
    pub const fn reply_field(
        tool: &'static str,
        path: &'static str,
        deprecated_in: &'static str,
        what_to_do: &'static str,
        decision: &'static str,
    ) -> Self {
        Self::new(
            Item::ReplyField {
                tool: Cow::Borrowed(tool),
                path: Cow::Borrowed(path),
            },
            deprecated_in,
            what_to_do,
            decision,
        )
    }

    /// A node type.
    pub const fn node_type(
        node_type: &'static str,
        deprecated_in: &'static str,
        what_to_do: &'static str,
        decision: &'static str,
    ) -> Self {
        Self::new(
            Item::NodeType {
                node_type: Cow::Borrowed(node_type),
            },
            deprecated_in,
            what_to_do,
            decision,
        )
    }

    /// An edge type.
    pub const fn edge_type(
        edge_type: &'static str,
        deprecated_in: &'static str,
        what_to_do: &'static str,
        decision: &'static str,
    ) -> Self {
        Self::new(
            Item::EdgeType {
                edge_type: Cow::Borrowed(edge_type),
            },
            deprecated_in,
            what_to_do,
            decision,
        )
    }

    /// A property of a node type.
    pub const fn node_property(
        node_type: &'static str,
        property: &'static str,
        deprecated_in: &'static str,
        what_to_do: &'static str,
        decision: &'static str,
    ) -> Self {
        Self::new(
            Item::NodeProperty {
                node_type: Cow::Borrowed(node_type),
                property: Cow::Borrowed(property),
            },
            deprecated_in,
            what_to_do,
            decision,
        )
    }

    /// THE MARK: "DEPRECATED in 0.85.0, removed in the next release: <what to do>".
    pub fn mark(&self) -> String {
        format!(
            "DEPRECATED in {}, removed in the next release: {}",
            self.deprecated_in, self.what_to_do
        )
    }

    /// The whole sentence: what it is, then the mark.
    pub fn says(&self) -> String {
        format!("{} is {}", self.item.describe(), self.mark())
    }

    /// The entry as a client reads it — in a notice and in describe_schema's
    /// `deprecations`: the stored fields, plus `what` and `says` in words.
    pub fn to_json(&self) -> Value {
        let mut v = serde_json::to_value(self).unwrap_or(Value::Null);
        if let Value::Object(m) = &mut v {
            m.insert("what".into(), json!(self.item.describe()));
            m.insert("says".into(), json!(self.says()));
        }
        v
    }
}

/// ⭐ THE REGISTRY: everything this build is retiring.
///
/// To deprecate something, add its entry here AND a `changelog.d/` fragment
/// that names it under `### Deprecated` with a "What to do" line
/// (`tests/every_deprecation_is_announced_in_the_changelog.rs` fails without
/// it); `deprecated_in` is the release that will ship it. In the release after
/// that, remove the item. For a TOOL, keep its entry for that one release, so
/// its old name is refused with what to do, and take the entry out in the
/// release after; for anything else, take the entry out with the item.
pub const REGISTRY: &[Deprecation] = &[
    // THE FIRST REAL ENTRY (2026-10-14). On Anthony's word, 2026-10-13
    // (dec:anchor-retire-or-implement): "Remove the unused Anchor type, but
    // only after a rule for retiring things exists, because it would be the
    // first thing ever taken off what Alex's MCP sees."
    //
    // Nothing in reflow2 creates or reads an Anchor (no tool, no
    // `node::ANCHOR`; `preserve.rs` only classifies the type by name), and
    // the seven designs these MCP servers serve held ZERO on 2026-10-14. ONE
    // ENTRY FOR THE TYPE, none for its properties: `design_intent` and
    // `key_decisions` are declared on no other type and leave with it, its
    // `name`, `checksum` and `created_at` stay declared on the types that
    // share them, no edge type names Anchor as an endpoint (it is reached
    // only through `*`), and the surface check already lets a deprecated
    // type take its properties with it. Separate property entries would only
    // repeat the notice on every Anchor write.
    Deprecation::node_type(
        "Anchor",
        "0.85.0",
        "Nothing reads an Anchor; record a checksummed baseline with `add_epoch` \
         (epoch_type `baseline`, with its `checksum`) and capture a node's state at it with \
         `snapshot_before_change` (a Snapshot) instead.",
        "dec:anchor-retire-or-implement",
    ),
];

// ---- the mark ------------------------------------------------------------------------

/// The served tool list with the mark on every deprecated tool, argument and
/// reply field. An empty registry returns the list untouched.
pub fn mark_tools(mut tools: Vec<Tool>, entries: &[Deprecation]) -> Vec<Tool> {
    if entries.is_empty() {
        return tools;
    }
    for t in tools.iter_mut() {
        let mut lines = Vec::new();
        for d in entries {
            match &d.item {
                Item::Tool { tool } if tool == t.name.as_ref() => lines.push(d.mark()),
                Item::ReplyField { tool, path } if tool == t.name.as_ref() => {
                    lines.push(format!("The reply's `{path}` is {}", d.mark()))
                }
                Item::Argument { tool, path } if tool == t.name.as_ref() => {
                    let mut schema = Value::Object((*t.input_schema).clone());
                    if let Some(ptr) = property_pointer(&schema, path)
                        && let Some(Value::Object(prop)) = schema.pointer_mut(&ptr)
                    {
                        let text = match prop.get("description").and_then(Value::as_str) {
                            Some(old) => format!("{}\n\n{old}", d.mark()),
                            None => d.mark(),
                        };
                        prop.insert("description".into(), json!(text));
                        if let Value::Object(m) = schema {
                            t.input_schema = std::sync::Arc::new(m);
                        }
                    }
                }
                _ => {}
            }
        }
        if !lines.is_empty() {
            let own = t.description.as_deref().unwrap_or("");
            t.description = Some(format!("{}\n\n{own}", lines.join("\n")).into());
        }
    }
    tools
}

/// JSON-pointer escaping of one key.
fn escape(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

/// Follow `$ref`s and the first `anyOf`/`oneOf`/`allOf` branch that leads to
/// an object `having` the key, from the schema node at `ptr`.
fn resolve_to(root: &Value, ptr: &str, having: &str) -> Option<String> {
    let mut ptr = ptr.to_string();
    for _ in 0..16 {
        let node = root.pointer(&ptr)?;
        if node.get(having).is_some() {
            return Some(ptr);
        }
        if let Some(name) = node
            .get("$ref")
            .and_then(Value::as_str)
            .and_then(|r| r.strip_prefix("#/$defs/"))
        {
            ptr = format!("/$defs/{}", escape(name));
            continue;
        }
        let branch = ["anyOf", "oneOf", "allOf"].iter().find_map(|k| {
            node.get(*k)?
                .as_array()?
                .iter()
                .enumerate()
                .find_map(|(i, _)| {
                    let p = format!("{ptr}/{k}/{i}");
                    resolve_to(root, &p, having)
                })
        });
        return branch;
    }
    None
}

/// The JSON pointer, in an input schema, of the property an argument `path`
/// names (`accepts[].disposition`), or `None` when the schema has none there.
pub fn property_pointer(root: &Value, path: &str) -> Option<String> {
    let segments: Vec<&str> = path.split('.').collect();
    let mut holder = String::new();
    for (i, segment) in segments.iter().enumerate() {
        let name = segment.trim_end_matches("[]");
        let lists = (segment.len() - name.len()) / 2;
        if name.is_empty() {
            return None;
        }
        let at = resolve_to(root, &holder, "properties")?;
        let prop = format!("{at}/properties/{}", escape(name));
        root.pointer(&prop)?;
        if i + 1 == segments.len() {
            return Some(prop);
        }
        let mut next = prop;
        for _ in 0..lists {
            next = format!("{}/items", resolve_to(root, &next, "items")?);
        }
        holder = next;
    }
    None
}

/// describe_schema's answer with the mark, as `deprecated`, on every
/// deprecated node type, property and edge type it shows — and, where it
/// shows a pair of types, `from_type_deprecated` / `to_type_deprecated`. An
/// empty registry leaves the answer untouched.
pub fn mark_vocabulary(reply: &mut Value, entries: &[Deprecation]) {
    if entries.is_empty() {
        return;
    }
    let node_type = |name: &str| {
        entries
            .iter()
            .find(|d| matches!(&d.item, Item::NodeType { node_type } if node_type == name))
    };
    match reply {
        Value::Object(obj) => {
            if let Some(t) = obj
                .get("node_type")
                .and_then(Value::as_str)
                .map(String::from)
            {
                if let Some(d) = node_type(&t) {
                    obj.insert("deprecated".into(), json!(d.mark()));
                }
                if let Some(Value::Array(props)) = obj.get_mut("properties") {
                    for p in props.iter_mut() {
                        let Some(name) = p.get("name").and_then(Value::as_str) else {
                            continue;
                        };
                        let found = entries.iter().find(|d| {
                            matches!(&d.item, Item::NodeProperty { node_type, property }
                                if node_type == &t && property == name)
                        });
                        if let (Some(d), Value::Object(p)) = (found, p) {
                            p.insert("deprecated".into(), json!(d.mark()));
                        }
                    }
                }
            }
            if let Some(e) = obj.get("edge_type").and_then(Value::as_str)
                && let Some(d) = entries
                    .iter()
                    .find(|d| matches!(&d.item, Item::EdgeType { edge_type } if edge_type == e))
            {
                obj.insert("deprecated".into(), json!(d.mark()));
            }
            for key in ["from_type", "to_type"] {
                if let Some(t) = obj.get(key).and_then(Value::as_str)
                    && let Some(d) = node_type(t)
                {
                    obj.insert(format!("{key}_deprecated"), json!(d.mark()));
                }
            }
            for (_, v) in obj.iter_mut() {
                if v.is_object() || v.is_array() {
                    mark_vocabulary(v, entries);
                }
            }
        }
        Value::Array(items) => {
            for v in items.iter_mut() {
                mark_vocabulary(v, entries);
            }
        }
        _ => {}
    }
}

/// Every deprecation as a client reads it, for describe_schema's full answer —
/// `None` when there are none, so an empty registry adds no key.
pub fn listing(entries: &[Deprecation]) -> Option<Value> {
    (!entries.is_empty()).then(|| Value::Array(entries.iter().map(Deprecation::to_json).collect()))
}

// ---- the notice ----------------------------------------------------------------------

/// Is there a value that is not null at `path` (`a.b`, `items[].k`)?
pub fn present(v: &Value, path: &str) -> bool {
    let segments: Vec<(&str, usize)> = path
        .split('.')
        .map(|s| {
            let name = s.trim_end_matches("[]");
            (name, (s.len() - name.len()) / 2)
        })
        .collect();
    fn walk(v: &Value, segments: &[(&str, usize)]) -> bool {
        let Some(((name, lists), rest)) = segments.split_first() else {
            return true;
        };
        match v.get(*name).filter(|x| !x.is_null()) {
            Some(x) => expand(x, *lists, rest),
            None => false,
        }
    }
    fn expand(x: &Value, lists: usize, rest: &[(&str, usize)]) -> bool {
        if lists == 0 {
            return walk(x, rest);
        }
        x.as_array()
            .is_some_and(|a| a.iter().any(|e| expand(e, lists - 1, rest)))
    }
    walk(v, &segments)
}

/// Does `v` name the vocabulary item `name` — a string equal to it under a
/// `node_type`, `edge_type`, `relation` or `*_type` key, or in a `*_types`
/// list? Node types are CamelCase and edge types UPPER_SNAKE, so neither is
/// mistaken for the snake_case values other `*_type` keys hold.
fn names_vocabulary(v: &Value, name: &str) -> bool {
    match v {
        Value::Object(m) => m.iter().any(|(k, x)| {
            let one = (k == "relation" || k.ends_with("_type")) && x.as_str() == Some(name);
            let many = k.ends_with("_types")
                && x.as_array()
                    .is_some_and(|a| a.iter().any(|e| e.as_str() == Some(name)));
            one || many || names_vocabulary(x, name)
        }),
        Value::Array(a) => a.iter().any(|x| names_vocabulary(x, name)),
        _ => false,
    }
}

/// Does `v` name the tool `name` under a `tool` key (a bulk call's items)?
fn names_tool(v: &Value, name: &str) -> bool {
    match v {
        Value::Object(m) => m
            .iter()
            .any(|(k, x)| (k == "tool" && x.as_str() == Some(name)) || names_tool(x, name)),
        Value::Array(a) => a.iter().any(|x| names_tool(x, name)),
        _ => false,
    }
}

/// Does `v` hold a node of type `node_type` carrying `property` — under its
/// `properties` or `props`, or as a field of its own?
fn carries_property(v: &Value, node_type: &str, property: &str) -> bool {
    match v {
        Value::Object(m) => {
            let here = m.get("node_type").and_then(Value::as_str) == Some(node_type)
                && (["properties", "props"].iter().any(|k| {
                    m.get(*k)
                        .and_then(|p| p.get(property))
                        .is_some_and(|x| !x.is_null())
                }) || m.get(property).is_some_and(|x| !x.is_null()));
            here || m.values().any(|x| carries_property(x, node_type, property))
        }
        Value::Array(a) => a.iter().any(|x| carries_property(x, node_type, property)),
        _ => false,
    }
}

/// The deprecated items a call to `tool` with `arguments`, answered with
/// `reply`, uses — in registry order. See the module documentation for what
/// counts. Empty for an empty registry.
pub fn used_by<'a>(
    entries: &'a [Deprecation],
    tool: &str,
    arguments: Option<&Value>,
    reply: Option<&Value>,
) -> Vec<&'a Deprecation> {
    let reads_schema = tool == DESCRIBE_SCHEMA;
    let either = |f: &dyn Fn(&Value) -> bool| arguments.is_some_and(f) || reply.is_some_and(f);
    entries
        .iter()
        .filter(|d| match &d.item {
            Item::Tool { tool: t } => {
                t == tool || (!reads_schema && arguments.is_some_and(|a| names_tool(a, t)))
            }
            Item::Argument { tool: t, path } => {
                t == tool && arguments.is_some_and(|a| present(a, path))
            }
            Item::ReplyField { tool: t, path } => {
                t == tool && reply.is_some_and(|r| present(r, path))
            }
            Item::NodeType { node_type } => {
                !reads_schema && either(&|v| names_vocabulary(v, node_type))
            }
            Item::EdgeType { edge_type } => {
                !reads_schema && either(&|v| names_vocabulary(v, edge_type))
            }
            Item::NodeProperty {
                node_type,
                property,
            } => {
                !reads_schema
                    && ((crate::writers::node_type_writers(node_type)
                        .iter()
                        .any(|w| w == tool)
                        && arguments
                            .and_then(|a| a.get(property.as_ref()))
                            .is_some_and(|x| !x.is_null()))
                        || either(&|v| carries_property(v, node_type, property)))
            }
        })
        .collect()
}

/// Put the notice on a reply: the [`NOTICE_KEY`] field of a structured reply,
/// or one more text block on a reply with no structured half. Nothing when
/// nothing deprecated was used.
pub fn attach_notice(result: &mut CallToolResult, used: &[&Deprecation]) {
    if used.is_empty() {
        return;
    }
    match result.structured_content.as_mut() {
        Some(Value::Object(m)) => {
            m.insert(
                NOTICE_KEY.into(),
                Value::Array(used.iter().map(|d| d.to_json()).collect()),
            );
        }
        _ => result.content.push(ContentBlock::text(format!(
            "DEPRECATION NOTICE: this call used what is going. {}",
            used.iter().map(|d| d.says()).collect::<Vec<_>>().join(" ")
        ))),
    }
}

/// The stub a tool leaves for the one release after its notice: when `name`
/// is a deprecated tool this build no longer serves, the sentence its refusal
/// carries. The caller asks only for a name nothing serves.
pub fn removed_tool(entries: &[Deprecation], name: &str) -> Option<String> {
    entries
        .iter()
        .find(|d| matches!(&d.item, Item::Tool { tool } if tool == name))
        .map(|d| {
            format!(
                "`{name}` was DEPRECATED in {} and has been removed after that release of \
                 notice. What to do: {} ({})",
                d.deprecated_in, d.what_to_do, d.decision
            )
        })
}

// ---- the registry's own checks -------------------------------------------------------

/// `x.y.z` as numbers.
pub fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let parts: Vec<u64> = v
        .split('.')
        .map(|p| p.parse::<u64>().ok())
        .collect::<Option<_>>()?;
    match parts[..] {
        [a, b, c] => Some((a, b, c)),
        _ => None,
    }
}

/// Every problem with the registry against this build: a version that is not
/// one, no what-to-do, a decision that is not a `dec:` id, an item named
/// twice, and an item this build does not serve — except a tool, which may be
/// gone (its stub release). `vocabulary` is describe_schema's full answer.
pub fn validate(entries: &[Deprecation], served: &[Tool], vocabulary: &Value) -> Vec<String> {
    let mut problems = Vec::new();
    let tool = |name: &str| served.iter().find(|t| t.name == name);
    let node_type = |name: &str| {
        vocabulary["node_types"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|n| n["node_type"] == name)
    };
    for (i, d) in entries.iter().enumerate() {
        let at = format!("entry {i} ({})", d.item.describe());
        if parse_version(&d.deprecated_in).is_none() {
            problems.push(format!(
                "{at}: deprecated_in `{}` is not a version (x.y.z)",
                d.deprecated_in
            ));
        }
        if d.what_to_do.trim().is_empty() || d.what_to_do.contains('\n') {
            problems.push(format!("{at}: what_to_do must be one sentence"));
        }
        if d.decision.strip_prefix("dec:").is_none_or(str::is_empty) {
            problems.push(format!(
                "{at}: decision `{}` is not the Decision that retires it (`dec:…`)",
                d.decision
            ));
        }
        if entries[..i].iter().any(|e| e.item == d.item) {
            problems.push(format!("{at}: named twice"));
        }
        match &d.item {
            Item::Tool { .. } => {}
            Item::Argument { tool: t, path } => match tool(t) {
                None => problems.push(format!("{at}: `{t}` is not served")),
                Some(t) => {
                    let schema = Value::Object((*t.input_schema).clone());
                    if property_pointer(&schema, path).is_none() {
                        problems.push(format!("{at}: `{}` publishes no `{path}`", t.name));
                    }
                }
            },
            Item::ReplyField { tool: t, path } => {
                if tool(t).is_none() {
                    problems.push(format!("{at}: `{t}` is not served"));
                }
                if path.split('.').any(|s| s.trim_end_matches("[]").is_empty()) {
                    problems.push(format!("{at}: `{path}` is not a field path"));
                }
            }
            Item::NodeType { node_type: t } => {
                if node_type(t).is_none() {
                    problems.push(format!("{at}: the schema declares no `{t}`"));
                }
            }
            Item::EdgeType { edge_type } => {
                let declared = vocabulary["edge_types"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|e| e["edge_type"] == edge_type.as_ref());
                if !declared {
                    problems.push(format!("{at}: the schema declares no `{edge_type}`"));
                }
            }
            Item::NodeProperty {
                node_type: t,
                property,
            } => {
                let declared = node_type(t).is_some_and(|n| {
                    n["properties"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .any(|p| p["name"] == property.as_ref())
                });
                if !declared {
                    problems.push(format!("{at}: `{t}` declares no `{property}`"));
                }
            }
        }
    }
    problems
}

/// The entries under `### Deprecated` in Markdown, one per top-level bullet.
fn deprecated_entries(markdown: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    let mut current: Option<String> = None;
    for line in markdown.lines() {
        if line.starts_with('#') {
            out.extend(current.take());
            inside =
                line.trim_start_matches('#').trim() == "Deprecated" && line.starts_with("### ");
            continue;
        }
        if !inside {
            continue;
        }
        if line.starts_with("- ") || line.starts_with("* ") {
            out.extend(current.take());
            current = Some(line.to_string());
        } else if let Some(c) = current.as_mut() {
            c.push('\n');
            c.push_str(line);
        }
    }
    out.extend(current);
    out
}

/// CHANGELOG.md's section for one release: from its `## [<version>]` heading
/// to the next `## ` heading.
fn release_section<'a>(changelog: &'a str, version: &str) -> Option<&'a str> {
    let heading = format!("## [{version}]");
    let start = changelog
        .match_indices(&heading)
        .find(|(i, _)| *i == 0 || changelog[..*i].ends_with('\n'))?
        .0;
    let body = &changelog[start + heading.len()..];
    let end = body
        .match_indices("\n## ")
        .next()
        .map(|(i, _)| i)
        .unwrap_or(body.len());
    Some(&body[..end])
}

/// Does a changelog entry announce `d`: every name in backticks, and a "What
/// to do" line?
fn announces(entry: &str, d: &Deprecation) -> bool {
    d.item
        .names()
        .iter()
        .all(|n| entry.contains(&format!("`{n}`")))
        && entry.to_lowercase().contains("what to do")
}

/// Every registry entry the changelog does not announce. `fragments` are the
/// `changelog.d/` files (name, text), `changelog` is CHANGELOG.md, and
/// `current_version` this build's. An entry whose release is still to be cut
/// (`deprecated_in` at or after the current version) is announced by a
/// fragment, or by its release's section once the cut has assembled it; one
/// whose release is already cut is announced only by that release's section.
pub fn changelog_problems(
    entries: &[Deprecation],
    fragments: &[(String, String)],
    changelog: &str,
    current_version: &str,
) -> Vec<String> {
    let current = parse_version(current_version);
    let fragment_entries: Vec<String> = fragments
        .iter()
        .flat_map(|(_, text)| deprecated_entries(text))
        .collect();
    let mut problems = Vec::new();
    for d in entries {
        let what = d.item.describe();
        let names = d
            .item
            .names()
            .iter()
            .map(|n| format!("`{n}`"))
            .collect::<Vec<_>>()
            .join(" and ");
        let Some(version) = parse_version(&d.deprecated_in) else {
            problems.push(format!(
                "{what}: deprecated_in `{}` is not a version (x.y.z)",
                d.deprecated_in
            ));
            continue;
        };
        let in_release = release_section(changelog, &d.deprecated_in)
            .is_some_and(|s| deprecated_entries(s).iter().any(|e| announces(e, d)));
        if in_release {
            continue;
        }
        if current.is_some_and(|c| version < c) {
            problems.push(format!(
                "{what} is deprecated in {}, a release already cut, and CHANGELOG.md's [{}] \
                 section names it ({names}) in no `### Deprecated` entry with a What to do line",
                d.deprecated_in, d.deprecated_in
            ));
            continue;
        }
        if !fragment_entries.iter().any(|e| announces(e, d)) {
            problems.push(format!(
                "{what} (deprecated in {}) is announced by no changelog.d fragment: a `### \
                 Deprecated` entry must name {names} and say what to do",
                d.deprecated_in
            ));
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_is_present_only_where_a_value_is() {
        let v = json!({"a": {"b": 1}, "items": [{"k": null}, {"k": 2}], "n": null});
        assert!(present(&v, "a.b"));
        assert!(present(&v, "items[].k"));
        assert!(!present(&v, "n"));
        assert!(!present(&v, "a.c"));
        assert!(!present(&v, "items[].z"));
    }

    #[test]
    fn an_argument_path_finds_its_property_through_refs_and_lists() {
        let schema = json!({
            "type": "object",
            "properties": {
                "accepts": {"type": "array", "items": {"$ref": "#/$defs/A"}},
                "opt": {"anyOf": [{"$ref": "#/$defs/A"}, {"type": "null"}]}
            },
            "$defs": {"A": {"type": "object", "properties": {"disposition": {"type": "string"}}}}
        });
        assert_eq!(
            property_pointer(&schema, "accepts[].disposition").as_deref(),
            Some("/$defs/A/properties/disposition")
        );
        assert!(property_pointer(&schema, "opt.disposition").is_some());
        assert!(property_pointer(&schema, "accepts").is_some());
        assert!(property_pointer(&schema, "accepts[].nothing").is_none());
        assert!(property_pointer(&schema, "nothing").is_none());
    }

    #[test]
    fn an_empty_registry_changes_nothing() {
        let mut v = json!({"node_type": "Anchor", "edge_type": "X"});
        mark_vocabulary(&mut v, &[]);
        assert_eq!(v, json!({"node_type": "Anchor", "edge_type": "X"}));
        assert!(listing(&[]).is_none());
        assert!(used_by(&[], "get_node", Some(&v), Some(&v)).is_empty());
        let mut r = CallToolResult::structured(json!({"a": 1}));
        let before = r.clone();
        attach_notice(&mut r, &[]);
        assert_eq!(r, before);
    }
}
