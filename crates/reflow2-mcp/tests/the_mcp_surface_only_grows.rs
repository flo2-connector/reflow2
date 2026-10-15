//! THE MCP SURFACE SHRINKS ONLY THROUGH A DEPRECATING RELEASE: what a client of
//! the last release sent and read still works the same way on this build,
//! unless that release announced it was going.
//!
//! The file keeps its old name, `the_mcp_surface_only_grows`, so its history
//! and every record that cites it (the design's Verification, AGENTS.md, the
//! re-bless command in each cut's notes) still find it. What it checks moved
//! on 2026-10-14, below.
//!
//! # Why this exists — the owner's constraint, verbatim
//!
//! "I just want to make sure the changes we implement for vscode don't break
//! it for how it was originally used/designed as an mcp server. My brother
//! likes using as is." (Anthony, 2026-10-05, approving stage 2 of the work-door
//! increment.)
//!
//! And its amendment, on his word, 2026-10-13
//! (`req:the-mcp-server-keeps-working-as-is-and-door-improvements-are-additive-or-opt-in`):
//! "never removed" becomes "never removed WITHOUT A RELEASE OF NOTICE".
//! Anything a client can call or read is retired in two releases — a
//! DEPRECATING release, in which it still works and says it is going, then the
//! NEXT release removes it — and "the check that the served surface only grows
//! becomes a check that it shrinks only through a deprecating release."
//!
//! The toolsnap goldens (`tools/toolsnaps/`) make every surface change a
//! reviewed diff, but they are re-blessed in the same pull request that
//! changes the surface, so they say THAT the surface moved and never WHETHER
//! it only grew. This compares the build against a frozen record of the LAST
//! RELEASE:
//!
//! - every tool the release served is served, with the same read/write hint;
//! - every argument it published is published at the same path, with no type
//!   taken away, no allowed value taken away, no new required field, no
//!   tighter minimum, and an open object not closed;
//! - every reply in a fixed scenario of ordinary calls keeps every field it
//!   had, at the same path and of the same JSON type, and no call the release
//!   answered is refused;
//! - every node type, node property and edge type its describe_schema served
//!   is served.
//!
//! # What may go: only what the last release announced
//!
//! The record also holds the release's DEPRECATIONS — its
//! `deprecation::REGISTRY`, as its describe_schema listed them. Anything the
//! record holds that this build no longer serves PASSES only when the record
//! marks it deprecated, which means the release that record was taken from
//! announced it; anything else missing fails exactly as before. An item the
//! record marks deprecated that is still served passes too: removal is
//! allowed, not forced. A deprecated tool may go whole, so any narrowing of it
//! passes; a deprecated argument may take its sub-fields with it; a deprecated
//! node type, edge type or property may leave the vocabulary, and an enum
//! value naming that type, and the reply fields of a call the release itself
//! told it used them.
//!
//! A TOOL is retired one step further, because the accepted tool rule says a
//! removal "ships first as a stub under the old name that refuses and names
//! the replacement — for one release"
//! (`dec:a-tool-is-never-deleted-without-usage-evidence-across-projects-and-never-without-a-refusing-stub-first`).
//! So a deprecated tool this build no longer lists must still be ANSWERED by
//! its old name, with a refusal carrying what the release said to do; its
//! registry entry stays for that one release and comes out in the next.
//!
//! # The two narrowings a reader would otherwise mistake for breaks
//!
//! Each is listed below WITH ITS PROOF, so it is a judgement on the record and
//! not a silenced check:
//!
//! - [`UNADVERTISED`]: an argument taken out of the PUBLISHED schema that is
//!   still ACCEPTED. The test calls it and asserts the old answer.
//! - [`ENUM_ATTACHED`]: a value set published as an enum where the release
//!   published prose. The release refused a value outside the set one step
//!   later, in its handler; the frozen record holds that refusal (a scenario
//!   step the release answered with a refusal), and the test reads it from
//!   there. The refusal only moved earlier.
//!
//! # Re-blessing, at a cut
//!
//! The record is `fixtures/mcp_surface_at_last_release.json`, taken by driving
//! a real binary: `--list-tools --full`, the scenario through `--call`, and
//! `describe_schema` for the vocabulary and the deprecations. After a release
//! is tagged, take it from that release's binary — which writes that
//! release's deprecations into the record, so the NEXT release may remove them:
//!
//! ```text
//! REFLOW2_BLESS_SURFACE=target/release/reflow2-mcp \
//!     cargo test -p reflow2-mcp --test in_memory the_mcp_surface_only_grows::
//! ```
//!
//! A deliberate break the owner approved is the only other reason to touch it,
//! and it goes in with the approval named in the commit.
// Some tests here open a design store in a folder: each is marked
// `#[cfg(feature = "rocksdb")]`, and a build without on-disk storage compiles
// them out — and with them helpers only they use, which is all this allows.
#![cfg_attr(not(feature = "rocksdb"), allow(dead_code, unused_imports))]

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::{Command, Output, Stdio};

use reflow2_mcp::deprecation::{Deprecation, Item, used_by};
use serde_json::{Map, Value, json};

const RECORD: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/mcp_surface_at_last_release.json"
);

/// Arguments taken out of the published schema and still accepted, each with
/// a call that proves it: the arguments, and words the answer must hold.
const UNADVERTISED: &[(&str, &str, &str, &str, &str)] = &[(
    "add_change_event",
    "description",
    "a decoy that existed only to redirect the commonest mistake; find_tools listed it as a \
     real field (fact:the-changeevent-description-decoy-is-listed-by-find-tools-and-passes-the-argument-check-2026-10-05)",
    r#"{"id":"chg:decoy","name":"D","change_type":"new_feature","description":"prose"}"#,
    "has no `description`",
)];

/// Value sets published as an enum where the release published prose, each
/// naming the scenario step whose recorded answer shows the release refused a
/// value outside the set.
const ENUM_ATTACHED: &[(&str, &str, &str)] = &[
    (
        "add_requirement",
        "provenance",
        "add_requirement with provenance `user`",
    ),
    (
        "set_artifact_checksum",
        "disposition",
        "set_artifact_checksum with disposition `design_change`",
    ),
    (
        "set_artifact_checksums",
        "accepts[].disposition",
        "set_artifact_checksums with disposition `design_change`",
    ),
];

/// The scenario: ordinary calls a client of the release makes, in order, on a
/// fresh design. Each is (label, tool, arguments). Arguments are what the
/// RELEASE takes, so the comparison is of one call made two ways.
fn scenario() -> Vec<(&'static str, &'static str, Value)> {
    vec![
        (
            "add_project",
            "add_project",
            json!({"id": "proj:s", "name": "S"}),
        ),
        (
            "add_requirement",
            "add_requirement",
            json!({"id": "req:s", "name": "R", "statement": "The system records what it is told.",
                   "provenance": "imported"}),
        ),
        (
            "add_requirement with provenance `user`",
            "add_requirement",
            json!({"id": "req:u", "name": "U", "statement": "S", "provenance": "user"}),
        ),
        (
            "add_capability",
            "add_capability",
            json!({"id": "cap:s", "name": "C", "description": "Records.", "satisfies": "req:s"}),
        ),
        (
            "add_decision",
            "add_decision",
            json!({"id": "dec:base", "name": "Base", "decision": "the base choice"}),
        ),
        (
            "add_decision (second)",
            "add_decision",
            json!({"id": "dec:q", "name": "Q", "decision": "the dependent choice"}),
        ),
        (
            "add_decision, an edge-only revise",
            "add_decision",
            json!({"id": "dec:q", "related_to": [{"other_id": "dec:base", "relation": "BLOCKS",
                    "evidence": "q cannot settle before base"}]}),
        ),
        (
            "add_epoch",
            "add_epoch",
            json!({"id": "epoch:s", "name": "E", "sequence": 10, "epoch_type": "milestone"}),
        ),
        (
            "add_change_event",
            "add_change_event",
            json!({"id": "chg:s", "name": "Ch", "change_type": "new_feature",
                   "summary": "Something changed."}),
        ),
        (
            "add_artifact",
            "add_artifact",
            json!({"id": "art:a", "name": "a", "location": "src/a.rs"}),
        ),
        (
            "add_artifact at a pinned URL",
            "add_artifact",
            json!({"id": "art:b", "name": "b",
                   "location": "https://github.com/org/repo/blob/0123abc/src/b.rs"}),
        ),
        (
            "set_artifact_checksum",
            "set_artifact_checksum",
            json!({"artifact_id": "art:a", "checksum": "abc",
                   "disposition": "baseline_established"}),
        ),
        (
            "set_artifact_checksum with disposition `design_change`",
            "set_artifact_checksum",
            json!({"artifact_id": "art:a", "checksum": "abd", "disposition": "design_change"}),
        ),
        (
            "set_artifact_checksums with disposition `design_change`",
            "set_artifact_checksums",
            json!({"accepts": [{"artifact_id": "art:a", "checksum": "abe",
                                 "disposition": "design_change"}]}),
        ),
        (
            "coverage_report",
            "coverage_report",
            json!({"observed": [{"path": "src/a.rs"}, {"path": "src/b.rs"}]}),
        ),
        (
            "scan_nodes of an undeclared type",
            "scan_nodes",
            json!({"node_type": "Fact"}),
        ),
        (
            "scan_nodes",
            "scan_nodes",
            json!({"node_type": "Requirement"}),
        ),
        (
            "find_tools",
            "find_tools",
            json!({"query": "update properties of an existing node"}),
        ),
        (
            "describe_schema of a tool",
            "describe_schema",
            json!({"tool": "add_epoch"}),
        ),
        (
            "describe_schema of a type",
            "describe_schema",
            json!({"node_type": "Requirement"}),
        ),
        (
            "add_verification",
            "add_verification",
            json!({"id": "ver:s", "name": "V", "status": "blocked"}),
        ),
        (
            "set_verification_status",
            "set_verification_status",
            json!({"verification_id": "ver:s", "status": "failing"}),
        ),
        (
            "record_finding",
            "record_finding",
            json!({"id": "fact:s", "subject_id": "cap:s", "statement": "Observed."}),
        ),
        (
            "import_graph",
            "import_graph",
            json!({"document": {"nodes": [{"node_type": "Component", "node_id": "cmp:i",
                    "properties": {"name": "I", "purpose": "imported"}}]}}),
        ),
        ("get_node", "get_node", json!({"id": "cmp:i"})),
        ("loop_status", "loop_status", json!({})),
    ]
}

// ---- driving a binary ---------------------------------------------------------------

fn run(bin: &str, dir: &Path, args: &[&str]) -> Output {
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();
    Command::new(bin)
        .current_dir(dir)
        .args(args)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("REFLOW2_CONFIG_DIR", home.join("reflow2-config"))
        .env_remove("RUST_LOG")
        .env_remove("REFLOW2_TRUSTED_GATEWAY")
        .env_remove("REFLOW2_CONTRIBUTOR_ID")
        .env_remove("REFLOW2_CONTRIBUTOR_MAP")
        .stdin(Stdio::null())
        .output()
        .expect("the binary runs")
}

/// A schema with its prose taken out: what a client BINDS to. Only keyword
/// positions are touched — a property NAMED `description` is kept.
fn structure(v: &Value) -> Value {
    match v {
        Value::Object(m) => {
            let mut out = Map::new();
            for (k, x) in m {
                match k.as_str() {
                    "description" | "title" | "default" | "examples" => {}
                    "properties" | "$defs" => {
                        let inner = x
                            .as_object()
                            .map(|p| {
                                p.iter()
                                    .map(|(n, s)| (n.clone(), structure(s)))
                                    .collect::<Map<_, _>>()
                            })
                            .unwrap_or_default();
                        out.insert(k.clone(), Value::Object(inner));
                    }
                    "enum" | "required" => {
                        out.insert(k.clone(), x.clone());
                    }
                    _ => {
                        out.insert(k.clone(), structure(x));
                    }
                }
            }
            Value::Object(out)
        }
        Value::Array(a) => Value::Array(a.iter().map(structure).collect()),
        other => other.clone(),
    }
}

/// Every field path in a reply with its JSON type: `items[].node_id:string`.
fn shape(v: &Value, path: &str, out: &mut BTreeSet<String>) {
    let kind = match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    };
    if !path.is_empty() {
        out.insert(format!("{path}:{kind}"));
    }
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                let p = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                shape(x, &p, out);
            }
        }
        Value::Array(a) => {
            for x in a {
                shape(x, &format!("{path}[]"), out);
            }
        }
        _ => {}
    }
}

/// The surface of one binary: its tools, the scenario's answers, its
/// vocabulary, and the deprecations it announces.
fn take(bin: &str) -> Value {
    let dir = tempfile::tempdir().expect("tempdir");
    let version = String::from_utf8_lossy(&run(bin, dir.path(), &["--version"]).stdout)
        .trim()
        .to_string();

    let listing_graph = dir.path().join("listing").join("graph");
    let o = run(
        bin,
        dir.path(),
        &[
            "--graph-path",
            listing_graph.to_str().unwrap(),
            "--list-tools",
            "--full",
        ],
    );
    assert!(
        o.status.success(),
        "{bin} --list-tools --full: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    let listed: Value = serde_json::from_slice(&o.stdout).expect("--list-tools prints JSON");
    let mut tools = Map::new();
    for t in listed["tools"].as_array().expect("tools") {
        let name = t["name"].as_str().expect("name").to_string();
        tools.insert(
            name,
            json!({
                "read_only": t["annotations"]["readOnlyHint"],
                "input_schema": structure(&t["inputSchema"]),
            }),
        );
    }

    let graph = dir.path().join("scenario").join("graph");
    let mut replies = Vec::new();
    for (label, tool, args) in scenario() {
        let a = args.to_string();
        let o = run(
            bin,
            dir.path(),
            &[
                "--graph-path",
                graph.to_str().unwrap(),
                "--no-export",
                "--call",
                tool,
                "--args",
                &a,
            ],
        );
        let ok = o.status.success();
        let mut fields = BTreeSet::new();
        let mut uses = Vec::new();
        if ok {
            let v: Value = serde_json::from_slice(&o.stdout).unwrap_or_else(|e| {
                panic!(
                    "{bin} `{label}`: stdout is not JSON ({e}): {}",
                    String::from_utf8_lossy(&o.stdout)
                )
            });
            shape(&v, "", &mut fields);
            // What the binary itself told the client this call used that is
            // going: its notice, read back as items.
            for n in v["deprecation_notice"].as_array().into_iter().flatten() {
                uses.push(n["item"].clone());
            }
        }
        let mut reply = json!({
            "step": label,
            "tool": tool,
            "arguments": args,
            "answered": ok,
            "exit": o.status.code(),
            "shape": fields.into_iter().collect::<Vec<_>>(),
        });
        // Only when there is something to say, so a release that announced
        // nothing writes the line it always wrote.
        if !uses.is_empty() {
            reply["uses"] = Value::Array(uses);
        }
        replies.push(reply);
    }

    // The vocabulary and the deprecations, from the binary's own
    // describe_schema on the design the scenario made: every node type with
    // its property names, every edge type, and the list of what is going. The
    // budget is raised so nothing is trimmed; names never are, and this makes
    // that not matter.
    let o = run(
        bin,
        dir.path(),
        &[
            "--graph-path",
            graph.to_str().unwrap(),
            "--no-export",
            "--call",
            "describe_schema",
            "--args",
            r#"{"budget_chars": 100000000}"#,
        ],
    );
    assert!(
        o.status.success(),
        "{bin} describe_schema: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    let schema: Value = serde_json::from_slice(&o.stdout).expect("describe_schema is JSON");
    let mut node_types = Map::new();
    for n in schema["node_types"].as_array().expect("node_types") {
        let props: Vec<Value> = n["properties"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|p| p["name"].as_str().map(|s| json!(s)))
            .collect();
        node_types.insert(
            n["node_type"].as_str().expect("node_type").to_string(),
            Value::Array(props),
        );
    }
    let edge_types: Vec<Value> = schema["edge_types"]
        .as_array()
        .expect("edge_types")
        .iter()
        .filter_map(|e| e["edge_type"].as_str().map(|s| json!(s)))
        .collect();
    // No `deprecations` key is a build that announces nothing: it has none, or
    // it predates the rule (every release up to 0.84.0). Either way nothing it
    // served may go in the release after it.
    let deprecations: Vec<Value> = match schema.get("deprecations") {
        None => Vec::new(),
        Some(d) => {
            let parsed: Vec<Deprecation> = serde_json::from_value(d.clone())
                .unwrap_or_else(|e| panic!("{bin}'s deprecations do not read ({e}): {d}"));
            parsed
                .iter()
                .map(|x| serde_json::to_value(x).expect("a deprecation serialises"))
                .collect()
        }
    };

    json!({
        "taken_with": version,
        "tools": tools,
        "replies": replies,
        "deprecations": deprecations,
        "vocabulary": {"node_types": node_types, "edge_types": edge_types},
    })
}

/// The record as committed: one tool, one reply, one deprecation and one node
/// type per line, so a re-bless reads as a diff of what moved.
fn write_record(taken: &Value) {
    let mut s = String::from("{\n");
    s.push_str(&format!(
        "\"_what\": {},\n",
        json!(
            "The MCP surface of the LAST RELEASE, for tests/the_mcp_surface_only_grows.rs: each \
             tool's read/write hint and input schema with its prose taken out, the reply shape \
             of a fixed scenario of ordinary calls, the vocabulary (node types with their \
             properties, and edge types), and the deprecations the release announced — the only \
             things the next release may remove. Taken by driving the release binary \
             (--list-tools --full, --call, and describe_schema). Re-bless at a cut: \
             REFLOW2_BLESS_SURFACE=<that release's binary> cargo test -p reflow2-mcp --test \
             in_memory the_mcp_surface_only_grows::."
        )
    ));
    s.push_str(&format!("\"taken_with\": {},\n", taken["taken_with"]));
    let lines = |s: &mut String, items: Vec<String>| {
        let n = items.len();
        for (i, line) in items.into_iter().enumerate() {
            s.push_str(&format!("{line}{}\n", if i + 1 < n { "," } else { "" }));
        }
    };
    s.push_str("\"tools\": {\n");
    lines(
        &mut s,
        taken["tools"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(name, t)| format!("{}: {t}", json!(name)))
            .collect(),
    );
    s.push_str("},\n\"replies\": [\n");
    lines(
        &mut s,
        taken["replies"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r.to_string())
            .collect(),
    );
    s.push_str("],\n\"deprecations\": [\n");
    lines(
        &mut s,
        taken["deprecations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d.to_string())
            .collect(),
    );
    s.push_str("],\n\"vocabulary\": {\n\"edge_types\": ");
    s.push_str(&taken["vocabulary"]["edge_types"].to_string());
    s.push_str(",\n\"node_types\": {\n");
    lines(
        &mut s,
        taken["vocabulary"]["node_types"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(name, props)| format!("{}: {props}", json!(name)))
            .collect(),
    );
    s.push_str("}\n}\n}\n");
    std::fs::write(RECORD, s).expect("write the record");
}

// ---- comparing ----------------------------------------------------------------------

/// What the record says the last release announced it would remove, and the
/// questions the comparison asks of it. Every answer is an excuse for one
/// loss, and each excuse used is reported, so a pass is never silent about
/// what it let go.
struct Announced {
    deprecations: Vec<Deprecation>,
}

/// Is `path` the deprecated `prefix` itself, or under it?
fn under(path: &str, prefix: &str) -> bool {
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('.') || rest.starts_with("[]"))
}

impl Announced {
    fn from_record(record: &Value) -> Self {
        let deprecations = match record.get("deprecations") {
            None => Vec::new(),
            Some(d) => serde_json::from_value(d.clone())
                .unwrap_or_else(|e| panic!("the record's deprecations do not read ({e}): {d}")),
        };
        Self { deprecations }
    }

    fn find(&self, wanted: impl Fn(&Item) -> bool) -> Option<&Deprecation> {
        self.deprecations.iter().find(|d| wanted(&d.item))
    }

    fn tool(&self, tool: &str) -> Option<&Deprecation> {
        self.find(|i| matches!(i, Item::Tool { tool: t } if t == tool))
    }

    /// An argument at `path`, or anything of it, of `tool`.
    fn argument(&self, tool: &str, path: &str) -> bool {
        self.tool(tool).is_some()
            || self
                .find(|i| matches!(i, Item::Argument { tool: t, path: p } if t == tool && under(path, p)))
                .is_some()
    }

    fn node_type(&self, name: &str) -> bool {
        self.find(|i| matches!(i, Item::NodeType { node_type } if node_type == name))
            .is_some()
    }

    fn edge_type(&self, name: &str) -> bool {
        self.find(|i| matches!(i, Item::EdgeType { edge_type } if edge_type == name))
            .is_some()
    }

    fn property(&self, node_type: &str, property: &str) -> bool {
        self.node_type(node_type)
            || self
                .find(|i| {
                    matches!(i, Item::NodeProperty { node_type: t, property: p }
                        if t == node_type && p == property)
                })
                .is_some()
    }

    /// A value an enum may list that names a vocabulary item announced going.
    fn names_vocabulary(&self, v: &Value) -> bool {
        v.as_str()
            .is_some_and(|s| self.node_type(s) || self.edge_type(s))
    }

    /// What a recorded scenario step used that the release announced going:
    /// what its arguments name (the same rule a served call's notice follows,
    /// `deprecation::used_by`), and what the release's own reply said it used.
    fn used_by_step(&self, step: &Value) -> Vec<Item> {
        let tool = step["tool"].as_str().unwrap_or("");
        let mut used: Vec<Item> = used_by(&self.deprecations, tool, Some(&step["arguments"]), None)
            .into_iter()
            .map(|d| d.item.clone())
            .collect();
        for u in step["uses"].as_array().into_iter().flatten() {
            let item: Item = serde_json::from_value(u.clone())
                .unwrap_or_else(|e| panic!("a recorded use does not read ({e}): {u}"));
            if !used.contains(&item) {
                used.push(item);
            }
        }
        used
    }

    /// Is a reply field the step lost one the release announced? Its tool
    /// going takes everything; a deprecated reply field takes what is under
    /// it; a node or edge type the call used takes the reply's fields with it;
    /// a property the call used takes the fields named after it; and once the
    /// item has gone, so has the notice about it.
    fn reply_field(&self, step: &Value, used: &[Item], field: &str) -> bool {
        let tool = step["tool"].as_str().unwrap_or("");
        let path = field.rsplit_once(':').map(|(p, _)| p).unwrap_or(field);
        if self.tool(tool).is_some() {
            return true;
        }
        if self
            .find(|i| matches!(i, Item::ReplyField { tool: t, path: p } if t == tool && under(path, p)))
            .is_some()
        {
            return true;
        }
        if !used.is_empty() && under(path, "deprecation_notice") {
            return true;
        }
        used.iter().any(|i| match i {
            Item::NodeType { .. } | Item::EdgeType { .. } => true,
            Item::NodeProperty { property, .. } => {
                path.rsplit('.')
                    .next()
                    .map(|last| last.trim_end_matches("[]"))
                    == Some(property)
            }
            _ => false,
        })
    }
}

/// What the comparison found: the breaks, the listed narrowings it allowed,
/// the removals the last release announced, and the tools that went whose old
/// names must still answer with a refusal saying what to do.
#[derive(Default)]
struct Verdict {
    problems: Vec<String>,
    allowed: BTreeSet<String>,
    announced: BTreeSet<String>,
    /// (tool, the version that deprecated it, what the release said to do).
    stubs: Vec<(String, String, String)>,
}

struct Schemas<'a> {
    old_root: &'a Value,
    new_root: &'a Value,
    tool: &'a str,
    announced: &'a Announced,
    verdict: &'a mut Verdict,
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

fn types(s: &Value) -> Option<BTreeSet<String>> {
    match s.get("type")? {
        Value::String(t) => Some([t.clone()].into()),
        Value::Array(ts) => Some(
            ts.iter()
                .filter_map(|t| t.as_str().map(String::from))
                .collect(),
        ),
        _ => None,
    }
}

impl Schemas<'_> {
    /// A loss at `path`: a break, unless the last release announced the
    /// argument (or its tool) was going.
    fn problem(&mut self, path: &str, what: String) {
        let line = format!("{}.{path}: {what}", self.tool);
        if self.announced.argument(self.tool, path) {
            self.verdict.announced.insert(line);
        } else {
            self.verdict.problems.push(line);
        }
    }

    fn compare(&mut self, old: &Value, new: &Value, path: &str, depth: usize) {
        if depth > 12 {
            return;
        }
        let old = resolve(self.old_root, old);
        let new = resolve(self.new_root, new);

        match (types(old), types(new)) {
            (Some(o), Some(n)) if !o.is_subset(&n) => self.problem(
                path,
                format!("accepted types {o:?}, now {n:?} — a value the release took is refused"),
            ),
            (None, Some(n)) => {
                self.problem(path, format!("took any value, now only {n:?}"));
            }
            _ => {}
        }
        match (old.get("enum"), new.get("enum")) {
            (Some(o), Some(n)) => {
                let n = n.as_array().cloned().unwrap_or_default();
                let lost: Vec<&Value> = o
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|v| !n.contains(v))
                    .collect();
                // A value naming a node or edge type the release announced
                // going leaves the enum with it.
                let (going, lost): (Vec<&Value>, Vec<&Value>) = lost
                    .into_iter()
                    .partition(|v| self.announced.names_vocabulary(v));
                if !going.is_empty() {
                    self.verdict.announced.insert(format!(
                        "{}.{path}: allowed values of announced vocabulary taken away: {going:?}",
                        self.tool
                    ));
                }
                if !lost.is_empty() {
                    self.problem(path, format!("allowed values taken away: {lost:?}"));
                }
            }
            (None, Some(_)) => {
                if ENUM_ATTACHED
                    .iter()
                    .any(|(t, p, _)| *t == self.tool && *p == path)
                {
                    self.verdict.allowed.insert(format!("{}.{path}", self.tool));
                } else {
                    self.problem(
                        path,
                        "now publishes an enum where the release took any string — list it in \
                         ENUM_ATTACHED with the scenario step that shows the release refused a \
                         value outside the set"
                            .into(),
                    );
                }
            }
            _ => {}
        }
        for key in ["minimum", "minItems"] {
            if let (Some(o), Some(n)) = (
                old.get(key).and_then(Value::as_f64),
                new.get(key).and_then(Value::as_f64),
            ) && n > o
            {
                self.problem(path, format!("{key} raised from {o} to {n}"));
            } else if old.get(key).is_none() && new.get(key).is_some() {
                self.problem(path, format!("{key} added: {}", new[key]));
            }
        }
        if old.get("format") != new.get("format") && old.get("format").is_some() {
            self.problem(
                path,
                format!("format changed from {} to {}", old["format"], new["format"]),
            );
        }
        if new.get("additionalProperties") == Some(&Value::Bool(false))
            && old.get("additionalProperties") != Some(&Value::Bool(false))
            && old.get("properties").is_some()
        {
            self.problem(path, "an open object is closed".into());
        }

        let empty = Map::new();
        let old_props = old
            .get("properties")
            .and_then(Value::as_object)
            .unwrap_or(&empty);
        let new_props = new
            .get("properties")
            .and_then(Value::as_object)
            .unwrap_or(&empty);
        for (name, o) in old_props {
            let p = if path.is_empty() {
                name.clone()
            } else {
                format!("{path}.{name}")
            };
            match new_props.get(name) {
                Some(n) => self.compare(o, n, &p, depth + 1),
                None => {
                    if UNADVERTISED
                        .iter()
                        .any(|(t, f, ..)| *t == self.tool && *f == p)
                    {
                        self.verdict.allowed.insert(format!("{}.{p}", self.tool));
                    } else {
                        self.problem(&p, "no longer published".into());
                    }
                }
            }
        }
        let req = |s: &Value| -> BTreeSet<String> {
            s.get("required")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        };
        let newly: Vec<String> = req(new).difference(&req(old)).cloned().collect();
        if !newly.is_empty() {
            self.problem(path, format!("newly required: {newly:?}"));
        }
        if let (Some(o), Some(n)) = (old.get("items"), new.get("items")) {
            self.compare(o, n, &format!("{path}[]"), depth + 1);
        }
    }
}

/// The vocabulary the record holds, against this build's: a node type, a
/// property or an edge type that is gone is a break unless announced.
fn compare_vocabulary(record: &Value, now: &Value, announced: &Announced, verdict: &mut Verdict) {
    let Some(old) = record.get("vocabulary") else {
        return; // A record taken before the vocabulary was recorded holds none.
    };
    let Some(new) = now.get("vocabulary") else {
        verdict
            .problems
            .push("the record holds a vocabulary and this build's was not read".into());
        return;
    };
    let empty = Map::new();
    let new_types = new["node_types"].as_object().unwrap_or(&empty);
    for (t, props) in old["node_types"].as_object().unwrap_or(&empty) {
        let Some(now_props) = new_types.get(t) else {
            let line = format!("node type `{t}`: no longer served");
            if announced.node_type(t) {
                verdict.announced.insert(line);
            } else {
                verdict.problems.push(line);
            }
            continue;
        };
        for p in props.as_array().into_iter().flatten() {
            let held = now_props
                .as_array()
                .is_some_and(|a| a.iter().any(|x| x == p));
            if !held {
                let p = p.as_str().unwrap_or_default();
                let line = format!("node type `{t}`: property `{p}` no longer served");
                if announced.property(t, p) {
                    verdict.announced.insert(line);
                } else {
                    verdict.problems.push(line);
                }
            }
        }
    }
    let new_edges: BTreeSet<&str> = new["edge_types"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    for e in old["edge_types"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        if !new_edges.contains(e) {
            let line = format!("edge type `{e}`: no longer served");
            if announced.edge_type(e) {
                verdict.announced.insert(line);
            } else {
                verdict.problems.push(line);
            }
        }
    }
}

fn compare(record: &Value, now: &Value) -> Verdict {
    let announced = Announced::from_record(record);
    let mut verdict = Verdict::default();
    let old_tools = record["tools"].as_object().expect("record tools");
    let new_tools = now["tools"].as_object().expect("live tools");
    for (name, old) in old_tools {
        let Some(new) = new_tools.get(name) else {
            match announced.tool(name) {
                Some(d) => {
                    verdict
                        .announced
                        .insert(format!("{name}: no longer served"));
                    verdict.stubs.push((
                        name.clone(),
                        d.deprecated_in.to_string(),
                        d.what_to_do.to_string(),
                    ));
                }
                None => verdict.problems.push(format!("{name}: no longer served")),
            }
            continue;
        };
        if old["read_only"] != new["read_only"] {
            let line = format!(
                "{name}: read_only {} became {} — `reflow2 read` approvals key on it",
                old["read_only"], new["read_only"]
            );
            if announced.tool(name).is_some() {
                verdict.announced.insert(line);
            } else {
                verdict.problems.push(line);
            }
        }
        let (o, n) = (&old["input_schema"], &new["input_schema"]);
        Schemas {
            old_root: o,
            new_root: n,
            tool: name,
            announced: &announced,
            verdict: &mut verdict,
        }
        .compare(o, n, "", 0);
    }

    compare_vocabulary(record, now, &announced, &mut verdict);

    let new_replies: BTreeMap<&str, &Value> = now["replies"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r["step"].as_str().unwrap(), r))
        .collect();
    for old in record["replies"].as_array().expect("record replies") {
        let step = old["step"].as_str().unwrap();
        let Some(new) = new_replies.get(step) else {
            verdict
                .problems
                .push(format!("scenario step `{step}` is no longer run"));
            continue;
        };
        let used = announced.used_by_step(old);
        if old["answered"] == true && new["answered"] != true {
            let line = format!(
                "`{step}`: the release answered this call and this build refuses it (exit {})",
                new["exit"]
            );
            if used.is_empty() {
                verdict.problems.push(line);
            } else {
                verdict.announced.insert(line);
            }
            continue;
        }
        if old["answered"] == true {
            let have: BTreeSet<&str> = new["shape"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(Value::as_str)
                .collect();
            let (going, lost): (Vec<&str>, Vec<&str>) = old["shape"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(Value::as_str)
                .filter(|f| !have.contains(f))
                .partition(|f| announced.reply_field(old, &used, f));
            if !going.is_empty() {
                verdict
                    .announced
                    .insert(format!("`{step}`: announced reply fields gone: {going:?}"));
            }
            if !lost.is_empty() {
                verdict.problems.push(format!(
                    "`{step}`: the reply lost fields the release sent (path:type): {lost:?}"
                ));
            }
        }
    }
    verdict
}

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

/// Call `tool` on `bin` in a fresh folder: whether it answered, and all it said.
fn call_fresh(bin: &str, tool: &str, args: &str) -> (bool, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let graph = dir.path().join("graph");
    let o = run(
        bin,
        dir.path(),
        &[
            "--graph-path",
            graph.to_str().unwrap(),
            "--no-export",
            "--call",
            tool,
            "--args",
            args,
        ],
    );
    (
        o.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ),
    )
}

#[cfg(feature = "rocksdb")]
#[test]
fn the_mcp_surface_shrinks_only_through_a_deprecating_release() {
    if let Ok(release) = std::env::var("REFLOW2_BLESS_SURFACE") {
        write_record(&take(&release));
    }
    let record: Value = serde_json::from_str(
        &std::fs::read_to_string(RECORD).expect("the record of the last release is committed"),
    )
    .expect("the record is JSON");
    let now = take(bin());
    let verdict = compare(&record, &now);
    assert!(
        verdict.problems.is_empty(),
        "this build takes away from the MCP surface of {} what that release did not announce:\n  \
         {}\n\nAn MCP client of the last release breaks on each of these. Additions only — \
         anything else goes through a release that deprecates it first (deprecation::REGISTRY), \
         or the owner's approval, named.",
        record["taken_with"],
        verdict.problems.join("\n  ")
    );
    if !verdict.announced.is_empty() {
        eprintln!(
            "removed after {} announced it:\n  {}",
            record["taken_with"],
            verdict
                .announced
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join("\n  ")
        );
    }

    // NOT VACUOUS: the walk reached the surface and the scenario ran, and the
    // record holds the vocabulary and says what it announced (an empty list
    // is an answer; a missing key is not).
    assert!(record["tools"].as_object().unwrap().len() >= 190);
    let answered = record["replies"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["answered"] == true)
        .count();
    assert!(
        answered >= 20,
        "only {answered} scenario calls were answered"
    );
    assert!(
        record["vocabulary"]["node_types"]
            .as_object()
            .is_some_and(|t| t.len() >= 28)
            && record["vocabulary"]["edge_types"]
                .as_array()
                .is_some_and(|e| e.len() >= 66),
        "the record holds the vocabulary its release served — re-bless from that release's binary"
    );
    assert!(
        record["deprecations"].is_array(),
        "the record says what its release announced, if only that it announced nothing"
    );

    // A TOOL THAT WENT STILL ANSWERS ITS OLD NAME, for the one release after
    // its notice, with a refusal saying what the release said to do.
    for (tool, version, what) in &verdict.stubs {
        let (answered, said) = call_fresh(bin(), tool, "{}");
        assert!(
            !answered && said.contains(&format!("DEPRECATED in {version}")) && said.contains(what),
            "`{tool}` was deprecated in {version} and is no longer served, so its old name must \
             be refused with what to do ({what:?}) for one release — a removed door is never a \
             silent absence. Got:\n{said}"
        );
    }

    // Every listed narrowing is real and carries its proof.
    for (tool, field, why, args, answer) in UNADVERTISED {
        if !verdict.allowed.contains(&format!("{tool}.{field}")) {
            continue; // Not yet taken out (a record taken after the cut).
        }
        let (_, said) = call_fresh(bin(), tool, args);
        assert!(
            said.contains(answer) && !said.contains("is not a parameter"),
            "`{tool}.{field}` is unadvertised ({why}) and must still be ACCEPTED with the \
             release's answer ({answer:?}); got:\n{said}"
        );
    }
    let replies = record["replies"].as_array().unwrap();
    for (tool, field, step) in ENUM_ATTACHED {
        if !verdict.allowed.contains(&format!("{tool}.{field}")) {
            continue;
        }
        let r = replies
            .iter()
            .find(|r| r["step"] == *step)
            .unwrap_or_else(|| panic!("ENUM_ATTACHED names step `{step}`, not in the scenario"));
        assert_eq!(
            r["answered"], false,
            "`{tool}.{field}`: the release ANSWERED a value outside the set (`{step}`), so \
             publishing the enum would refuse what it took"
        );
    }
}

/// One way of taking something away from a recorded surface.
type Breaks = Box<dyn Fn(&mut Value)>;

/// A hand-made surface: one tool with every kind of argument, one reply, and
/// a small vocabulary.
fn hand_made() -> Value {
    json!({
        "tools": {"t": {"read_only": false, "input_schema": {
            "type": "object",
            "properties": {
                "a": {"type": ["string", "null"]},
                "b": {"type": "string", "enum": ["x", "y"]},
                "c": {"type": "array", "items": {"$ref": "#/$defs/I"}},
                "r": {"type": "string", "enum": ["SATISFIES", "ANTICIPATES"]}
            },
            "required": ["a"],
            "$defs": {"I": {"type": "object", "properties": {"k": {"type": "integer"}}}}
        }}, "u": {"read_only": true, "input_schema": {}}},
        "replies": [{"step": "s", "tool": "t", "arguments": {"a": "q"}, "answered": true,
                     "exit": 0, "shape": ["count:number", "items:array", "items[].k:number"]},
                    {"step": "read an anchor", "tool": "u", "arguments": {},
                     "answered": true, "exit": 0,
                     "shape": ["node:object", "node.properties:object",
                               "node.properties.checksum:string"]}],
        "vocabulary": {"node_types": {"Anchor": ["name", "checksum"],
                                      "Requirement": ["name", "kind"]},
                       "edge_types": ["SATISFIES", "ANTICIPATES"]},
        "deprecations": []
    })
}

/// The comparison itself, on hand-made surfaces: a removal, a narrowed type,
/// a dropped value, a newly required field, a lost reply field and a lost
/// vocabulary item are each caught; an addition is not.
#[test]
fn the_comparison_catches_each_way_of_taking_away() {
    let record = hand_made();
    let v = compare(&record, &record);
    assert!(v.problems.is_empty(), "{:?}", v.problems);

    let mut grown = record.clone();
    grown["tools"]["t"]["input_schema"]["properties"]["z"] = json!({"type": "string"});
    grown["tools"]["t2"] = json!({"read_only": true, "input_schema": {}});
    grown["replies"][0]["shape"] = json!([
        "count:number",
        "items:array",
        "items[].k:number",
        "new:string"
    ]);
    grown["vocabulary"]["node_types"]["New"] = json!(["name"]);
    grown["vocabulary"]["edge_types"] = json!(["SATISFIES", "ANTICIPATES", "NEW_EDGE"]);
    let v = compare(&record, &grown);
    assert!(
        v.problems.is_empty(),
        "additions are allowed: {:?}",
        v.problems
    );

    for (needle, break_it) in breaks() {
        let mut broken = record.clone();
        break_it(&mut broken);
        let v = compare(&record, &broken);
        assert!(
            v.problems.iter().any(|x| x.contains(needle)),
            "the comparison missed `{needle}`: {:?}",
            v.problems
        );
    }
}

/// Every way of taking something away, each with the words its problem
/// carries.
fn breaks() -> Vec<(&'static str, Breaks)> {
    vec![
        (
            "t: no longer served",
            Box::new(|v| {
                v["tools"].as_object_mut().unwrap().remove("t");
            }),
        ),
        (
            "read_only",
            Box::new(|v| v["tools"]["t"]["read_only"] = json!(true)),
        ),
        (
            "no longer published",
            Box::new(|v| {
                v["tools"]["t"]["input_schema"]["properties"]
                    .as_object_mut()
                    .unwrap()
                    .remove("a");
            }),
        ),
        (
            "accepted types",
            Box::new(|v| {
                v["tools"]["t"]["input_schema"]["properties"]["a"]["type"] = json!("string")
            }),
        ),
        (
            "values taken away",
            Box::new(|v| v["tools"]["t"]["input_schema"]["properties"]["b"]["enum"] = json!(["x"])),
        ),
        (
            "values taken away: [String(\"ANTICIPATES\")]",
            Box::new(|v| {
                v["tools"]["t"]["input_schema"]["properties"]["r"]["enum"] = json!(["SATISFIES"])
            }),
        ),
        (
            "now publishes an enum",
            Box::new(|v| {
                v["tools"]["t"]["input_schema"]["properties"]["a"]["enum"] = json!(["only"])
            }),
        ),
        (
            "newly required",
            Box::new(|v| v["tools"]["t"]["input_schema"]["required"] = json!(["a", "b"])),
        ),
        (
            "accepted types",
            Box::new(|v| {
                v["tools"]["t"]["input_schema"]["$defs"]["I"]["properties"]["k"]["type"] =
                    json!("string")
            }),
        ),
        (
            "lost fields",
            Box::new(|v| v["replies"][0]["shape"] = json!(["count:number", "items:array"])),
        ),
        (
            "refuses it",
            Box::new(|v| v["replies"][0]["answered"] = json!(false)),
        ),
        (
            "node type `Anchor`: no longer served",
            Box::new(|v| {
                v["vocabulary"]["node_types"]
                    .as_object_mut()
                    .unwrap()
                    .remove("Anchor");
            }),
        ),
        (
            "property `kind` no longer served",
            Box::new(|v| v["vocabulary"]["node_types"]["Requirement"] = json!(["name"])),
        ),
        (
            "edge type `ANTICIPATES`: no longer served",
            Box::new(|v| v["vocabulary"]["edge_types"] = json!(["SATISFIES"])),
        ),
    ]
}

/// ⭐ THE RULE: a removal passes ONLY when the last release announced it. The
/// same removals that fail above pass once the record marks each item
/// deprecated, each is reported as announced rather than passed in silence,
/// and an item marked deprecated and still served passes too.
#[test]
fn a_removal_passes_only_when_the_last_release_announced_it() {
    let dec = "dec:test-only-a-retirement";
    let announced = |items: Vec<Deprecation>| {
        let mut r = hand_made();
        r["deprecations"] = serde_json::to_value(items).unwrap();
        r
    };

    // Each removal, and the deprecation that announces it.
    let cases: Vec<(&str, Deprecation, Breaks)> = vec![
        (
            "a tool",
            Deprecation::tool("t", "0.85.0", "Call t2.", dec),
            Box::new(|v| {
                v["tools"].as_object_mut().unwrap().remove("t");
                // Its scenario call is refused now, and that goes with it.
                v["replies"][0]["answered"] = json!(false);
            }),
        ),
        (
            "an argument",
            Deprecation::argument("t", "a", "0.85.0", "Leave it out.", dec),
            Box::new(|v| {
                v["tools"]["t"]["input_schema"]["properties"]
                    .as_object_mut()
                    .unwrap()
                    .remove("a");
                v["tools"]["t"]["input_schema"]["required"] = json!([]);
                // The step passed `a`, so this build refuses it.
                v["replies"][0]["answered"] = json!(false);
            }),
        ),
        (
            "an argument's inner field",
            Deprecation::argument("t", "c", "0.85.0", "Leave it out.", dec),
            Box::new(|v| {
                v["tools"]["t"]["input_schema"]["$defs"]["I"]["properties"]["k"]["type"] =
                    json!("string")
            }),
        ),
        (
            "a reply field",
            Deprecation::reply_field("t", "items", "0.85.0", "Read count.", dec),
            Box::new(|v| v["replies"][0]["shape"] = json!(["count:number"])),
        ),
        (
            "a node type",
            Deprecation::node_type("Anchor", "0.85.0", "Record a Snapshot.", dec),
            Box::new(|v| {
                v["vocabulary"]["node_types"]
                    .as_object_mut()
                    .unwrap()
                    .remove("Anchor");
            }),
        ),
        (
            "a node property",
            Deprecation::node_property("Requirement", "kind", "0.85.0", "Say it.", dec),
            Box::new(|v| v["vocabulary"]["node_types"]["Requirement"] = json!(["name"])),
        ),
        (
            "an edge type, and the enum value naming it",
            Deprecation::edge_type("ANTICIPATES", "0.85.0", "Draw ENABLES.", dec),
            Box::new(|v| {
                v["vocabulary"]["edge_types"] = json!(["SATISFIES"]);
                v["tools"]["t"]["input_schema"]["properties"]["r"]["enum"] = json!(["SATISFIES"]);
            }),
        ),
    ];
    for (what, deprecation, break_it) in cases {
        let mut broken = hand_made();
        break_it(&mut broken);
        let unannounced = compare(&hand_made(), &broken);
        assert!(
            !unannounced.problems.is_empty(),
            "removing {what} with no release of notice must fail"
        );
        let record = announced(vec![deprecation.clone()]);
        let v = compare(&record, &broken);
        assert!(
            v.problems.is_empty(),
            "removing {what} after the last release announced it ({deprecation:?}) must pass: \
             {:?}",
            v.problems
        );
        assert!(
            !v.announced.is_empty(),
            "the pass names what it let go, for {what}"
        );
        // Removal is allowed, not forced: still served passes as well.
        let kept = compare(&record, &hand_made());
        assert!(
            kept.problems.is_empty(),
            "{what} still served: {:?}",
            kept.problems
        );
    }

    // Announcing one thing excuses nothing else.
    let record = announced(vec![Deprecation::node_type(
        "Anchor",
        "0.85.0",
        "Record a Snapshot.",
        dec,
    )]);
    for (needle, break_it) in breaks() {
        if needle.contains("Anchor") {
            continue;
        }
        let mut broken = hand_made();
        break_it(&mut broken);
        let v = compare(&record, &broken);
        assert!(
            v.problems.iter().any(|x| x.contains(needle)),
            "announcing `Anchor` must not excuse `{needle}`: {:?}",
            v.problems
        );
    }

    // A tool that went must still answer by its old name (checked against the
    // binary in the main test): the comparison hands it over with what the
    // release said to do.
    let record = announced(vec![Deprecation::tool("t", "0.85.0", "Call t2.", dec)]);
    let mut gone = hand_made();
    gone["tools"].as_object_mut().unwrap().remove("t");
    gone["replies"][0]["answered"] = json!(false);
    let v = compare(&record, &gone);
    assert_eq!(
        v.stubs,
        vec![(
            "t".to_string(),
            "0.85.0".to_string(),
            "Call t2.".to_string()
        )]
    );

    // A reply the release itself said used a deprecated node type: its fields
    // may go with the type.
    let mut record = announced(vec![Deprecation::node_property(
        "Anchor",
        "checksum",
        "0.85.0",
        "Leave it out.",
        dec,
    )]);
    let mut lost = hand_made();
    lost["replies"][1]["shape"] = json!(["node:object", "node.properties:object"]);
    assert!(
        !compare(&record, &lost).problems.is_empty(),
        "a reply field the release did not say the call used is not excused by a property"
    );
    record["replies"][1]["uses"] =
        json!([{"kind": "node_property", "node_type": "Anchor", "property": "checksum"}]);
    let v = compare(&record, &lost);
    assert!(v.problems.is_empty(), "{:?}", v.problems);
}
