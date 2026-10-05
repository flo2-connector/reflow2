//! Every argument refusal names the tool and the field path.
//!
//! A tool's arguments are checked against that tool's PUBLISHED input schema
//! before anything deserialises them, at the one choke point every served call
//! passes through (`ReflowService::call_tool`). A call that does not fit is
//! refused there, before any handler runs, with every place it does not fit:
//! the tool, the path into the arguments (`related_to[0].evidence`), what the
//! schema expects at that path (its type, its allowed values), and the field's
//! own published description.
//!
//! # The class this closes, measured on 0.77.0 (2026-10-02)
//!
//! - A WRONG-TYPED argument came back as serde's bare `failed to deserialize
//!   parameters: invalid type: string "core", expected a sequence` on every
//!   tool, naming neither the tool nor the field
//!   (`fact:root-cause-a-wrong-type-argument-is-refused-in-the-bare-serde-string-because-the-interception-matches-two-phrasings-2026-10-02`).
//!   The interception it replaces matched two of serde's English phrasings,
//!   `missing field` and `unknown field`, one at a time; serde's type error
//!   carries no path at all, so no further phrasing could have named the field.
//! - A MISSING NESTED field was looked up among the schema's top-level
//!   properties only, so 22 of the 35 required nested fields on the surface were
//!   refused with "its own schema publishes no description of it" when the
//!   schema did describe them, one `$ref` away in `$defs`
//!   (`fact:root-cause-a-missing-nested-field-refusal-says-the-schema-publishes-no-description-when-it-does-2026-10-02`).
//!
//! Both came from the same shape: a refusal rebuilt from a deserialiser's
//! words, which know one field at one depth. The schema knows every field at
//! every depth, so the refusal is now built from the schema
//! (`dec:idea-every-argument-refusal-names-the-tool-and-the-field-path`).
//!
//! # Why a small validator here and not a JSON Schema crate
//!
//! reflow2 depended on no JSON Schema validator. The general crates validate
//! all of draft 2020-12 (regex patterns, formats, remote references) and pull a
//! tree of dependencies that the dependency-currency rule then owes a monthly
//! look at. Their error text is their own, and the refusal this module writes
//! needs things only reflow2 has: the field's description, the nearest legal
//! name, and the argument ALIASES the published schema deliberately leaves out
//! (below). The schemas reflow2 serves use a small subset — fifteen keywords on
//! 195 tools — and this file checks exactly that subset.
//!
//! ⚠️ THE SUBSET IS ENFORCED, NOT ASSUMED. [`CHECKED`], [`ANNOTATIONS`] and
//! [`FORMATS`] are the whole vocabulary this validator understands, and
//! `tests/every_argument_refusal_names_the_tool_and_the_field_path.rs` walks
//! every served schema and FAILS on any keyword, format or `$ref` outside it.
//! A schema construct this file does not check therefore cannot ship silently
//! unchecked; it arrives as a red build naming the keyword.
//!
//! # Aliases: a courtesy on the way in, never a second name on the way out
//!
//! Many request fields accept other spellings (`id` and `node_id` for
//! `decision_id`, `properties` for `props`) through `#[serde(alias)]`
//! (`dec:idea-one-way-to-name-which-node-across-the-tool-surface`), and the
//! published schema carries only the typed spelling on purpose. A check against
//! the published schema alone would refuse every alias. So each aliased field
//! also declares its aliases as `x-reflow2-aliases` on its schema, which the
//! check reads and the published listing strips ([`strip_internal`]). The
//! integration test probes serde's own field list at every object of every tool
//! and fails when the two disagree, so the copy cannot drift from what serde
//! accepts.

use serde_json::{Map, Value};

/// The schema extension naming a field's accepted aliases. Read by the check,
/// stripped from the published listing.
pub const ALIASES: &str = "x-reflow2-aliases";

/// The schema extension marking a field that is NOT A FIELD: a name accepted
/// only so a mistake can be answered with a redirect, whose value is that
/// redirect. Read by the check, which refuses the name with it beside every
/// other problem in the call; stripped from the published listing WITH the
/// property, so no catalogue offers it
/// (fact:the-changeevent-description-decoy-is-listed-by-find-tools-and-passes-the-argument-check-2026-10-05).
pub const NOT_A_FIELD: &str = "x-reflow2-not-a-field";

/// Keywords this validator CHECKS.
pub const CHECKED: &[&str] = &[
    "type",
    "properties",
    "required",
    "additionalProperties",
    "items",
    "enum",
    "$ref",
    "minimum",
    "minItems",
    "format",
    ALIASES,
    NOT_A_FIELD,
];

/// Keywords that carry no constraint and are read only for the refusal's words
/// (or not at all).
pub const ANNOTATIONS: &[&str] = &["description", "default", "$schema", "$defs", "title"];

/// The `format` values this validator understands. An integer format is a
/// range the deserialiser enforces, so it is checked; `double` and `float`
/// accept any JSON number.
pub const FORMATS: &[&str] = &[
    "uint", "uint8", "uint16", "uint32", "uint64", "int", "int8", "int16", "int32", "int64",
    "double", "float",
];

/// Which way the caller reached this server. It decides one piece of advice:
/// over an MCP session a client keeps the tool list it fetched when it
/// connected, so a name the server knows can be one the client never saw; the
/// `--call` door reads the schema from this same binary on every call, so that
/// can never be the cause and saying "reconnect" there means nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    /// An MCP session (stdio, HTTP, or a shared daemon's client).
    Session,
    /// The one-shot `reflow2-mcp --call` door.
    CallDoor,
}

tokio::task_local! {
    /// The transport of the tool call now being served, set by `call_tool`
    /// around the handler — so a handler that checks arguments of its own (the
    /// bulk `draw_edges` runs each item's helper check) gives the same advice
    /// the call itself would.
    pub(crate) static TRANSPORT: Transport;
}

/// The transport of the call now being served; a session when no call scope is
/// set (a handler driven directly).
pub(crate) fn current_transport() -> Transport {
    TRANSPORT.try_with(|t| *t).unwrap_or(Transport::Session)
}

/// Which door a call came through: the `--call` door names itself at
/// handshake (`crate::service::CALL_DOOR_CLIENT`), and every other client is
/// an MCP session.
pub fn transport_of(context: &rmcp::service::RequestContext<rmcp::RoleServer>) -> Transport {
    match context.peer.peer_info() {
        Some(info) if info.client_info.name == crate::service::CALL_DOOR_CLIENT => {
            Transport::CallDoor
        }
        _ => Transport::Session,
    }
}

/// The whole check for one call on a surface with no write receipts (the
/// latent and degraded surfaces): the refusal to answer with, or `None` when
/// the arguments fit. The full surface checks against its PUBLISHED schema,
/// `echo` included, in `ReflowService::call_tool`.
pub fn refuse_unfit(
    tool: &rmcp::model::Tool,
    request: &rmcp::model::CallToolRequestParams,
    transport: Transport,
) -> Option<rmcp::model::CallToolResponse> {
    let empty = Map::new();
    let violations = check(
        &tool.input_schema,
        request.arguments.as_ref().unwrap_or(&empty),
    );
    (!violations.is_empty()).then(|| {
        rmcp::model::CallToolResponse::Complete(rmcp::model::CallToolResult::error(vec![
            rmcp::model::ContentBlock::text(refusal(&tool.name, &violations, transport)),
        ]))
    })
}

/// What is wrong at one place in the arguments.
#[derive(Debug, Clone, PartialEq)]
pub enum Problem {
    /// The value's JSON type is not one the schema allows here.
    WrongType { got: Value },
    /// A required field is absent. `passed` names the required fields of the
    /// same object that the call DID give, so the refusal can set them apart
    /// rather than leave the caller to diff the obligation against the call.
    Missing { passed: Vec<String> },
    /// A key this object does not publish, under any accepted spelling.
    Unknown {
        legal: Vec<String>,
        nearest: Option<String>,
    },
    /// A value outside the published set.
    NotAllowed { got: Value },
    /// A number outside the published minimum or the integer format's range.
    OutOfRange { got: Value },
    /// Fewer items than the published minimum.
    TooFew { got: usize, min: u64 },
    /// One field given under two of its spellings.
    Duplicate { names: Vec<String> },
    /// A name the schema keeps only to redirect a mistake (`NOT_A_FIELD`).
    NotAField { redirect: String },
}

/// One place where the arguments do not fit the published schema.
#[derive(Debug, Clone, PartialEq)]
pub struct Violation {
    /// The path into the arguments, as a caller would write it:
    /// `related_to[0].evidence`.
    pub path: String,
    pub problem: Problem,
    /// What the schema accepts at `path`, in words.
    pub expects: String,
    /// The field's own published description, whitespace-folded and cut to
    /// its opening; `None` when the schema publishes none.
    pub description: Option<String>,
}

/// One step into the arguments.
#[derive(Debug, Clone)]
enum Step {
    Key(String),
    Index(usize),
}

fn render(path: &[Step]) -> String {
    let mut out = String::new();
    for s in path {
        match s {
            Step::Key(k) => {
                let plain =
                    !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
                if plain {
                    if !out.is_empty() {
                        out.push('.');
                    }
                    out.push_str(k);
                } else {
                    out.push_str(&format!("[{}]", Value::String(k.clone())));
                }
            }
            Step::Index(i) => out.push_str(&format!("[{i}]")),
        }
    }
    out
}

/// Check `args` against a tool's input schema and return EVERY place they do
/// not fit, in the order met. Empty means the call fits the schema.
pub fn check(schema: &Map<String, Value>, args: &Map<String, Value>) -> Vec<Violation> {
    let mut out = Vec::new();
    let mut path = Vec::new();
    let checker = Checker { root: schema };
    checker.object(schema, args, &mut path, &mut out);
    out
}

struct Checker<'a> {
    root: &'a Map<String, Value>,
}

/// How deep a `$ref` chain may go before it is treated as a cycle. reflow2's
/// schemas reference `$defs` one level deep; this only stops a malformed one
/// from looping.
const MAX_REF_HOPS: usize = 16;

impl<'a> Checker<'a> {
    /// The schema a `$ref` names, when it is a local `#/$defs/…` reference.
    fn target(&self, reference: &str) -> Option<&'a Map<String, Value>> {
        let name = reference.strip_prefix("#/$defs/")?;
        self.root.get("$defs")?.get(name)?.as_object()
    }

    /// Follow `$ref`s from `s` to the schema that carries the constraints.
    fn resolve<'s>(&self, mut s: &'s Map<String, Value>) -> &'s Map<String, Value>
    where
        'a: 's,
    {
        for _ in 0..MAX_REF_HOPS {
            match s
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(|r| self.target(r))
            {
                Some(t) => s = t,
                None => break,
            }
        }
        s
    }

    /// Check one value against one (sub)schema.
    fn value<'s>(
        &self,
        schema: &'s Value,
        v: &Value,
        path: &mut Vec<Step>,
        out: &mut Vec<Violation>,
    ) where
        'a: 's,
    {
        let Some(own) = schema.as_object() else {
            // `true` accepts anything. `false` appears only as
            // `additionalProperties`, which the object check handles.
            if schema == &Value::Bool(false) {
                out.push(self.violation(path, Problem::NotAllowed { got: v.clone() }, schema));
            }
            return;
        };
        let s = self.resolve(own);

        if let Some(types) = types_of(s)
            && !types.iter().any(|t| has_type(v, t))
        {
            out.push(self.violation(path, Problem::WrongType { got: v.clone() }, schema));
            return;
        }
        if let Some(allowed) = s.get("enum").and_then(Value::as_array)
            && !allowed.contains(v)
        {
            out.push(self.violation(path, Problem::NotAllowed { got: v.clone() }, schema));
            return;
        }
        if let Value::Number(n) = v {
            let in_format = s
                .get("format")
                .and_then(Value::as_str)
                .is_none_or(|f| fits_format(n, f));
            let above_minimum = s
                .get("minimum")
                .and_then(Value::as_f64)
                .is_none_or(|min| n.as_f64().is_some_and(|x| x >= min));
            if !in_format || !above_minimum {
                out.push(self.violation(path, Problem::OutOfRange { got: v.clone() }, schema));
                return;
            }
        }
        match v {
            Value::Array(items) => {
                if let Some(min) = s.get("minItems").and_then(Value::as_u64)
                    && (items.len() as u64) < min
                {
                    out.push(self.violation(
                        path,
                        Problem::TooFew {
                            got: items.len(),
                            min,
                        },
                        schema,
                    ));
                }
                if let Some(item_schema) = s.get("items") {
                    for (i, item) in items.iter().enumerate() {
                        path.push(Step::Index(i));
                        self.value(item_schema, item, path, out);
                        path.pop();
                    }
                }
            }
            Value::Object(map) => self.object(s, map, path, out),
            _ => {}
        }
    }

    /// Check an object's keys: every required field present under one of its
    /// spellings, no key the schema does not publish (when it publishes that it
    /// takes no others), and each value against its own schema.
    fn object<'s>(
        &self,
        schema: &'s Map<String, Value>,
        map: &Map<String, Value>,
        path: &mut Vec<Step>,
        out: &mut Vec<Violation>,
    ) where
        'a: 's,
    {
        let empty = Map::new();
        let props = schema
            .get("properties")
            .and_then(Value::as_object)
            .unwrap_or(&empty);
        let mut spelling: std::collections::HashMap<&str, &str> = Default::default();
        for (name, prop) in props {
            spelling.insert(name.as_str(), name.as_str());
            for alias in aliases_of(prop) {
                spelling.insert(alias, name.as_str());
            }
        }

        let mut given_as: std::collections::BTreeMap<&str, Vec<&str>> = Default::default();
        for (key, v) in map {
            path.push(Step::Key(key.clone()));
            match spelling.get(key.as_str()) {
                Some(field) => {
                    given_as.entry(*field).or_default().push(key.as_str());
                    match props[*field].get(NOT_A_FIELD).and_then(Value::as_str) {
                        Some(redirect) => out.push(Violation {
                            path: render(path),
                            problem: Problem::NotAField {
                                redirect: redirect.to_string(),
                            },
                            expects: "no such parameter".into(),
                            description: None,
                        }),
                        None => self.value(&props[*field], v, path, out),
                    }
                }
                None => match schema.get("additionalProperties") {
                    Some(Value::Bool(false)) => {
                        let legal: Vec<String> = props.keys().cloned().collect();
                        let order = candidates(schema, props, &spelling);
                        let nearest = nearest_name(key, &order).map(|n| {
                            spelling
                                .get(n.as_str())
                                .map_or(n.clone(), |c| c.to_string())
                        });
                        out.push(Violation {
                            path: render(path),
                            problem: Problem::Unknown { legal, nearest },
                            expects: "no such parameter".into(),
                            description: None,
                        });
                    }
                    Some(extra @ Value::Object(_)) => self.value(extra, v, path, out),
                    _ => {}
                },
            }
            path.pop();
        }

        for (field, names) in &given_as {
            if names.len() > 1 {
                path.push(Step::Key((*field).to_string()));
                out.push(self.violation(
                    path,
                    Problem::Duplicate {
                        names: names.iter().map(|n| n.to_string()).collect(),
                    },
                    &props[*field],
                ));
                path.pop();
            }
        }

        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            let required: Vec<&str> = required.iter().filter_map(Value::as_str).collect();
            let passed: Vec<String> = required
                .iter()
                .filter(|f| given_as.contains_key(**f))
                .map(|f| f.to_string())
                .collect();
            for field in required.iter().filter(|f| !given_as.contains_key(**f)) {
                path.push(Step::Key(field.to_string()));
                let undeclared = Value::Bool(true);
                let sub = props.get(*field).unwrap_or(&undeclared);
                out.push(self.violation(
                    path,
                    Problem::Missing {
                        passed: passed.clone(),
                    },
                    sub,
                ));
                path.pop();
            }
        }
    }

    fn violation<'s>(&self, path: &[Step], problem: Problem, schema: &'s Value) -> Violation
    where
        'a: 's,
    {
        Violation {
            path: render(path),
            problem,
            expects: self.expects(schema),
            description: self.description(schema),
        }
    }

    /// The field's own description, or else the description of what its `$ref`
    /// names — the property's words first, because they say what the field is
    /// FOR, where a `$defs` entry says what the shape is.
    fn description<'s>(&self, schema: &'s Value) -> Option<String>
    where
        'a: 's,
    {
        let own = schema.as_object()?;
        let text = own
            .get("description")
            .and_then(Value::as_str)
            .or_else(|| self.resolve(own).get("description").and_then(Value::as_str))?;
        let folded = text.split_whitespace().collect::<Vec<_>>().join(" ");
        (!folded.is_empty()).then(|| opening(&folded))
    }

    /// What the schema accepts, in words a caller can act on.
    fn expects<'s>(&self, schema: &'s Value) -> String
    where
        'a: 's,
    {
        let Some(own) = schema.as_object() else {
            return "any value".into();
        };
        let s = self.resolve(own);
        if let Some(allowed) = s.get("enum").and_then(Value::as_array) {
            let named: Vec<String> = allowed
                .iter()
                .filter(|v| !v.is_null())
                .map(|v| match v {
                    Value::String(t) => format!("`{t}`"),
                    other => other.to_string(),
                })
                .collect();
            let nullable = allowed.iter().any(Value::is_null);
            return format!(
                "one of {}{}",
                named.join(", "),
                if nullable { ", or null" } else { "" }
            );
        }
        let types = types_of(s).unwrap_or_default();
        let nullable = types.iter().any(|t| t == "null");
        let words: Vec<String> = types
            .iter()
            .filter(|t| *t != "null")
            .map(|t| self.type_words(t, s))
            .collect();
        let body = if words.is_empty() {
            "any value".to_string()
        } else {
            words.join(" or ")
        };
        if nullable {
            format!("{body}, or null")
        } else {
            body
        }
    }

    fn type_words<'s>(&self, t: &str, s: &'s Map<String, Value>) -> String
    where
        'a: 's,
    {
        match t {
            "string" => "a string".into(),
            "boolean" => "true or false".into(),
            "number" => "a number".into(),
            "integer" => {
                let floor = s.get("minimum").and_then(Value::as_f64).or_else(|| {
                    s.get("format")
                        .and_then(Value::as_str)
                        .filter(|f| f.starts_with("uint"))
                        .map(|_| 0.0)
                });
                match floor {
                    Some(min) => format!("a whole number of at least {min}"),
                    None => "a whole number".into(),
                }
            }
            "array" => {
                let item = s
                    .get("items")
                    .map(|i| self.expects(i))
                    .unwrap_or_else(|| "any value".into());
                format!("a list, each item {item}")
            }
            "object" => {
                let required: Vec<String> = s
                    .get("required")
                    .and_then(Value::as_array)
                    .map(|r| {
                        r.iter()
                            .filter_map(Value::as_str)
                            .map(|f| format!("`{f}`"))
                            .collect()
                    })
                    .unwrap_or_default();
                if required.is_empty() {
                    "an object".into()
                } else {
                    format!("an object with {}", required.join(", "))
                }
            }
            other => format!("a JSON {other}"),
        }
    }
}

/// The aliases a property schema declares.
fn aliases_of(prop: &Value) -> impl Iterator<Item = &str> {
    prop.get(ALIASES)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
}

/// The names an unknown key is compared with, in the order a tie is broken:
/// the REQUIRED fields first, in the order the schema lists them (so the
/// primary node of a setter — its first required `…_id` — is the one `id` and
/// `node_id` reach), then the other published names, then the aliases, so a
/// tie goes to the spelling the schema teaches.
fn candidates<'n>(
    schema: &'n Map<String, Value>,
    props: &'n Map<String, Value>,
    spelling: &std::collections::HashMap<&'n str, &'n str>,
) -> Vec<&'n str> {
    let mut out: Vec<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    for name in props.keys() {
        if !out.contains(&name.as_str()) {
            out.push(name.as_str());
        }
    }
    let mut aliases: Vec<&str> = spelling
        .keys()
        .copied()
        .filter(|n| !out.contains(n))
        .collect();
    aliases.sort_unstable();
    out.extend(aliases);
    out
}

fn types_of(s: &Map<String, Value>) -> Option<Vec<String>> {
    match s.get("type")? {
        Value::String(t) => Some(vec![t.clone()]),
        Value::Array(ts) => Some(
            ts.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect(),
        ),
        _ => None,
    }
}

/// JSON-type membership as the DESERIALISER sees it, not only as JSON Schema
/// does: `1.0` is not an integer here, because serde will not read it into
/// one, and a check more lenient than the deserialiser would let through a
/// call it then refuses with the bare string this module exists to end.
fn has_type(v: &Value, t: &str) -> bool {
    match (t, v) {
        ("null", Value::Null) => true,
        ("boolean", Value::Bool(_)) => true,
        ("string", Value::String(_)) => true,
        ("array", Value::Array(_)) => true,
        ("object", Value::Object(_)) => true,
        ("number", Value::Number(_)) => true,
        ("integer", Value::Number(n)) => n.is_i64() || n.is_u64(),
        _ => false,
    }
}

fn fits_format(n: &serde_json::Number, format: &str) -> bool {
    let unsigned = |bits: u32| n.as_u64().is_some_and(|x| bits >= 64 || x < (1u64 << bits));
    let signed = |bits: u32| {
        n.as_i64()
            .is_some_and(|x| bits >= 64 || (x >= -(1i64 << (bits - 1)) && x < (1i64 << (bits - 1))))
    };
    match format {
        "uint" | "uint64" => unsigned(64),
        "uint8" => unsigned(8),
        "uint16" => unsigned(16),
        "uint32" => unsigned(32),
        "int" | "int64" => signed(64),
        "int8" => signed(8),
        "int16" => signed(16),
        "int32" => signed(32),
        // `double`, `float`, or a format this file does not know — the walk
        // test refuses the last, so it cannot reach a served schema.
        _ => true,
    }
}

/// Descriptions in this schema run to paragraphs; a refusal wants the opening,
/// not the essay. The full text is one `tools/list` away.
fn opening(folded: &str) -> String {
    const CUT: usize = 280;
    if folded.chars().count() > CUT {
        let cut: String = folded.chars().take(CUT).collect();
        format!("{}…", cut.trim_end())
    } else {
        folded.to_string()
    }
}

fn shown(v: &Value) -> String {
    let text = match v {
        Value::Null => return "null".into(),
        Value::Bool(b) => return format!("{b}"),
        Value::Number(n) => return format!("the number {n}"),
        Value::Array(a) => return format!("a list of {}", a.len()),
        Value::Object(_) => return "an object".into(),
        Value::String(s) => s,
    };
    let quoted = Value::String(text.clone()).to_string();
    if quoted.chars().count() > 80 {
        let cut: String = quoted.chars().take(77).collect();
        format!("the string {cut}…\"")
    } else {
        format!("the string {quoted}")
    }
}

/// The refusal a caller receives, naming the tool, then every place the
/// arguments do not fit — all of them, so the call is fixed in one round trip
/// rather than one refusal per field.
pub fn refusal(tool: &str, violations: &[Violation], transport: Transport) -> String {
    let n = violations.len();
    let mut out = format!(
        "`{tool}` was refused before it ran: its arguments do not fit its published input schema, \
         so nothing was read or written. {n} {} — every one is listed, so the call can be fixed \
         in one round trip:\n",
        if n == 1 { "problem" } else { "problems" }
    );
    for v in violations {
        let p = &v.path;
        let line = match &v.problem {
            Problem::WrongType { got } => {
                format!("`{p}`: expected {}; got {}.", v.expects, shown(got))
            }
            Problem::Missing { .. } => {
                format!("`{p}` is required and was not given: {}.", v.expects)
            }
            Problem::Unknown { legal, nearest } => {
                let near = nearest
                    .as_ref()
                    .map(|n| format!(" Nearest served parameter: `{n}`."))
                    .unwrap_or_default();
                let parent = parent_of(p);
                let names = legal
                    .iter()
                    .map(|l| format!("`{l}`"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let (at, listed) = match (&parent, legal.is_empty()) {
                    (None, true) => ("of this tool".to_string(), "it takes none".to_string()),
                    (None, false) => ("of this tool".to_string(), format!("it takes {names}")),
                    (Some(a), true) => (format!("at `{a}`"), "that object takes none".into()),
                    (Some(a), false) => {
                        (format!("at `{a}`"), format!("the names there are {names}"))
                    }
                };
                format!("`{p}` is not a parameter {at}; {listed}.{near}")
            }
            Problem::NotAllowed { got } => {
                format!("`{p}`: expected {}; got {}.", v.expects, shown(got))
            }
            Problem::OutOfRange { got } => {
                format!("`{p}`: expected {}; got {}.", v.expects, shown(got))
            }
            Problem::TooFew { got, min } => format!(
                "`{p}`: expected at least {min} {}; got {got}.",
                if *min == 1 { "item" } else { "items" }
            ),
            Problem::NotAField { redirect } => format!("`{p}`: {redirect}"),
            Problem::Duplicate { names } => format!(
                "`{p}` was given twice, as {} — they are one field; pass it once.",
                names
                    .iter()
                    .map(|n| format!("`{n}`"))
                    .collect::<Vec<_>>()
                    .join(" and ")
            ),
        };
        out.push_str("  · ");
        out.push_str(&line);
        match (&v.problem, &v.description) {
            (Problem::Unknown { .. } | Problem::NotAField { .. }, _) => {}
            (_, Some(d)) => {
                out.push_str(" — ");
                out.push_str(d);
            }
            (_, None) => out.push_str(
                " Its schema publishes no description of this field, so what it is for cannot \
                 be quoted here.",
            ),
        }
        out.push('\n');
    }
    // The required fields this call DID give, set apart once per object, so
    // the missing ones above are a diff and not the whole obligation
    // (cap:a-missing-field-refusal-names-the-fields-this-call-lacked).
    let mut set_apart: Vec<(String, &Vec<String>)> = Vec::new();
    for v in violations {
        if let Problem::Missing { passed } = &v.problem
            && !passed.is_empty()
        {
            let at = parent_of(&v.path)
                .map(|a| format!(" at `{a}`"))
                .unwrap_or_default();
            if !set_apart.iter().any(|(a, _)| *a == at) {
                set_apart.push((at, passed));
            }
        }
    }
    for (at, passed) in set_apart {
        out.push_str(&format!(
            "Already passed{at}: {}.\n",
            passed
                .iter()
                .map(|f| format!("`{f}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if violations
        .iter()
        .any(|v| matches!(v.problem, Problem::Unknown { .. }))
    {
        out.push_str(&match transport {
            // THE NEAREST NAME COMES FIRST and this is offered as a
            // possibility, after it. Measured 2026-09-18 (flo2): eight
            // rejected calls were reported upstream as schema drift because
            // the refusal LED with this, when five of six names had never
            // existed in any release — the agent had guessed `node_id` for
            // `id` (fact:defect-a-clients-tool-list-is-fixed-at-connection-so-a-restarted-servers-new-fields-are-unreachable).
            Transport::Session => format!(
                "If nothing listed is what you meant, your client's tool list may predate this \
                 server ({}). Reconnect (a new session) to refresh the schema.",
                env!("CARGO_PKG_VERSION")
            ),
            Transport::CallDoor => format!(
                "The --call door reads this schema from the binary it runs ({}) on every call, so \
                 a stale tool list is not the cause: the names listed are the ones this version \
                 takes.",
                env!("CARGO_PKG_VERSION")
            ),
        });
        out.push('\n');
    }
    out
}

/// Where an unknown key sits, in words: at the top level, or inside the item
/// or object its path names.
fn parent_of(path: &str) -> Option<String> {
    match path.rfind(['.', '[']) {
        Some(i) if i > 0 => Some(path[..i].to_string()),
        _ => None,
    }
}

/// The usage ledger's class for a refusal built here — derived from what was
/// found, never from the refusal's wording.
pub fn usage_class(violations: &[Violation]) -> crate::usage::RefusalClass {
    use crate::usage::RefusalClass;
    if violations.iter().any(|v| {
        !matches!(
            v.problem,
            Problem::Missing { .. } | Problem::Unknown { .. } | Problem::NotAField { .. }
        )
    }) {
        RefusalClass::InvalidArgument
    } else if violations
        .iter()
        .any(|v| matches!(v.problem, Problem::Missing { .. }))
    {
        RefusalClass::MissingArgument
    } else {
        RefusalClass::UnknownArgument
    }
}

/// The nearest of `legal` to `unknown`, or `None` when nothing is near enough
/// to offer — a wrong "nearest" is worse than none.
///
/// THE SAME CONCEPT UNDER A DIFFERENT NAME is the case that matters (flo2 F10,
/// 2026-09-19: `id` for `decision_id`, `node_id` for `target_id`, `properties`
/// for `props`), and edit distance alone gets it wrong — `id` is closer to
/// `status` than to `decision_id` by letters. Three rules, in order: (1) a
/// node-naming field (`id`, `…_id`) maps to the node-naming fields — `id` and
/// `node_id` to the FIRST one, which is the primary node (fields are listed in
/// declaration order), any other `…_id` to the closest by letters; (2)
/// otherwise a candidate sharing a whole `_` token or a four-letter prefix with
/// the unknown (`props`/`properties`) wins, closest by letters among those; (3)
/// only then letters alone, and only when the distance is under half the name.
pub fn nearest_name(unknown: &str, legal: &[&str]) -> Option<String> {
    if legal.is_empty() {
        return None;
    }
    let indexed: Vec<(usize, &str)> = legal.iter().copied().enumerate().collect();
    if is_node_field(unknown) {
        let node_fields: Vec<(usize, &str)> = indexed
            .iter()
            .copied()
            .filter(|(_, c)| is_node_field(c))
            .collect();
        if !node_fields.is_empty() {
            let pick = if unknown == "id" || unknown == "node_id" {
                node_fields.first().copied()
            } else {
                closest_by_letters(unknown, node_fields)
            };
            return pick.map(|(_, c)| c.to_string());
        }
    }
    let sharing: Vec<(usize, &str)> = indexed
        .iter()
        .copied()
        .filter(|(_, c)| shares_a_token(unknown, c))
        .collect();
    if let Some((_, c)) = closest_by_letters(unknown, sharing) {
        return Some(c.to_string());
    }
    let (_, c) = closest_by_letters(unknown, indexed)?;
    let d = edit_distance(unknown, c);
    (d * 2 < unknown.len().max(c.len())).then(|| c.to_string())
}

fn is_node_field(s: &str) -> bool {
    s == "id" || s.ends_with("_id")
}

fn name_tokens(s: &str) -> Vec<&str> {
    s.split('_').filter(|t| !t.is_empty()).collect()
}

fn shares_a_token(unknown: &str, cand: &str) -> bool {
    let unknown_tokens = name_tokens(unknown);
    name_tokens(cand).iter().any(|t| unknown_tokens.contains(t))
        || (unknown.len() >= 4
            && cand.len() >= 4
            && unknown.is_char_boundary(4)
            && cand.is_char_boundary(4)
            && unknown[..4] == cand[..4])
}

fn closest_by_letters<'a>(unknown: &str, cands: Vec<(usize, &'a str)>) -> Option<(usize, &'a str)> {
    cands
        .into_iter()
        .min_by_key(|(order, c)| (edit_distance(unknown, c), *order))
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur.push((prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// A schema with the server-internal extensions taken out — what a client is
/// shown: every [`ALIASES`] declaration, and every property marked
/// [`NOT_A_FIELD`], whole.
pub fn strip_internal(schema: &Map<String, Value>) -> Map<String, Value> {
    fn strip(v: &mut Value) {
        match v {
            Value::Object(m) => {
                m.remove(ALIASES);
                if let Some(Value::Object(props)) = m.get_mut("properties") {
                    props.retain(|_, p| p.get(NOT_A_FIELD).is_none());
                }
                for sub in m.values_mut() {
                    strip(sub);
                }
            }
            Value::Array(items) => items.iter_mut().for_each(strip),
            _ => {}
        }
    }
    let mut v = Value::Object(schema.clone());
    strip(&mut v);
    match v {
        Value::Object(m) => m,
        _ => Map::new(),
    }
}

/// Whether a schema carries any server-internal extension, so the listing
/// clones only the schemas it must change.
pub fn has_internal(schema: &Map<String, Value>) -> bool {
    fn any(v: &Value) -> bool {
        match v {
            Value::Object(m) => {
                m.contains_key(ALIASES) || m.contains_key(NOT_A_FIELD) || m.values().any(any)
            }
            Value::Array(items) => items.iter().any(any),
            _ => false,
        }
    }
    schema.contains_key(ALIASES) || schema.values().any(any)
}

/// A tool as a client is shown it: the server-internal extensions taken out of
/// its input schema. The schema is cloned only when it carries one.
pub fn published(mut tool: rmcp::model::Tool) -> rmcp::model::Tool {
    if has_internal(&tool.input_schema) {
        tool.input_schema = std::sync::Arc::new(strip_internal(&tool.input_schema));
    }
    tool
}

/// `add_change_event`'s `description`: accepted, never advertised, and
/// answered with the redirect to `summary` / `rationale`.
pub fn change_event_description_not_a_field(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": ["string", "null"],
        "x-reflow2-not-a-field": crate::service::CHANGE_EVENT_HAS_NO_DESCRIPTION,
    })
}

/// Close every no-argument schema on an assembled router — see
/// [`close_empty_schema`].
pub fn close_empty_schemas<S>(
    mut router: rmcp::handler::server::router::tool::ToolRouter<S>,
) -> rmcp::handler::server::router::tool::ToolRouter<S> {
    for route in router.map.values_mut() {
        let mut schema = (*route.attr.input_schema).clone();
        close_empty_schema(&mut schema);
        if schema != *route.attr.input_schema {
            route.attr.input_schema = std::sync::Arc::new(schema);
        }
    }
    router
}

/// A tool that takes no arguments PUBLISHES that it takes none.
///
/// rmcp gives a handler with no parameters the schema `{"type": "object",
/// "properties": {}}`, which says any key is welcome, and then ignores every
/// argument it is sent. So `mirrors` called with `{"project_id": …}` answered
/// as if the argument had been read — a silent drop. Twelve tools were served
/// that way while every other no-argument tool's empty request struct already
/// said `additionalProperties: false`. Closing the schema here, once, at
/// assembly, covers the next such tool as well as these.
pub fn close_empty_schema(schema: &mut Map<String, Value>) {
    let no_properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .is_none_or(Map::is_empty);
    if no_properties && !schema.contains_key("additionalProperties") {
        schema.insert("additionalProperties".into(), Value::Bool(false));
    }
}

/// The refusal for a call whose arguments FIT the published schema and that
/// the deserialiser refused anyway. That disagreement is a defect in reflow2 —
/// the published schema and the request type say different things — and the
/// refusal says so, names the tool, and keeps the deserialiser's words.
/// Anything that is not a deserialisation refusal is returned unchanged.
pub fn deserializer_refusal(tool: &str, text: &str, transport: Transport) -> Option<String> {
    let rest = text
        .strip_prefix("failed to deserialize parameters:")?
        .trim();
    let nearest = rest
        .split("unknown field `")
        .nth(1)
        .and_then(|r| r.split('`').next())
        .and_then(|unknown| {
            let legal: Vec<&str> = rest
                .split("expected ")
                .nth(1)?
                .split('`')
                .skip(1)
                .step_by(2)
                .collect();
            nearest_name(unknown, &legal)
        })
        .map(|n| format!(" Nearest served parameter: `{n}`."))
        .unwrap_or_default();
    let stale = match transport {
        Transport::Session if rest.contains("unknown field") => format!(
            " If that is not what you meant, your client's tool list may predate this server \
             ({}). Reconnect (a new session) to refresh the schema.",
            env!("CARGO_PKG_VERSION")
        ),
        _ => String::new(),
    };
    Some(format!(
        "`{tool}` was refused before it ran, so nothing was written: {rest}.{nearest}{stale} These \
         arguments fit the tool's published input schema and its request type refused them all \
         the same, which is a defect in reflow2 (the schema and the type disagree) — report it \
         with this text."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn obj(v: Value) -> Map<String, Value> {
        v.as_object().cloned().expect("object")
    }

    fn schema() -> Map<String, Value> {
        obj(json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["id", "links"],
            "properties": {
                "id": {"type": "string", "description": "The node.", "x-reflow2-aliases": ["node_id"]},
                "count": {"type": ["integer", "null"], "format": "uint", "minimum": 0},
                "mode": {"type": "string", "enum": ["a", "b"]},
                "links": {"type": "array", "items": {"$ref": "#/$defs/Link"}, "description": "Links."}
            },
            "$defs": {
                "Link": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["other_id", "evidence"],
                    "properties": {
                        "other_id": {"type": "string"},
                        "evidence": {"type": "string", "description": "WHY it holds."}
                    }
                }
            }
        }))
    }

    #[test]
    fn a_fitting_call_has_no_violations_and_an_alias_fits() {
        let s = schema();
        let ok = obj(json!({"id": "x", "links": [{"other_id": "y", "evidence": "e"}]}));
        assert!(check(&s, &ok).is_empty());
        let alias = obj(json!({"node_id": "x", "links": []}));
        assert!(check(&s, &alias).is_empty(), "{:?}", check(&s, &alias));
    }

    #[test]
    fn every_class_is_found_with_its_path() {
        let s = schema();
        let bad = obj(json!({
            "id": 5,
            "count": -1,
            "mode": "c",
            "zz": 1,
            "links": [{"other_id": "y", "bogus": true}]
        }));
        let v = check(&s, &bad);
        let at = |p: &str| v.iter().find(|x| x.path == p).map(|x| x.problem.clone());
        assert!(matches!(at("id"), Some(Problem::WrongType { .. })), "{v:?}");
        assert!(
            matches!(at("count"), Some(Problem::OutOfRange { .. })),
            "{v:?}"
        );
        assert!(
            matches!(at("mode"), Some(Problem::NotAllowed { .. })),
            "{v:?}"
        );
        assert!(matches!(at("zz"), Some(Problem::Unknown { .. })), "{v:?}");
        assert!(
            matches!(at("links[0].bogus"), Some(Problem::Unknown { .. })),
            "{v:?}"
        );
        assert!(
            matches!(at("links[0].evidence"), Some(Problem::Missing { .. })),
            "{v:?}"
        );
        let missing = v.iter().find(|x| x.path == "links[0].evidence").unwrap();
        assert_eq!(missing.description.as_deref(), Some("WHY it holds."));
    }

    #[test]
    fn a_field_given_under_two_spellings_is_refused() {
        let v = check(
            &schema(),
            &obj(json!({"id": "x", "node_id": "x", "links": []})),
        );
        assert!(matches!(v[0].problem, Problem::Duplicate { .. }), "{v:?}");
    }

    #[test]
    fn a_float_is_not_an_integer_because_the_deserialiser_says_so() {
        let v = check(
            &schema(),
            &obj(json!({"id": "x", "links": [], "count": 1.0})),
        );
        assert!(matches!(v[0].problem, Problem::WrongType { .. }), "{v:?}");
    }

    #[test]
    fn the_refusal_names_the_tool_every_path_and_the_door_is_told_no_reconnect() {
        let v = check(&schema(), &obj(json!({"zz": 1, "links": [{}]})));
        let session = refusal("t", &v, Transport::Session);
        let door = refusal("t", &v, Transport::CallDoor);
        for text in [&session, &door] {
            assert!(text.contains("`t`"), "{text}");
            for p in ["`zz`", "`id`", "`links[0].other_id`", "`links[0].evidence`"] {
                assert!(text.contains(p), "{p} in {text}");
            }
            assert!(text.contains("WHY it holds."), "{text}");
            assert!(text.contains("publishes no description"), "{text}");
        }
        assert!(session.contains("Reconnect"), "{session}");
        assert!(!door.contains("Reconnect"), "{door}");
    }

    #[test]
    fn an_empty_schema_is_closed_and_a_declared_one_is_left_alone() {
        let mut open = obj(json!({"type": "object", "properties": {}}));
        close_empty_schema(&mut open);
        assert_eq!(open["additionalProperties"], json!(false));
        let mut declared = obj(json!({"type": "object", "additionalProperties": true}));
        close_empty_schema(&mut declared);
        assert_eq!(declared["additionalProperties"], json!(true));
    }

    #[test]
    fn the_listing_strips_aliases_and_nothing_else() {
        let s = schema();
        assert!(has_internal(&s));
        let shown = strip_internal(&s);
        assert!(!has_internal(&shown));
        assert_eq!(shown["properties"]["id"]["description"], json!("The node."));
    }
}
