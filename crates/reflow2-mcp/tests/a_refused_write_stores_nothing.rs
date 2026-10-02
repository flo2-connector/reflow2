//! A REFUSED WRITE STORES NOTHING — asked of every write tool the server
//! serves, and of each measured way a typed constructor used to leave half a
//! write behind.
//!
//! # The failure this pins
//!
//! `fact:root-cause-a-typed-constructor-writes-the-node-before-its-later-checks-and-a-refusal-leaves-it-2026-10-02`
//! (limitation 17 of the VS Code `--call` field report, measured on 0.77.0):
//! 13 typed constructors stored the node, then checked an optional enum, a link
//! item or an edge's endpoint type in a LATER write, with no batch around the
//! call. A refusal at that later check left the node behind, and three things
//! followed that the report had not named:
//! - on an existing id, `add_capability`'s refusal said "nothing was written"
//!   after it had overwritten the description;
//! - the corrected retry landed as a REVISE, so the duplicate guard and the
//!   relatedness refusal never ran;
//! - the leftover became a near-match that blocked the next, different create.
//!
//! # The cure (`dec:idea-a-refused-typed-write-stores-nothing`, accepted)
//!
//! ONE ATOMIC WRITE PER TOOL CALL, AT THE STORE'S SINGLE WRITE POINT. A served
//! write tool's every write is staged in one unit and committed once when the
//! handler answers, or discarded when it refuses. So these tests ask the
//! SERVED path — `call_tool`, through an in-process client — and never call a
//! handler directly, which is not a path any session takes.
//!
//! # Why the walk is generated, not listed
//!
//! The class is "a handler writes, then a later check refuses". A hand-kept
//! list of the 13 measured constructors is exactly the instance fix the
//! decision rejected: the next optional field added through `set_optional_props`
//! would reopen it with nothing noticing. So [`every_write_tool_stores_nothing_when_it_refuses`]
//! reads the served catalogue (`read_only_hint`, the predicate `--call` and the
//! receipt use), synthesises arguments from each tool's own input schema that
//! pass deserialisation and fail a later check, and asserts the stored design
//! is node-for-node and edge-for-edge identical after every refusal.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Map, Value, json};
use std::time::Duration;

struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "a-refused-write-stores-nothing".to_string();
        cfg.client_info.version = "test".to_string();
        cfg
    }
}

type Client = rmcp::service::RunningService<rmcp::RoleClient, TestClient>;

/// Serve `service` to an in-process client, the way a session reaches it.
async fn connect_to(service: ReflowService) -> Client {
    let (server_rx, client_tx) = tokio::io::duplex(1 << 22);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 22);
    tokio::spawn(async move {
        if let Ok(running) = service.serve((server_rx, server_tx)).await {
            let _ = running.waiting().await;
        }
    });
    TestClient
        .serve((client_rx, client_tx))
        .await
        .expect("the in-process handshake")
}

/// An in-memory design that never goes out over the network: the walk passes
/// made-up addresses, and a test must not reach for them.
async fn connect() -> Client {
    connect_to(
        ReflowService::in_memory()
            .expect("in-memory service")
            .without_reaching_out(),
    )
    .await
}

/// Call a tool by NAME through `call_tool`. `Ok(structured reply)` or
/// `Err(refusal text)` — an `Err` and an `isError` reply are both refusals.
async fn call(c: &Client, tool: &str, args: Value) -> Result<Value, String> {
    let Value::Object(arguments) = args else {
        panic!("arguments for {tool} must be an object");
    };
    let answer = tokio::time::timeout(
        Duration::from_secs(30),
        c.call_tool(CallToolRequestParams::new(tool.to_string()).with_arguments(arguments)),
    )
    .await
    .unwrap_or_else(|_| {
        panic!("{tool} did not answer within 30 s — a held write unit would look like this")
    });
    match answer {
        Ok(r) if r.is_error == Some(true) => Err(format!("{:?}", r.content)),
        Ok(r) => Ok(r.structured_content.unwrap_or(Value::Null)),
        Err(e) => Err(e.to_string()),
    }
}

async fn ok(c: &Client, tool: &str, args: Value) -> Value {
    call(c, tool, args)
        .await
        .unwrap_or_else(|e| panic!("{tool} refused: {e}"))
}

/// The stored design: every node and every edge, as `export_graph` (a READ)
/// returns them — sorted, so two reads of an unchanged store are identical.
async fn stored(c: &Client) -> (Value, Value) {
    let export = ok(c, "export_graph", json!({})).await;
    (export["nodes"].clone(), export["edges"].clone())
}

async fn node(c: &Client, id: &str) -> Value {
    ok(c, "get_node", json!({ "id": id })).await["node"].clone()
}

/// What changed between two reads of the store, named by id, so a failure
/// says WHAT a refusal left behind rather than only that something did.
fn difference(before: &(Value, Value), after: &(Value, Value)) -> String {
    fn keyed(v: &Value, key: fn(&Value) -> String) -> std::collections::BTreeMap<String, String> {
        v.as_array()
            .map(|a| a.iter().map(|x| (key(x), x.to_string())).collect())
            .unwrap_or_default()
    }
    let node_key = |n: &Value| format!("{} {}", n["node_type"], n["node_id"]);
    let edge_key = |e: &Value| format!("{} {} -> {}", e["edge_type"], e["from_id"], e["to_id"]);
    let mut out = Vec::new();
    for (what, b, a) in [
        (
            "node",
            keyed(&before.0, node_key),
            keyed(&after.0, node_key),
        ),
        (
            "edge",
            keyed(&before.1, edge_key),
            keyed(&after.1, edge_key),
        ),
    ] {
        for (k, v) in &a {
            match b.get(k) {
                None => out.push(format!("{what} ADDED {k}")),
                Some(old) if old != v => out.push(format!("{what} CHANGED {k}")),
                _ => {}
            }
        }
        for k in b.keys() {
            if !a.contains_key(k) {
                out.push(format!("{what} REMOVED {k}"));
            }
        }
    }
    out.join("; ")
}

/// Refuse with `args`, and prove the store did not move.
async fn refused_and_untouched(c: &Client, tool: &str, args: Value) -> String {
    let before = stored(c).await;
    let refusal = match call(c, tool, args).await {
        Err(e) => e,
        Ok(v) => panic!("{tool} was expected to refuse, and answered {v}"),
    };
    let after = stored(c).await;
    assert!(
        before == after,
        "{tool} REFUSED and still changed the stored design — {}\nthe refusal: {refusal}",
        difference(&before, &after)
    );
    refusal
}

/// A short paragraph long enough for the near-match check to judge (it
/// declines below twelve words), worded so two calls resemble each other.
const IDEA: &str = "Every refused write leaves the stored design exactly as it was before the \
                    call, so a corrected retry is judged as a brand new capture";

// ---- the measured classes, one failing-first case each ---------------------

/// CLASS 1 — A BAD ENUM CHECKED AFTER THE NODE WRITE. `priority` is written
/// by a later upsert, so a bad one used to be refused with the requirement
/// already stored.
#[tokio::test]
async fn a_bad_enum_after_the_node_write_stores_nothing() {
    let c = connect().await;
    ok(
        &c,
        "add_requirement",
        json!({"id": "req:already-here", "name": "Already here", "statement": "A node the design held before."}),
    )
    .await;
    refused_and_untouched(
        &c,
        "add_requirement",
        json!({
            "id": "req:urgent",
            "name": "Urgent",
            "statement": "A need whose priority is not one of the four.",
            "priority": "urgent-ish",
        }),
    )
    .await;
    assert!(
        node(&c, "req:urgent").await.is_null(),
        "the refused requirement must not exist afterwards"
    );
}

/// CLASS 2 — A BAD LINK ITEM. `related_to` naming a node that does not exist
/// was refused after the Decision was stored.
#[tokio::test]
async fn a_bad_link_item_stores_nothing() {
    let c = connect().await;
    refused_and_untouched(
        &c,
        "add_decision",
        json!({
            "id": "dec:probe-link",
            "name": "Probe link",
            "decision": "A decision whose relation names nothing.",
            "related_to": [{
                "relation": "DEPENDS_ON",
                "other_id": "dec:no-such-decision",
                "evidence": "named on purpose, so the link cannot resolve",
            }],
        }),
    )
    .await;
    assert!(node(&c, "dec:probe-link").await.is_null());
}

/// CLASS 3 — A BAD EDGE (a VERIFIES onto a type it cannot verify). The edge
/// is checked when it is drawn, after the Verification is stored.
#[tokio::test]
async fn a_bad_edge_stores_nothing() {
    let c = connect().await;
    ok(
        &c,
        "add_decision",
        json!({"id": "dec:a-ruling", "name": "A ruling", "decision": "Something was decided."}),
    )
    .await;
    refused_and_untouched(
        &c,
        "add_verification",
        json!({
            "id": "ver:checks-a-ruling",
            "name": "Checks a ruling",
            "verifies": [{"target_id": "dec:a-ruling"}],
        }),
    )
    .await;
    assert!(node(&c, "ver:checks-a-ruling").await.is_null());
}

/// A REFUSED REVISE KEEPS WHAT WAS THERE. `add_capability` on an existing id,
/// with a new description and a `satisfies` naming nothing, used to say
/// "nothing was written" after overwriting the description: the hand-written
/// rollback was guarded by `!existed`, so a revise kept the overwrite.
#[tokio::test]
async fn a_refused_revise_keeps_the_old_description() {
    let c = connect().await;
    ok(
        &c,
        "add_capability",
        json!({"id": "cap:keeps", "name": "Keeps", "description": "the description before"}),
    )
    .await;
    let refusal = refused_and_untouched(
        &c,
        "add_capability",
        json!({
            "id": "cap:keeps",
            "description": "the description the refused call carried",
            "satisfies": "req:no-such-requirement",
        }),
    )
    .await;
    assert!(
        refusal.contains("nothing was written"),
        "the refusal says nothing was written, and now that must be TRUE: {refusal}"
    );
    assert_eq!(
        node(&c, "cap:keeps").await["properties"]["description"],
        json!("the description before"),
        "a refused revise must leave the description it found"
    );
}

/// THE RETRY IS A CREATE AGAIN, SO THE DUPLICATE GUARD RUNS. A near-duplicate
/// idea refused on a bad `kind` used to be stored anyway; the corrected retry
/// then counted as a revise and landed past both guards. The control is the
/// same corrected call on a store the refusal never touched.
#[tokio::test]
async fn the_retry_after_a_refusal_is_a_create_and_the_duplicate_guard_runs() {
    let c = connect().await;
    ok(
        &c,
        "add_decision",
        json!({
            "id": "dec:the-first",
            "name": "The first",
            "decision": IDEA,
            "kind": "choice",
        }),
    )
    .await;
    let second = |kind: &str| {
        json!({
            "id": "dec:the-second",
            "name": "The second",
            "decision": IDEA,
            "kind": kind,
        })
    };
    refused_and_untouched(&c, "add_decision", second("Exploratory")).await;
    let retry = call(&c, "add_decision", second("choice")).await;
    match retry {
        Err(e) => assert!(
            e.contains("dec:the-first"),
            "the retry must be refused by the duplicate guard, naming the near-match: {e}"
        ),
        Ok(v) => panic!(
            "the corrected retry was ACCEPTED, so it was treated as a revise of a node the \
             refused call left behind and the duplicate guard never ran: {v}"
        ),
    }
    assert!(node(&c, "dec:the-second").await.is_null());
}

/// A SUCCESS STILL COMMITS, AND ITS RECEIPT STILL NAMES WHAT IT WROTE. The
/// unit is all-or-nothing, not nothing.
#[tokio::test]
async fn a_successful_write_is_committed_and_its_receipt_names_it() {
    let c = connect().await;
    let reply = ok(
        &c,
        "add_capability",
        json!({"id": "cap:lands", "name": "Lands", "description": "a write that succeeds"}),
    )
    .await;
    assert!(
        reply.to_string().contains("cap:lands"),
        "the receipt names the node it wrote: {reply}"
    );
    assert_eq!(
        node(&c, "cap:lands").await["properties"]["name"],
        json!("Lands")
    );
}

/// A REFUSED CALL SETS OFF NOTHING. The write-through export is rung only by a
/// write that was committed: a refusal that rang it would export a design the
/// call never changed, and claim a write took place.
#[tokio::test]
async fn a_refused_write_triggers_no_export_and_a_committed_one_does() {
    let dir = std::env::temp_dir().join(format!(
        "reflow2-refused-write-export-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let file = dir.join("design.json");
    let mut service = ReflowService::new(&dir.join("graph").display().to_string())
        .expect("a real store")
        .without_reaching_out();
    service
        .start_auto_export(file.display().to_string())
        .expect("write-through starts");
    let watch = service.share();
    let c = connect_to(service).await;

    let _ = call(
        &c,
        "add_requirement",
        json!({"id": "req:refused", "name": "Refused", "statement": "Refused.", "priority": "nope"}),
    )
    .await
    .expect_err("a bad priority is refused");
    // Well past the write-through's quiet period (2 s).
    tokio::time::sleep(Duration::from_secs(4)).await;
    let (_, status) = watch.auto_export_status().expect("status");
    assert_eq!(
        status.exports, 0,
        "a refused call must not set off an export ({status:?})"
    );
    assert!(!file.exists(), "nothing on disk after a refused call");

    ok(
        &c,
        "add_requirement",
        json!({"id": "req:committed", "name": "Committed", "statement": "Committed."}),
    )
    .await;
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while !file.exists() && std::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let raw = std::fs::read_to_string(&file).expect("a committed write is exported");
    assert!(raw.contains("req:committed"));
    assert!(
        !raw.contains("req:refused"),
        "the refused requirement must not reach the export"
    );
}

// ---- the walk over every served write tool ---------------------------------

/// A value that names nothing in any design and is no member of any enum.
const NAMES_NOTHING: &str = "zz-walk-names-nothing";
/// A path that cannot be read or written, so a tool taking a file never
/// touches the test's own tree.
const NOWHERE: &str = "/nonexistent-reflow2-walk/zz/nothing.json";

fn is_pathish(field: &str) -> bool {
    ["path", "location", "file", "dir", "root", "export"]
        .iter()
        .any(|w| field.contains(w))
}

/// Text fields a constructor needs to CREATE: given real text in the
/// one-field-at-a-time pass, so the call reaches its node write and the one
/// bad field is checked after it, which is the class.
const TEXT_FIELDS: &[&str] = &[
    "name",
    "statement",
    "description",
    "decision",
    "rationale",
    "summary",
];

fn resolve<'a>(schema: &'a Value, root: &'a Value) -> &'a Value {
    match schema.get("$ref").and_then(Value::as_str) {
        Some(r) => {
            let name = r.rsplit('/').next().unwrap_or_default();
            root.get("$defs")
                .and_then(|d| d.get(name))
                .or_else(|| root.get("definitions").and_then(|d| d.get(name)))
                .unwrap_or(schema)
        }
        None => schema,
    }
}

/// A value that DESERIALISES as `schema` and means nothing to the design: an
/// enum takes its first member (so deserialisation passes), a string names
/// nothing, a path points nowhere, and an object fills every property it has.
fn synth(schema: &Value, root: &Value, field: &str, depth: usize) -> Value {
    let schema = resolve(schema, root);
    if let Some(members) = schema.get("enum").and_then(Value::as_array)
        && let Some(first) = members.iter().find(|m| !m.is_null())
    {
        return first.clone();
    }
    for key in ["anyOf", "oneOf"] {
        if let Some(alts) = schema.get(key).and_then(Value::as_array)
            && let Some(alt) = alts
                .iter()
                .find(|a| resolve(a, root).get("type") != Some(&json!("null")))
        {
            return synth(alt, root, field, depth);
        }
    }
    let ty = match schema.get("type") {
        Some(Value::String(t)) => t.clone(),
        Some(Value::Array(ts)) => ts
            .iter()
            .filter_map(Value::as_str)
            .find(|t| *t != "null")
            .unwrap_or("null")
            .to_string(),
        _ if schema.get("properties").is_some() => "object".into(),
        _ => "string".into(),
    };
    match ty.as_str() {
        "string" if is_pathish(field) => json!(NOWHERE),
        "string" => json!(NAMES_NOTHING),
        "integer" => json!(1),
        "number" => json!(0.5),
        "boolean" => json!(false),
        "array" if depth < 4 => match schema.get("items") {
            Some(items) => json!([synth(items, root, field, depth + 1)]),
            None => json!([]),
        },
        "array" => json!([]),
        "object" => {
            let mut out = Map::new();
            if depth < 4
                && let Some(props) = schema.get("properties").and_then(Value::as_object)
            {
                for (k, v) in props {
                    out.insert(k.clone(), synth(v, root, k, depth + 1));
                }
            }
            Value::Object(out)
        }
        _ => Value::Null,
    }
}

/// The served write tools with their input schemas, read off the catalogue
/// with the same predicate `--call` uses.
async fn write_tools(c: &Client) -> Vec<(String, Value)> {
    let tools = c.list_all_tools().await.expect("tools/list");
    let mut out: Vec<(String, Value)> = tools
        .into_iter()
        .filter(|t| {
            !t.annotations
                .as_ref()
                .and_then(|a| a.read_only_hint)
                .unwrap_or(false)
        })
        .map(|t| {
            (
                t.name.to_string(),
                serde_json::to_value(&t.input_schema).expect("schema"),
            )
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// The argument sets the walk sends one tool: every property filled at once,
/// then — so a constructor reaches its node write before the one bad field is
/// checked — a fresh id and real text with ONE other property filled at a
/// time.
fn argument_sets(tool: &str, schema: &Value) -> Vec<(String, Value)> {
    let props: Vec<(String, Value)> = schema
        .get("properties")
        .and_then(Value::as_object)
        .map(|p| {
            p.iter()
                .filter(|(k, _)| k.as_str() != "echo")
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
        .unwrap_or_default();
    let required: Vec<String> = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|r| {
            r.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let mut sets = Vec::new();
    // Every property at once.
    let mut all = Map::new();
    for (k, v) in &props {
        let value = if k == "id" {
            json!(format!("walk-{tool}-all"))
        } else {
            synth(v, schema, k, 0)
        };
        all.insert(k.clone(), value);
    }
    sets.push(("<every property>".to_string(), Value::Object(all)));

    // A base that can create, plus one property at a time.
    let base = |tag: &str| {
        let mut m = Map::new();
        for (k, v) in &props {
            if k == "id" {
                m.insert(k.clone(), json!(format!("walk-{tool}-{tag}")));
            } else if TEXT_FIELDS.contains(&k.as_str()) {
                m.insert(k.clone(), json!(format!("walk {tool} {k}")));
            } else if required.contains(k) {
                m.insert(k.clone(), synth(v, schema, k, 0));
            }
        }
        m
    };
    sets.push(("<base>".to_string(), Value::Object(base("base"))));
    for (k, v) in &props {
        if k == "id" || TEXT_FIELDS.contains(&k.as_str()) || required.contains(k) {
            continue;
        }
        let mut m = base(k);
        m.insert(k.clone(), synth(v, schema, k, 0));
        sets.push((k.clone(), Value::Object(m)));
    }
    sets
}

/// Seed a little design so references can resolve and a revise has something
/// to revise. Tools that succeed on the walk add to it, which is fine: the
/// assertion is about each REFUSAL, against the store as it stood just before.
async fn seed(c: &Client) {
    ok(
        c,
        "add_requirement",
        json!({"id": "req:seed", "name": "Seed requirement", "statement": "The design holds this."}),
    )
    .await;
    ok(
        c,
        "add_capability",
        json!({"id": "cap:seed", "name": "Seed capability", "description": "It does one thing.", "satisfies": "req:seed"}),
    )
    .await;
    ok(
        c,
        "add_component",
        json!({"id": "comp:seed", "name": "Seed component", "description": "It holds the capability."}),
    )
    .await;
    ok(
        c,
        "add_decision",
        json!({"id": "dec:seed", "name": "Seed decision", "decision": "Something was decided."}),
    )
    .await;
}

/// THE CLASS GUARD. Every served write tool, every generated argument set:
/// whatever it refuses, it refuses with the stored design untouched.
#[tokio::test]
async fn every_write_tool_stores_nothing_when_it_refuses() {
    let c = connect().await;
    seed(&c).await;
    let tools = write_tools(&c).await;
    assert!(
        tools.len() > 50,
        "expected the served surface's write tools, found {}",
        tools.len()
    );

    let mut left_behind = Vec::new();
    let mut refusals = 0usize;
    let mut answered = 0usize;
    let mut tools_refusing = std::collections::BTreeSet::new();
    let mut before = stored(&c).await;
    for (tool, schema) in &tools {
        for (which, args) in argument_sets(tool, schema) {
            match call(&c, tool, args.clone()).await {
                Err(refusal) => {
                    refusals += 1;
                    tools_refusing.insert(tool.clone());
                    let after = stored(&c).await;
                    if after != before {
                        left_behind.push(format!(
                            "{tool} [{which}] refused and left: {}\n    refusal: {}",
                            difference(&before, &after),
                            refusal.chars().take(240).collect::<String>()
                        ));
                        before = after;
                    }
                }
                Ok(_) => {
                    // A write that went through changed the store on purpose.
                    answered += 1;
                    before = stored(&c).await;
                }
            }
        }
    }
    eprintln!(
        "the walk: {} write tools, {} argument sets refused and {} answered; {} tools refused \
         at least once",
        tools.len(),
        refusals,
        answered,
        tools_refusing.len()
    );
    assert!(
        left_behind.is_empty(),
        "{} refused call(s) changed the stored design (of {refusals} refusals and {answered} \
         answers across {} write tools):\n{}",
        left_behind.len(),
        tools.len(),
        left_behind.join("\n")
    );
    // A walk that refused nothing proved nothing. Most write tools must have
    // been made to refuse at least once, or the generated arguments have
    // stopped reaching the checks this guards.
    assert!(
        tools_refusing.len() * 4 >= tools.len() * 3,
        "only {} of {} write tools were made to refuse — the walk is not reaching the checks \
         it exists to guard",
        tools_refusing.len(),
        tools.len()
    );
}
