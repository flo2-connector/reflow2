//! Every argument refusal names the tool and the field path.
//!
//! `dec:idea-every-argument-refusal-names-the-tool-and-the-field-path`
//! (accepted 2026-10-02): a tool's arguments are checked against its PUBLISHED
//! input schema before anything deserialises them, so a wrong type, a missing
//! field and an unknown field are refused at any depth naming the tool, the
//! full path, what the schema expects there and the field's own description.
//!
//! # The class, measured on 0.77.0 before the fix
//!
//! - `external_dependency` with `components: "core"` answered `failed to
//!   deserialize parameters: invalid type: string "core", expected a sequence`
//!   — no tool, no field — and so did every tool with a typed parameter
//!   (`fact:root-cause-a-wrong-type-argument-is-refused-in-the-bare-serde-string-because-the-interception-matches-two-phrasings-2026-10-02`).
//! - `add_decision` with `related_to: [{other_id, relation}]` answered that
//!   `evidence` was missing and that "its own schema publishes no description
//!   of it" — false, the description sits one `$ref` away — and said nothing of
//!   `related_to[0]` (`fact:root-cause-a-missing-nested-field-refusal-says-the-schema-publishes-no-description-when-it-does-2026-10-02`).
//!
//! # Why these tests ask a SERVER
//!
//! The 2026-09-11 interception was dead for months because its test called a
//! pure function while rmcp delivered the refusal another way. Every class test
//! here is a call over an in-process MCP session, through `call_tool`, so what
//! is asserted is what a caller receives. `tools/refusal_speaks.py` asks the
//! real binary the same questions in CI.
//!
//! # The generated walk
//!
//! [`every_kind_of_argument_failure_on_every_tool_names_the_tool_and_the_path`]
//! is not a list of cases. It reads every served tool's published schema and
//! derives the probes from it — a wrong-typed value at every typed location, an
//! unknown key in every closed object, a value outside every enum, and an empty
//! item wherever an item has required fields — so the 196th tool joins it the
//! moment it is served.

use reflow2_mcp::service::{CALL_DOOR_CLIENT, ReflowService};
use rmcp::ServiceExt;
use serde_json::{Map, Value, json};

struct Client {
    name: &'static str,
}

impl rmcp::ClientHandler for Client {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = self.name.to_string();
        cfg
    }
}

struct Session {
    client: rmcp::service::RunningService<rmcp::service::RoleClient, Client>,
}

/// A served in-memory design, holding one Requirement so a reference can name
/// something real.
async fn session_as(name: &'static str) -> Session {
    let svc = ReflowService::in_memory().expect("in-memory service");
    let (server_rx, client_tx) = tokio::io::duplex(1 << 22);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 22);
    tokio::spawn(async move {
        if let Ok(running) = svc.serve((server_rx, server_tx)).await {
            let _ = running.waiting().await;
        }
    });
    let client = Client { name }
        .serve((client_rx, client_tx))
        .await
        .expect("in-process handshake");
    let s = Session { client };
    s.call(
        "add_requirement",
        json!({"id": ANCHOR, "name": "Anchor", "statement": "A node a reference can name."}),
    )
    .await
    .expect("the anchor is written");
    s
}

async fn session() -> Session {
    session_as("argument-refusal-probe").await
}

const ANCHOR: &str = "req:anchor";

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

    async fn refused(&self, tool: &str, args: Value) -> String {
        match self.call(tool, args.clone()).await {
            Err(text) => text,
            Ok(reply) => panic!("`{tool}` {args} was NOT refused: {reply}"),
        }
    }

    async fn tools(&self) -> Vec<rmcp::model::Tool> {
        self.client.list_all_tools().await.expect("tools/list")
    }
}

/// The checks every argument refusal must pass: the tool by name, the path,
/// and not the bare deserialiser string.
fn names_tool_and_path(text: &str, tool: &str, path: &str) {
    assert!(
        !text.starts_with("failed to deserialize parameters"),
        "`{tool}` answered with the bare deserialiser string:\n{text}"
    );
    assert!(
        text.contains(&format!("`{tool}`")),
        "the refusal does not name the TOOL `{tool}`:\n{text}"
    );
    assert!(
        text.contains(&format!("`{path}`")),
        "the refusal does not name the PATH `{path}`:\n{text}"
    );
}

// ---- one failing-first test per class -----------------------------------------

/// The reporter's call, and a scalar on a second tool.
#[tokio::test]
async fn a_wrong_type_at_the_top_level_names_the_tool_the_field_and_the_type() {
    let s = session().await;
    let text = s
        .refused(
            "external_dependency",
            json!({"name": "serde", "components": "core"}),
        )
        .await;
    names_tool_and_path(&text, "external_dependency", "components");
    assert!(
        text.contains("a list"),
        "the expected TYPE is not said:\n{text}"
    );
    assert!(
        text.contains("\"core\""),
        "what was sent is not shown:\n{text}"
    );

    let text = s
        .refused(
            "add_requirement",
            json!({"id": "req:typed", "name": 42, "statement": "s"}),
        )
        .await;
    names_tool_and_path(&text, "add_requirement", "name");
    assert!(text.contains("a string"), "{text}");
}

/// A wrong type INSIDE an item, named by its full path.
#[tokio::test]
async fn a_wrong_type_inside_an_item_names_the_full_path() {
    let s = session().await;
    let text = s
        .refused(
            "add_decision",
            json!({"id": "dec:typed", "name": "N", "decision": "D",
                   "related_to": [{"other_id": 5, "relation": "DEPENDS_ON", "evidence": "e"}]}),
        )
        .await;
    names_tool_and_path(&text, "add_decision", "related_to[0].other_id");
    assert!(text.contains("a string"), "{text}");
}

/// The measured `add_decision` case: a missing NESTED field is named with its
/// path and quoted from its own description, which the schema does publish.
#[tokio::test]
async fn a_missing_nested_field_is_named_with_its_path_and_its_description() {
    let s = session().await;
    let text = s
        .refused(
            "add_decision",
            json!({"id": "dec:nested", "name": "N", "decision": "D",
                   "related_to": [{"other_id": ANCHOR, "relation": "DEPENDS_ON"}]}),
        )
        .await;
    names_tool_and_path(&text, "add_decision", "related_to[0].evidence");
    assert!(
        text.contains("WHY this relation is true"),
        "the nested field's own published description is not quoted:\n{text}"
    );
    assert!(
        !text.contains("publishes no description"),
        "the refusal claims no description where the schema has one:\n{text}"
    );
}

/// With the call's arguments in hand the refusal names exactly what THIS call
/// lacked and sets apart what it passed. On 0.77.0 that listing existed only in
/// an interception arm rmcp 3 never reaches: on the wire the refusal listed
/// the whole obligation instead (measured through `--call` on 2026-10-02).
#[tokio::test]
async fn the_refusal_names_what_this_call_lacked_and_sets_apart_what_it_passed() {
    let s = session().await;
    let text = s
        .refused("set_requirement_status", json!({"status": "accepted"}))
        .await;
    names_tool_and_path(&text, "set_requirement_status", "requirement_id");
    assert!(
        !text.contains("`status` is required"),
        "a field this call passed is listed as missing:\n{text}"
    );
    let (_, passed) = text
        .split_once("Already passed")
        .unwrap_or_else(|| panic!("what the call passed is not set apart:\n{text}"));
    assert!(passed.contains("`status`"), "{text}");
}

/// An unknown key inside an item is refused with its path, and the legal names
/// THERE are listed.
#[tokio::test]
async fn an_unknown_field_inside_an_item_names_its_path() {
    let s = session().await;
    let text = s
        .refused(
            "create_edges",
            json!({"edges": [{"edge_type": "DEPENDS_ON", "from_id": ANCHOR, "to_id": ANCHOR,
                              "bogus": 1}]}),
        )
        .await;
    names_tool_and_path(&text, "create_edges", "edges[0].bogus");
    assert!(
        text.contains("`from_id`"),
        "the names it takes are not listed:\n{text}"
    );
}

/// A value outside the published set names every value in it — and the call
/// is refused before it ran, so it wrote nothing.
#[tokio::test]
async fn a_value_outside_the_published_set_names_the_set_and_writes_nothing() {
    let s = session().await;
    let text = s
        .refused(
            "set_requirement_status",
            json!({"requirement_id": ANCHOR, "status": "bogus"}),
        )
        .await;
    names_tool_and_path(&text, "set_requirement_status", "status");
    for v in ["proposed", "accepted", "deferred", "dropped", "met"] {
        assert!(
            text.contains(&format!("`{v}`")),
            "allowed value {v} is not named:\n{text}"
        );
    }
    assert!(text.contains("\"bogus\""), "{text}");

    // NOTHING WAS WRITTEN — the refusal says so, and this is what makes it true.
    let text = s
        .refused(
            "add_requirement",
            json!({"id": "req:never", "name": 42, "statement": "s"}),
        )
        .await;
    assert!(text.contains("nothing was read or written"), "{text}");
    let node = s
        .call("get_node", json!({"id": "req:never"}))
        .await
        .expect("get_node reads");
    assert_eq!(
        node["node"],
        Value::Null,
        "a refused call stored a node: {node}"
    );
}

/// `set_artifact_checksums`: the item's `disposition` lives in `$defs`, and its
/// three values are in its published description. The FIRST refusal names
/// them — the report said they appeared only after a wrong one.
#[tokio::test]
async fn a_defs_item_field_is_described_on_the_first_refusal() {
    let s = session().await;
    let text = s
        .refused(
            "set_artifact_checksums",
            json!({"accepts": [{"artifact_id": "art:x", "checksum": "abc"}]}),
        )
        .await;
    names_tool_and_path(&text, "set_artifact_checksums", "accepts[0].disposition");
    for v in ["design_holds", "design_updated", "baseline_established"] {
        assert!(
            text.contains(v),
            "{v} is not named on the first refusal:\n{text}"
        );
    }
}

/// The advice fits the transport: a session may hold a stale tool list and is
/// told to reconnect; the `--call` door reads the schema from its own binary
/// on every call and is told that instead.
#[tokio::test]
async fn the_reconnect_advice_reaches_a_session_and_never_the_call_door() {
    let s = session().await;
    let text = s
        .refused("loop_status", json!({"zz_no_such_field": 1}))
        .await;
    names_tool_and_path(&text, "loop_status", "zz_no_such_field");
    assert!(
        text.contains("Reconnect"),
        "a session is not told it may be stale:\n{text}"
    );

    let door = session_as(CALL_DOOR_CLIENT).await;
    let text = door
        .refused("loop_status", json!({"zz_no_such_field": 1}))
        .await;
    names_tool_and_path(&text, "loop_status", "zz_no_such_field");
    assert!(
        !text.contains("Reconnect"),
        "the --call door is told to reconnect, which means nothing there:\n{text}"
    );
}

/// The aliases are a courtesy the check must keep: the typed spelling is the
/// only one published, and `id` / `node_id` / `properties` still work.
#[tokio::test]
async fn an_alias_the_schema_does_not_publish_is_still_accepted() {
    let s = session().await;
    s.call("get_node", json!({"node_id": ANCHOR}))
        .await
        .expect("get_node takes node_id");
    s.call(
        "set_requirement_status",
        json!({"id": ANCHOR, "status": "proposed"}),
    )
    .await
    .expect("set_requirement_status takes id");
    s.call(
        "add_requirement",
        json!({"id": "req:other", "name": "Other", "statement": "Another node."}),
    )
    .await
    .expect("second node");
    s.call(
        "create_edges",
        json!({"edges": [{"edge_type": "DEPENDS_ON", "from_id": ANCHOR, "to_id": "req:other",
                          "properties": {"evidence": "measured"}}]}),
    )
    .await
    .expect("create_edges items take `properties` for `props`");
}

/// A tool that takes no arguments says so, and refuses one instead of
/// dropping it silently.
#[tokio::test]
async fn a_tool_that_takes_no_arguments_refuses_one() {
    let s = session().await;
    let text = s.refused("mirrors", json!({"project_id": "proj:x"})).await;
    names_tool_and_path(&text, "mirrors", "project_id");
    assert!(text.contains("it takes none"), "{text}");
}

/// The surface served before any design exists checks arguments the same way:
/// one validator, every surface.
#[tokio::test]
async fn the_latent_surface_refuses_arguments_the_same_way() {
    let dir = tempfile::tempdir().expect("scratch");
    let graph = dir.path().join(".reflow2").join("graph");
    let svc = reflow2_mcp::latent::LatentService::new(graph.display().to_string());
    let (server_rx, client_tx) = tokio::io::duplex(1 << 20);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 20);
    tokio::spawn(async move {
        if let Ok(running) = svc.serve((server_rx, server_tx)).await {
            let _ = running.waiting().await;
        }
    });
    let client = Client {
        name: "argument-refusal-probe",
    }
    .serve((client_rx, client_tx))
    .await
    .expect("handshake");
    let s = Session { client };
    let text = s
        .refused("describe_designs", json!({"paths": "/repo/.reflow2/graph"}))
        .await;
    names_tool_and_path(&text, "describe_designs", "paths");
    assert!(text.contains("a list"), "{text}");
}

// ---- the generated walk ---------------------------------------------------------

/// One probe: the arguments to send, the path the refusal must name, and the
/// description (opening words) it must quote, if the schema has one.
struct Probe {
    what: &'static str,
    args: Value,
    path: String,
    described: Option<String>,
}

/// Put `leaf` at `steps` inside an otherwise empty argument object.
fn place(steps: &[Step], leaf: Value) -> Value {
    let mut v = leaf;
    for s in steps.iter().rev() {
        v = match s {
            Step::Key(k) => {
                let mut m = Map::new();
                m.insert(k.clone(), v);
                Value::Object(m)
            }
            Step::Index => Value::Array(vec![v]),
        };
    }
    v
}

#[derive(Clone)]
enum Step {
    Key(String),
    Index,
}

fn path_of(steps: &[Step]) -> String {
    let mut out = String::new();
    for s in steps {
        match s {
            Step::Key(k) => {
                if !out.is_empty() {
                    out.push('.');
                }
                out.push_str(k);
            }
            Step::Index => out.push_str("[0]"),
        }
    }
    out
}

fn resolve<'a>(root: &'a Value, mut s: &'a Value) -> &'a Value {
    for _ in 0..16 {
        match s.get("$ref").and_then(Value::as_str) {
            Some(r) => match r
                .strip_prefix("#/$defs/")
                .and_then(|n| root["$defs"].get(n))
            {
                Some(t) => s = t,
                None => break,
            },
            None => break,
        }
    }
    s
}

fn types(s: &Value) -> Vec<String> {
    match &s["type"] {
        Value::String(t) => vec![t.clone()],
        Value::Array(ts) => ts
            .iter()
            .filter_map(|t| t.as_str().map(String::from))
            .collect(),
        _ => vec![],
    }
}

fn opening(root: &Value, s: &Value) -> Option<String> {
    let d = s["description"]
        .as_str()
        .or_else(|| resolve(root, s)["description"].as_str())?;
    let words: Vec<&str> = d.split_whitespace().take(4).collect();
    (!words.is_empty()).then(|| words.join(" "))
}

/// A value whose JSON type the schema does not allow here, if there is one.
/// `7.5` is a number and not an integer, so it probes an integer field too.
fn wrong_typed(allowed: &[String]) -> Option<Value> {
    if allowed.is_empty() {
        return None;
    }
    let fits = |t: &str| allowed.iter().any(|a| a == t);
    [
        ("string", json!("zz-wrong-type")),
        ("number", json!(7.5)),
        ("boolean", json!(true)),
        ("array", json!([])),
        ("object", json!({})),
    ]
    .into_iter()
    .find(|(t, _)| !fits(t))
    .map(|(_, v)| v)
}

fn probes(root: &Value) -> Vec<Probe> {
    let mut out = Vec::new();
    walk(root, root, &mut Vec::new(), &mut out, 0);
    out
}

fn walk(root: &Value, schema: &Value, at: &mut Vec<Step>, out: &mut Vec<Probe>, depth: usize) {
    if depth > 6 {
        return;
    }
    let s = resolve(root, schema);
    let ts = types(s);
    // A wrong type here — not at the root, which is always an object.
    if !at.is_empty()
        && let Some(bad) = wrong_typed(&ts)
    {
        out.push(Probe {
            what: "wrong type",
            args: place(at, bad),
            path: path_of(at),
            described: opening(root, schema),
        });
    }
    // A value outside the published set.
    if s["enum"].is_array() && !at.is_empty() && ts.iter().any(|t| t == "string") {
        out.push(Probe {
            what: "outside the enum",
            args: place(at, json!("zz-not-a-value")),
            path: path_of(at),
            described: opening(root, schema),
        });
    }
    if ts.iter().any(|t| t == "object") || s.get("properties").is_some() {
        if s["additionalProperties"] == json!(false) {
            at.push(Step::Key("zz_no_such_field".into()));
            out.push(Probe {
                what: "unknown field",
                args: place(at, json!(1)),
                path: path_of(at),
                described: None,
            });
            at.pop();
        }
        if !at.is_empty() {
            for r in s["required"].as_array().into_iter().flatten() {
                let Some(r) = r.as_str() else { continue };
                at.push(Step::Key(r.to_string()));
                let field = &s["properties"][r];
                out.push(Probe {
                    what: "missing nested field",
                    // The containing object present and empty.
                    args: place(&at[..at.len() - 1], json!({})),
                    path: path_of(at),
                    described: opening(root, field),
                });
                at.pop();
            }
        }
        if let Some(props) = s["properties"].as_object() {
            for (name, sub) in props {
                at.push(Step::Key(name.clone()));
                walk(root, sub, at, out, depth + 1);
                at.pop();
            }
        }
    }
    if ts.iter().any(|t| t == "array")
        && let Some(items) = s.get("items")
    {
        at.push(Step::Index);
        walk(root, items, at, out, depth + 1);
        at.pop();
    }
}

/// THE GENERATED WALK: every kind of argument failure the published schemas
/// can express, at every location, on every served tool.
#[tokio::test]
async fn every_kind_of_argument_failure_on_every_tool_names_the_tool_and_the_path() {
    let s = session().await;
    let tools = s.tools().await;
    assert!(tools.len() >= 190, "the surface shrank to {}", tools.len());

    let mut failures = Vec::new();
    let mut probed = 0usize;
    let mut kinds: std::collections::BTreeMap<&str, usize> = Default::default();
    for t in &tools {
        let root = serde_json::to_value(&*t.input_schema).expect("schema");
        let name = t.name.as_ref();

        // Every required top-level field, from one empty call.
        let required: Vec<String> = root["required"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        if !required.is_empty() {
            probed += 1;
            *kinds.entry("missing field").or_default() += 1;
            match s.call(name, json!({})).await {
                Ok(_) => failures.push(format!("{name} {{}}: NOT refused")),
                Err(text) => {
                    for r in &required {
                        if !text.contains(&format!("`{r}`")) || !text.contains(&format!("`{name}`"))
                        {
                            failures.push(format!(
                                "{name} {{}}: does not name the tool and `{r}`: {}",
                                &text[..text.len().min(200)]
                            ));
                        }
                    }
                }
            }
        }

        for p in probes(&root) {
            probed += 1;
            *kinds.entry(p.what).or_default() += 1;
            match s.call(name, p.args.clone()).await {
                Ok(_) => failures.push(format!("{name} {} [{}]: NOT refused", p.args, p.what)),
                Err(text) => {
                    let ok = !text.starts_with("failed to deserialize parameters")
                        && text.contains(&format!("`{name}`"))
                        && text.contains(&format!("`{}`", p.path));
                    if !ok {
                        failures.push(format!(
                            "{name} {} [{}]: refusal does not name the tool and `{}`: {}",
                            p.args,
                            p.what,
                            p.path,
                            &text[..text.len().min(240)]
                        ));
                        continue;
                    }
                    if p.what != "unknown field" {
                        let quoted = match &p.described {
                            Some(words) => text.contains(words.as_str()),
                            None => text.contains("publishes no description"),
                        };
                        if !quoted {
                            failures.push(format!(
                                "{name} `{}` [{}]: the refusal does not quote the field's \
                                 description ({:?}) or say it has none: {}",
                                p.path,
                                p.what,
                                p.described,
                                &text[..text.len().min(240)]
                            ));
                        }
                    }
                }
            }
        }
    }
    assert!(
        probed > 1000,
        "the walk probed only {probed} cases ({kinds:?}) — it is not reaching the surface"
    );
    assert!(
        failures.is_empty(),
        "{} of {probed} argument refusals do not name the tool and the path ({kinds:?}):\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}
