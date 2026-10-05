//! The tool surface tells the truth: stage 2 of the work-door increment.
//!
//! Each test below pins one finding from the VS Code field log
//! (`art:vscode-call-field-log-2026-10-05`), triaged into
//! `docs/design/reflow2/` on 2026-10-05, and each FAILED on main (7f5984e)
//! before its fix. Approved by the owner on 2026-10-05 ("Concur with 1 and 2"),
//! then narrowed the same day to five items: `--args @file` (1), create-or-
//! revise said where it is read, and a node type's constructor found by the
//! type's name (2), value sets published as enums (4), the
//! ChangeEvent `description` decoy un-advertised (6), and edge-aware revision
//! receipts (8). The section numbers below are those items'.
//!
//! ⚠️ EVERY CHANGE HERE IS ADDITIVE for an MCP client — the owner's
//! constraint, verbatim: "I just want to make sure the changes we implement
//! for vscode don't break it for how it was originally used/designed as an mcp
//! server." No tool, argument or reply field is renamed, removed or re-meant,
//! and an argument that was accepted stays accepted. The standing pin for that
//! is `the_mcp_surface_only_grows.rs`; the tests here also assert the old
//! half of each reply where a fix sits beside it (`changed`, `count`).
//!
//! Driven over a served session (the path a client takes: the argument check,
//! the receipt, the refusal words) and, for the shell door, over the real
//! binary.

use std::process::{Command, Output, Stdio};

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

// ---- a served session ----------------------------------------------------------

struct Client;
impl rmcp::ClientHandler for Client {}

struct Session {
    client: rmcp::service::RunningService<rmcp::service::RoleClient, Client>,
}

async fn session() -> Session {
    let svc = ReflowService::in_memory().expect("in-memory service");
    let (server_rx, client_tx) = tokio::io::duplex(1 << 22);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 22);
    tokio::spawn(async move {
        if let Ok(running) = svc.serve((server_rx, server_tx)).await {
            let _ = running.waiting().await;
        }
    });
    let client = Client
        .serve((client_rx, client_tx))
        .await
        .expect("in-process handshake");
    Session { client }
}

impl Session {
    /// The reply, or the words of the refusal — an `isError` result and a
    /// JSON-RPC error alike, because a caller reads both.
    async fn call(&self, tool: &str, args: Value) -> Result<Value, String> {
        let arguments = args.as_object().cloned().unwrap_or_default();
        match self
            .client
            .call_tool(
                rmcp::model::CallToolRequestParams::new(tool.to_string()).with_arguments(arguments),
            )
            .await
        {
            Ok(r) if r.is_error.unwrap_or(false) => Err(r
                .content
                .iter()
                .filter_map(|c| c.as_text().map(|t| t.text.clone()))
                .collect::<Vec<_>>()
                .join("\n")),
            Ok(r) => Ok(r.structured_content.unwrap_or(Value::Null)),
            Err(rmcp::service::ServiceError::McpError(e)) => Err(e.message.to_string()),
            Err(e) => Err(format!("transport: {e}")),
        }
    }

    async fn ok(&self, tool: &str, args: Value) -> Value {
        match self.call(tool, args.clone()).await {
            Ok(v) => v,
            Err(e) => panic!("`{tool}` {args} was refused:\n{e}"),
        }
    }

    async fn refused(&self, tool: &str, args: Value) -> String {
        match self.call(tool, args.clone()).await {
            Err(text) => text,
            Ok(reply) => panic!("`{tool}` {args} was NOT refused: {reply}"),
        }
    }

    async fn tools(&self) -> Vec<rmcp::model::Tool> {
        self.client.list_all_tools().await.expect("tools/list")
    }

    async fn find(&self, query: &str, limit: usize) -> Value {
        self.ok("find_tools", json!({"query": query, "limit": limit}))
            .await
    }
}

fn find_item<'a>(reply: &'a Value, tool: &str) -> &'a Value {
    reply["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|i| i["tool"] == tool)
        .unwrap_or_else(|| panic!("find_tools did not return `{tool}`: {reply}"))
}

fn prop<'a>(tools: &'a [rmcp::model::Tool], tool: &str) -> &'a serde_json::Map<String, Value> {
    &tools
        .iter()
        .find(|t| t.name == tool)
        .unwrap_or_else(|| panic!("`{tool}` is served"))
        .input_schema
}

// ---- the shell door ----------------------------------------------------------------

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

/// One door run in a scratch HOME, so no machine setup reaches it.
fn door(dir: &std::path::Path, args: &[&str]) -> Output {
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();
    Command::new(bin())
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

fn stdout_json(o: &Output) -> Value {
    let text = String::from_utf8_lossy(&o.stdout);
    serde_json::from_str(&text).unwrap_or_else(|e| {
        panic!(
            "stdout is not one JSON document ({e}):\n{text}\nstderr:\n{}",
            String::from_utf8_lossy(&o.stderr)
        )
    })
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

// ---- 1. `--args @file` ---------------------------------------------------------------
// dec:idea-the-door-reads-its-arguments-from-a-named-file

#[test]
fn the_door_reads_its_arguments_from_a_named_file_for_call_read_and_write() {
    let dir = tempfile::tempdir().expect("tempdir");
    let graph = dir.path().join("graph");
    let graph = graph.to_str().unwrap();

    // Prose with every character a shell would trip on, which is the point:
    // it never passes through the command text.
    let prose = "It's \"quoted\", $HOME stays literal, `backticks` too; and a newline\nhere.";
    let project = dir.path().join("project.json");
    let name = "A project named in a file, with \"quotes\" and $HOME and `ticks`";
    std::fs::write(&project, json!({"id": "proj:f", "name": name}).to_string()).unwrap();
    let at = format!("@{}", project.display());
    let o = door(
        dir.path(),
        &[
            "--graph-path",
            graph,
            "--call",
            "add_project",
            "--args",
            &at,
        ],
    );
    assert!(o.status.success(), "--call --args @file:\n{}", stderr(&o));
    assert_eq!(stdout_json(&o)["node_id"], "proj:f");

    // `write`, with the flag.
    let req = dir.path().join("req.json");
    std::fs::write(
        &req,
        json!({"id": "req:f", "name": "From a file", "statement": prose}).to_string(),
    )
    .unwrap();
    let at = format!("@{}", req.display());
    let o = door(
        dir.path(),
        &[
            "--graph-path",
            graph,
            "write",
            "add_requirement",
            "--args",
            &at,
        ],
    );
    assert!(o.status.success(), "write --args @file:\n{}", stderr(&o));

    // `read`, positionally — and the prose came through byte for byte.
    let get = dir.path().join("get.json");
    std::fs::write(&get, r#"{"id": "req:f"}"#).unwrap();
    let at = format!("@{}", get.display());
    let o = door(
        dir.path(),
        &["--graph-path", graph, "read", "get_node", &at],
    );
    assert!(o.status.success(), "read TOOL @file:\n{}", stderr(&o));
    assert_eq!(stdout_json(&o)["node"]["properties"]["statement"], prose);
}

#[test]
fn a_named_file_that_is_missing_or_not_one_object_is_refused_plainly() {
    let dir = tempfile::tempdir().expect("tempdir");
    let graph = dir.path().join("graph");
    let graph_s = graph.to_str().unwrap();

    let missing = dir.path().join("nowhere.json");
    let at = format!("@{}", missing.display());
    let o = door(
        dir.path(),
        &[
            "--graph-path",
            graph_s,
            "--call",
            "add_project",
            "--args",
            &at,
        ],
    );
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    let err = stderr(&o);
    assert!(
        err.contains("nowhere.json") && err.contains("--args"),
        "a missing file is named, as the --args file it is:\n{err}"
    );
    assert!(!graph.exists(), "a refused argument file opened no store");

    let list = dir.path().join("list.json");
    std::fs::write(&list, r#"[{"id": "proj:x"}]"#).unwrap();
    let at = format!("@{}", list.display());
    let o = door(
        dir.path(),
        &[
            "--graph-path",
            graph_s,
            "--call",
            "add_project",
            "--args",
            &at,
        ],
    );
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    let err = stderr(&o);
    assert!(
        err.contains("list.json") && err.contains("one JSON object"),
        "a file holding a list is refused as not one object, naming the file:\n{err}"
    );

    let broken = dir.path().join("broken.json");
    std::fs::write(&broken, "{\"id\": ").unwrap();
    let at = format!("@{}", broken.display());
    let o = door(
        dir.path(),
        &[
            "--graph-path",
            graph_s,
            "--call",
            "add_project",
            "--args",
            &at,
        ],
    );
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    let err = stderr(&o);
    assert!(
        err.contains("broken.json") && err.contains("not JSON"),
        "a file that is not JSON is refused, naming the file:\n{err}"
    );
    assert!(!graph.exists(), "no refused argument file opened a store");
}

// ---- 2. create-or-revise is discoverable ----------------------------------------------
// fact:a-constructors-revise-role-never-reaches-the-find-tools-summary-so-update-an-existing-node-misses-it-2026-10-05
// dec:idea-an-agent-that-wants-to-update-a-node-finds-the-revise-path

/// The tools whose own text says a repeated id revises.
fn revising_constructors(tools: &[rmcp::model::Tool]) -> Vec<String> {
    tools
        .iter()
        .filter(|t| {
            let d = t.description.as_deref().unwrap_or("");
            let id = t
                .input_schema
                .get("properties")
                .and_then(|p| p.get("id"))
                .and_then(|p| p["description"].as_str())
                .unwrap_or("");
            d.contains("REQUIRED TO CREATE AND OPTIONAL TO REVISE") || id.contains("REVISES")
        })
        .map(|t| t.name.to_string())
        .collect()
}

#[tokio::test]
async fn every_constructor_that_revises_says_so_in_the_line_find_tools_shows() {
    let s = session().await;
    let tools = s.tools().await;
    let revisers = revising_constructors(&tools);
    assert!(
        revisers.len() >= 21,
        "the walk found only {} revising constructors: {revisers:?}",
        revisers.len()
    );
    let mut silent = Vec::new();
    for name in &revisers {
        let reply = s.find(name, 200).await;
        let summary = find_item(&reply, name)["summary"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        if !summary.starts_with("Create or revise") {
            silent.push(format!("{name}: {summary}"));
        }
    }
    assert!(
        silent.is_empty(),
        "these constructors revise on a repeated id and their find_tools line does not say so:\n  {}",
        silent.join("\n  ")
    );
}

#[tokio::test]
async fn a_guessed_update_tool_is_pointed_at_the_revise_path_over_a_session() {
    let s = session().await;
    for guess in ["update_node", "update", "set_node"] {
        let text = s.refused(guess, json!({"id": "req:x"})).await;
        assert!(
            text.contains(guess) && text.contains("same id") && text.contains("add_"),
            "`{guess}` is refused with the revise route:\n{text}"
        );
    }
}

#[test]
fn a_guessed_update_tool_is_pointed_at_the_revise_path_through_the_door() {
    let dir = tempfile::tempdir().expect("tempdir");
    let graph = dir.path().join("graph");
    let o = door(
        dir.path(),
        &[
            "--graph-path",
            graph.to_str().unwrap(),
            "--call",
            "update_node",
            "--args",
            r#"{"id":"art:x","status":"realized"}"#,
        ],
    );
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    let err = stderr(&o);
    assert!(
        err.contains("update_node") && err.contains("same id") && err.contains("add_artifact"),
        "the door names the revise route for a guessed setter:\n{err}"
    );
}

/// A query that names a node type as the schema spells it is answered first
/// with the constructor that creates it. Measured on 0.79.0 (2026-10-05):
/// record_finding was in no top 8 for the field agent's TemporalFact queries.
#[tokio::test]
async fn a_query_naming_a_node_type_ranks_its_constructor_first() {
    let s = session().await;
    for (query, constructor, ty) in [
        (
            "TemporalFact fact_type valid_from subject_id add fact",
            "record_finding",
            "TemporalFact",
        ),
        ("Decision", "add_decision", "Decision"),
    ] {
        let reply = s.find(query, 5).await;
        let first = &reply["items"][0];
        assert_eq!(first["tool"], constructor, "{query}: {reply}");
        assert_eq!(
            first["creates"], ty,
            "the item says which type it creates: {first}"
        );
    }
    // A lowercase word in an ordinary sentence is not a type's name.
    let reply = s
        .find("say which component is responsible for this capability", 5)
        .await;
    assert!(
        reply["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|i| i.get("creates").is_none()),
        "{reply}"
    );
}

/// A guessed tool name that names a node type is pointed at the tool that
/// creates it.
#[tokio::test]
async fn a_guessed_name_that_names_a_node_type_suggests_its_constructor() {
    let s = session().await;
    for guess in ["add_temporal_fact", "record_fact", "add_fact"] {
        let text = s.refused(guess, json!({"id": "fact:x"})).await;
        assert!(
            text.contains("`TemporalFact`") && text.contains("`record_finding`"),
            "`{guess}` is pointed at record_finding:\n{text}"
        );
    }
}

// ---- 4. value sets are published as enums --------------------------------------------------
// fact:a-value-set-published-without-an-enum-escapes-the-argument-check-so-every-one-is-listed-is-false-2026-10-05

fn enum_of(schema: &serde_json::Map<String, Value>, path: &[&str]) -> Vec<String> {
    let mut v = Value::Object(schema.clone());
    for step in path {
        v = v[*step].clone();
    }
    v["enum"]
        .as_array()
        .unwrap_or_else(|| panic!("{path:?} publishes no enum: {v}"))
        .iter()
        .filter_map(|x| x.as_str().map(String::from))
        .collect()
}

#[tokio::test]
async fn the_value_sets_the_field_met_publish_their_values() {
    let s = session().await;
    let tools = s.tools().await;
    let provenance = enum_of(
        prop(&tools, "add_requirement"),
        &["properties", "provenance"],
    );
    for v in [
        "authored",
        "imported",
        "inferred",
        "healed",
        "planned",
        "reconciled",
    ] {
        assert!(provenance.contains(&v.to_string()), "{provenance:?}");
    }
    let dispositions = ["design_holds", "design_updated", "baseline_established"];
    let one = enum_of(
        prop(&tools, "set_artifact_checksum"),
        &["properties", "disposition"],
    );
    let bulk = enum_of(
        prop(&tools, "set_artifact_checksums"),
        &["$defs", "ChecksumAcceptReq", "properties", "disposition"],
    );
    for set in [&one, &bulk] {
        let mut got = set.clone();
        got.sort();
        let mut want: Vec<String> = dispositions.iter().map(|d| d.to_string()).collect();
        want.sort();
        assert_eq!(
            got, want,
            "the disposition set is the one the handler enforces"
        );
    }
}

#[tokio::test]
async fn every_problem_is_listed_when_one_is_a_value_outside_its_set() {
    let s = session().await;
    // The reporter's shape: two problems, one of them a provenance the
    // schema's set does not hold. Both are named in ONE refusal now.
    let text = s
        .refused(
            "add_requirement",
            json!({"id": "req:p", "name": "P", "statement": "S",
                   "priority": "should", "provenance": "user"}),
        )
        .await;
    assert!(
        text.contains("`priority`") && text.contains("`provenance`"),
        "both problems are listed, as the refusal promises:\n{text}"
    );
    let text = s
        .refused(
            "set_artifact_checksums",
            json!({"accepts": [{"artifact_id": "art:x", "checksum": "abc",
                                 "disposition": "design_change"}], "bogus": 1}),
        )
        .await;
    assert!(
        text.contains("accepts[0].disposition") && text.contains("`bogus`"),
        "{text}"
    );
}

/// The guard, broadened: a value set written as prose, however it is
/// punctuated, publishes an enum or is exempt with the reason on record.
/// `tools/toolsnap.py`'s `enum_invariants` applies the same rule to the
/// committed goldens.
#[tokio::test]
async fn a_value_set_written_in_prose_publishes_an_enum_or_is_exempt_with_a_reason() {
    let s = session().await;
    let tools = s.tools().await;
    let tool_names: std::collections::BTreeSet<String> =
        tools.iter().map(|t| t.name.to_string()).collect();
    let mut unpublished = Vec::new();
    for t in &tools {
        let root = Value::Object((*t.input_schema).clone());
        let mut names = std::collections::BTreeSet::new();
        property_names(&root, &mut names);
        let mut found = Vec::new();
        string_properties(&root, "", &mut found);
        for (path, p) in found {
            if p.get("enum").is_some() {
                continue;
            }
            let desc = p["description"].as_str().unwrap_or_default();
            let values: std::collections::BTreeSet<String> = quoted_tokens(desc)
                .into_iter()
                .filter(|v| !tool_names.contains(v) && !names.contains(v))
                .collect();
            if values.len() >= 3
                && !EXEMPT
                    .iter()
                    .any(|(tl, pa, _)| *tl == t.name && *pa == path)
            {
                unpublished.push(format!("{}.{path}: {values:?}", t.name));
            }
        }
    }
    assert!(
        unpublished.is_empty(),
        "these parameters list a set of values in prose and publish no enum, so the argument \
         check cannot list a value outside it:\n  {}",
        unpublished.join("\n  ")
    );
}

/// Each judged by reading its parameter: the values named are not its set.
const EXEMPT: &[(&str, &str, &str)] = &[
    (
        "add_component",
        "level",
        "an open ladder: the five are the DEFAULT rungs and Project.decomposition_levels adds others",
    ),
    (
        "seam_coverage",
        "altitude",
        "Component.level, an open ladder (dec:the-decomposition-ladder-is-open-not-a-fixed-enum)",
    ),
    (
        "export_graph",
        "path",
        "the list is the RESULT's `wrote` values, not the input path's",
    ),
    (
        "reconcile_dependencies",
        "$defs.ObservedDependencyDto.name",
        "examples of dependency names",
    ),
    (
        "record_finding",
        "fact_type",
        "TemporalFact.fact_type is free text in the schema; the values are examples",
    ),
    (
        "replace_text",
        "field",
        "any text property of the node; examples",
    ),
    (
        "set_interface_designation",
        "interface_id",
        "names `designation`'s values, which publishes its enum",
    ),
    (
        "set_requirement_lineage",
        "requirement_id",
        "names `lineage`'s values, which publishes its enum",
    ),
    (
        "recall_resolutions",
        "resolution_keys",
        "names the parts of a key, not values",
    ),
];

fn quoted_tokens(desc: &str) -> Vec<String> {
    // `x`, "x", 'x', “x”, ‘x’ — however the set is punctuated between them.
    let mut out = Vec::new();
    let chars: Vec<char> = desc.chars().collect();
    let opens = ['`', '"', '\'', '“', '‘'];
    let mut i = 0;
    while i < chars.len() {
        if opens.contains(&chars[i]) {
            let close = match chars[i] {
                '“' => '”',
                '‘' => '’',
                c => c,
            };
            let mut j = i + 1;
            while j < chars.len()
                && (chars[j].is_ascii_lowercase() || chars[j].is_ascii_digit() || chars[j] == '_')
            {
                j += 1;
            }
            if j > i + 1
                && j < chars.len()
                && chars[j] == close
                && chars[i + 1].is_ascii_lowercase()
            {
                let tok: String = chars[i + 1..j].iter().collect();
                if !matches!(tok.as_str(), "true" | "false" | "null") {
                    out.push(tok);
                }
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn property_names(v: &Value, out: &mut std::collections::BTreeSet<String>) {
    match v {
        Value::Object(m) => {
            if let Some(Value::Object(props)) = m.get("properties") {
                out.extend(props.keys().cloned());
            }
            for x in m.values() {
                property_names(x, out);
            }
        }
        Value::Array(a) => a.iter().for_each(|x| property_names(x, out)),
        _ => {}
    }
}

/// Every string-valued property (or list of strings) at any depth, with its
/// path as toolsnap.py writes it.
fn string_properties<'a>(v: &'a Value, path: &str, out: &mut Vec<(String, &'a Value)>) {
    let Some(m) = v.as_object() else { return };
    if let Some(Value::Object(props)) = m.get("properties") {
        for (name, sub) in props {
            let p = if path.is_empty() {
                name.clone()
            } else {
                format!("{path}.{name}")
            };
            let types: Vec<&str> = match &sub["type"] {
                Value::String(t) => vec![t.as_str()],
                Value::Array(ts) => ts.iter().filter_map(Value::as_str).collect(),
                _ => vec![],
            };
            let item_is_plain_string = sub["items"]["type"] == "string"
                && sub["items"].get("enum").is_none()
                && sub["items"].get("$ref").is_none();
            if types.contains(&"string") || (types.contains(&"array") && item_is_plain_string) {
                out.push((p.clone(), sub));
            }
            string_properties(sub, &p, out);
        }
    }
    if let Some(Value::Object(defs)) = m.get("$defs") {
        for (name, sub) in defs {
            let p = if path.is_empty() {
                format!("$defs.{name}")
            } else {
                format!("{path}.$defs.{name}")
            };
            string_properties(sub, &p, out);
        }
    }
}

// ---- 6. the ChangeEvent `description` decoy ----------------------------------------------
// fact:the-changeevent-description-decoy-is-listed-by-find-tools-and-passes-the-argument-check-2026-10-05

#[tokio::test]
async fn the_changeevent_decoy_is_not_advertised_and_is_still_redirected() {
    let s = session().await;
    let tools = s.tools().await;
    assert!(
        prop(&tools, "add_change_event")["properties"]
            .get("description")
            .is_none(),
        "tools/list no longer offers `description` on add_change_event"
    );
    let reply = s.find("record a change event", 10).await;
    let params = find_item(&reply, "add_change_event")["parameters"].clone();
    assert!(
        !params.as_array().unwrap().contains(&json!("description")),
        "find_tools no longer lists it: {params}"
    );

    // A client that sends it still gets the same redirect, naming the two
    // fields that ARE the prose.
    let text = s
        .refused(
            "add_change_event",
            json!({"id": "chg:x", "name": "X", "change_type": "new_feature",
                   "description": "what changed"}),
        )
        .await;
    assert!(
        text.contains("has no `description`")
            && text.contains("`summary`")
            && text.contains("`rationale`"),
        "{text}"
    );
    // And beside other problems it is LISTED with them, so "every one is
    // listed" is true.
    let text = s
        .refused(
            "add_change_event",
            json!({"id": "chg:y", "name": "Y", "change_type": "new_feature",
                   "description": "what changed", "subject": "code"}),
        )
        .await;
    assert!(
        text.contains("`subject`") && text.contains("has no `description`"),
        "{text}"
    );
}

// ---- 8. an edge-only revise says what moved ------------------------------------------------
// fact:the-four-call-decision-was-a-leaked-node-and-an-edge-only-revise-still-says-nothing-moved-2026-10-05

#[tokio::test]
async fn an_edge_only_revise_reports_the_edges_it_drew_beside_unchanged_properties() {
    let s = session().await;
    s.ok(
        "add_decision",
        json!({"id": "dec:base", "name": "Base", "decision": "the base choice"}),
    )
    .await;
    s.ok(
        "add_decision",
        json!({"id": "dec:q", "name": "Q", "decision": "the dependent choice"}),
    )
    .await;
    let v = s
        .ok(
            "add_decision",
            json!({"id": "dec:q", "related_to": [{"other_id": "dec:base", "relation": "BLOCKS",
                    "evidence": "q cannot settle before base"}]}),
        )
        .await;
    assert_eq!(v["edges_drawn"], json!(["dec:q BLOCKS dec:base"]), "{v}");
    let rev = &v["revision"];
    // `changed` keeps its meaning: the node's PROPERTIES did not change.
    assert_eq!(rev["changed"], false, "{v}");
    assert_eq!(
        rev["edges_changed"], 1,
        "the receipt counts the edge it drew: {v}"
    );
    let note = rev["note"].as_str().unwrap_or_default();
    assert!(
        !note.contains("nothing moved") && note.contains("edge"),
        "the note no longer says nothing moved beside a drawn edge: {note}"
    );

    // A true no-op still says so, and counts no edge.
    let v = s
        .ok(
            "add_decision",
            json!({"id": "dec:q", "related_to": [{"other_id": "dec:base", "relation": "BLOCKS",
                    "evidence": "q cannot settle before base"}]}),
        )
        .await;
    assert_eq!(v["revision"]["changed"], false, "{v}");
    assert_eq!(v["revision"]["edges_changed"], 0, "{v}");
    assert!(
        v["revision"]["note"]
            .as_str()
            .is_some_and(|n| n.contains("nothing moved")),
        "{v}"
    );
}
