//! EVERY CALL THAT CAN SETTLE INTENT DECLARES WHAT SETTLES IT — on the served
//! tool, under `_meta["reflow2/settles"]` — AND THE DECLARATION IS WHAT THE
//! TOOL DOES.
//!
//! Until 2026-09-29 reflow2 decided "does this call settle?" inside each
//! handler and said so nowhere a program could read. flo2's gateway, which
//! signs settles on its signed-in caller's behalf, had to guess, guessed "an
//! argument named `status`", and let `add_design_rule`'s `enforced` through
//! unsigned (flo2 fact:root-cause-the-gateway-reads-settling-from-a-field-named-status-and-lets-an-enforced-rule-through-unsigned-2026-09-29);
//! flo2 #102 contained it with a hand-kept copy. `collapse_decision` settled a
//! Decision with no approver parameter at all
//! (fact:collapse-decision-settles-a-decision-with-no-approver-and-says-nothing-2026-09-28).
//!
//! Membership is read off the SERVED surface, never a hand list: a tool whose
//! input schema takes an `approver` (at the top level, or inside a list's
//! items) must declare, and a declaration must be true of the tool when it is
//! called — refused unsigned where it says `refused`, recorded with a
//! nobody's-name note where it says `recorded_with_note`, silent when signed,
//! and silent when the call does not settle.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};

const META_KEY: &str = "reflow2/settles";
const BOSS: &str = "who:boss";

struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "every-settling-call-declares-what-settles-it".to_string();
        cfg.client_info.version = "test".to_string();
        cfg
    }
}

type Client = rmcp::service::RunningService<rmcp::RoleClient, TestClient>;

async fn connect() -> Client {
    let service = ReflowService::in_memory().expect("in-memory service");
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

/// `Ok(structured reply)` or `Err(refusal text)`.
async fn call(c: &Client, tool: &str, args: Value) -> Result<Value, String> {
    let Value::Object(arguments) = args else {
        panic!("arguments for {tool} must be an object");
    };
    match c
        .call_tool(CallToolRequestParams::new(tool.to_string()).with_arguments(arguments))
        .await
    {
        Ok(r) if r.is_error == Some(true) => Err(format!("{:?}", r.content)),
        Ok(r) => Ok(r.structured_content.unwrap_or(Value::Null)),
        Err(e) => Err(e.to_string()),
    }
}

/// Where a schema takes an approver: `approver`, or `list[].approver`.
fn approver_paths(schema: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let Some(props) = schema["properties"].as_object() else {
        return out;
    };
    if props.contains_key("approver") {
        out.push("approver".to_string());
    }
    for (name, prop) in props {
        let items = &prop["items"];
        let item_props = items["properties"].as_object().or_else(|| {
            // A `$ref` to a definition: follow it one hop.
            let r = items["$ref"].as_str()?;
            let key = r.rsplit('/').next()?;
            schema["$defs"][key]["properties"]
                .as_object()
                .or_else(|| schema["definitions"][key]["properties"].as_object())
        });
        if item_props.is_some_and(|p| p.contains_key("approver")) {
            out.push(format!("{name}[].approver"));
        }
    }
    out
}

async fn served(c: &Client) -> Vec<(String, Value, Option<Value>)> {
    let tools = c.list_all_tools().await.expect("tools/list");
    tools
        .into_iter()
        .map(|t| {
            let decl = t.meta.as_ref().and_then(|m| m.0.get(META_KEY)).cloned();
            (
                t.name.to_string(),
                serde_json::to_value(&t.input_schema).expect("schema"),
                decl,
            )
        })
        .collect()
}

/// Fewer declarations than this means a filter broke, not that every tool
/// stopped settling — the nine settle paths that existed when this was written.
const FLOOR: usize = 9;

#[tokio::test]
async fn every_tool_that_takes_an_approver_declares_what_settles_it() {
    let c = connect().await;
    let mut problems = Vec::new();
    let mut declared = 0;
    for (tool, schema, decl) in served(&c).await {
        let paths = approver_paths(&schema);
        match (paths.is_empty(), decl) {
            (true, None) => {}
            (false, None) => problems.push(format!(
                "{tool} takes an approver at {paths:?} and declares nothing under \
                 _meta[\"{META_KEY}\"] — a gateway signing on its caller's behalf cannot tell \
                 which of its calls settle"
            )),
            (true, Some(d)) => problems.push(format!(
                "{tool} declares that it settles ({d}) but its schema takes no approver"
            )),
            (false, Some(d)) => {
                declared += 1;
                let at = d["approver"].as_str().unwrap_or_default();
                if !paths.iter().any(|p| p == at) {
                    problems.push(format!(
                        "{tool} declares its signature at `{at}`, but its schema takes it at \
                         {paths:?}"
                    ));
                }
                match d["argument"].as_str() {
                    Some(arg) if schema["properties"][arg].is_null() => problems.push(format!(
                        "{tool} declares `{arg}` as what settles it, and takes no such argument"
                    )),
                    Some(_) => {}
                    None if d["when"] != json!("always") => problems.push(format!(
                        "{tool} names no argument, so its rule must be `always`: {d}"
                    )),
                    None => {}
                }
                if !matches!(
                    d["unsigned"].as_str(),
                    Some("refused" | "recorded_with_note")
                ) {
                    problems.push(format!("{tool} declares an unknown `unsigned`: {d}"));
                }
                if d["version"] != json!(1) {
                    problems.push(format!("{tool} declares no version: {d}"));
                }
            }
        }
    }
    assert!(
        declared >= FLOOR,
        "only {declared} served tools declare what settles them (floor {FLOOR}) — the \
         declaration is not being served"
    );
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// Put the signature where the declaration says it goes.
fn sign(mut args: Value, at: &str, who: &str) -> Value {
    if let Some((list, field)) = at.split_once("[].") {
        for item in args[list].as_array_mut().expect("the declared list") {
            item[field] = json!(who);
        }
    } else {
        args[at] = json!(who);
    }
    args
}

/// Arguments for one call of `tool`, building whatever it needs first.
/// `settling` picks a value the declaration says settles; `None` when the tool
/// has no non-settling form. Unknown tools return `Err` so a new settle path
/// cannot pass untested.
async fn case(c: &Client, tool: &str, settling: bool, n: usize) -> Result<Option<Value>, String> {
    let word = [
        "copper", "willow", "harbour", "lantern", "meadow", "quarry", "saddle", "thistle",
        "orchard", "beacon", "cobble", "furrow",
    ][n % 12];
    let ok = |r: Result<Value, String>, what: &str| {
        r.unwrap_or_else(|e| panic!("fixture {what} for {tool}: {e}"));
    };
    Ok(Some(match tool {
        "add_requirement" => {
            let mut a = json!({"id": format!("req:settle-{n}"), "name": format!("The {word} gate holds"), "statement": format!("A {word} statement for case {n}")});
            if settling {
                a["status"] = json!("accepted");
            }
            a
        }
        "add_decision" => {
            let mut a = json!({"id": format!("dec:settle-{n}"), "name": format!("Use the {word} route"), "decision": format!("Take the {word} route, case {n}"), "rationale": "fixture", "kind": "choice"});
            if settling {
                a["status"] = json!("accepted");
            }
            a
        }
        "add_design_rule" => {
            let mut a = json!({"id": format!("rule:settle-{n}"), "name": format!("Always {word}"), "statement": format!("We always {word}, case {n}")});
            if settling {
                a["enforced"] = json!(false);
            }
            a
        }
        "set_requirement_status" => {
            let id = format!("req:status-{n}");
            ok(call(c, "add_requirement", json!({"id": id, "name": format!("A {word} need"), "statement": format!("{word} need {n}")})).await, "requirement");
            json!({"requirement_id": id, "status": if settling { "accepted" } else { "proposed" }})
        }
        "set_decision_status" => {
            let id = format!("dec:status-{n}");
            ok(call(c, "add_decision", json!({"id": id, "name": format!("Pick the {word}"), "decision": format!("{word} pick {n}"), "rationale": "fixture", "kind": "choice"})).await, "decision");
            json!({"decision_id": id, "status": if settling { "accepted" } else { "proposed" }})
        }
        "collapse_decision" => {
            if !settling {
                return Ok(None);
            }
            let id = format!("dec:fork-{n}");
            ok(call(c, "add_decision", json!({"id": id, "name": format!("Fork at the {word}"), "decision": format!("{word} fork {n}"), "rationale": "fixture", "kind": "choice"})).await, "decision");
            for side in ["a", "b"] {
                ok(call(c, "register_alternative", json!({"decision_id": id, "artifact_id": format!("art:alt-{n}-{side}"), "name": format!("{word} option {side}"), "location": format!("alt/{n}/{side}.md")})).await, "alternative");
            }
            json!({"decision_id": id, "winner_id": format!("art:alt-{n}-a")})
        }
        "acknowledge_gap" => {
            if !settling {
                return Ok(None);
            }
            json!({"gap_id": format!("gap:fixture-{n}"), "affected_ids": ["req:seed"], "reason": format!("{word} reason")})
        }
        "acknowledge_defect" => {
            if !settling {
                return Ok(None);
            }
            json!({"defect_id": format!("defect:fixture-{n}"), "affected_ids": ["req:seed"], "reason": format!("{word} reason")})
        }
        "acknowledge_gaps" => {
            if !settling {
                return Ok(None);
            }
            json!({"gaps": [{"gap_id": format!("gap:batch-{n}"), "affected_ids": ["req:seed"], "reason": format!("{word} reason")}]})
        }
        other => {
            return Err(format!(
                "no fixture for `{other}`: a settle path this test cannot drive"
            ));
        }
    }))
}

fn says_nobodys_name(reply: &Value) -> bool {
    reply.to_string().to_lowercase().contains("nobody's name")
}

#[tokio::test]
async fn every_declaration_is_what_the_tool_does() {
    let c = connect().await;
    call(
        &c,
        "add_contributor",
        json!({"id": BOSS, "name": "The owner", "kind": "person"}),
    )
    .await
    .expect("seed contributor");
    call(&c, "add_requirement", json!({"id": "req:seed", "name": "A seed requirement", "statement": "exists so an acknowledgement has something to touch"}))
        .await
        .expect("seed requirement");

    let mut problems = Vec::new();
    let mut driven = 0;
    let mut n = 0;
    for (tool, _schema, decl) in served(&c).await {
        let Some(d) = decl else { continue };
        let at = d["approver"].as_str().unwrap_or("approver").to_string();
        let refused = d["unsigned"] == json!("refused");

        n += 1;
        match case(&c, &tool, true, n).await {
            Err(e) => {
                problems.push(e);
                continue;
            }
            Ok(None) => problems.push(format!("{tool}: no settling form to drive")),
            Ok(Some(args)) => match (call(&c, &tool, args).await, refused) {
                (Err(_), true) => {}
                (Ok(r), true) => problems.push(format!(
                    "{tool} declares `refused` and RECORDED an unsigned settle: {r}"
                )),
                (Err(e), false) => problems.push(format!(
                    "{tool} declares `recorded_with_note` and REFUSED an unsigned settle: {e}"
                )),
                (Ok(r), false) if !says_nobodys_name(&r) => problems.push(format!(
                    "{tool} recorded an unsigned settle and did not say it carries nobody's \
                     name: {r}"
                )),
                (Ok(_), false) => {}
            },
        }

        n += 1;
        if let Ok(Some(args)) = case(&c, &tool, true, n).await {
            match call(&c, &tool, sign(args, &at, BOSS)).await {
                Ok(r) if says_nobodys_name(&r) => problems.push(format!(
                    "{tool}: a SIGNED settle still says it carries nobody's name: {r}"
                )),
                Ok(_) => {}
                Err(e) => problems.push(format!("{tool}: a signed settle was refused: {e}")),
            }
        }

        n += 1;
        if let Ok(Some(args)) = case(&c, &tool, false, n).await {
            match call(&c, &tool, args).await {
                Ok(r) if says_nobodys_name(&r) => problems.push(format!(
                    "{tool}: a call the declaration says does NOT settle was flagged as \
                     carrying nobody's name: {r}"
                )),
                Ok(_) => {}
                Err(e) => problems.push(format!(
                    "{tool}: a call the declaration says does NOT settle was refused unsigned: {e}"
                )),
            }
        }
        driven += 1;
    }
    assert!(
        driven >= FLOOR,
        "only {driven} declared tools were driven (floor {FLOOR})"
    );
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// The fork's choice is the owner's act: `collapse_decision` signs it.
#[tokio::test]
async fn choosing_a_forks_winner_draws_the_owners_signature() {
    let c = connect().await;
    call(
        &c,
        "add_contributor",
        json!({"id": BOSS, "name": "The owner", "kind": "person"}),
    )
    .await
    .expect("seed contributor");
    let args = case(&c, "collapse_decision", true, 99)
        .await
        .expect("fixture")
        .expect("settling form");
    call(&c, "collapse_decision", sign(args, "approver", BOSS))
        .await
        .expect("a signed choice is recorded");
    let decision = call(&c, "get_node", json!({"id": "dec:fork-99"}))
        .await
        .expect("get_node");
    assert_eq!(decision["node"]["properties"]["status"], json!("accepted"));
    let unknown = call(
        &c,
        "collapse_decision",
        json!({"decision_id": "dec:fork-99", "winner_id": "art:alt-99-a", "approver": "who:nobody"}),
    )
    .await;
    assert!(
        unknown.is_err(),
        "an approver naming no Contributor is refused before anything is written"
    );
}
