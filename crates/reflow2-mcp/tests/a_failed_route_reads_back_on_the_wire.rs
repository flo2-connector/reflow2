//! A FAILED ROUTE, OVER THE MCP SURFACE —
//! `req:a-failed-route-is-a-record-of-its-own-read-back-with-the-reasoning`.
//! The core's `a_failed_route_is_a_record_of_its_own.rs` pins the readings;
//! this file pins what a client calls and reads:
//!
//! - `add_requirement` takes `statement_kind: failed_route` with `route_to`,
//!   `fails_at`, `fails_at_step`, `fails_because` and `route_rests_on`, and
//!   refuses those fields on any other statement, a reference to nothing, and
//!   a record that already takes part in a step — writing nothing;
//! - `get_node` reads the route (`statement_reading`, `failed_route`);
//! - `reasoning_report` reads it whole under the claim it was a route to, and
//!   tallies it, while the claim reads as it did;
//! - `search_design` finds it by its obstacle and names its `route_to`;
//! - `add_inference` refuses it as a premise.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "failed-route-probe".to_string();
        cfg
    }
}

struct Session {
    client: rmcp::service::RunningService<rmcp::service::RoleClient, Probe>,
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
    let client = Probe
        .serve((client_rx, client_tx))
        .await
        .expect("in-process handshake");
    Session { client }
}

impl Session {
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
            Err(e) => Err(format!("transport failure: {e}")),
        }
    }

    async fn ok(&self, tool: &str, args: Value) -> Value {
        let shown = args.to_string();
        self.call(tool, args)
            .await
            .unwrap_or_else(|e| panic!("`{tool}` {shown} was refused: {e}"))
    }

    async fn refused(&self, tool: &str, args: Value) -> String {
        let shown = args.to_string();
        match self.call(tool, args).await {
            Ok(v) => panic!("`{tool}` {shown} should have been refused, and answered {v:#}"),
            Err(e) => e,
        }
    }

    async fn node(&self, id: &str) -> Value {
        self.ok("get_node", json!({"id": id})).await["node"].clone()
    }
}

/// A small map with one hole: a definition, a lemma shown from it, and an
/// open conjecture nothing shows yet.
async fn map_with_a_hole(s: &Session) {
    s.ok(
        "add_constraint",
        json!({"id": "con:def-sequence", "name": "Definition of the sequence",
               "statement": "a(0) = 1 and a(n+1) = 2 a(n) + 1.",
               "statement_kind": "definition"}),
    )
    .await;
    s.ok(
        "add_requirement",
        json!({"id": "req:lemma-closed-form", "name": "Lemma: a(n) = 2^(n+1) − 1",
               "statement": "For every n, a(n) = 2^(n+1) − 1.", "statement_kind": "lemma"}),
    )
    .await;
    s.ok(
        "add_inference",
        json!({"id": "inf:closed-form", "rule": "all", "premises": ["con:def-sequence"],
               "conclusion": "req:lemma-closed-form", "rule_name": "induction",
               "warrant": "Unfold the recurrence and induct on n."}),
    )
    .await;
    s.ok(
        "add_requirement",
        json!({"id": "req:hole", "name": "Conjecture: infinitely many a(n) are prime",
               "statement": "Infinitely many terms of the sequence are prime.",
               "statement_kind": "conjecture"}),
    )
    .await;
}

fn route_args() -> Value {
    json!({
        "id": "req:route-by-congruences",
        "name": "Failed route: covering congruences",
        "statement": "Show infinitely many prime terms by a covering system of congruences \
                      on the exponent.",
        "statement_kind": "failed_route",
        "route_to": "req:hole",
        "fails_at": "the step that turns a covering system into a prime term",
        "fails_at_step": "req:lemma-closed-form",
        "fails_because": "a covering system proves terms COMPOSITE, never prime: the obstruction \
                          points the wrong way",
        "route_rests_on": ["req:lemma-closed-form", "con:def-sequence"],
    })
}

#[tokio::test]
async fn a_failed_route_is_written_read_and_found_through_the_tools() {
    let s = session().await;
    map_with_a_hole(&s).await;
    let before = s.ok("reasoning_report", json!({"claim": "req:hole"})).await;
    let hole_before = before["claims"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "req:hole")
        .cloned()
        .unwrap();
    assert!(before.get("failed_routes").is_none(), "{before:#}");

    s.ok("add_requirement", route_args()).await;

    // get_node reads the record whole.
    let got = s
        .ok("get_node", json!({"id": "req:route-by-congruences"}))
        .await;
    let n = &got["node"];
    assert_eq!(n["statement_reading"]["reads"], "failed_route", "{got:#}");
    assert_eq!(n["failed_route"]["route_to"]["id"], "req:hole", "{got:#}");
    assert_eq!(
        n["failed_route"]["route_to"]["name"],
        "Conjecture: infinitely many a(n) are prime"
    );
    assert_eq!(
        n["failed_route"]["fails_at_step"]["id"],
        "req:lemma-closed-form"
    );
    assert_eq!(n["failed_route"]["names_step_and_reason"], true);

    // reasoning_report reads it under the hole, and the hole reads as it did.
    let after = s.ok("reasoning_report", json!({"claim": "req:hole"})).await;
    let mut hole = after["claims"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "req:hole")
        .cloned()
        .unwrap();
    let routes = hole["failed_routes"].as_array().expect("routes").clone();
    assert_eq!(routes.len(), 1, "{after:#}");
    assert_eq!(routes[0]["id"], "req:route-by-congruences");
    assert!(
        routes[0]["fails_because"]
            .as_str()
            .unwrap()
            .contains("COMPOSITE")
    );
    hole.as_object_mut().unwrap().remove("failed_routes");
    assert_eq!(hole, hole_before, "the hole's reading moved");
    assert_eq!(hole["statement"]["reads"], "open");
    assert_eq!(after["failed_routes"]["total"], 1, "{after:#}");
    assert_eq!(after["failed_routes"]["with_step_and_reason"], 1);
    assert!(
        !after["unsupported"]
            .as_array()
            .unwrap()
            .iter()
            .any(|u| u["id"] == "req:route-by-congruences")
    );

    // search_design finds it by its obstacle, and says what it was a route to.
    let found = s
        .ok(
            "search_design",
            json!({"query": "covering system composite obstruction"}),
        )
        .await;
    let hit = found["hits"]
        .as_array()
        .unwrap()
        .iter()
        .find(|h| h["node_id"] == "req:route-by-congruences")
        .unwrap_or_else(|| panic!("the route is found: {found:#}"))
        .clone();
    assert_eq!(hit["route_to"], "req:hole", "{hit:#}");
    // Every other hit reads as before: no `route_to`.
    for h in found["hits"].as_array().unwrap() {
        if h["node_id"] != "req:route-by-congruences" {
            assert!(h.get("route_to").is_none(), "{h:#}");
        }
    }

    // It is never a premise.
    let e = s
        .refused(
            "add_inference",
            json!({"id": "inf:uses-the-route", "rule": "all",
                   "premises": ["req:route-by-congruences"], "conclusion": "req:hole"}),
        )
        .await;
    assert!(e.contains("is a failed route"), "{e}");
    assert!(
        s.node("inf:uses-the-route").await.is_null(),
        "nothing is written"
    );
}

#[tokio::test]
async fn a_failed_routes_fields_are_refused_where_they_do_not_fit_and_nothing_is_written() {
    let s = session().await;
    map_with_a_hole(&s).await;

    // On a statement of another kind.
    let mut lemma = route_args();
    lemma["id"] = json!("req:not-a-route");
    lemma["statement_kind"] = json!("lemma");
    let e = s.refused("add_requirement", lemma).await;
    assert!(e.contains("belong to a FAILED ROUTE only"), "{e}");
    assert!(
        s.node("req:not-a-route").await.is_null(),
        "nothing is written"
    );

    // A reference to nothing.
    let mut dangling = route_args();
    dangling["route_rests_on"] = json!(["req:no-such-lemma"]);
    let e = s.refused("add_requirement", dangling).await;
    assert!(e.contains("req:no-such-lemma"), "{e}");
    assert!(
        s.node("req:route-by-congruences").await.is_null(),
        "nothing is written"
    );
    let mut to_nothing = route_args();
    to_nothing["route_to"] = json!("req:no-such-claim");
    let e = s.refused("add_requirement", to_nothing).await;
    assert!(e.contains("req:no-such-claim"), "{e}");
    assert!(s.node("req:route-by-congruences").await.is_null());

    // A route to itself.
    let mut itself = route_args();
    itself["route_to"] = json!("req:route-by-congruences");
    let e = s.refused("add_requirement", itself).await;
    assert!(e.contains("names 'req:route-by-congruences' itself"), "{e}");

    // A record that already takes part in a step cannot become one.
    let e = s
        .refused(
            "add_requirement",
            json!({"id": "req:lemma-closed-form", "statement_kind": "failed_route",
                   "route_to": "req:hole"}),
        )
        .await;
    assert!(e.contains("cannot become a failed route"), "{e}");
    assert_eq!(
        s.node("req:lemma-closed-form").await["properties"]["statement_kind"],
        "lemma",
        "nothing is written"
    );

    // Revising a route's fields needs no kind repeated: it is one already.
    s.ok("add_requirement", route_args()).await;
    s.ok(
        "add_requirement",
        json!({"id": "req:route-by-congruences",
               "fails_because": "the obstruction points the wrong way"}),
    )
    .await;
    assert_eq!(
        s.node("req:route-by-congruences").await["properties"]["fails_because"],
        "the obstruction points the wrong way"
    );
}

/// `req:a-failed-route-records-the-natural-condition-under-which-it-would-hold`:
/// `would_hold_if`, the natural condition under which the route WOULD go
/// through, is written by add_requirement, read back with the route by
/// get_node and under its claim by reasoning_report, and moves no reading.
/// On any other statement it is refused, and nothing is written.
#[tokio::test]
async fn a_route_records_what_would_make_it_go_through_and_nothing_moves() {
    let s = session().await;
    map_with_a_hole(&s).await;
    const CONDITION: &str = "if the exponents are restricted to the primes";
    let before = s.ok("reasoning_report", json!({"claim": "req:hole"})).await;
    let hole_before = before["claims"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "req:hole")
        .cloned()
        .unwrap();

    let mut args = route_args();
    args["would_hold_if"] = json!(CONDITION);
    s.ok("add_requirement", args).await;

    let n = s.node("req:route-by-congruences").await;
    assert_eq!(n["failed_route"]["would_hold_if"], CONDITION, "{n:#}");
    assert_eq!(n["statement_reading"]["reads"], "failed_route");

    let after = s.ok("reasoning_report", json!({"claim": "req:hole"})).await;
    let mut hole = after["claims"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "req:hole")
        .cloned()
        .unwrap();
    assert_eq!(
        hole["failed_routes"][0]["would_hold_if"], CONDITION,
        "{after:#}"
    );
    hole.as_object_mut().unwrap().remove("failed_routes");
    assert_eq!(hole, hole_before, "the hole's reading moved");
    assert_eq!(hole["statement"]["reads"], "open");

    // Refused on a statement that is not a failed route.
    let e = s
        .refused(
            "add_requirement",
            json!({"id": "req:not-a-route", "name": "A lemma", "statement": "A lemma.",
                   "statement_kind": "lemma", "would_hold_if": CONDITION}),
        )
        .await;
    assert!(e.contains("`would_hold_if`"), "{e}");
    assert!(e.contains("belong to a FAILED ROUTE only"), "{e}");
    assert!(
        s.node("req:not-a-route").await.is_null(),
        "nothing is written"
    );
}
