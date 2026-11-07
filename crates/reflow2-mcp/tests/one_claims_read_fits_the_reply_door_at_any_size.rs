//! ONE CLAIM'S REASONING READ FITS THE REPLY DOOR AT ANY SIZE
//! (req:one-claims-reasoning-read-fits-the-reply-door-at-any-size).
//!
//! Seen 2026-11-07 in a proof trial on reflow2: after four rounds the target
//! claim, under the routes tried toward it that fail, read at 39,060
//! characters. reasoning_report on it at budget_chars 3,000 answered "0 of 28
//! claims" with `next_offset` 0, a trial harness read that empty page as
//! "nothing closed" and undid a sound round
//! (fact:root-cause-the-aa-attempts-r-arm-round-4-read-as-failing-because-the-harness-stopped-on-an-empty-page-2026-11-07),
//! and through flo2.io's door of about 30,000 characters the claim could not be
//! read whole at any budget a chat can ask for. It grows with every failed
//! route recorded under it.
//!
//! THE CLASS PINNED: a claim whose head fits is never answered with zero
//! claims, at any budget from the trial's 3,000 to the door's 30,000. Over the
//! budget it comes as its HEAD (what it says, how it reads and why, and counts
//! of its parts) and its PARTS (steps of support, failed routes, objections),
//! each whole, every page within the budget; `part_offset` reads through them,
//! and together they are exactly the whole claim's parts, in order.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "claim-in-parts-probe".to_string();
        cfg
    }
}

struct Session {
    client: rmcp::service::RunningService<rmcp::service::RoleClient, Probe>,
}

async fn session() -> Session {
    let svc = ReflowService::in_memory().expect("in-memory service");
    let (server_rx, client_tx) = tokio::io::duplex(1 << 23);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 23);
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
        let arguments = args.as_object().cloned().unwrap_or_default();
        let r = self
            .client
            .call_tool(
                rmcp::model::CallToolRequestParams::new(tool.to_string()).with_arguments(arguments),
            )
            .await
            .unwrap_or_else(|e| panic!("`{tool}`: {e}"));
        assert!(!r.is_error.unwrap_or(false), "`{tool}` was refused: {r:?}");
        r.structured_content.unwrap_or(Value::Null)
    }
}

fn chars(v: &Value) -> usize {
    serde_json::to_string(v).unwrap().len()
}

const TARGET: &str = "req:target";
const FAILED_ROUTES: usize = 15;
const OBJECTIONS: usize = 2;

/// The trial's shape: a short conjecture, one step toward it, two objections
/// against it, and fifteen failed routes, each stating why it fails at length,
/// so the claim alone reads at about 39,000 characters.
async fn a_claim_under_many_failed_routes(s: &Session) {
    s.ok(
        "add_requirement",
        json!({"id": "req:lemma", "name": "Lemma: the bound holds for degree one",
               "statement": "For degree one the influence bound holds with C = 1.",
               "statement_kind": "lemma"}),
    )
    .await;
    s.ok(
        "add_requirement",
        json!({"id": TARGET, "name": "Conjecture: one coordinate carries a fixed share",
               "statement": "Every bounded polynomial with variance at least ε has a \
                             coordinate whose influence is at least (ε/d)^C.",
               "statement_kind": "conjecture"}),
    )
    .await;
    s.ok(
        "add_inference",
        json!({"id": "inf:degree-one", "rule": "all", "premises": ["req:lemma"],
               "conclusion": TARGET, "rule_name": "special case",
               "warrant": "Degree one is the base of an induction on degree, not yet carried \
                           further."}),
    )
    .await;
    // Written in bulk: records alike in their words are what a long attempt
    // holds, and the typed constructor would ask whether they are one.
    let mut nodes: Vec<Value> = (0..OBJECTIONS)
        .map(|i| {
            json!({"node_type": "Requirement", "id": format!("req:objection-{i}"),
                   "props": {"name": format!("Objection {i}: a candidate counterexample"),
                             "statement": format!("Candidate counterexample {i}: a tribes-like \
                                                   function whose largest influence decays \
                                                   faster than any power of ε/d.")}})
        })
        .collect();
    nodes.extend((0..FAILED_ROUTES).map(|i| {
        let because = format!(
            "Route {i} fails because the bound it needs at the inductive step loses a factor \
             of the degree each time it is applied, and over d steps that loss is no longer \
             polynomial. "
        )
        .repeat(12);
        json!({"node_type": "Requirement", "id": format!("req:route-{i:02}"),
               "props": {"name": format!("Failed route {i}: an inductive bound on the influences"),
                         "statement": format!("Route {i}: bound the largest influence by \
                                               induction on the degree through a \
                                               hypercontractive estimate."),
                         "statement_kind": "failed_route",
                         "route_to": TARGET,
                         "fails_at": "the inductive step on the degree",
                         "fails_at_step": "req:lemma",
                         "fails_because": because}})
    }));
    s.ok("create_nodes", json!({"nodes": nodes})).await;
    let edges: Vec<Value> = (0..OBJECTIONS)
        .map(|i| {
            json!({"edge_type": "CONTRADICTS", "from_type": "Requirement",
                   "from_id": format!("req:objection-{i}"),
                   "to_type": "Requirement", "to_id": TARGET,
                   "properties": {"evidence": "If the decay holds, the conjecture fails."}})
        })
        .collect();
    s.ok("create_edges", json!({"edges": edges})).await;
}

/// The claim read whole, at a budget nothing reaches.
async fn whole(s: &Session) -> Value {
    let r = s
        .ok(
            "reasoning_report",
            json!({"claim": TARGET, "limit": 1, "budget_chars": 5_000_000}),
        )
        .await;
    let c = r["claims"][0].clone();
    assert_eq!(c["id"], TARGET, "{:#}", r["page"]);
    assert!(c.get("in_parts").is_none(), "read in parts at 5,000,000");
    c
}

/// The whole claim's parts, in the order the head numbers them.
fn parts_of(c: &Value) -> Vec<(String, Value)> {
    ["support", "failed_routes", "objections"]
        .iter()
        .flat_map(|list| {
            c[*list]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(move |item| (list.to_string(), item))
        })
        .collect()
}

/// Every page of the claim, from `part_offset` 0 until no part remains, read
/// as a reader is told to: at `budget`, and where a page says its next part
/// needs more (`part_needs_budget_chars`), at that budget for that part. Each
/// page keeps to the budget it was asked for.
async fn read_in_parts(s: &Session, budget: usize, head_page: Value) -> Vec<(String, Value)> {
    let mut got = Vec::new();
    let mut page = head_page;
    let mut asked = budget;
    for _ in 0..=2 * (1 + FAILED_ROUTES + OBJECTIONS) {
        let n = chars(&page);
        assert!(
            n <= asked,
            "budget_chars {asked} answered {n}: {:#}",
            page["page"]
        );
        let c = &page["claims"][0];
        assert_eq!(c["id"], TARGET, "{:#}", page["page"]);
        let parts = c["parts"].as_array().expect("parts");
        for p in parts {
            assert_eq!(p["part"], got.len(), "a part skipped or repeated: {p:#}");
            got.push((p["in"].as_str().unwrap().to_string(), p["item"].clone()));
        }
        let Some(next) = c["in_parts"]["next_part_offset"].as_u64() else {
            return got;
        };
        assert!(
            next as usize == got.len(),
            "next_part_offset {next} after {} parts read: {:#}",
            got.len(),
            c["in_parts"]
        );
        asked = if parts.is_empty() {
            let needs = c["in_parts"]["part_needs_budget_chars"]
                .as_u64()
                .unwrap_or_else(|| {
                    panic!(
                        "a page with no part names no budget for the next: {:#}",
                        c["in_parts"]
                    )
                }) as usize;
            assert!(needs > asked, "part {next} needs {needs}, asked {asked}");
            needs
        } else {
            budget
        };
        page = s
            .ok(
                "reasoning_report",
                json!({"claim": TARGET, "limit": 1, "budget_chars": asked,
                       "offset": 0, "part_offset": next}),
            )
            .await;
    }
    panic!("the parts of {TARGET} never ended at budget_chars {budget}");
}

/// ⭐ THE TRIAL'S CASE, AT EVERY BUDGET A CHAT CAN ASK FOR. The read under the
/// claim never answers zero claims; the head says what the claim says and how
/// it reads, exactly as the whole read does; every page keeps to its budget;
/// and paging `part_offset` returns every part whole, once, in order.
#[tokio::test]
async fn a_claim_larger_than_the_budget_comes_as_its_head_and_its_parts() {
    let s = session().await;
    a_claim_under_many_failed_routes(&s).await;
    let whole = whole(&s).await;
    let whole_chars = chars(&whole);
    assert!(
        whole_chars > 36_000,
        "the claim reads at {whole_chars} characters: the trial's was 39,060"
    );
    let want = parts_of(&whole);
    assert_eq!(want.len(), 1 + FAILED_ROUTES + OBJECTIONS, "{whole:#}");

    for budget in [6_000, 12_000, 30_000] {
        let r = s
            .ok(
                "reasoning_report",
                json!({"claim": TARGET, "budget_chars": budget}),
            )
            .await;
        let claims = r["claims"].as_array().expect("claims");
        assert!(
            !claims.is_empty(),
            "budget_chars {budget} answered ZERO claims for a claim whose head fits: {:#}",
            r["page"]
        );
        let head = &claims[0];
        assert_eq!(head["id"], TARGET, "{:#}", r["page"]);
        for field in ["says", "truth", "why", "statement", "name"] {
            assert_eq!(
                head[field], whole[field],
                "the head's `{field}` at {budget}"
            );
        }
        let counts = &head["in_parts"]["counts"];
        assert_eq!(counts["support"], 1, "{:#}", head["in_parts"]);
        assert_eq!(counts["failed_routes"], FAILED_ROUTES);
        assert_eq!(counts["objections"], OBJECTIONS);
        assert_eq!(head["in_parts"]["total"], want.len());
        for list in ["support", "failed_routes", "objections"] {
            assert!(
                head.get(list).is_none(),
                "the head lists `{list}` beside its parts, so a partial list reads as whole"
            );
        }
        let got = read_in_parts(&s, budget, r).await;
        assert_eq!(
            got.len(),
            want.len(),
            "budget_chars {budget}: {} parts read of {}",
            got.len(),
            want.len()
        );
        for (i, (g, w)) in got.iter().zip(&want).enumerate() {
            assert_eq!(
                g, w,
                "part {i} at budget_chars {budget} is not the whole part"
            );
        }
    }
}

/// A PART LARGER THAN THE ROOM IS NEVER CUT. At the trial's 3,000 one failed
/// route does not fit beside the head and the summary: the head still comes,
/// with no part, the budget that part needs, and the same `part_offset` to
/// read on; at that budget the part comes whole.
#[tokio::test]
async fn at_three_thousand_the_head_comes_and_names_what_its_next_part_needs() {
    let s = session().await;
    a_claim_under_many_failed_routes(&s).await;
    let whole = whole(&s).await;
    let r = s
        .ok(
            "reasoning_report",
            json!({"claim": TARGET, "budget_chars": 3_000}),
        )
        .await;
    let head = &r["claims"][0];
    assert_eq!(
        head["id"], TARGET,
        "budget_chars 3,000 answered no claim: {:#}",
        r["page"]
    );
    assert_eq!(head["truth"], whole["truth"]);
    let ip = &head["in_parts"];
    assert_eq!(ip["returned"], 0, "{ip:#}");
    assert_eq!(ip["next_part_offset"], 0, "{ip:#}");
    let needs = ip["part_needs_budget_chars"]
        .as_u64()
        .unwrap_or_else(|| panic!("no budget named for the next part: {ip:#}"))
        as usize;
    assert!(needs > 3_000, "{needs}");
    let r = s
        .ok(
            "reasoning_report",
            json!({"claim": TARGET, "budget_chars": needs, "offset": 0, "part_offset": 0}),
        )
        .await;
    assert!(chars(&r) <= needs, "{} > {needs}", chars(&r));
    let p = &r["claims"][0]["parts"][0];
    assert_eq!(p["in"], "support", "{:#}", r["claims"][0]["in_parts"]);
    assert_eq!(p["item"], whole["support"][0], "the part was cut");
}

/// A READER THAT FOLLOWS ONLY `next_offset` STILL READS EVERY CLAIM. The whole
/// design read page by page at the door's budget: no page is empty while a
/// claim remains, each claim comes once, and the large one comes as its head.
#[tokio::test]
async fn paging_the_whole_design_by_next_offset_meets_every_claim() {
    let s = session().await;
    a_claim_under_many_failed_routes(&s).await;
    let mut offset = 0;
    let mut seen = Vec::new();
    let mut total = None;
    for _ in 0..100 {
        let r = s
            .ok(
                "reasoning_report",
                json!({"budget_chars": 30_000, "offset": offset}),
            )
            .await;
        assert!(chars(&r) <= 30_000, "{}", chars(&r));
        total = r["total_claims"].as_u64();
        let claims = r["claims"].as_array().expect("claims");
        assert!(
            !claims.is_empty() || r.get("next_offset").is_none(),
            "an empty page with claims remaining, at offset {offset}: {:#}",
            r["page"]
        );
        for c in claims {
            seen.push(c["id"].as_str().unwrap().to_string());
            if c["id"] == TARGET {
                assert!(c.get("in_parts").is_some(), "{TARGET} came whole at 30,000");
            }
        }
        match r["next_offset"].as_u64() {
            Some(n) => {
                assert!(
                    n as usize > offset,
                    "next_offset {n} did not move on from {offset}"
                );
                offset = n as usize;
            }
            None => break,
        }
    }
    assert_eq!(Some(seen.len() as u64), total, "{seen:#?}");
    assert!(seen.iter().any(|c| c == TARGET), "{seen:#?}");
}
