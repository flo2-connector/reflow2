//! A claim's `proof_status.total_steps` counts each step under it ONCE.
//!
//! `fact:root-cause-a-claims-step-count-sums-its-lines-so-a-shared-step-counts-twice-2026-10-18`
//! (Anthony, 2026-10-18: "Yes, build the fixes. Do in one reflow2 PR").
//!
//! # The measured case
//!
//! Proof trial rung 2 (design bbcd344a71ff1cbd, 0.86.0): `req:prop-uniqueness`
//! has two lines of proof, of 27 and 20 steps, sharing 7. Its `total_steps`
//! read 47, the SUM of its lines, while the theorem built on it read 41: a
//! claim reported more steps than the theorem resting on it. Rung 1
//! (d2ea1acfee6090c8): the theorem's two lines (Euclid's and the Fermat
//! numbers') share a lemma, and it read 26 steps in a design that holds 25.
//! One walk answered two questions, "is this line wholly confirmed?" and "how
//! many steps are under this claim?", and nothing said what the count meant.
//!
//! # What these tests pin
//!
//! - THE CLASS, on both trial designs: for EVERY concluded claim, `total_steps`
//!   is the number of distinct Inferences reachable below it through its
//!   holding lines, computed here from the design's own edges, and never more
//!   than the design's Inference count; `confirmed_steps` and
//!   `unconfirmed_steps` split that same set;
//! - two lines sharing a step count it once, and `proven` stays a judgement of
//!   ONE line: a line is proven only if all of its own steps, shared ones
//!   included, are confirmed;
//! - every `proof_status` says, in its own `counts`, what the numbers mean.

use std::collections::{BTreeMap, BTreeSet};

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

/// Rung 2, the fundamental theorem of arithmetic, exported read-only from
/// flo2.io on 2026-10-17.
const RUNG_2: &str = include_str!("fixtures/proof_trial_rung_2_fta.json");
/// Rung 1, infinitely many primes, exported read-only from flo2.io on
/// 2026-10-18.
const RUNG_1: &str = include_str!("fixtures/proof_trial_rung_1_primes.json");

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "step-count-probe".to_string();
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
        let r = self
            .client
            .call_tool(
                rmcp::model::CallToolRequestParams::new(tool.to_string()).with_arguments(arguments),
            )
            .await
            .unwrap_or_else(|e| panic!("`{tool}` {shown} failed: {e}"));
        assert!(
            !r.is_error.unwrap_or(false),
            "`{tool}` {shown} was refused: {:?}",
            r.content
        );
        r.structured_content.unwrap_or(Value::Null)
    }

    async fn node(&self, id: &str) -> Value {
        self.ok("get_node", json!({"id": id})).await["node"].clone()
    }

    async fn step(&self, id: &str, premises: &[&str], conclusion: &str, confirmed: bool) {
        let mut args = json!({
            "id": id,
            "rule": "all",
            "premises": premises,
            "conclusion": conclusion,
            "warrant": "it follows"
        });
        if confirmed {
            args["confirmed_by"] = json!("who:ajs");
            args["confirmed_how"] = json!("person");
            args["confirmed_at"] = json!("2026-10-18T12:00:00Z");
        }
        self.ok("add_inference", args).await;
    }
}

async fn imported(fixture: &str) -> (Session, Value) {
    let doc: Value = serde_json::from_str(fixture).expect("the fixture parses");
    let s = session().await;
    s.ok("import_graph", json!({"document": doc})).await;
    (s, doc)
}

/// The design's step links, read from the exported edges themselves: which
/// steps conclude each node, and which nodes are premises of each step.
struct Links {
    concluding: BTreeMap<String, BTreeSet<String>>,
    premises: BTreeMap<String, BTreeSet<String>>,
}

fn links(doc: &Value) -> Links {
    let mut l = Links {
        concluding: BTreeMap::new(),
        premises: BTreeMap::new(),
    };
    for e in doc["edges"].as_array().expect("edges") {
        let (from, to) = (
            e["from_id"].as_str().unwrap().to_string(),
            e["to_id"].as_str().unwrap().to_string(),
        );
        match e["edge_type"].as_str() {
            Some("CONCLUDES") => {
                l.concluding.entry(to).or_default().insert(from);
            }
            Some("PREMISE_OF") => {
                l.premises.entry(to).or_default().insert(from);
            }
            _ => {}
        }
    }
    l
}

/// Every Inference reachable below `step`: the step itself, and every step
/// concluding one of its premises, all the way down.
fn below(l: &Links, step: &str, out: &mut BTreeSet<String>) {
    if !out.insert(step.to_string()) {
        return;
    }
    for p in l.premises.get(step).into_iter().flatten() {
        for s in l.concluding.get(p).into_iter().flatten() {
            below(l, s, out);
        }
    }
}

/// THE INVARIANT, over a whole design: every concluded claim counts the
/// distinct steps under its holding lines, never more than the design holds.
/// Returns each claim's count, for the named cases.
async fn every_claim_counts_each_step_once(s: &Session, doc: &Value) -> BTreeMap<String, u64> {
    let l = links(doc);
    let inferences = doc["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["node_type"] == "Inference")
        .count() as u64;
    let mut counts = BTreeMap::new();
    let mut wrong = Vec::new();
    for (claim, tops) in &l.concluding {
        let mut steps = BTreeSet::new();
        for top in tops {
            let verdict = s.node(top).await["inference_reading"]["verdict"].clone();
            if verdict == "holds" {
                below(&l, top, &mut steps);
            }
        }
        let mut confirmed = 0u64;
        for st in &steps {
            let how =
                s.node(st).await["inference_reading"]["confirmation"]["confirmed_how"].clone();
            if how == "person" || how == "checker" {
                confirmed += 1;
            }
        }
        let ps = s.node(claim).await["proof_status"].clone();
        let total = ps["total_steps"]
            .as_u64()
            .expect("a concluded claim has a count");
        if total != steps.len() as u64
            || total > inferences
            || ps["confirmed_steps"].as_u64() != Some(confirmed)
            || ps["unconfirmed_steps"].as_u64() != Some(steps.len() as u64 - confirmed)
        {
            wrong.push(format!(
                "{claim}: total_steps {total}, confirmed {}, unconfirmed {}; expected {} distinct \
                 ({confirmed} confirmed) of the design's {inferences}",
                ps["confirmed_steps"],
                ps["unconfirmed_steps"],
                steps.len()
            ));
        }
        counts.insert(claim.clone(), total);
    }
    assert!(
        wrong.is_empty(),
        "a claim's count is not the distinct steps under it:\n{}",
        wrong.join("\n")
    );
    counts
}

#[tokio::test]
async fn every_claim_of_rung_2_counts_each_step_once() {
    let (s, doc) = imported(RUNG_2).await;
    let counts = every_claim_counts_each_step_once(&s, &doc).await;
    assert_eq!(
        counts["req:prop-uniqueness"], 40,
        "two lines of 27 and 20 steps sharing 7 are 40 steps, not 47"
    );
    assert_eq!(counts["req:thm-fundamental-theorem-of-arithmetic"], 41);
    assert!(
        counts["req:thm-fundamental-theorem-of-arithmetic"] >= counts["req:prop-uniqueness"],
        "a theorem resting on a claim has at least its steps"
    );
}

#[tokio::test]
async fn every_claim_of_rung_1_counts_each_step_once() {
    let (s, doc) = imported(RUNG_1).await;
    let counts = every_claim_counts_each_step_once(&s, &doc).await;
    let theorem = counts["req:theorem-infinitely-many-primes"];
    assert!(
        theorem <= 25,
        "the theorem read {theorem} steps in a design holding 25"
    );
}

#[tokio::test]
async fn two_lines_sharing_a_step_count_it_once_and_proven_stays_per_line() {
    let s = session().await;
    for (id, name) in [
        ("req:claim", "The claim"),
        ("req:lemma", "A lemma both lines use"),
    ] {
        s.ok(
            "add_requirement",
            json!({"id": id, "name": name, "statement": format!("{name}.")}),
        )
        .await;
    }
    s.ok(
        "add_constraint",
        json!({"id": "con:given", "name": "Given", "statement": "Taken as given."}),
    )
    .await;
    // The shared lemma, confirmed; line A confirmed; line B not.
    s.step("inf:lemma", &["con:given"], "req:lemma", true).await;
    s.step("inf:line-a", &["req:lemma"], "req:claim", true)
        .await;
    s.step(
        "inf:line-b",
        &["req:lemma", "con:given"],
        "req:claim",
        false,
    )
    .await;

    let ps = s.node("req:claim").await["proof_status"].clone();
    assert_eq!(ps["total_steps"], 3, "the shared lemma counts once: {ps:#}");
    let counts = ps["counts"].as_str().unwrap_or_default();
    assert!(
        counts.contains("distinct steps") && counts.contains("each counted once"),
        "the reading says what its count means: {ps:#}"
    );
    assert_eq!(ps["confirmed_steps"], 2, "{ps:#}");
    assert_eq!(ps["unconfirmed_steps"], 1, "{ps:#}");
    assert_eq!(
        ps["status"], "proven",
        "line A is wholly confirmed, shared step included: {ps:#}"
    );

    // Unconfirm the shared step's line by recording an unconfirmed lemma
    // step instead: now neither line is wholly confirmed.
    let s = session().await;
    for id in ["req:claim", "req:lemma"] {
        s.ok(
            "add_requirement",
            json!({"id": id, "name": id, "statement": "A claim."}),
        )
        .await;
    }
    s.ok(
        "add_constraint",
        json!({"id": "con:given", "name": "Given", "statement": "Taken as given."}),
    )
    .await;
    s.step("inf:lemma", &["con:given"], "req:lemma", false)
        .await;
    s.step("inf:line-a", &["req:lemma"], "req:claim", true)
        .await;
    s.step("inf:line-b", &["req:lemma", "con:given"], "req:claim", true)
        .await;
    let ps = s.node("req:claim").await["proof_status"].clone();
    assert_eq!(ps["total_steps"], 3, "{ps:#}");
    assert_eq!(ps["confirmed_steps"], 2, "{ps:#}");
    assert_eq!(ps["unconfirmed_steps"], 1, "{ps:#}");
    assert_eq!(
        ps["status"], "shown",
        "a line is proven only if its OWN steps, the shared one included, are all confirmed: \
         {ps:#}"
    );
}
