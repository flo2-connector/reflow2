//! A PROOF HOLE IS WORKED WITH A SERVED SKILL THAT AGREES WITH THE TOOLS —
//! `req:a-served-skill-works-a-proof-hole-refute-first-with-each-candidate-a-path-in-the-graph`
//! (option (d) of `dec:idea-an-agent-works-one-proof-hole-at-a-time-and-writes-every-try-down`:
//! root-cause's refute-first procedure run on the graph's fork structure),
//! over the MCP surface.
//!
//! - `find_skills` finds `work-a-proof-hole` from the ways a person asks, and
//!   `design-a-proof` keeps its own queries.
//! - Every field, value and tool the skill names to record a route is one the
//!   build SERVES: `add_requirement`'s `statement_kind: failed_route` and its
//!   five fields, `register_alternative`'s and `review_relations`' arguments,
//!   read from the published input schemas rather than from memory.
//! - The skill's recipe, run through the tools a client calls, reads as the
//!   skill says: the hole's decision point lands `proposed` and takes routes;
//!   the cap is discipline (a fourth open route is not refused); a refuter
//!   written into the decision point keeps its earlier wording; a route that
//!   fails is read under the hole and moves nothing; a change of route carries
//!   `CAUSES` from what forced it; the route that closes the hole leaves it
//!   `shown`, never `proven`, with its statement unchanged.

use reflow2_mcp::service::ReflowService;
use reflow2_mcp::skills::SKILLS;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "proof-hole-probe".to_string();
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
    async fn ok(&self, tool: &str, args: Value) -> Value {
        let shown = args.to_string();
        let arguments = args.as_object().cloned().unwrap_or_default();
        match self
            .client
            .call_tool(
                rmcp::model::CallToolRequestParams::new(tool.to_string()).with_arguments(arguments),
            )
            .await
        {
            Ok(r) if r.is_error.unwrap_or(false) => panic!(
                "`{tool}` {shown} was refused: {}",
                r.content
                    .iter()
                    .filter_map(|c| c.as_text().map(|t| t.text.clone()))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            Ok(r) => r.structured_content.unwrap_or(Value::Null),
            Err(e) => panic!("`{tool}` {shown} failed: {e}"),
        }
    }

    async fn node(&self, id: &str) -> Value {
        self.ok("get_node", json!({"id": id})).await["node"].clone()
    }

    async fn claim(&self, id: &str) -> Value {
        let r = self.ok("reasoning_report", json!({"claim": id})).await;
        r["claims"]
            .as_array()
            .expect("claims")
            .iter()
            .find(|c| c["id"] == id)
            .cloned()
            .unwrap_or_else(|| panic!("{id} is read back: {r:#}"))
    }

    /// The published input schema of one tool, as `tools/list` serves it.
    async fn input_schema(&self, tool: &str) -> Value {
        let tools = self.client.list_all_tools().await.expect("tools/list");
        let t = tools
            .iter()
            .find(|t| t.name == tool)
            .unwrap_or_else(|| panic!("{tool} is served"));
        Value::Object((*t.input_schema).clone())
    }
}

fn skill() -> &'static str {
    SKILLS
        .iter()
        .find(|s| s.name == "work-a-proof-hole")
        .expect("a served skill named work-a-proof-hole")
        .body
}

fn proof_skill() -> &'static str {
    SKILLS
        .iter()
        .find(|s| s.name == "design-a-proof")
        .expect("design-a-proof is served")
        .body
}

#[tokio::test]
async fn find_skills_finds_the_hole_skill_from_the_ways_people_ask() {
    let s = session().await;
    for query in [
        "close a hole in my proof",
        "this step of the proof does not go through, try routes to close it",
        "work one gap in a proof and keep the routes that fail",
    ] {
        let found = s
            .ok("find_skills", json!({"query": query, "limit": 3}))
            .await;
        assert_eq!(
            found["items"][0]["name"], "work-a-proof-hole",
            "{query}: {found:#}"
        );
    }
    // The proof skill keeps the queries that are its own.
    for query in ["design a proof", "prove a theorem step by step"] {
        let found = s
            .ok("find_skills", json!({"query": query, "limit": 3}))
            .await;
        assert_eq!(
            found["items"][0]["name"], "design-a-proof",
            "{query}: {found:#}"
        );
    }
}

/// The skill names a route's record exactly as the build serves it: each
/// field it tells an agent to pass is in the published input schema of the
/// tool it names, so a renamed field fails here and not in a session.
#[tokio::test]
async fn the_skill_names_each_record_as_the_build_serves_it() {
    let body = skill();
    let s = session().await;

    let req = s.input_schema("add_requirement").await;
    let props = req["properties"].as_object().expect("properties");
    for field in [
        "route_to",
        "fails_at",
        "fails_at_step",
        "fails_because",
        "route_rests_on",
        "would_hold_if",
    ] {
        assert!(props.contains_key(field), "add_requirement serves {field}");
        assert!(
            body.contains(&format!("`{field}`")),
            "the skill names `{field}`"
        );
    }
    let kinds = req.to_string();
    assert!(
        kinds.contains("\"failed_route\""),
        "statement_kind serves failed_route"
    );
    assert!(body.contains("`statement_kind` `failed_route`"));

    let alt = s.input_schema("register_alternative").await;
    for field in ["decision_id", "artifact_id", "name", "location"] {
        assert!(
            alt["properties"].get(field).is_some(),
            "register_alternative serves {field}"
        );
        assert!(
            body.contains(&format!("`{field}`")),
            "the skill names `{field}`"
        );
    }
    let rel = s.input_schema("review_relations").await.to_string();
    for field in ["relation", "other_id", "incoming", "evidence", "basis"] {
        assert!(
            rel.contains(&format!("\"{field}\"")),
            "review_relations serves {field}"
        );
        assert!(
            body.contains(&format!("`{field}`")),
            "the skill names `{field}`"
        );
    }

    for taught in [
        // The gates, read from the record, never from the agent's account.
        "never\nfrom your account",
        "The hole's statement does not change",
        "No new axiom and nothing new taken as given",
        "No step reads proven on your word",
        "Never pass `confirmed_by` or `confirmed_how`",
        "still hold at its end",
        "at least one step",
        // State, search, generate, refute, record.
        "State the hole as an OBSERVATION",
        "`failed_routes`",
        "the given sources",
        "At least three, and three is a floor",
        "a stated reason to be on the list",
        "No more than three routes are open at once",
        "name what would REFUTE it",
        "refute-first",
        "`without_step_or_reason`",
        "`with_step_and_reason`",
        "Never draw DEPENDS_ON",
        // A natural condition under which the route would go through, in
        // one line, never a trivial one, and checked against the literature.
        "natural",
        "a constant function",
        "literature check",
        "`CAUSES`",
        "names the source",
        // Closing and ending.
        "`collapse_decision`",
        "`ask_question`",
        "not\n  `proven`",
        "does not compute the most efficient",
        "what is closed, what is open, the next route",
        "Honest limits",
    ] {
        assert!(body.contains(taught), "the skill teaches {taught:?}");
    }
    // The proof skill points to it.
    assert!(
        proof_skill().contains("work-a-proof-hole"),
        "design-a-proof points to work-a-proof-hole"
    );
}

/// A small proof with one hole: a theorem resting on a lemma nothing shows.
async fn proof_with_a_hole(s: &Session) {
    s.ok(
        "add_constraint",
        json!({"id": "con:def-sum", "name": "Definition of S(n)",
               "statement": "S(0) = 0 and S(n+1) = S(n) + 2n + 1.",
               "statement_kind": "definition"}),
    )
    .await;
    s.ok(
        "add_requirement",
        json!({"id": "req:lemma-square", "name": "Lemma: S(n) = n²",
               "statement": "For every n, S(n) = n².", "statement_kind": "lemma"}),
    )
    .await;
    s.ok(
        "add_requirement",
        json!({"id": "req:thm-square", "name": "Theorem: S(n) is a perfect square",
               "statement": "For every n, S(n) is a perfect square.",
               "statement_kind": "theorem"}),
    )
    .await;
    s.ok(
        "add_inference",
        json!({"id": "inf:thm-from-lemma", "rule": "all", "premises": ["req:lemma-square"],
               "conclusion": "req:thm-square", "rule_name": "instantiation",
               "warrant": "n² is a square."}),
    )
    .await;
}

/// THE RECIPE THROUGH THE WIRE, step by step as the skill gives it.
#[tokio::test]
async fn the_recipe_reads_as_the_skill_says() {
    let s = session().await;
    proof_with_a_hole(&s).await;
    let mut used: Vec<&str> = Vec::new();

    // ① The hole, as observed: a lemma no step concludes.
    let hole = s.claim("req:lemma-square").await;
    used.push("reasoning_report");
    let statement_before = s.node("req:lemma-square").await["properties"]["statement"].clone();
    assert_ne!(hole["truth"], "holds", "{hole:#}");

    // Its decision point lands proposed, which is what takes routes.
    let opened = s
        .ok(
            "add_decision",
            json!({"id": "dec:close-lemma-square", "kind": "choice",
                   "name": "Which route closes Lemma: S(n) = n²?",
                   "decision": "The hole: req:lemma-square, S(n) = n² for every n, from the definition of S."}),
        )
        .await;
    used.push("add_decision");
    assert_eq!(
        s.node("dec:close-lemma-square").await["properties"]["status"],
        "proposed",
        "{opened:#}"
    );

    // ④ Routes registered under it. A fourth is not refused: the cap is the
    // skill's discipline, and the record shows how many are open.
    for (slug, name) in [
        ("route-telescoping", "Telescope the differences"),
        ("route-induction", "Induction on n"),
        ("route-closed-form", "Guess a closed form and check it"),
        ("route-counterexample", "Search for a counterexample"),
    ] {
        s.ok(
            "register_alternative",
            json!({"decision_id": "dec:close-lemma-square",
                   "artifact_id": format!("alt:{slug}"), "name": name,
                   "location": format!("this design: records named {slug}")}),
        )
        .await;
    }
    used.extend(["register_alternative", "alternatives_for"]);
    let alts = s
        .ok(
            "alternatives_for",
            json!({"decision_id": "dec:close-lemma-square"}),
        )
        .await;
    assert_eq!(
        alts["items"].as_array().map(Vec::len),
        Some(4),
        "four routes registered: {alts:#}"
    );

    // ⑤ The refuters, written into the decision point before the work: the
    // earlier wording is kept.
    let revised = s
        .ok(
            "add_decision",
            json!({"id": "dec:close-lemma-square", "echo": "node",
                   "decision": "The hole: req:lemma-square. Refuters: telescoping is refuted if the differences do not sum to n²; induction if the step from n to n+1 needs more than the definition."}),
        )
        .await;
    assert!(
        revised.to_string().contains("from the definition of S."),
        "the earlier wording is kept: {revised:#}"
    );

    // ⑦ A route that fails is a failed_route, read under the hole, and the
    // hole's reading does not move.
    s.ok(
        "add_requirement",
        json!({"id": "req:route-telescoping", "name": "Failed route: telescoping",
               "statement": "Sum the differences S(k+1) − S(k) and compare with n² directly.",
               "statement_kind": "failed_route", "route_to": "req:lemma-square",
               "fails_at": "the comparison of the sum of 2k + 1 with n²",
               "fails_because": "it needs the sum of the first n odd numbers, which is the lemma itself: circular",
               "route_rests_on": ["con:def-sum"]}),
    )
    .await;
    used.push("add_requirement");
    let mut after = s.claim("req:lemma-square").await;
    let routes = after["failed_routes"].as_array().expect("routes").clone();
    assert_eq!(routes.len(), 1, "{after:#}");
    assert_eq!(routes[0]["id"], "req:route-telescoping");
    after.as_object_mut().unwrap().remove("failed_routes");
    assert_eq!(after, hole, "the failed route moved the hole's reading");
    let report = s
        .ok("reasoning_report", json!({"claim": "req:lemma-square"}))
        .await;
    assert_eq!(report["failed_routes"]["total"], 1, "{report:#}");
    assert_eq!(report["failed_routes"]["with_step_and_reason"], 1);

    // ⑧ The change of route, caused by what forced it.
    s.ok(
        "add_decision",
        json!({"id": "dec:turn-telescoping-to-induction", "kind": "choice",
               "name": "Leave telescoping for induction on n",
               "decision": "Route left: telescoping. Route taken: induction on n. What changed: telescoping needs the lemma itself."}),
    )
    .await;
    s.ok(
        "review_relations",
        json!({"node_id": "dec:turn-telescoping-to-induction",
               "links": [{"relation": "CAUSES", "other_id": "req:route-telescoping",
                          "incoming": true, "basis": "observed",
                          "evidence": "The telescoping route reads circular, so it cannot close the hole."}]}),
    )
    .await;
    used.push("review_relations");
    let turn = s
        .ok(
            "get_node",
            json!({"id": "dec:turn-telescoping-to-induction",
                   "include_edges": {"edge_types": ["CAUSES"]}}),
        )
        .await;
    let causes = turn["edges"]["items"].as_array().expect("edges");
    assert!(
        causes
            .iter()
            .any(|e| e["from_id"] == "req:route-telescoping" && e["direction"] == "in"),
        "the turn is caused by the failed route: {turn:#}"
    );

    // ⑥ / ⑨ The route that closes the hole: shown, never proven on the
    // agent's word, and the hole's statement is the one it started with.
    s.ok(
        "add_inference",
        json!({"id": "inf:route-induction-1", "rule": "all", "premises": ["con:def-sum"],
               "conclusion": "req:lemma-square", "rule_name": "induction",
               "warrant": "S(0) = 0 = 0², and S(n+1) = n² + 2n + 1 = (n+1)²."}),
    )
    .await;
    used.push("add_inference");
    let closed = s.claim("req:lemma-square").await;
    assert_eq!(closed["truth"], "holds", "{closed:#}");
    let lemma = s.node("req:lemma-square").await;
    assert_eq!(lemma["properties"]["statement"], statement_before);
    let status = lemma.to_string();
    assert!(
        !status.contains("\"proven\""),
        "nothing confirmed it, so it never reads proven: {lemma:#}"
    );
    assert!(status.contains("\"shown\""), "{lemma:#}");
    // The decision point waits for the owner.
    assert_eq!(
        s.node("dec:close-lemma-square").await["properties"]["status"],
        "proposed"
    );

    // ⑩ The session ends on the record.
    s.ok("loop_status", json!({})).await;
    used.push("loop_status");

    let body = skill();
    for tool in used {
        assert!(
            body.contains(&format!("`{tool}`")),
            "the skill names `{tool}`, a call its recipe makes"
        );
    }
}
