//! A PROOF SEARCH IS READ FROM A ONE-PAGE MAP, over the MCP surface. Three
//! requirements, all governed by
//! `dec:the-aa-attempts-lessons-are-built-into-reflow2-before-increment-5`:
//!
//! 1. `req:a-proof-hole-records-the-claim-it-serves-and-whether-closing-it-closes-that-claim`
//!    — a hole's decision point names the claim it is the hole for (`hole`),
//!    the claim it serves (`serves`) and what closing it does to that claim
//!    (`closing_it`: closes, narrows or restates). The map says, along the
//!    path to the target, whether closing the hole could close the target, and
//!    an unsaid link reads unsaid, never `closes`.
//! 2. `req:a-proof-round-starts-from-a-one-page-map-of-its-search` —
//!    `reasoning_report` with `map: true` reads the target, the tree of holes
//!    under the claims they serve with each hole's routes (open with its
//!    refuter, failed in one line, waiting with its reason), each closed
//!    result in one line, and the rounds since progress, in one reply at any
//!    size. The arguments are not in it.
//! 3. `req:a-proof-search-checks-convergence-and-goes-back-to-the-highest-fork-with-an-untried-branch`
//!    — reflow2 counts the rounds since the last progress that served the
//!    target (a round is a `search_round` finding about the target; only a
//!    step closed or a route failed on a hole whose closing could close the
//!    target is progress), lists the forks holding a waiting branch highest
//!    first, and the served skill work-a-proof-hole reads that at the start of
//!    each round and goes back at N.
//!
//! The fixture is a small proof search built here, never a real attempt.

use reflow2_mcp::skills::SKILLS;
use rmcp::ServiceExt;
use serde_json::{Value, json};

use reflow2_mcp::service::ReflowService;

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "proof-search-map-probe".to_string();
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

    /// The map of the search under the target.
    async fn map(&self) -> Value {
        self.ok("reasoning_report", json!({"claim": "req:t", "map": true}))
            .await
    }
}

/// The fixture's decision points, for a later one judged distinct from them.
const HOLES: [&str; 6] = [
    "dec:close-t",
    "dec:close-h-sym",
    "dec:close-h-sym-small",
    "dec:close-lem-a",
    "dec:close-lem-b",
    "dec:close-lem-c",
];

/// The words of one step's warrant: an ARGUMENT, which the map must never
/// carry (it is fetched on demand by id).
const WARRANT: &str = "Every term is built from integers by the recurrence, so by induction \
                       on n each term is an integer: ARGUMENT-PROSE-THE-MAP-NEVER-CARRIES.";

/// A small proof search, built through the tools a client calls:
///
/// ```text
/// req:t  Conjecture T (the target)          dec:close-t, the target's own hole
///   ├─ req:h-sym   T for symmetric sequences dec:close-h-sym   narrows T
///   │     └─ req:h-sym-small                 dec:close-h-sym-small closes h-sym
///   └─ req:lem-a   every sequence is bounded dec:close-lem-a   closes T
///         ├─ req:lem-b  a uniform bound      dec:close-lem-b   closes lem-a
///         └─ req:lem-c  bounded on average   dec:close-lem-c   (unsaid)
/// ```
///
/// Routes under dec:close-t: one open with its refuter, one failed, one
/// waiting with its reason; under dec:close-lem-a one waiting. req:lem-d is
/// a closed result, shown by a step whose warrant is an argument.
async fn search(s: &Session) {
    s.ok(
        "add_constraint",
        json!({"id": "con:def-family", "name": "Definition of the family F",
               "statement": "F is the set of integer sequences obeying a(n+2) = a(n+1) - a(n).",
               "statement_kind": "definition"}),
    )
    .await;
    for (id, name, statement, kind) in [
        (
            "req:t",
            "Conjecture T: every sequence in F is eventually periodic",
            "Every integer sequence obeying the recurrence of F is eventually periodic.",
            "conjecture",
        ),
        (
            "req:h-sym",
            "Conjecture T restricted to palindromic seeds",
            "Every sequence of F whose two seed terms are equal is eventually periodic.",
            "conjecture",
        ),
        (
            "req:h-sym-small",
            "Palindromic seeds below ten",
            "For seed values under ten with equal seeds the orbit repeats.",
            "conjecture",
        ),
        (
            "req:lem-a",
            "Lemma A: every member of F is bounded",
            "Each sequence of the family takes only finitely many values.",
            "lemma",
        ),
        (
            "req:lem-b",
            "Lemma B: a uniform bound by the seeds",
            "Every term is at most twice the larger seed in absolute value.",
            "lemma",
        ),
        (
            "req:lem-c",
            "Lemma C: an averaged bound over windows",
            "The mean of any six consecutive terms vanishes.",
            "lemma",
        ),
        (
            "req:lem-d",
            "Lemma D: integrality of the terms",
            "Each term of each member is an integer.",
            "lemma",
        ),
    ] {
        s.ok(
            "add_requirement",
            json!({"id": id, "name": name, "statement": statement, "statement_kind": kind}),
        )
        .await;
    }
    s.ok(
        "add_inference",
        json!({"id": "inf:d", "rule": "all", "premises": ["con:def-family"],
               "conclusion": "req:lem-d", "rule_name": "induction", "warrant": WARRANT}),
    )
    .await;

    // The holes, each a decision point that says what it serves.
    let mut opened: Vec<String> = Vec::new();
    for (id, name, hole, serves, closing) in [
        (
            "dec:close-t",
            "Which route shows the whole family is eventually periodic?",
            "req:t",
            None,
            None,
        ),
        (
            "dec:close-h-sym",
            "Which route settles the palindromic-seed case?",
            "req:h-sym",
            Some("req:t"),
            Some("narrows"),
        ),
        (
            "dec:close-h-sym-small",
            "Which computation covers seeds under ten?",
            "req:h-sym-small",
            Some("req:h-sym"),
            Some("closes"),
        ),
        (
            "dec:close-lem-a",
            "Which argument bounds every member?",
            "req:lem-a",
            Some("req:t"),
            Some("closes"),
        ),
        (
            "dec:close-lem-b",
            "How is the seed bound obtained?",
            "req:lem-b",
            Some("req:lem-a"),
            Some("closes"),
        ),
        (
            "dec:close-lem-c",
            "Does the windowed mean argument work?",
            "req:lem-c",
            Some("req:lem-a"),
            None,
        ),
    ] {
        // Six decision points that each ask which route closes a hole read
        // alike; each was judged a different hole.
        let mut args = json!({"id": id, "kind": "choice", "name": name,
                              "decision": format!("The hole as observed: {hole}, verbatim."),
                              "hole": hole, "distinct_from": opened.clone()});
        opened.push(id.to_string());
        if let Some(sv) = serves {
            args["serves"] = json!(sv);
        }
        if let Some(c) = closing {
            args["closing_it"] = json!(c);
        }
        s.ok("add_decision", args).await;
    }

    // Routes under the target's own hole: open, failed, waiting.
    s.ok(
        "register_alternative",
        json!({"decision_id": "dec:close-t", "artifact_id": "alt:route-induction",
               "name": "Induction on the period", "location": "this design: route-induction",
               "refuter": "a member of F whose first six terms never recur"}),
    )
    .await;
    s.ok(
        "register_alternative",
        json!({"decision_id": "dec:close-t", "artifact_id": "alt:route-cases",
               "name": "Split on the sign of the seeds", "location": "this design: route-cases"}),
    )
    .await;
    s.ok(
        "register_alternative",
        json!({"decision_id": "dec:close-t", "artifact_id": "alt:route-transform",
               "name": "A generating-function transform", "location": "this design: route-transform",
               "waiting": true, "promise_rank": 2,
               "promise": "the transform turns the recurrence into a rational function"}),
    )
    .await;
    s.ok(
        "register_alternative",
        json!({"decision_id": "dec:close-lem-a", "artifact_id": "alt:route-a-compactness",
               "name": "Compactness of the seed square", "location": "this design: route-a-compactness",
               "waiting": true, "promise_rank": 1,
               "promise": "a bounded seed square forces finitely many states"}),
    )
    .await;
    s.ok(
        "register_alternative",
        json!({"decision_id": "dec:close-h-sym", "artifact_id": "alt:route-sym-average",
               "name": "Average the palindromic orbit", "location": "this design: route-sym-average",
               "refuter": "an equal-seed orbit whose average drifts"}),
    )
    .await;
}

/// The failed route on the split of signs, toward the target itself.
async fn route_cases_fails(s: &Session) {
    s.ok(
        "add_requirement",
        json!({"id": "req:route-cases", "name": "Failed route: split on the sign of the seeds",
               "statement": "Split on the signs of the two seeds and show each case recurs.",
               "statement_kind": "failed_route", "route_to": "req:t",
               "fails_at": "the case of seeds of opposite sign",
               "fails_because": "the sign pattern does not fix the period, so the cases do not \
                                 close"}),
    )
    .await;
}

fn hole<'a>(map: &'a Value, id: &str) -> &'a Value {
    map["holes"]
        .as_array()
        .unwrap_or_else(|| panic!("the map lists its holes: {map:#}"))
        .iter()
        .find(|h| h["id"] == id)
        .unwrap_or_else(|| panic!("the map holds the hole {id}: {map:#}"))
}

fn ids(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x["id"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

// ── 1. A hole names the claim it serves and what closing it does ──────────

#[tokio::test]
async fn a_hole_names_the_claim_it_serves_and_what_closing_it_does() {
    let s = session().await;
    search(&s).await;

    let n = s.node("dec:close-h-sym").await;
    assert_eq!(n["properties"]["hole"], "req:h-sym", "{n:#}");
    assert_eq!(n["properties"]["serves"], "req:t", "{n:#}");
    assert_eq!(n["properties"]["closing_it"], "narrows", "{n:#}");

    // Unsaid stays unsaid: no default is stored.
    let c = s.node("dec:close-lem-c").await;
    assert!(c["properties"].get("closing_it").is_none(), "{c:#}");

    // Refused, writing nothing: a value outside the three, a claim the design
    // does not hold, what closing does with no claim it does it to, and a hole
    // that serves itself.
    let e = s
        .refused(
            "add_decision",
            json!({"id": "dec:close-x", "kind": "choice", "name": "A hole with a fourth effect",
                   "decision": "x", "serves": "req:t", "closing_it": "helps"}),
        )
        .await;
    assert!(e.contains("closing_it"), "{e}");
    let e = s
        .refused(
            "add_decision",
            json!({"id": "dec:close-y", "kind": "choice", "name": "A hole serving nothing held",
                   "decision": "y", "serves": "req:no-such-claim", "closing_it": "closes"}),
        )
        .await;
    assert!(e.contains("req:no-such-claim"), "{e}");
    let e = s
        .refused(
            "add_decision",
            json!({"id": "dec:close-z", "kind": "choice", "name": "A hole closing what it serves",
                   "decision": "z", "closing_it": "closes"}),
        )
        .await;
    assert!(e.contains("serves"), "{e}");
    let e = s
        .refused(
            "add_decision",
            json!({"id": "dec:close-lem-b", "serves": "dec:close-lem-b"}),
        )
        .await;
    assert!(e.contains("itself"), "{e}");
    for id in ["dec:close-x", "dec:close-y", "dec:close-z"] {
        assert!(s.node(id).await.is_null(), "{id} was written");
    }
}

#[tokio::test]
async fn the_map_says_along_the_path_whether_closing_a_hole_could_close_the_target() {
    let s = session().await;
    search(&s).await;
    let map = s.map().await;

    let t = hole(&map, "dec:close-t");
    assert_eq!(t["can_close_target"], "yes", "the target's own hole: {t:#}");
    let a = hole(&map, "dec:close-lem-a");
    assert_eq!(a["closing_it"], "closes", "{a:#}");
    assert_eq!(a["can_close_target"], "yes", "{a:#}");
    let b = hole(&map, "dec:close-lem-b");
    assert_eq!(b["can_close_target"], "yes", "closes, then closes: {b:#}");

    // A hole that only narrows is SHOWN, never hidden, and cannot close it.
    let sym = hole(&map, "dec:close-h-sym");
    assert_eq!(sym["closing_it"], "narrows", "{sym:#}");
    assert_eq!(sym["can_close_target"], "no", "{sym:#}");
    // Nor can one that closes a hole that only narrows: the path decides.
    let small = hole(&map, "dec:close-h-sym-small");
    assert_eq!(small["closing_it"], "closes", "{small:#}");
    assert_eq!(small["can_close_target"], "no", "{small:#}");
    let why = small["why"].as_str().unwrap_or_default();
    assert!(
        why.contains("narrows"),
        "the path names the narrowing link: {small:#}"
    );

    // Unsaid reads unsaid, never closes.
    let c = hole(&map, "dec:close-lem-c");
    assert_eq!(c["closing_it"], "unsaid", "{c:#}");
    assert_eq!(c["can_close_target"], "unsaid", "{c:#}");

    // A restatement is shown as one: closing an equivalent form closes the
    // claim, and the map says that is what it is.
    s.ok(
        "add_requirement",
        json!({"id": "req:t-matrix", "name": "T as a matrix power statement",
               "statement": "Some power of the companion matrix of F is the identity.",
               "statement_kind": "conjecture", "distinct_from": ["req:t"]}),
    )
    .await;
    s.ok(
        "add_decision",
        json!({"id": "dec:close-t-matrix", "kind": "choice",
               "name": "Which route finds the identity power of the companion matrix?",
               "decision": "The hole as observed: req:t-matrix.", "hole": "req:t-matrix",
               "serves": "req:t", "closing_it": "restates", "distinct_from": HOLES}),
    )
    .await;
    let map = s.map().await;
    let m = hole(&map, "dec:close-t-matrix");
    assert_eq!(m["closing_it"], "restates", "{m:#}");
    assert_eq!(m["can_close_target"], "yes", "{m:#}");
    assert!(
        m["why"].as_str().unwrap_or_default().contains("restates"),
        "{m:#}"
    );
}

// ── 2. A round starts from a one-page map ──────────────────────────────────

#[tokio::test]
async fn a_round_starts_from_a_one_page_map_of_its_search() {
    let s = session().await;
    search(&s).await;
    route_cases_fails(&s).await;
    let map = s.map().await;

    // The target and how it reads.
    assert_eq!(map["target"]["id"], "req:t", "{map:#}");
    assert_eq!(map["target"]["reads"], "open", "{map:#}");
    assert!(
        map["target"]["says"]
            .as_str()
            .unwrap_or_default()
            .contains("eventually periodic"),
        "{map:#}"
    );

    // The tree of holes, highest first, each under the claim it serves.
    let order: Vec<String> = ids(&map["holes"]);
    assert_eq!(
        order.first().map(String::as_str),
        Some("dec:close-t"),
        "{map:#}"
    );
    let pos = |id: &str| order.iter().position(|x| x == id).unwrap();
    assert!(pos("dec:close-lem-a") < pos("dec:close-lem-b"), "{order:?}");
    assert!(
        pos("dec:close-h-sym") < pos("dec:close-h-sym-small"),
        "{order:?}"
    );
    assert_eq!(hole(&map, "dec:close-t")["depth"], 0);
    assert_eq!(hole(&map, "dec:close-lem-a")["depth"], 1);
    assert_eq!(hole(&map, "dec:close-lem-b")["depth"], 2);
    assert_eq!(hole(&map, "dec:close-lem-b")["serves"], "req:lem-a");
    assert_eq!(hole(&map, "dec:close-lem-b")["parent"], "dec:close-lem-a");

    // Each hole's routes: open with its refuter, failed in one line with
    // where and why, waiting with its reason.
    let routes = &hole(&map, "dec:close-t")["routes"];
    let open = &routes["open"];
    assert_eq!(ids(open), vec!["alt:route-induction"], "{routes:#}");
    assert_eq!(
        open[0]["refuter"], "a member of F whose first six terms never recur",
        "{routes:#}"
    );
    let failed = &routes["failed"];
    assert_eq!(ids(failed), vec!["req:route-cases"], "{routes:#}");
    let line = failed[0]["fails"].as_str().unwrap_or_default();
    assert!(line.contains("opposite sign"), "where: {line}");
    assert!(line.contains("does not fix the period"), "why: {line}");
    assert!(!line.contains('\n'), "one line: {line}");
    let waiting = &routes["waiting"];
    assert_eq!(ids(waiting), vec!["alt:route-transform"], "{routes:#}");
    assert_eq!(waiting[0]["rank"], 2, "{routes:#}");
    assert!(
        waiting[0]["promise"]
            .as_str()
            .unwrap_or_default()
            .contains("rational function"),
        "{routes:#}"
    );
    // A route with no refuter said reads so.
    let sym_open = &hole(&map, "dec:close-h-sym")["routes"]["open"];
    assert_eq!(
        sym_open[0]["refuter"],
        "an equal-seed orbit whose average drifts"
    );

    // Each closed result in one line.
    let closed = map["closed"].as_array().expect("closed results");
    let d = closed
        .iter()
        .find(|c| c["id"] == "req:lem-d")
        .unwrap_or_else(|| panic!("lem-d is a closed result: {map:#}"));
    assert_eq!(d["reads"], "shown", "{d:#}");
    assert_eq!(
        d["off_path"], true,
        "shown, and no step joins it to the target or a hole yet: {d:#}"
    );
    assert!(
        !closed.iter().any(|c| c["id"] == "con:def-family"),
        "a definition is a foundation, not a result: {map:#}"
    );

    // The arguments are not in it: they are fetched on demand by id.
    let text = map.to_string();
    assert!(
        !text.contains("ARGUMENT-PROSE-THE-MAP-NEVER-CARRIES"),
        "a step's warrant is an argument, fetched by id: {map:#}"
    );
    assert!(
        map["fetch"]
            .as_str()
            .unwrap_or_default()
            .contains("get_node"),
        "{map:#}"
    );
}

#[tokio::test]
async fn the_map_fits_one_reply_at_any_size() {
    let s = session().await;
    search(&s).await;
    // A large attempt: 120 holes under the target, each with six routes whose
    // names and failures are long, written in two batches.
    let long = "a long description of what was tried, ".repeat(8);
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for i in 0..120 {
        nodes.push(json!({"node_type": "Requirement", "id": format!("req:sub-{i}"),
            "props": {"name": format!("Sub-claim {i} {long}"), "statement": format!("Sub {i} {long}"),
                      "statement_kind": "conjecture"}}));
        nodes.push(
            json!({"node_type": "Decision", "id": format!("dec:close-sub-{i}"),
            "props": {"name": format!("Hole {i} {long}"), "decision": long.clone(),
                      "kind": "choice", "hole": format!("req:sub-{i}"), "serves": "req:t",
                      "closing_it": if i % 2 == 0 { "closes" } else { "narrows" }}}),
        );
        for j in 0..6 {
            let alt = format!("alt:route-sub-{i}-{j}");
            let mut props = json!({"name": format!("Route {i}.{j} {long}"),
                                   "artifact_type": "model", "location": "this design"});
            if j == 5 {
                props["waiting"] = json!(true);
                props["promise"] = json!(long.clone());
            } else {
                props["refuter"] = json!(long.clone());
            }
            nodes.push(json!({"node_type": "Artifact", "id": alt, "props": props}));
            edges.push(json!({"edge_type": "GOVERNED_BY", "from_id": alt,
                              "to_id": format!("dec:close-sub-{i}")}));
            if j < 3 {
                nodes.push(json!({"node_type": "Requirement",
                    "id": format!("req:route-sub-{i}-{j}"),
                    "props": {"name": format!("Failed {i}.{j}"), "statement": long.clone(),
                              "statement_kind": "failed_route", "route_to": format!("req:sub-{i}"),
                              "fails_at": long.clone(), "fails_because": long.clone()}}));
            }
        }
    }
    s.ok("create_nodes", json!({"nodes": nodes})).await;
    s.ok("create_edges", json!({"edges": edges})).await;

    let map = s.map().await;
    let chars = map.to_string().len();
    assert!(chars <= 30_000, "one reply at any size: {chars} characters");
    assert_eq!(map["counts"]["holes"], 126, "{}", map["counts"]);
    assert!(
        !map["page"]["withheld"]
            .as_array()
            .map(Vec::is_empty)
            .unwrap_or(true),
        "what did not fit is NAMED, never silently dropped: {}",
        map["page"]
    );
    // The top of the tree is always there, so a round knows where it is.
    assert_eq!(map["target"]["id"], "req:t");
    assert!(
        ids(&map["holes"]).contains(&"dec:close-t".to_string()),
        "{}",
        map["page"]
    );
    assert!(
        !map["forks_with_an_untried_branch"]
            .as_array()
            .map(Vec::is_empty)
            .unwrap_or(true),
        "the forks to go back to survive the cut: {}",
        map["page"]
    );
}

// ── 3. Convergence: rounds since progress, and the forks to go back to ────

async fn round(s: &Session, n: usize, did: &str) {
    s.ok(
        "record_finding",
        json!({"id": format!("fact:round-{n}"), "subject_id": "req:t",
               "fact_type": "search_round",
               "name": format!("Round {n} of the search on T"),
               "statement": format!("Round {n} ended: {did}")}),
    )
    .await;
}

#[tokio::test]
async fn rounds_since_progress_count_only_progress_that_could_close_the_target() {
    let s = session().await;
    search(&s).await;

    // No round recorded: the count is unsaid, never zero.
    let map = s.map().await;
    assert!(
        map["rounds"]["since_progress"].is_null(),
        "{}",
        map["rounds"]
    );
    assert_eq!(map["rounds"]["recorded"], 0, "{}", map["rounds"]);

    // Round 1: a route toward the target fails. That is progress on it.
    route_cases_fails(&s).await;
    round(&s, 1, "the sign split failed at opposite seeds").await;
    // Round 2: work on the palindromic special case only, which narrows the
    // target: a failed route there and a step shown there are NOT progress
    // toward the target, though a gate counting any step would count them.
    s.ok(
        "add_requirement",
        json!({"id": "req:route-sym-parity", "name": "Failed route: parity of palindromic orbits",
               "statement": "Show equal-seed orbits recur by a parity count.",
               "statement_kind": "failed_route", "route_to": "req:h-sym",
               "fails_at": "the parity count at even seeds",
               "fails_because": "even seeds give no parity obstruction"}),
    )
    .await;
    s.ok(
        "add_inference",
        json!({"id": "inf:h-sym-small", "rule": "all", "premises": ["con:def-family"],
               "conclusion": "req:h-sym-small", "rule_name": "finite check",
               "warrant": "The equal-seed orbits under ten are listed and each repeats."}),
    )
    .await;
    round(&s, 2, "the palindromic case narrowed, nothing on T itself").await;
    // Round 3: nothing at all.
    round(&s, 3, "no step closed and no route failed").await;

    let map = s.map().await;
    let r = &map["rounds"];
    assert_eq!(r["recorded"], 3, "{r:#}");
    assert_eq!(
        r["since_progress"], 2,
        "rounds 2 and 3 served nothing: {r:#}"
    );
    assert_eq!(r["last_progress"]["round"], 1, "{r:#}");
    assert_eq!(r["last_progress"]["record"], "req:route-cases", "{r:#}");

    // Round 4, in progress: a route on a hole that could close the target fails.
    s.ok(
        "add_requirement",
        json!({"id": "req:route-b-telescoping", "name": "Failed route: telescope the recurrence",
               "statement": "Telescope the recurrence to bound each term by the seeds.",
               "statement_kind": "failed_route", "route_to": "req:lem-b",
               "fails_at": "the telescoped sum at step six",
               "fails_because": "the sum cancels and gives no bound"}),
    )
    .await;
    let map = s.map().await;
    let r = &map["rounds"];
    assert_eq!(r["since_progress"], 2, "closed rounds only: {r:#}");
    assert_eq!(
        r["this_round"]["progress"],
        json!(["req:route-b-telescoping"]),
        "{r:#}"
    );
    round(&s, 4, "the telescoping route on Lemma B failed").await;
    // Round 5: a step closed on a hole that closes the target's lemma.
    s.ok(
        "add_inference",
        json!({"id": "inf:b", "rule": "all", "premises": ["con:def-family", "req:lem-d"],
               "conclusion": "req:lem-b", "rule_name": "direct computation",
               "warrant": "Six steps of the recurrence return the seeds with alternating signs."}),
    )
    .await;
    round(&s, 5, "Lemma B shown").await;
    let map = s.map().await;
    let r = &map["rounds"];
    assert_eq!(r["since_progress"], 0, "{r:#}");
    assert_eq!(r["last_progress"]["round"], 5, "{r:#}");
    assert_eq!(r["last_progress"]["record"], "inf:b", "{r:#}");
}

#[tokio::test]
async fn the_forks_with_an_untried_branch_are_listed_highest_first() {
    let s = session().await;
    search(&s).await;
    let map = s.map().await;
    let forks = map["forks_with_an_untried_branch"]
        .as_array()
        .unwrap_or_else(|| panic!("{map:#}"));
    let order: Vec<&str> = forks.iter().filter_map(|f| f["hole"].as_str()).collect();
    assert_eq!(order, vec!["dec:close-t", "dec:close-lem-a"], "{map:#}");
    assert_eq!(forks[0]["depth"], 0);
    assert_eq!(forks[0]["can_close_target"], "yes");
    assert_eq!(forks[0]["waiting"][0]["id"], "alt:route-transform");
    assert!(
        forks[0]["waiting"][0]["promise"]
            .as_str()
            .unwrap_or_default()
            .contains("rational function"),
        "{map:#}"
    );

    // Opening the waiting route: `waiting` is unset, its refuter written,
    // and the fork no longer holds an untried branch.
    s.ok(
        "add_artifact",
        json!({"id": "alt:route-transform", "waiting": null,
               "refuter": "a member of F whose generating function has a pole off the unit circle"}),
    )
    .await;
    let map = s.map().await;
    let open = ids(&hole(&map, "dec:close-t")["routes"]["open"]);
    assert!(open.contains(&"alt:route-transform".to_string()), "{map:#}");
    let order: Vec<String> = map["forks_with_an_untried_branch"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|f| f["hole"].as_str().map(str::to_string))
        .collect();
    assert_eq!(order, vec!["dec:close-lem-a"], "{map:#}");
}

// ── 4. Every idea charted at its fork, worked by stated rank ──────────────

async fn chart(s: &Session, fork: &str, id: &str, name: &str, rank: Option<i64>) {
    let mut args = json!({"decision_id": fork, "artifact_id": id, "name": name,
                          "location": "this design", "waiting": true});
    if let Some(r) = rank {
        args["promise_rank"] = json!(r);
        args["promise"] = json!(format!("{name}, ranked {r}"));
    }
    s.ok("register_alternative", args).await;
}

#[tokio::test]
async fn the_untried_branches_are_ranked_by_what_closing_them_does_then_by_stated_rank() {
    let s = session().await;
    search(&s).await;
    s.ok(
        "add_requirement",
        json!({"id": "req:t-matrix", "name": "T as a matrix power statement",
               "statement": "Some power of the companion matrix of F is the identity.",
               "statement_kind": "conjecture", "distinct_from": ["req:t"]}),
    )
    .await;
    s.ok(
        "add_decision",
        json!({"id": "dec:close-t-matrix", "kind": "choice",
               "name": "Which route finds the identity power of the companion matrix?",
               "decision": "The hole as observed: req:t-matrix.", "hole": "req:t-matrix",
               "serves": "req:t", "closing_it": "restates", "distinct_from": HOLES}),
    )
    .await;
    // One candidate at each kind of fork, each ranked first at its fork,
    // and one charted with no reason and no rank: it is KEPT, unranked.
    chart(
        &s,
        "dec:close-t-matrix",
        "alt:route-matrix-order",
        "Order of the companion matrix",
        Some(1),
    )
    .await;
    chart(
        &s,
        "dec:close-lem-c",
        "alt:route-c-window",
        "Sum over six-term windows",
        Some(1),
    )
    .await;
    chart(
        &s,
        "dec:close-h-sym",
        "alt:route-sym-mod3",
        "Palindromic orbits modulo three",
        Some(1),
    )
    .await;
    chart(
        &s,
        "dec:close-lem-b",
        "alt:route-b-unranked",
        "Something with the seeds",
        None,
    )
    .await;

    let map = s.map().await;
    let order: Vec<String> = ids(&map["untried_branches_by_promise"]);
    // Closing paths first (the stated rank before the higher fork), then a
    // restatement, then an unsaid link; a narrowing branch comes last and is
    // past the top five.
    assert_eq!(
        order,
        vec![
            "alt:route-a-compactness",
            "alt:route-transform",
            "alt:route-b-unranked",
            "alt:route-matrix-order",
            "alt:route-c-window",
        ],
        "{:#}",
        map["untried_branches_by_promise"]
    );
    assert_eq!(map["counts"]["waiting"], 6, "{}", map["counts"]);
    let first = &map["untried_branches_by_promise"][0];
    assert_eq!(first["hole"], "dec:close-lem-a", "{first:#}");
    assert_eq!(first["rank"], 1, "{first:#}");
    let unranked = &map["untried_branches_by_promise"][2];
    assert!(
        unranked.get("rank").is_none(),
        "unranked reads unranked: {unranked:#}"
    );
    let b = hole(&map, "dec:close-lem-b");
    assert_eq!(
        ids(&b["routes"]["waiting"]),
        vec!["alt:route-b-unranked"],
        "{b:#}"
    );
}

#[tokio::test]
async fn the_map_shows_the_top_of_each_fork_with_its_rank_and_the_rest_by_id() {
    let s = session().await;
    search(&s).await;
    // dec:close-t already charts alt:route-transform at rank 2.
    chart(
        &s,
        "dec:close-t",
        "alt:route-t-fourth",
        "Bound the period by the seeds",
        Some(4),
    )
    .await;
    chart(
        &s,
        "dec:close-t",
        "alt:route-t-first",
        "Reduce modulo a prime",
        Some(1),
    )
    .await;
    chart(
        &s,
        "dec:close-t",
        "alt:route-t-unranked",
        "Try a computer search",
        None,
    )
    .await;
    chart(
        &s,
        "dec:close-t",
        "alt:route-t-third",
        "Use the characteristic roots",
        Some(3),
    )
    .await;
    let map = s.map().await;
    let t = hole(&map, "dec:close-t");
    let top = &t["routes"]["waiting"];
    assert_eq!(
        ids(top),
        vec![
            "alt:route-t-first",
            "alt:route-transform",
            "alt:route-t-third"
        ],
        "{t:#}"
    );
    let ranks: Vec<i64> = top
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|w| w["rank"].as_i64())
        .collect();
    assert_eq!(ranks, vec![1, 2, 3], "{t:#}");
    assert_eq!(
        t["more_waiting"],
        json!(["alt:route-t-fourth", "alt:route-t-unranked"]),
        "the rest of the fork, by id: {t:#}"
    );
    let fork = map["forks_with_an_untried_branch"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["hole"] == "dec:close-t")
        .cloned()
        .unwrap_or_default();
    assert_eq!(fork["waiting_total"], 5, "{fork:#}");

    // A rank below 1 is refused, writing nothing.
    let e = s
        .refused(
            "register_alternative",
            json!({"decision_id": "dec:close-t", "artifact_id": "alt:route-t-zero",
                   "name": "A rank of zero", "location": "this design", "waiting": true,
                   "promise_rank": 0}),
        )
        .await;
    assert!(e.contains("promise_rank"), "{e}");
    assert!(s.node("alt:route-t-zero").await.is_null());
}

fn skill(name: &str) -> &'static str {
    SKILLS
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("a served skill named {name}"))
        .body
}

#[test]
fn work_a_proof_hole_starts_each_round_from_the_map_and_goes_back_at_n() {
    let body = skill("work-a-proof-hole");
    for needle in [
        // the round starts from the map
        "`map: true`",
        // a hole says what it serves and what closing it does
        "`serves`",
        "`closing_it`",
        "`narrows`",
        "`restates`",
        // the routes say their refuter; every idea is charted, ranked
        "`refuter`",
        "`waiting: true`",
        "`promise_rank`",
        "one line each",
        "new-ideas pass",
        "`untried_branches_by_promise`",
        // convergence: rounds counted, N, the highest fork, the turn's cause
        "`search_round`",
        "`since_progress`",
        "`forks_with_an_untried_branch`",
        "N = 3",
        "highest fork",
    ] {
        assert!(
            body.contains(needle),
            "work-a-proof-hole does not say {needle}"
        );
    }
    assert!(
        !body.contains("is filler there to satisfy the floor, and is cut"),
        "a candidate with no reason is kept, unranked, never cut"
    );
    let proof = skill("design-a-proof");
    for needle in ["`serves`", "`closing_it`", "`map: true`"] {
        assert!(
            proof.contains(needle),
            "design-a-proof does not say {needle}"
        );
    }
}
