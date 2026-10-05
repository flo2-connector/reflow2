//! THE MCP SURFACE ONLY GROWS: what a client of the last release sent and read
//! still works the same way on this build.
//!
//! # Why this exists — the owner's constraint, verbatim
//!
//! "I just want to make sure the changes we implement for vscode don't break
//! it for how it was originally used/designed as an mcp server. My brother
//! likes using as is." (Anthony, 2026-10-05, approving stage 2 of the work-door
//! increment.)
//!
//! The toolsnap goldens (`tools/toolsnaps/`) make every surface change a
//! reviewed diff, but they are re-blessed in the same pull request that
//! changes the surface, so they say THAT the surface moved and never WHETHER
//! it only grew. This compares the build against a frozen record of the LAST
//! RELEASE and allows additions only:
//!
//! - every tool the release served is served, with the same read/write hint;
//! - every argument it published is published at the same path, with no type
//!   taken away, no allowed value taken away, no new required field, no
//!   tighter minimum, and an open object not closed;
//! - every reply in a fixed scenario of ordinary calls keeps every field it
//!   had, at the same path and of the same JSON type, and no call the release
//!   answered is refused.
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
//! a real binary: `--list-tools --full` and the scenario through `--call`.
//! After a release is tagged, take it from that release's binary:
//!
//! ```text
//! REFLOW2_BLESS_SURFACE=target/release/reflow2-mcp \
//!     cargo test -p reflow2-mcp --test the_mcp_surface_only_grows
//! ```
//!
//! A deliberate break the owner approved is the only other reason to touch it,
//! and it goes in with the approval named in the commit.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::{Command, Output, Stdio};

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

/// The surface of one binary: its tools and the scenario's answers.
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
        if ok {
            let v: Value = serde_json::from_slice(&o.stdout).unwrap_or_else(|e| {
                panic!(
                    "{bin} `{label}`: stdout is not JSON ({e}): {}",
                    String::from_utf8_lossy(&o.stdout)
                )
            });
            shape(&v, "", &mut fields);
        }
        replies.push(json!({
            "step": label,
            "tool": tool,
            "arguments": args,
            "answered": ok,
            "exit": o.status.code(),
            "shape": fields.into_iter().collect::<Vec<_>>(),
        }));
    }
    json!({"taken_with": version, "tools": tools, "replies": replies})
}

/// The record as committed: one tool and one reply per line, so a re-bless
/// reads as a diff of what moved.
fn write_record(taken: &Value) {
    let mut s = String::from("{\n");
    s.push_str(&format!(
        "\"_what\": {},\n",
        json!(
            "The MCP surface of the LAST RELEASE, for tests/the_mcp_surface_only_grows.rs: each \
             tool's read/write hint and input schema with its prose taken out, and the reply \
             shape of a fixed scenario of ordinary calls. Taken by driving the release binary \
             (--list-tools --full, and --call). Re-bless at a cut: REFLOW2_BLESS_SURFACE=<that \
             release's binary> cargo test -p reflow2-mcp --test the_mcp_surface_only_grows."
        )
    ));
    s.push_str(&format!("\"taken_with\": {},\n", taken["taken_with"]));
    s.push_str("\"tools\": {\n");
    let tools = taken["tools"].as_object().unwrap();
    let n = tools.len();
    for (i, (name, t)) in tools.iter().enumerate() {
        s.push_str(&format!(
            "{}: {}{}\n",
            json!(name),
            t,
            if i + 1 < n { "," } else { "" }
        ));
    }
    s.push_str("},\n\"replies\": [\n");
    let replies = taken["replies"].as_array().unwrap();
    for (i, r) in replies.iter().enumerate() {
        s.push_str(&format!(
            "{}{}\n",
            r,
            if i + 1 < replies.len() { "," } else { "" }
        ));
    }
    s.push_str("]\n}\n");
    std::fs::write(RECORD, s).expect("write the record");
}

// ---- comparing ----------------------------------------------------------------------

struct Schemas<'a> {
    old_root: &'a Value,
    new_root: &'a Value,
    tool: &'a str,
    problems: &'a mut Vec<String>,
    allowed: &'a mut BTreeSet<String>,
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
    fn problem(&mut self, path: &str, what: String) {
        self.problems.push(format!("{}.{path}: {what}", self.tool));
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
                if !lost.is_empty() {
                    self.problem(path, format!("allowed values taken away: {lost:?}"));
                }
            }
            (None, Some(_)) => {
                if ENUM_ATTACHED
                    .iter()
                    .any(|(t, p, _)| *t == self.tool && *p == path)
                {
                    self.allowed.insert(format!("{}.{path}", self.tool));
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
                        self.allowed.insert(format!("{}.{p}", self.tool));
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

fn compare(record: &Value, now: &Value) -> (Vec<String>, BTreeSet<String>) {
    let mut problems = Vec::new();
    let mut allowed = BTreeSet::new();
    let old_tools = record["tools"].as_object().expect("record tools");
    let new_tools = now["tools"].as_object().expect("live tools");
    for (name, old) in old_tools {
        let Some(new) = new_tools.get(name) else {
            problems.push(format!("{name}: no longer served"));
            continue;
        };
        if old["read_only"] != new["read_only"] {
            problems.push(format!(
                "{name}: read_only {} became {} — `reflow2 read` approvals key on it",
                old["read_only"], new["read_only"]
            ));
        }
        let (o, n) = (&old["input_schema"], &new["input_schema"]);
        Schemas {
            old_root: o,
            new_root: n,
            tool: name,
            problems: &mut problems,
            allowed: &mut allowed,
        }
        .compare(o, n, "", 0);
    }

    let new_replies: BTreeMap<&str, &Value> = now["replies"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r["step"].as_str().unwrap(), r))
        .collect();
    for old in record["replies"].as_array().expect("record replies") {
        let step = old["step"].as_str().unwrap();
        let Some(new) = new_replies.get(step) else {
            problems.push(format!("scenario step `{step}` is no longer run"));
            continue;
        };
        if old["answered"] == true && new["answered"] != true {
            problems.push(format!(
                "`{step}`: the release answered this call and this build refuses it (exit {})",
                new["exit"]
            ));
            continue;
        }
        if old["answered"] == true {
            let have: BTreeSet<&str> = new["shape"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(Value::as_str)
                .collect();
            let lost: Vec<&str> = old["shape"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(Value::as_str)
                .filter(|f| !have.contains(f))
                .collect();
            if !lost.is_empty() {
                problems.push(format!(
                    "`{step}`: the reply lost fields the release sent (path:type): {lost:?}"
                ));
            }
        }
    }
    (problems, allowed)
}

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

#[test]
fn the_mcp_surface_only_grows_since_the_last_release() {
    if let Ok(release) = std::env::var("REFLOW2_BLESS_SURFACE") {
        write_record(&take(&release));
    }
    let record: Value = serde_json::from_str(
        &std::fs::read_to_string(RECORD).expect("the record of the last release is committed"),
    )
    .expect("the record is JSON");
    let now = take(bin());
    let (problems, allowed) = compare(&record, &now);
    assert!(
        problems.is_empty(),
        "this build takes away from the MCP surface of {}:\n  {}\n\nAn MCP client of the last \
         release breaks on each of these. Additions only — or the owner's approval, named.",
        record["taken_with"],
        problems.join("\n  ")
    );

    // NOT VACUOUS: the walk reached the surface and the scenario ran.
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

    // Every listed narrowing is real and carries its proof.
    for (tool, field, why, args, answer) in UNADVERTISED {
        if !allowed.contains(&format!("{tool}.{field}")) {
            continue; // Not yet taken out (a record taken after the cut).
        }
        let dir = tempfile::tempdir().expect("tempdir");
        let graph = dir.path().join("graph");
        let o = run(
            bin(),
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
        let said = format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        assert!(
            said.contains(answer) && !said.contains("is not a parameter"),
            "`{tool}.{field}` is unadvertised ({why}) and must still be ACCEPTED with the \
             release's answer ({answer:?}); got:\n{said}"
        );
    }
    let replies = record["replies"].as_array().unwrap();
    for (tool, field, step) in ENUM_ATTACHED {
        if !allowed.contains(&format!("{tool}.{field}")) {
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

/// The comparison itself, on hand-made surfaces: a removal, a narrowed type,
/// a dropped value, a newly required field and a lost reply field are each
/// caught; an addition is not.
#[test]
fn the_comparison_catches_each_way_of_taking_away() {
    let record = json!({
        "tools": {"t": {"read_only": false, "input_schema": {
            "type": "object",
            "properties": {
                "a": {"type": ["string", "null"]},
                "b": {"type": "string", "enum": ["x", "y"]},
                "c": {"type": "array", "items": {"$ref": "#/$defs/I"}}
            },
            "required": ["a"],
            "$defs": {"I": {"type": "object", "properties": {"k": {"type": "integer"}}}}
        }}},
        "replies": [{"step": "s", "answered": true, "exit": 0,
                     "shape": ["count:number", "items:array"]}]
    });
    let (p, _) = compare(&record, &record);
    assert!(p.is_empty(), "{p:?}");

    let mut grown = record.clone();
    grown["tools"]["t"]["input_schema"]["properties"]["z"] = json!({"type": "string"});
    grown["tools"]["t2"] = json!({"read_only": true, "input_schema": {}});
    grown["replies"][0]["shape"] = json!(["count:number", "items:array", "new:string"]);
    let (p, _) = compare(&record, &grown);
    assert!(p.is_empty(), "additions are allowed: {p:?}");

    let cases: Vec<(&str, Breaks)> = vec![
        (
            "no longer served",
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
            Box::new(|v| v["replies"][0]["shape"] = json!(["count:number"])),
        ),
        (
            "refuses it",
            Box::new(|v| v["replies"][0]["answered"] = json!(false)),
        ),
    ];
    for (needle, break_it) in cases {
        let mut broken = record.clone();
        break_it(&mut broken);
        let (p, _) = compare(&record, &broken);
        assert!(
            p.iter().any(|x| x.contains(needle)),
            "the comparison missed `{needle}`: {p:?}"
        );
    }
}
