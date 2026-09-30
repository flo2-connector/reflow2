//! A write and an approval record the AGENT they went through beside the
//! person they were for — the ACTS_FOR rung, driven through the real request
//! path (`call_tool`) of an in-process client.
//!
//! `req:a-write-and-an-approval-record-the-agent-and-the-person-it-acts-for`
//! (accepted, Anthony 2026-09-29; idea 1 and root causes I1/I21 of the
//! two-agent exercise). What this pins, as a CLASS:
//!   · EVERY settle path reflow2 serves — enumerated from the tools' own
//!     `_meta["reflow2/settles"]` declaration, never from a hand list — records
//!     the agent in `approved_via` beside the person who is the approver of
//!     record, and the agent is drawn `ACTS_FOR` them;
//!   · a credited write carries the agent in `authored_via`;
//!   · the agent arrives by the request's `_meta` (the route a hosting gateway
//!     uses), by the session's `writes_for` declaration, or by the client's
//!     handshake name matching an agent Contributor's `handle` — never minted;
//!   · with no agent known nothing is recorded, and a read says so;
//!   · naming an agent never signs anything, and an unknown agent, or a
//!     person named as the agent, is refused before anything is written.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};

const META_KEY: &str = "reflow2/settles";
const BOSS: &str = "who:boss";
const AGENT: &str = "who:owner-agent";
const AGENT_CLIENT: &str = "the-owner-agents-client";

struct TestClient(&'static str);

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = self.0.to_string();
        cfg.client_info.version = "test".to_string();
        cfg
    }
}

type Client = rmcp::service::RunningService<rmcp::RoleClient, TestClient>;

async fn connect(name: &'static str) -> Client {
    let service = ReflowService::in_memory().expect("in-memory service");
    let (server_rx, client_tx) = tokio::io::duplex(1 << 22);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 22);
    tokio::spawn(async move {
        if let Ok(running) = service.serve((server_rx, server_tx)).await {
            let _ = running.waiting().await;
        }
    });
    TestClient(name)
        .serve((client_rx, client_tx))
        .await
        .expect("the in-process handshake")
}

/// `Ok(structured reply)` or `Err(refusal text)`, with an optional request
/// `_meta` — the route a hosting gateway names the person and the agent by.
async fn call_with(
    c: &Client,
    tool: &str,
    args: Value,
    meta: Option<Value>,
) -> Result<Value, String> {
    let mut params = json!({"name": tool, "arguments": args});
    if let Some(m) = meta {
        params["_meta"] = m;
    }
    let params: CallToolRequestParams = serde_json::from_value(params).expect("params");
    match c.call_tool(params).await {
        Ok(r) if r.is_error == Some(true) => Err(format!("{:?}", r.content)),
        Ok(r) => Ok(r.structured_content.unwrap_or(Value::Null)),
        Err(e) => Err(e.to_string()),
    }
}

async fn call(c: &Client, tool: &str, args: Value) -> Result<Value, String> {
    call_with(c, tool, args, None).await
}

fn through_agent() -> Value {
    json!({"reflow2/writes_for": BOSS, "reflow2/acting_agent": AGENT})
}

async fn seed(c: &Client) {
    call(
        c,
        "add_contributor",
        json!({"id": BOSS, "name": "The owner", "kind": "person"}),
    )
    .await
    .expect("seed person");
    call(
        c,
        "add_contributor",
        json!({"id": AGENT, "name": "The owner agent", "kind": "automated_agent", "handle": AGENT_CLIENT}),
    )
    .await
    .expect("seed agent");
    call(c, "add_requirement", json!({"id": "req:seed", "name": "A seed requirement", "statement": "exists so an acknowledgement has something to touch"}))
        .await
        .expect("seed requirement");
}

/// Every edge in the design, read back from the export.
async fn edges(c: &Client) -> Vec<Value> {
    let doc = call(c, "export_graph", json!({})).await.expect("export");
    doc["edges"].as_array().cloned().unwrap_or_default()
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn authored_by_to<'a>(all: &'a [Value], to: &str) -> Vec<&'a Value> {
    all.iter()
        .filter(|e| e["edge_type"] == "AUTHORED_BY" && e["to_id"] == to)
        .collect()
}

fn acts_for(all: &[Value]) -> Vec<(String, String, String)> {
    all.iter()
        .filter(|e| e["edge_type"] == "ACTS_FOR")
        .map(|e| {
            (
                e["from_id"].as_str().unwrap_or_default().to_string(),
                e["to_id"].as_str().unwrap_or_default().to_string(),
                e["properties"]["route"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            )
        })
        .collect()
}

async fn served_settles(c: &Client) -> Vec<(String, Value)> {
    c.list_all_tools()
        .await
        .expect("tools/list")
        .into_iter()
        .filter_map(|t| {
            let d = t.meta.as_ref().and_then(|m| m.0.get(META_KEY)).cloned()?;
            Some((t.name.to_string(), d))
        })
        .collect()
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

/// Fewer than this means the declaration stopped being served, not that every
/// tool stopped settling — the nine settle paths that existed when this was
/// written (#628).
const FLOOR: usize = 9;

#[tokio::test]
async fn every_settle_path_records_the_agent_beside_the_person() {
    let c = connect("a-client-no-agent-claims").await;
    seed(&c).await;
    let mut problems = Vec::new();
    let mut driven = 0;
    for (n, (tool, d)) in served_settles(&c).await.into_iter().enumerate() {
        let at = d["approver"].as_str().unwrap_or("approver").to_string();
        match case(&c, &tool, true, 100 + n).await {
            Err(e) => problems.push(e),
            Ok(None) => problems.push(format!("{tool}: no settling form to drive")),
            Ok(Some(args)) => {
                match call_with(&c, &tool, sign(args, &at, BOSS), Some(through_agent())).await {
                    Ok(_) => driven += 1,
                    Err(e) => problems.push(format!(
                        "{tool}: a signed settle through an agent was refused: {e}"
                    )),
                }
            }
        }
    }
    assert!(
        driven >= FLOOR,
        "only {driven} settle paths were driven (floor {FLOOR}): {problems:?}"
    );
    let all = edges(&c).await;
    let approvals: Vec<_> = authored_by_to(&all, BOSS)
        .into_iter()
        .filter(|e| {
            strings(&e["properties"]["roles"])
                .iter()
                .any(|r| r == "approver")
        })
        .collect();
    if approvals.len() < driven {
        problems.push(format!(
            "{driven} settles were signed by {BOSS} but only {} approver edges point at them",
            approvals.len()
        ));
    }
    for e in &approvals {
        let via = strings(&e["properties"]["approved_via"]);
        if via != vec![AGENT.to_string()] {
            problems.push(format!(
                "the approval of {} by {BOSS} was recorded through {AGENT} and names {via:?} \
                 as the agent (approved_via)",
                e["from_id"]
            ));
        }
    }
    if !acts_for(&all).contains(&(AGENT.into(), BOSS.into(), "request".into())) {
        problems.push(format!(
            "no `{AGENT} ACTS_FOR {BOSS}` (route request): {:?}",
            acts_for(&all)
        ));
    }
    // The agent carried the owner's word; it never gave it.
    if !authored_by_to(&all, AGENT).is_empty() {
        problems.push(format!(
            "{AGENT} was recorded as an author or approver of something"
        ));
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[tokio::test]
async fn a_credited_write_carries_the_agent_beside_its_author() {
    let c = connect("a-client-no-agent-claims").await;
    seed(&c).await;
    call_with(
        &c,
        "add_requirement",
        json!({"id": "req:fast", "name": "It is fast", "statement": "reads under 20 ms"}),
        Some(through_agent()),
    )
    .await
    .expect("write through the agent");
    let all = edges(&c).await;
    let authored = authored_by_to(&all, BOSS);
    let e = authored
        .iter()
        .find(|e| e["from_id"] == "req:fast")
        .expect("req:fast is credited to its person");
    assert_eq!(
        strings(&e["properties"]["authored_via"]),
        vec![AGENT.to_string()]
    );
    assert!(acts_for(&all).contains(&(AGENT.into(), BOSS.into(), "request".into())));
}

#[tokio::test]
async fn a_session_names_its_agent_once() {
    let c = connect("a-client-no-agent-claims").await;
    seed(&c).await;
    call(
        &c,
        "writes_for",
        json!({"contributor_id": BOSS, "acting_agent": AGENT}),
    )
    .await
    .expect("declare the person and the agent");
    call(
        &c,
        "add_requirement",
        json!({"id": "req:session", "name": "Declared once", "statement": "no per-call naming"}),
    )
    .await
    .expect("write");
    let all = edges(&c).await;
    let e = authored_by_to(&all, BOSS)
        .into_iter()
        .find(|e| e["from_id"] == "req:session")
        .expect("credited");
    assert_eq!(
        strings(&e["properties"]["authored_via"]),
        vec![AGENT.to_string()]
    );
    assert!(acts_for(&all).contains(&(AGENT.into(), BOSS.into(), "session".into())));
}

#[tokio::test]
async fn the_clients_own_name_finds_its_agent_and_never_mints_one() {
    // A client whose handshake name IS an agent Contributor's handle.
    let c = connect(AGENT_CLIENT).await;
    seed(&c).await;
    call(&c, "writes_for", json!({"contributor_id": BOSS}))
        .await
        .expect("declare");
    call(&c, "add_requirement", json!({"id": "req:client", "name": "Found by name", "statement": "the client said who it is"}))
        .await
        .expect("write");
    let all = edges(&c).await;
    let e = authored_by_to(&all, BOSS)
        .into_iter()
        .find(|e| e["from_id"] == "req:client")
        .expect("credited");
    assert_eq!(
        strings(&e["properties"]["authored_via"]),
        vec![AGENT.to_string()]
    );
    assert!(acts_for(&all).contains(&(AGENT.into(), BOSS.into(), "client".into())));

    // A client nobody declared: nothing recorded, nobody minted.
    let s = connect("a-stranger-client").await;
    seed(&s).await;
    call(&s, "writes_for", json!({"contributor_id": BOSS}))
        .await
        .expect("declare");
    call(
        &s,
        "add_requirement",
        json!({"id": "req:stranger", "name": "No agent", "statement": "nobody said"}),
    )
    .await
    .expect("write");
    let all = edges(&s).await;
    let e = authored_by_to(&all, BOSS)
        .into_iter()
        .find(|e| e["from_id"] == "req:stranger")
        .expect("credited");
    assert!(
        e["properties"]["authored_via"].is_null(),
        "no agent was known: {e}"
    );
    assert!(acts_for(&all).is_empty());
    let doc = call(&s, "export_graph", json!({})).await.expect("export");
    let contributors = doc["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["node_type"] == "Contributor")
        .count();
    assert_eq!(contributors, 2, "only the two seeded contributors exist");
}

#[tokio::test]
async fn a_read_says_plainly_when_no_agent_is_known() {
    let c = connect("a-client-no-agent-claims").await;
    seed(&c).await;
    for (id, meta) in [
        ("dec:asked-plainly", None),
        ("dec:asked-through", Some(through_agent())),
    ] {
        call(&c, "add_decision", json!({"id": id, "name": id, "decision": "which way", "rationale": "fixture", "kind": "choice"}))
            .await
            .expect("decision");
        // Asking the owner to settle it: an approver edge on a proposed Decision.
        call_with(
            &c,
            "authored_by",
            json!({"from_id": id, "contributor_id": BOSS, "role": "approver"}),
            meta,
        )
        .await
        .expect("ask");
    }
    let status = call(&c, "loop_status", json!({}))
        .await
        .expect("loop_status");
    let assigned = status["assigned_decisions"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let row = |id: &str| {
        assigned
            .iter()
            .find(|r| r["decision_id"] == id)
            .unwrap_or_else(|| panic!("{id} is not among {assigned:?}"))
            .clone()
    };
    let plain = row("dec:asked-plainly");
    assert_eq!(plain["acted_through_note"], "no agent known", "{plain}");
    assert_eq!(strings(&plain["acted_through"]), Vec::<String>::new());
    let through = row("dec:asked-through");
    assert_eq!(
        strings(&through["acted_through"]),
        vec![AGENT.to_string()],
        "{through}"
    );
    assert!(through["acted_through_note"].is_null(), "{through}");
}

#[tokio::test]
async fn naming_an_agent_never_signs_anything() {
    let c = connect("a-client-no-agent-claims").await;
    seed(&c).await;
    call(&c, "add_decision", json!({"id": "dec:unsigned", "name": "Unsigned", "decision": "x", "rationale": "fixture", "kind": "choice"}))
        .await
        .expect("decision");
    // Settled with NO approver, through an agent: still carries nobody's name.
    let r = call_with(
        &c,
        "set_decision_status",
        json!({"decision_id": "dec:unsigned", "status": "accepted"}),
        Some(through_agent()),
    )
    .await
    .expect("recorded with a note");
    assert!(
        r.to_string().to_lowercase().contains("nobody's name"),
        "{r}"
    );
    let all = edges(&c).await;
    let approvers: Vec<_> = all
        .iter()
        .filter(|e| {
            e["edge_type"] == "AUTHORED_BY"
                && e["from_id"] == "dec:unsigned"
                && strings(&e["properties"]["roles"])
                    .iter()
                    .any(|r| r == "approver")
        })
        .collect();
    assert!(
        approvers.is_empty(),
        "an agent named on the call signed it: {approvers:?}"
    );
}

#[tokio::test]
async fn an_unknown_agent_or_a_person_named_as_one_is_refused_and_nothing_is_written() {
    let c = connect("a-client-no-agent-claims").await;
    seed(&c).await;
    for (agent, id) in [("who:nobody", "req:ghost"), (BOSS, "req:person-as-agent")] {
        let e = call_with(
            &c,
            "add_requirement",
            json!({"id": id, "name": id, "statement": "should not land"}),
            Some(json!({"reflow2/writes_for": BOSS, "reflow2/acting_agent": agent})),
        )
        .await
        .expect_err("refused");
        assert!(
            e.contains("acting agent") || e.contains("acting_agent"),
            "{e}"
        );
        let doc = call(&c, "export_graph", json!({})).await.expect("export");
        assert!(
            !doc["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|n| n["node_id"] == id),
            "{id} was written under a refused agent"
        );
    }
    let e = call(
        &c,
        "writes_for",
        json!({"contributor_id": BOSS, "acting_agent": "who:nobody"}),
    )
    .await
    .expect_err("a session cannot declare an unknown agent");
    assert!(e.contains("add_contributor"), "{e}");
}
