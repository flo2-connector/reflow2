//! Reasoning is attached to the structure it justifies.
//!
//! `req:an-edge-names-the-claim-that-argues-for-it-and-a-governed-requirement-reads-its-reason-to-exist`
//! (Anthony, 2026-10-18, phase 2 of the build list), which builds
//! `req:a-derived-requirement-inherits-its-reasoning-from-the-decision-that-created-it`
//! and the rest of `req:a-choice-that-rests-on-checkable-premises-is-recorded-as-a-chain`:
//!
//! 1. reading a requirement names the Decision(s) that govern it and each one's
//!    support, as its REASON TO EXIST, apart from whether it is delivered;
//! 2. a change walk that breaks a Decision's support names every requirement
//!    GOVERNED_BY it as losing its reason to exist;
//! 3. an edge names the claim that argues for it by a typed link (`argued_by`),
//!    the claim's verdict is read beside the edge, and `satisfies` sets the
//!    edge's declared `evidence`;
//! 4. a claim "X satisfies Y" whose premise is Y's delivery along that very
//!    edge reads `circular`;
//! 5. the lineage vocabulary covers "the owner's words, chosen through a
//!    Decision" (`derived`, its meaning clarified; no value added);
//! 6. the served instructions say how to record both.
//!
//! # The measured case
//!
//! Cases 2 and 4 of the 2026-10-18 proof of concept
//! (`fact:reasoning-embedded-in-a-traditional-design-works-within-one-design-and-stops-at-premises-with-no-truth-and-at-design-borders-2026-10-18`),
//! held by phase 1's fixture of reflow2's own design: the near-match
//! requirement GOVERNED_BY the Decision its chain concludes, and the claim that
//! the capability satisfies it. On 0.87.0, reasoning_report on the requirement
//! said only "delivered by the roll-up"; the walk from the passing check broke
//! the Decision's chain and named no requirement; the SATISFIES edge named the
//! claim only in its `evidence` prose; the claim's premise "the requirement is
//! delivered" was computed along that same edge.
//!
//! Every test is a CLASS over the six fixtures of phase 1 (π+e, Arrow, Brocard,
//! the proof of concept's slice, rungs 1 and 2), plus the hand-built variants
//! they do not hold. Each was watched failing on main b639be8 first. The file
//! talks to the served surface only, so it compiles there.

use std::collections::{BTreeMap, BTreeSet};

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

const PI_E: &str = include_str!("../../reflow2-core/tests/fixtures/proof_trial_pi_plus_e.json");
const ARROW: &str = include_str!("../../reflow2-core/tests/fixtures/proof_trial_arrow.json");
const BROCARD: &str = include_str!("../../reflow2-core/tests/fixtures/proof_trial_brocard.json");
const CASE: &str = include_str!(
    "../../reflow2-core/tests/fixtures/poc_case_1_a_decision_concluded_by_its_chain.json"
);
const RUNG_1: &str = include_str!("fixtures/proof_trial_rung_1_primes.json");
const RUNG_2: &str = include_str!("fixtures/proof_trial_rung_2_fta.json");

const ALL: [(&str, &str); 6] = [
    ("pi+e", PI_E),
    ("arrow", ARROW),
    ("brocard", BROCARD),
    ("poc", CASE),
    ("rung-1", RUNG_1),
    ("rung-2", RUNG_2),
];

const DEC: &str = "dec:idea-a-cross-type-near-match-is-a-note-and-only-a-same-type-one-refuses";
const REQ: &str =
    "req:a-near-match-across-types-or-within-a-line-of-reasoning-is-a-note-not-a-refusal";
const CLAIM: &str =
    "req:claim-the-near-match-note-capability-satisfies-the-near-match-note-requirement";
const CLAIM_STEP: &str = "inf:the-near-match-note-capability-satisfies-its-requirement";
const CAP: &str = "cap:a-near-match-across-types-or-within-reasoning-is-a-note";
const VER: &str = "ver:a-near-match-across-types-or-within-reasoning-is-a-note-tests";
const CHG: &str = "chg:a-cross-type-near-match-is-named-as-a-layer";
const FACT: &str = "fact:the-proof-trial-rung-2-ran-on-flo2-io-and-what-counts-holds-while-recording-a-proof-is-still-costly-2026-10-17";
const SECOND_CHAIN: &str =
    "inf:the-near-match-note-choice-stands-while-its-class-test-replaying-rung-2-passes";

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "reason-to-exist-probe".to_string();
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
    async fn call(&self, tool: &str, args: Value) -> (bool, Value, String) {
        let arguments = args.as_object().cloned().unwrap_or_default();
        let r = self
            .client
            .call_tool(
                rmcp::model::CallToolRequestParams::new(tool.to_string()).with_arguments(arguments),
            )
            .await;
        match r {
            Ok(r) => (
                !r.is_error.unwrap_or(false),
                r.structured_content.clone().unwrap_or(Value::Null),
                format!("{:?}", r.content),
            ),
            Err(e) => (false, Value::Null, e.to_string()),
        }
    }

    async fn ok(&self, tool: &str, args: Value) -> Value {
        let shown = args.to_string();
        let (ok, v, text) = self.call(tool, args).await;
        assert!(ok, "`{tool}` {shown} was refused: {text}");
        v
    }

    async fn node(&self, id: &str) -> Value {
        self.ok("get_node", json!({"id": id})).await["node"].clone()
    }

    /// One claim's reading in the read-back under it.
    async fn claim(&self, id: &str) -> Value {
        let r = self
            .ok("reasoning_report", json!({"claim": id, "limit": 500}))
            .await;
        r["claims"]
            .as_array()
            .expect("claims")
            .iter()
            .find(|c| c["id"] == id)
            .cloned()
            .unwrap_or_else(|| panic!("{id} is not in its own read-back: {r}"))
    }

    async fn radius(&self, seeds: &[&str]) -> Value {
        self.ok("propagate_from", json!({"seed_ids": seeds})).await
    }

    async fn edges(&self, id: &str, edge_type: &str) -> Vec<Value> {
        self.ok(
            "get_node",
            json!({"id": id, "include_edges": {"edge_types": [edge_type]}}),
        )
        .await["edges"]["items"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    }
}

async fn imported(fixture: &str) -> (Session, Value) {
    let doc: Value = serde_json::from_str(fixture).expect("the fixture parses");
    let s = session().await;
    s.ok("import_graph", json!({"document": doc})).await;
    (s, doc)
}

/// The design as its exported records say it, read from the document itself.
struct Doc {
    types: BTreeMap<String, String>,
    props: BTreeMap<String, Value>,
    edges: Vec<Value>,
}

fn doc(d: &Value) -> Doc {
    let mut out = Doc {
        types: BTreeMap::new(),
        props: BTreeMap::new(),
        edges: d["edges"].as_array().cloned().unwrap_or_default(),
    };
    for n in d["nodes"].as_array().expect("nodes") {
        let id = n["node_id"].as_str().unwrap().to_string();
        out.types
            .insert(id.clone(), n["node_type"].as_str().unwrap().to_string());
        out.props.insert(id, n["properties"].clone());
    }
    out
}

impl Doc {
    fn is(&self, id: &str, ty: &str) -> bool {
        self.types.get(id).map(String::as_str) == Some(ty)
    }

    /// Requirement → the Decisions it is GOVERNED_BY (no `parks` ruling).
    fn governed(&self) -> BTreeMap<String, BTreeSet<String>> {
        let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for e in &self.edges {
            if e["edge_type"] != "GOVERNED_BY" || e["properties"]["ruling"] == "parks" {
                continue;
            }
            let (f, t) = (e["from_id"].as_str().unwrap(), e["to_id"].as_str().unwrap());
            if self.is(f, "Requirement") && self.is(t, "Decision") {
                out.entry(f.to_string()).or_default().insert(t.to_string());
            }
        }
        out
    }

    fn concluded(&self, id: &str) -> bool {
        self.edges
            .iter()
            .any(|e| e["edge_type"] == "CONCLUDES" && e["to_id"] == id)
    }
}

// ─── 1. A requirement reads its reason to exist ─────────────────────────────

/// EVERY requirement GOVERNED_BY a Decision, in every fixture, reads a
/// `reason_to_exist` on get_node AND in reasoning_report naming exactly the
/// Decisions that govern it, each with the owner's `acceptance` and — exactly
/// when a step concludes the Decision — the chain's verdict beside it. A
/// requirement no Decision governs carries none. On main neither read said it.
#[tokio::test(flavor = "multi_thread")]
async fn every_governed_requirement_reads_its_reason_to_exist() {
    let mut asked = 0;
    for (name, fixture) in ALL {
        let (s, d) = imported(fixture).await;
        let d = doc(&d);
        let governed = d.governed();
        for (id, ty) in &d.types {
            if ty != "Requirement" {
                continue;
            }
            let node = s.node(id).await;
            let Some(decisions) = governed.get(id) else {
                assert!(
                    node.get("reason_to_exist").is_none(),
                    "{name}: {id} is governed by no Decision and reads a reason to exist"
                );
                continue;
            };
            asked += 1;
            for reading in [
                node["reason_to_exist"].clone(),
                s.claim(id).await["reason_to_exist"].clone(),
            ] {
                let named: BTreeSet<String> = reading["governed_by"]
                    .as_array()
                    .unwrap_or_else(|| panic!("{name}: {id} reads no reason to exist: {node}"))
                    .iter()
                    .map(|g| g["decision"].as_str().unwrap().to_string())
                    .collect();
                assert_eq!(&named, decisions, "{name}: {id}");
                for g in reading["governed_by"].as_array().unwrap() {
                    let dec = g["decision"].as_str().unwrap();
                    let status = d.props[dec]["status"].as_str().unwrap_or("proposed");
                    assert_eq!(g["acceptance"], status, "{name}: {id} / {dec}");
                    assert_eq!(
                        g["chain"]["verdict"].is_string(),
                        d.concluded(dec),
                        "{name}: {id} / {dec}: chain read iff a step concludes it: {g}"
                    );
                    assert!(g["support"].is_string() && g["why"].is_string(), "{g}");
                }
                assert!(reading["reads"].is_string(), "{reading}");
            }
        }
    }
    assert!(asked > 0, "no fixture held a governed requirement");
}

/// The measured case: the near-match requirement names the Decision and its
/// chain, and its reason to exist moves while its delivery does not. With the
/// first chain left alone and its measurement made a forecast, the Decision
/// stands on its owner's acceptance alone; the requirement still reads
/// delivered, exactly as before, and its reason to exist says so.
#[tokio::test(flavor = "multi_thread")]
async fn the_reason_to_exist_is_read_apart_from_delivery() {
    let (s, _) = imported(CASE).await;
    // The objection carried on the ChangeEvent stands against the Decision
    // (a ChangeEvent computes no truth), as phase 1 measured; take it away so
    // the support read is the chain's.
    s.ok(
        "delete_edge",
        json!({"edge_type": "CONTRADICTS", "from_id": CHG, "to_id": DEC}),
    )
    .await;
    let before = s.claim(REQ).await;
    let r = &before["reason_to_exist"];
    assert_eq!(r["reads"], "stands", "{r}");
    assert_eq!(r["governed_by"][0]["decision"], DEC, "{r}");
    assert_eq!(
        r["governed_by"][0]["support"], "accepted_and_its_chain_holds",
        "{r}"
    );
    assert_eq!(r["governed_by"][0]["chain"]["verdict"], "holds", "{r}");

    s.ok(
        "delete_edge",
        json!({"edge_type": "CONCLUDES", "from_id": SECOND_CHAIN, "to_id": DEC}),
    )
    .await;
    let mut props = s.node(FACT).await["properties"].clone();
    props["basis"] = json!("forecast");
    s.ok(
        "create_node",
        json!({"node_type": "TemporalFact", "id": FACT, "props": props}),
    )
    .await;
    let after = s.claim(REQ).await;
    let r = &after["reason_to_exist"];
    assert_eq!(r["reads"], "stands_on_acceptance_alone", "{r}");
    assert_eq!(
        r["governed_by"][0]["support"], "accepted_without_a_holding_chain",
        "{r}"
    );
    assert_eq!(
        after["truth"], before["truth"],
        "delivery moved with the reason"
    );
    assert_eq!(
        after["why"], before["why"],
        "delivery moved with the reason"
    );
}

/// Item 3 of req:a-choice-that-rests-on-checkable-premises-is-recorded-as-a-chain:
/// once recorded, a premise that stops holding makes the Decision's support
/// VISIBLY broken — `conclusion_does_not_follow` names it in detect_gaps —
/// while the Decision still reads accepted. Built by phase 1 (#770) and pinned
/// here on the measured case, where it was never asked.
#[tokio::test(flavor = "multi_thread")]
async fn a_premise_that_stops_holding_makes_the_decisions_support_visibly_broken() {
    let (s, _) = imported(CASE).await;
    s.ok(
        "delete_edge",
        json!({"edge_type": "CONTRADICTS", "from_id": CHG, "to_id": DEC}),
    )
    .await;
    let gaps = s.ok("detect_gaps", json!({})).await.to_string();
    assert!(
        !(gaps.contains("conclusion_does_not_follow") && gaps.contains(DEC)),
        "the Decision's support reads broken while its chain holds"
    );
    s.ok(
        "delete_edge",
        json!({"edge_type": "CONCLUDES", "from_id": SECOND_CHAIN, "to_id": DEC}),
    )
    .await;
    let mut props = s.node(FACT).await["properties"].clone();
    props["basis"] = json!("forecast");
    s.ok(
        "create_node",
        json!({"node_type": "TemporalFact", "id": FACT, "props": props}),
    )
    .await;
    let gaps = s.ok("detect_gaps", json!({})).await.to_string();
    assert!(
        gaps.contains("conclusion_does_not_follow") && gaps.contains(DEC),
        "a premise stopped holding and the Decision's support does not read broken"
    );
    assert_eq!(s.node(DEC).await["properties"]["status"], "accepted");
}

/// A `parks` ruling is not a reason to exist; a withdrawn Decision is one
/// lost; a proposed one is not settled.
#[tokio::test(flavor = "multi_thread")]
async fn a_parks_ruling_is_no_reason_and_a_withdrawn_decision_is_one_lost() {
    let s = session().await;
    s.ok(
        "add_contributor",
        json!({"id": "who:owner", "name": "The owner"}),
    )
    .await;
    for (id, status) in [
        ("dec:parks", "accepted"),
        ("dec:gone", "rejected"),
        ("dec:open", "proposed"),
    ] {
        s.ok(
            "add_decision",
            json!({"id": id, "name": id, "decision": "a choice", "rationale": "r"}),
        )
        .await;
        if status != "proposed" {
            s.ok(
                "set_decision_status",
                json!({"decision_id": id, "status": status, "approver": "who:owner", "chose": "a choice"}),
            )
            .await;
        }
    }
    for id in ["req:parked", "req:withdrawn", "req:open"] {
        s.ok(
            "add_requirement",
            json!({"id": id, "name": id, "statement": format!("{id} holds."), "distinct_from": ["req:parked", "req:withdrawn", "req:open"]}),
        )
        .await;
    }
    s.ok(
        "governed_by",
        json!({"from_id": "req:parked", "to_id": "dec:parks", "ruling": "parks"}),
    )
    .await;
    s.ok(
        "governed_by",
        json!({"from_id": "req:withdrawn", "to_id": "dec:gone"}),
    )
    .await;
    s.ok(
        "governed_by",
        json!({"from_id": "req:open", "to_id": "dec:open"}),
    )
    .await;
    assert!(s.node("req:parked").await.get("reason_to_exist").is_none());
    let w = s.node("req:withdrawn").await["reason_to_exist"].clone();
    assert_eq!(w["reads"], "lost", "{w}");
    assert_eq!(w["governed_by"][0]["support"], "withdrawn", "{w}");
    let o = s.node("req:open").await["reason_to_exist"].clone();
    assert_eq!(o["reads"], "not_settled", "{o}");
}

// ─── 2. A broken Decision names what it governs ─────────────────────────────

/// EVERY Decision that governs a requirement, in every fixture, seeded in a
/// change walk (what if it no longer held, or were re-opened?), names every
/// requirement GOVERNED_BY it under `reasons_to_exist_lost`. And EVERY premise
/// of a step concluding such a Decision, seeded, names them whenever the
/// walk's row leaves the Decision without a holding line. On main nothing
/// named them; they were distance rings, reached or not by coincidence.
#[tokio::test(flavor = "multi_thread")]
async fn a_broken_decision_names_every_requirement_it_governs() {
    let mut asked = 0;
    for (name, fixture) in ALL {
        let (s, d) = imported(fixture).await;
        let d = doc(&d);
        let mut by_decision: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (req, decs) in d.governed() {
            for dec in decs {
                by_decision.entry(dec).or_default().insert(req.clone());
            }
        }
        for (dec, reqs) in &by_decision {
            asked += 1;
            let r = s.radius(&[dec]).await;
            let row = r["reasons_to_exist_lost"]
                .as_array()
                .and_then(|a| a.iter().find(|x| x["decision"] == *dec))
                .unwrap_or_else(|| panic!("{name}: seeding {dec} names nothing it governs: {r}"))
                .clone();
            assert_eq!(row["decision_reads"], "counted_lost", "{name}: {row}");
            let named: BTreeSet<String> = row["requirements"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x["id"].as_str().unwrap().to_string())
                .collect();
            assert_eq!(&named, reqs, "{name}: {dec}");

            // Each premise of a step concluding it.
            let steps: Vec<String> = d
                .edges
                .iter()
                .filter(|e| e["edge_type"] == "CONCLUDES" && e["to_id"] == *dec)
                .map(|e| e["from_id"].as_str().unwrap().to_string())
                .collect();
            for st in &steps {
                for e in d
                    .edges
                    .iter()
                    .filter(|e| e["edge_type"] == "PREMISE_OF" && e["to_id"] == *st)
                {
                    let p = e["from_id"].as_str().unwrap();
                    let r = s.radius(&[p]).await;
                    let broke = r["inferences"].as_array().is_some_and(|rows| {
                        rows.iter().any(|w| {
                            w["conclusion"] == *dec && w["conclusion_reads"] != "still_supported"
                        })
                    });
                    let named: BTreeSet<String> = r["reasons_to_exist_lost"]
                        .as_array()
                        .and_then(|a| a.iter().find(|x| x["decision"] == *dec))
                        .map(|x| {
                            x["requirements"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|q| q["id"].as_str().unwrap().to_string())
                                .collect()
                        })
                        .unwrap_or_default();
                    if broke {
                        assert_eq!(&named, reqs, "{name}: losing {p} breaks {dec}: {r}");
                    } else {
                        assert!(
                            named.is_empty(),
                            "{name}: {dec} still stands, named {named:?}"
                        );
                    }
                }
            }
        }
    }
    assert!(
        asked > 0,
        "no fixture held a Decision that governs a requirement"
    );
}

/// The measured case: losing the passing check leaves the Decision on its
/// owner's acceptance alone, and the near-match requirement is named as
/// losing its reason to exist.
#[tokio::test(flavor = "multi_thread")]
async fn losing_the_check_names_the_near_match_requirement() {
    let (s, _) = imported(CASE).await;
    s.ok(
        "delete_edge",
        json!({"edge_type": "CONTRADICTS", "from_id": CHG, "to_id": DEC}),
    )
    .await;
    let r = s.radius(&[VER]).await;
    let row = r["reasons_to_exist_lost"]
        .as_array()
        .and_then(|a| a.iter().find(|x| x["decision"] == DEC))
        .unwrap_or_else(|| panic!("the Decision's break names nothing: {r}"));
    assert_eq!(
        row["decision_reads"], "accepted_without_a_holding_chain",
        "{row}"
    );
    assert_eq!(row["requirements"][0]["id"], REQ, "{row}");
}

// ─── 3. An edge names its argument ──────────────────────────────────────────

/// The measured case, through the typed tool: `satisfies` sets the edge's
/// declared `evidence` and names the claim in `argued_by`; the reply and
/// get_node's edges read the claim's verdict beside the edge; what the edge
/// already carried is kept; a claim that names nothing is refused; and losing
/// the check names the edge under `arguments_lost`.
#[tokio::test(flavor = "multi_thread")]
async fn satisfies_names_its_claim_and_the_verdict_reads_beside_the_edge() {
    let (s, _) = imported(CASE).await;
    s.ok(
        "satisfies",
        json!({"from_id": CAP, "to_id": REQ, "coverage": "full"}),
    )
    .await;
    let reply = s
        .ok(
            "satisfies",
            json!({"from_id": CAP, "to_id": REQ, "evidence": "the class test", "argued_by": CLAIM}),
        )
        .await;
    assert_eq!(reply["argument"]["claim"], CLAIM, "{reply}");
    let rows = s.edges(REQ, "SATISFIES").await;
    let row = rows
        .iter()
        .find(|e| e["from_id"] == CAP)
        .expect("the SATISFIES row");
    assert_eq!(row["properties"]["evidence"], "the class test", "{row}");
    assert_eq!(row["properties"]["argued_by"], CLAIM, "{row}");
    assert_eq!(
        row["properties"]["coverage"], "full",
        "re-drawing dropped coverage: {row}"
    );
    let claim = s.claim(CLAIM).await;
    assert_eq!(row["argument"]["truth"], claim["truth"], "{row}");
    assert_eq!(
        row["argument"]["proof_status"], claim["proof_status"],
        "{row}"
    );

    let (ok, _, text) = s
        .call(
            "satisfies",
            json!({"from_id": CAP, "to_id": REQ, "argued_by": "req:no-such-claim"}),
        )
        .await;
    assert!(!ok, "a claim naming nothing was accepted");
    assert!(text.contains("req:no-such-claim"), "{text}");

    let r = s.radius(&[VER]).await;
    let lost = r["arguments_lost"]
        .as_array()
        .unwrap_or_else(|| panic!("losing the check names no argued edge: {r}"));
    assert!(
        lost.iter().any(|a| a["edge_type"] == "SATISFIES"
            && a["from_id"] == CAP
            && a["to_id"] == REQ
            && a["claim"] == CLAIM),
        "{lost:?}"
    );
}

/// EVERY SATISFIES and DEPENDS_ON edge of every fixture, given an `argued_by`
/// naming a claim a step concludes, reads that claim's verdict beside it on
/// get_node, equal to the claim's own truth in reasoning_report; and the
/// claim's read-back lists the edge under `argues_for`. Deleting the claim
/// afterwards leaves the edge reading `missing` and detect_defects reporting
/// it, never a silent nothing.
#[tokio::test(flavor = "multi_thread")]
async fn every_argued_edge_reads_its_claims_verdict_beside_it() {
    let mut asked = 0;
    for (name, fixture) in ALL {
        let (s, d) = imported(fixture).await;
        let d = doc(&d);
        let claims: Vec<String> = d
            .edges
            .iter()
            .filter(|e| e["edge_type"] == "CONCLUDES")
            .map(|e| e["to_id"].as_str().unwrap().to_string())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let Some(claim) = claims.first() else {
            continue;
        };
        for e in d
            .edges
            .iter()
            .filter(|e| e["edge_type"] == "SATISFIES" || e["edge_type"] == "DEPENDS_ON")
        {
            asked += 1;
            let (ty, f, t) = (
                e["edge_type"].as_str().unwrap(),
                e["from_id"].as_str().unwrap(),
                e["to_id"].as_str().unwrap(),
            );
            let tool = if ty == "SATISFIES" {
                "satisfies"
            } else {
                "depends_on"
            };
            s.ok(tool, json!({"from_id": f, "to_id": t, "argued_by": claim}))
                .await;
            let row = s
                .edges(t, ty)
                .await
                .into_iter()
                .find(|r| r["from_id"] == f)
                .expect("row");
            let c = s.claim(claim).await;
            assert_eq!(row["argument"]["claim"], *claim, "{name}: {row}");
            assert_eq!(row["argument"]["truth"], c["truth"], "{name}: {row}");
            assert!(
                c["argues_for"].as_array().is_some_and(|a| a
                    .iter()
                    .any(|x| x["edge_type"] == ty && x["from_id"] == f && x["to_id"] == t)),
                "{name}: {claim} does not list the edge it argues for: {c}"
            );
        }
    }
    assert!(asked > 0, "no fixture held a SATISFIES or DEPENDS_ON edge");

    // A claim deleted after the edge names it.
    let (s, _) = imported(CASE).await;
    s.ok(
        "satisfies",
        json!({"from_id": CAP, "to_id": REQ, "argued_by": CLAIM}),
    )
    .await;
    s.ok(
        "delete_node",
        json!({"node_type": "Requirement", "id": CLAIM}),
    )
    .await;
    let row = s
        .edges(REQ, "SATISFIES")
        .await
        .into_iter()
        .find(|r| r["from_id"] == CAP)
        .expect("row");
    assert_eq!(row["argument"]["reads"], "missing", "{row}");
    let defects = s.ok("detect_defects", json!({})).await.to_string();
    assert!(
        defects.contains("dangling_reference") && defects.contains(CLAIM),
        "the dangling argument is not reported"
    );
}

// ─── 4. Delivery argued along its own edge is circular ──────────────────────

/// The measured case: the claim that the capability satisfies the requirement
/// rests on the requirement, which holds only because the capability delivers
/// it along that very SATISFIES. Beside the edge it reads `circular`, and in
/// the claim's read-back too. Without the requirement as a premise, it reads
/// `argued`.
#[tokio::test(flavor = "multi_thread")]
async fn the_measured_claim_is_circular_until_it_stops_resting_on_the_delivery() {
    let (s, _) = imported(CASE).await;
    s.ok(
        "satisfies",
        json!({"from_id": CAP, "to_id": REQ, "argued_by": CLAIM}),
    )
    .await;
    let row = s
        .edges(REQ, "SATISFIES")
        .await
        .into_iter()
        .find(|r| r["from_id"] == CAP)
        .expect("row");
    assert_eq!(row["argument"]["reads"], "circular", "{row}");
    assert!(
        row["argument"]["circular"]
            .as_str()
            .is_some_and(|w| w.contains(REQ) && w.contains(CAP)),
        "{row}"
    );
    let c = s.claim(CLAIM).await;
    assert_eq!(c["argues_for"][0]["reads"], "circular", "{c}");

    s.ok(
        "delete_edge",
        json!({"edge_type": "PREMISE_OF", "from_id": REQ, "to_id": CLAIM_STEP}),
    )
    .await;
    let row = s
        .edges(REQ, "SATISFIES")
        .await
        .into_iter()
        .find(|r| r["from_id"] == CAP)
        .expect("row");
    assert_eq!(row["argument"]["reads"], "argued", "{row}");
}

/// THE CLASS, against an oracle read from the wire: for every SATISFIES X→Y of
/// every fixture, a claim "X satisfies Y" concluded by a step over X and Y,
/// named on the edge, reads `circular` exactly when Y holds with the edge and
/// would not hold without it (the edge deleted and Y read again).
#[tokio::test(flavor = "multi_thread")]
async fn a_claim_resting_on_delivery_along_its_own_edge_is_circular() {
    let mut asked = 0;
    for (name, fixture) in ALL {
        let (s, d) = imported(fixture).await;
        let d = doc(&d);
        for (i, e) in d
            .edges
            .iter()
            .filter(|e| e["edge_type"] == "SATISFIES")
            .enumerate()
        {
            let (f, t) = (e["from_id"].as_str().unwrap(), e["to_id"].as_str().unwrap());
            if !d.is(t, "Requirement") {
                continue;
            }
            asked += 1;
            // The oracle: Y with and without the edge.
            let with = s.claim(t).await["truth"].clone();
            s.ok(
                "delete_edge",
                json!({"edge_type": "SATISFIES", "from_id": f, "to_id": t}),
            )
            .await;
            let without = s.claim(t).await["truth"].clone();
            let props = e["properties"].as_object().cloned().unwrap_or_default();
            s.ok(
                "create_edge",
                json!({"edge_type": "SATISFIES", "from_id": f, "to_id": t, "props": props}),
            )
            .await;
            let expect_circular = with == "holds" && without != "holds";

            let claim = format!("req:claim-{i}-x-satisfies-y");
            s.ok(
                "add_requirement",
                json!({"id": claim, "name": format!("Claim {i}: {f} satisfies {t}"), "statement": format!("{f} satisfies {t}."), "statement_kind": "proposition"}),
            )
            .await;
            s.ok(
                "add_inference",
                json!({"id": format!("inf:claim-{i}"), "rule": "all", "premises": [f, t], "conclusion": claim, "warrant": "it is realized and the need is met"}),
            )
            .await;
            s.ok(
                "satisfies",
                json!({"from_id": f, "to_id": t, "argued_by": claim}),
            )
            .await;
            let row = s
                .edges(t, "SATISFIES")
                .await
                .into_iter()
                .find(|r| r["from_id"] == f)
                .expect("row");
            assert_eq!(
                row["argument"]["reads"] == "circular",
                expect_circular,
                "{name}: {f} -> {t} (with {with}, without {without}): {row}"
            );
        }
    }
    assert!(asked > 0, "no fixture held a SATISFIES to a requirement");
}

// ─── 5. The lineage vocabulary ──────────────────────────────────────────────

/// `derived` covers the owner's words chosen through a Decision, said where a
/// writer reads it (the schema and set_requirement_lineage); and the value set
/// is unchanged, so the schema stamp does not move.
#[tokio::test(flavor = "multi_thread")]
async fn derived_covers_the_owners_words_chosen_through_a_decision() {
    let s = session().await;
    let schema = s
        .ok(
            "describe_schema",
            json!({"node_type": "Requirement", "budget_chars": 400000}),
        )
        .await
        .to_string();
    assert!(
        schema.contains("the owner's own words chosen THROUGH a Decision"),
        "the schema's lineage does not cover the owner's words chosen through a Decision"
    );
    let tools = s.client.list_all_tools().await.expect("tools/list");
    let lineage = tools
        .iter()
        .find(|t| t.name == "set_requirement_lineage")
        .expect("set_requirement_lineage");
    let words = lineage.description.as_deref().unwrap_or_default();
    assert!(
        words.contains("the owner's words chosen through a Decision"),
        "{words}"
    );
    let r = s
        .ok(
            "add_requirement",
            json!({"id": "req:x", "name": "x", "statement": "x holds."}),
        )
        .await;
    let _ = r;
    for v in ["original", "decomposed", "derived"] {
        s.ok(
            "set_requirement_lineage",
            json!({"requirement_id": "req:x", "lineage": v}),
        )
        .await;
    }
    let (ok, _, _) = s
        .call(
            "set_requirement_lineage",
            json!({"requirement_id": "req:x", "lineage": "chosen"}),
        )
        .await;
    assert!(!ok, "a new lineage value was accepted: the value set moved");
}

// ─── 6. The served instructions say how ─────────────────────────────────────

/// capture-intent, brainstorm and the served instructions say how to record a
/// requirement's reason to exist and how to argue for an edge, in the same
/// words.
#[test]
fn the_served_instructions_say_how_to_record_a_reason_to_exist_and_argue_for_an_edge() {
    const REASON: &str = "A requirement's reason to exist is the Decision it is GOVERNED_BY";
    const EDGE: &str = "To argue for an edge, record the claim as a statement of its own";
    for (name, text) in [
        (
            "capture-intent",
            include_str!("../../../getting-started/skills/capture-intent/SKILL.md"),
        ),
        (
            "brainstorm",
            include_str!("../../../getting-started/skills/brainstorm/SKILL.md"),
        ),
        (
            "AGENTS.md",
            include_str!("../../../getting-started/AGENTS.md"),
        ),
    ] {
        assert!(text.contains(REASON), "{name} does not say: {REASON}");
        assert!(text.contains(EDGE), "{name} does not say: {EDGE}");
        assert!(
            text.contains("`argued_by`"),
            "{name} does not name `argued_by`"
        );
    }
}
