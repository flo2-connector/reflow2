//! When the near-match check finds an existing node, the writer can say the new
//! node REPLACES it — and whichever way the writer chose, the choice is on the
//! record afterwards.
//!
//! # Why this exists
//!
//! `fact:root-cause-a-status-that-falls-behind-reality-is-silent-and-a-superseding-capability-leaves-no-trace-2026-09-30`.
//! The capture guard offered two routes past a same-type near-match: sharpen the
//! existing node, or create anyway with `distinct_from`. There was no route for
//! "this new node takes that one's place", and `distinct_from` was consumed in
//! the call and written nowhere. Measured on flo2's design the same day: three
//! functions shipped under NEW capabilities while their predecessors stayed
//! `planned`, with no edge between them, and two requirements still pointed only
//! at the old node. Nobody could say afterwards whether the writer of the new
//! node had ever seen the old one.
//!
//! # The shape, read off the design's own vocabulary
//!
//! - The supersession edge is `OBSOLETES`, drawn FROM THE SUCCESSOR — the edge
//!   the served retire-from-design skill names for "a Capability / Component
//!   with a successor". `SUPERSEDES` is declared for Fragment and Verification
//!   only, and which of the two survives is `dec:one-retire-edge`'s open
//!   question; nothing here settles it.
//! - The old node's final state, edges included, is snapshotted onto the
//!   timeline as a `deprecation` that `removed` it — retire-from-design's step 2
//!   — BEFORE its thread moves, so the moved edge survives as history.
//! - The old node's stored `status` does not move: it records what was BUILT
//!   (`dec:idea-does-a-capability-need-a-cancelled-state`). Withdrawing it from
//!   the detectors stays an accepted Decision's act
//!   (`dec:idea-discontinued-is-a-first-class-state`), and the reply says so.
//! - `distinct_from` is written onto the node it created, so "was the older
//!   node seen?" has an answer.

use reflow2_mcp::service::*;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

async fn svc() -> ReflowService {
    ReflowService::in_memory().expect("in-memory service")
}

/// The flo2 pair, abridged: a function planned under one node and built under
/// another that says nearly the same thing.
const OLD_NAME: &str = "A person lists and names their designs from chat";
const OLD_DESC: &str = "From a chat, a person lists the designs they hold and names each one, \
                        so they can pick which design to work on.";
const NEW_NAME: &str = "A person lists and names their designs from chat, served by the gateway";
const NEW_DESC: &str = "From a chat, a person lists the designs they hold and names each one, \
                        so they can pick which design to work on — served by the gateway.";

async fn doc(s: &ReflowService) -> Value {
    s.export_graph(Parameters(
        serde_json::from_value(json!({})).expect("export args"),
    ))
    .await
    .expect("export_graph")
    .structured_content
    .expect("structured")
}

fn has_edge(doc: &Value, edge: &str, from: &str, to: &str) -> bool {
    doc["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .any(|e| e["edge_type"] == edge && e["from_id"] == from && e["to_id"] == to)
}

fn edge<'a>(doc: &'a Value, edge: &str, from: &str, to: &str) -> Option<&'a Value> {
    doc["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .find(|e| e["edge_type"] == edge && e["from_id"] == from && e["to_id"] == to)
}

fn node<'a>(doc: &'a Value, id: &str) -> Option<&'a Value> {
    doc["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|n| n["node_id"] == id)
}

async fn get(s: &ReflowService, id: &str) -> Value {
    s.get_node(Parameters(
        serde_json::from_value(json!({ "id": id })).expect("get_node args"),
    ))
    .await
    .expect("get_node")
    .structured_content
    .expect("structured")["node"]
        .clone()
}

/// A requirement, and the planned capability that serves it.
async fn with_old_capability() -> ReflowService {
    let s = svc().await;
    s.add_requirement(Parameters(
        serde_json::from_value(json!({
            "id": "req:see-designs",
            "name": "A person can see which designs they have from chat",
            "statement": "From a chat session, a person can see every design they hold.",
        }))
        .expect("requirement args"),
    ))
    .await
    .expect("add_requirement");
    s.add_component(Parameters(
        serde_json::from_value(json!({
            "id": "cmp:connector",
            "name": "Connector",
            "description": "The chat connector.",
        }))
        .expect("component args"),
    ))
    .await
    .expect("add_component");
    s.add_capability(Parameters(
        serde_json::from_value(json!({
            "id": "cap:old",
            "name": OLD_NAME,
            "description": OLD_DESC,
            "satisfies": "req:see-designs",
            "allocated_to": "cmp:connector",
        }))
        .expect("capability args"),
    ))
    .await
    .expect("the first capability has nothing to resemble");
    s
}

fn new_capability(extra: Value) -> CapabilityReq {
    let mut v = json!({ "id": "cap:new", "name": NEW_NAME, "description": NEW_DESC });
    if let (Some(o), Some(x)) = (v.as_object_mut(), extra.as_object()) {
        for (k, val) in x {
            o.insert(k.clone(), val.clone());
        }
    }
    serde_json::from_value(v).expect(
        "the capture tool must accept this argument — `replaces` is the route this suite is \
         about",
    )
}

/// Precondition for everything below: the two really are near-matches, so the
/// guard stops the second capture and asks.
#[tokio::test]
async fn the_second_capture_is_stopped_at_the_near_match_check() {
    let s = with_old_capability().await;
    let err = s
        .add_capability(Parameters(new_capability(json!({}))))
        .await
        .expect_err("a same-type near-match must stop the capture");
    assert!(format!("{err:?}").contains("cap:old"), "{err:?}");
}

/// THE REFUSAL NAMES THREE ROUTES. Two was the defect: a writer whose new node
/// took the old one's place had only "distinct" to say, which was false.
#[tokio::test]
async fn the_refusal_names_sharpen_distinct_and_replaces() {
    let s = with_old_capability().await;
    let err = s
        .add_capability(Parameters(new_capability(json!({}))))
        .await
        .expect_err("stopped");
    let msg = format!("{err:?}");
    assert!(msg.contains("SHARPEN"), "route one: {msg}");
    assert!(msg.contains("distinct_from"), "route two: {msg}");
    assert!(
        msg.contains("replaces: [\\\"cap:old\\\"]") || msg.contains("replaces: [\"cap:old\"]"),
        "route three names the exact argument, with the id filled in: {msg}"
    );
}

/// `distinct_from` IS RECORDED. Before, the judgement was consumed in the call
/// and written nowhere, so no later reader could tell a writer who saw the old
/// node and judged it different from one who never looked.
#[tokio::test]
async fn distinct_from_is_recorded_on_the_node_it_created() {
    let s = with_old_capability().await;
    s.add_capability(Parameters(new_capability(
        json!({ "distinct_from": ["cap:old"] }),
    )))
    .await
    .expect("distinct_from is the deliberate route past the guard");
    let n = get(&s, "cap:new").await;
    assert_eq!(
        n["properties"]["distinct_from"],
        json!(["cap:old"]),
        "the judgement \"read cap:old, judged this different\" must be on the record: {n}"
    );
    // And it is a judgement, not a relation: nothing is drawn between them.
    let d = doc(&s).await;
    assert!(!has_edge(&d, "OBSOLETES", "cap:new", "cap:old"), "distinct is not replaces");
}

/// A second judgement on a revise ADDS to the first rather than erasing it —
/// constructors merge, and a judgement is history.
#[tokio::test]
async fn a_later_distinct_from_adds_to_the_recorded_judgement() {
    let s = with_old_capability().await;
    s.add_capability(Parameters(new_capability(
        json!({ "distinct_from": ["cap:old"] }),
    )))
    .await
    .expect("created");
    s.add_capability(Parameters(
        serde_json::from_value(json!({ "id": "cap:new", "distinct_from": ["req:see-designs"] }))
            .expect("revise args"),
    ))
    .await
    .expect("revised");
    let n = get(&s, "cap:new").await;
    let got: Vec<&str> = n["properties"]["distinct_from"]
        .as_array()
        .expect("recorded")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(got.contains(&"cap:old") && got.contains(&"req:see-designs"), "{n}");
}

/// THE CASE. A replacing capture moves the requirement's SATISFIES to the new
/// node, draws OBSOLETES from the successor, and records the old node's ending
/// on the timeline with its edges — so the moved link survives as history.
#[tokio::test]
async fn a_replacing_capture_moves_the_thread_and_retires_the_old_node() {
    let s = with_old_capability().await;
    let reply = s
        .add_capability(Parameters(new_capability(json!({ "replaces": ["cap:old"] }))))
        .await
        .expect("replaces is the third route past the guard")
        .structured_content
        .expect("structured");

    let d = doc(&s).await;
    assert!(
        has_edge(&d, "SATISFIES", "cap:new", "req:see-designs"),
        "the requirement's thread must now reach the new node"
    );
    assert!(
        !has_edge(&d, "SATISFIES", "cap:old", "req:see-designs"),
        "and no longer the old one — it stopped being true, so it moved"
    );
    let obs = edge(&d, "OBSOLETES", "cap:new", "cap:old")
        .expect("the successor OBSOLETES what it replaced (retire-from-design)");
    assert!(
        obs["properties"]["evidence"]
            .as_str()
            .is_some_and(|e| !e.trim().is_empty()),
        "a relation with no evidence cannot be checked or overturned: {obs}"
    );

    // The ending is on the timeline: a deprecation that removed cap:old, with a
    // snapshot that still holds the SATISFIES it had.
    let changed = d["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .find(|e| {
            e["edge_type"] == "CHANGED"
                && e["to_id"] == "cap:old"
                && e["properties"]["action"] == "removed"
        })
        .expect("a ChangeEvent records that cap:old was removed from the live design");
    let chg = node(&d, changed["from_id"].as_str().expect("from")).expect("the event");
    assert_eq!(chg["properties"]["change_type"], "deprecation", "{chg}");
    let snap = d["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|n| {
            n["node_type"] == "Snapshot"
                && n["properties"]["target_id"] == "cap:old"
                && n["properties"]["edges"]
                    .as_str()
                    .is_some_and(|e| e.contains("SATISFIES") && e.contains("req:see-designs"))
        });
    assert!(
        snap.is_some(),
        "the old node's final edges must survive as history before the thread moves"
    );

    // The old node keeps its stored status — it records what was built.
    let old = get(&s, "cap:old").await;
    assert_eq!(old["properties"]["status"], "planned", "{old}");

    // The reply says what moved, subject first.
    let drawn: Vec<&str> = reply["edges_drawn"]
        .as_array()
        .expect("every edge this call drew is named")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(drawn.contains(&"cap:new SATISFIES req:see-designs"), "{reply}");
    assert!(drawn.contains(&"cap:new OBSOLETES cap:old"), "{reply}");
    let said = reply["replaced"].to_string();
    assert!(
        said.contains("cap:old SATISFIES req:see-designs"),
        "the edge taken off the old node is named: {said}"
    );
    assert!(
        said.contains("cap:old ALLOCATED_TO cmp:connector"),
        "and what stays on it, so nothing is silently left behind: {said}"
    );
}

/// The fix-3 case: the successor ALREADY exists (it was built under its own
/// node), and the writer now says it replaces the older one. A revise carries
/// `replaces` exactly as a create does.
#[tokio::test]
async fn an_existing_node_can_be_said_to_replace_an_older_one() {
    let s = with_old_capability().await;
    s.add_capability(Parameters(new_capability(
        json!({ "distinct_from": ["cap:old"], "status": "realized" }),
    )))
    .await
    .expect("created as distinct first, the way flo2's were");
    s.add_capability(Parameters(
        serde_json::from_value(json!({ "id": "cap:new", "replaces": ["cap:old"] }))
            .expect("revise args"),
    ))
    .await
    .expect("a revise may say what it replaces");
    let d = doc(&s).await;
    assert!(has_edge(&d, "SATISFIES", "cap:new", "req:see-designs"));
    assert!(!has_edge(&d, "SATISFIES", "cap:old", "req:see-designs"));
    assert!(has_edge(&d, "OBSOLETES", "cap:new", "cap:old"));
}

/// A replacement is between two of the same kind. A Requirement is not a
/// Capability's predecessor; that pair is two layers, and the route for it is
/// `distinct_from`. Refused, and NOTHING is written.
#[tokio::test]
async fn replacing_a_node_of_another_type_is_refused_and_writes_nothing() {
    let s = with_old_capability().await;
    let err = s
        .add_capability(Parameters(new_capability(
            json!({ "replaces": ["req:see-designs"], "distinct_from": ["cap:old"] }),
        )))
        .await
        .expect_err("a capability cannot replace a requirement");
    let msg = format!("{err:?}");
    assert!(msg.contains("req:see-designs") && msg.contains("Requirement"), "{msg}");
    let d = doc(&s).await;
    assert!(node(&d, "cap:new").is_none(), "a refusal writes nothing");
}

#[tokio::test]
async fn replacing_an_id_that_names_nothing_is_refused_and_writes_nothing() {
    let s = with_old_capability().await;
    let err = s
        .add_capability(Parameters(new_capability(
            json!({ "replaces": ["cap:no-such"], "distinct_from": ["cap:old"] }),
        )))
        .await
        .expect_err("an unknown predecessor is a typo, not a judgement");
    assert!(format!("{err:?}").contains("cap:no-such"), "{err:?}");
    assert!(node(&doc(&s).await, "cap:new").is_none());
}

/// Saying one node is both distinct from and replaced by this one is two
/// contradictory judgements in one call.
#[tokio::test]
async fn distinct_and_replaces_on_the_same_id_is_refused() {
    let s = with_old_capability().await;
    let err = s
        .add_capability(Parameters(new_capability(
            json!({ "replaces": ["cap:old"], "distinct_from": ["cap:old"] }),
        )))
        .await
        .expect_err("contradiction");
    assert!(format!("{err:?}").contains("cap:old"), "{err:?}");
    assert!(node(&doc(&s).await, "cap:new").is_none());
}

/// A Component's thread is the capabilities allocated to it, so those move.
#[tokio::test]
async fn a_replacing_component_takes_the_old_ones_allocations() {
    let s = with_old_capability().await;
    s.add_component(Parameters(
        serde_json::from_value(json!({
            "id": "cmp:gateway",
            "name": "Connector gateway",
            "description": "The chat connector.",
            "replaces": ["cmp:connector"],
        }))
        .expect("component args"),
    ))
    .await
    .expect("a component may replace a component");
    let d = doc(&s).await;
    assert!(has_edge(&d, "ALLOCATED_TO", "cap:old", "cmp:gateway"));
    assert!(!has_edge(&d, "ALLOCATED_TO", "cap:old", "cmp:connector"));
    assert!(has_edge(&d, "OBSOLETES", "cmp:gateway", "cmp:connector"));
}

/// A Requirement's retirement is the owner's word (`dropped`), so a replacing
/// requirement draws the genealogy and moves NO satisfier: whether what was
/// built for the old wording meets the new one is a delivery claim nobody made.
#[tokio::test]
async fn a_replacing_requirement_draws_the_genealogy_and_moves_no_delivery_claim() {
    let s = with_old_capability().await;
    let reply = s
        .add_requirement(Parameters(
            serde_json::from_value(json!({
                "id": "req:see-and-start-designs",
                "name": "A person can see and start designs from chat",
                "statement": "From a chat session, a person can see every design they hold, \
                              and start a new one.",
                "replaces": ["req:see-designs"],
            }))
            .expect("requirement args"),
        ))
        .await
        .expect("a requirement may replace a requirement")
        .structured_content
        .expect("structured");
    let d = doc(&s).await;
    assert!(has_edge(&d, "OBSOLETES", "req:see-and-start-designs", "req:see-designs"));
    assert!(
        has_edge(&d, "SATISFIES", "cap:old", "req:see-designs"),
        "no satisfier is moved on a requirement's say-so"
    );
    let said = reply["replaced"].to_string();
    assert!(
        said.contains("set_requirement_status"),
        "the owner's step that retires it is named: {said}"
    );
}

/// A Decision OBSOLETING a Decision is `dec:reopen-supersedes`'s shape, and an
/// ACCEPTED one withdraws what it obsoletes — so the replaced decision reads
/// `discontinued` once the owner's word is on the new one.
#[tokio::test]
async fn an_accepted_replacing_decision_withdraws_the_old_one() {
    let s = svc().await;
    s.add_contributor(Parameters(
        serde_json::from_value(json!({ "id": "who:ann", "name": "Ann" })).expect("contributor"),
    ))
    .await
    .expect("add_contributor");
    s.add_decision(Parameters(
        serde_json::from_value(json!({
            "id": "dec:old",
            "name": "Send cumulative totals rather than deltas",
            "decision": "The outdoor unit sends cumulative totals rather than deltas.",
        }))
        .expect("decision args"),
    ))
    .await
    .expect("first decision");
    s.add_decision(Parameters(
        serde_json::from_value(json!({
            "id": "dec:new",
            "name": "Send cumulative totals rather than deltas, every minute",
            "decision": "The outdoor unit sends cumulative totals rather than deltas, every minute.",
            "status": "accepted",
            "approver": "who:ann",
            "acted_at": "2026-09-30",
            "replaces": ["dec:old"],
        }))
        .expect("decision args"),
    ))
    .await
    .expect("an accepted decision may replace a decision");
    let old = get(&s, "dec:old").await;
    assert_eq!(old["discontinued"], json!(true), "{old}");
}
