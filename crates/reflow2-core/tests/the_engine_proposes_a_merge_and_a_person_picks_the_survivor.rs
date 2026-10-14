//! WHICH NODE SURVIVES A MERGE IS A PERSON'S CHOICE. The engine proposes and a
//! person picks: `propose_heal` ranks both candidates and gives the reason for
//! each, recommends one only when a real difference separates them, and never
//! deletes on a tiebreak; `apply_heal` keeps the survivor the person chose.
//!
//! # The failure this pins
//!
//! `fact:root-cause-a-merge-deletes-the-wrong-node-because-the-survivor-rule-is-blind-to-status-priority-and-connections-2026-10-14`,
//! measured on 0.83.5 against reflow2's own design. The accepted rule
//! (`dec:merge-survivor-provenance`) picked the survivor by provenance and then
//! by node id, and nothing read status, priority or connections. The id decided
//! all 11 same-type pairs in the design, and both recorded cases reproduced:
//! - `ctr:anthony` (10 edges) kept over `who:ajs` (1,353 edges);
//! - a medium, accepted requirement kept over a critical, proposed one.
//!
//! And a person could not pick: `apply_heal` with keep and remove swapped
//! answered "operation … is not one HEAL proposes for this graph … Nothing was
//! changed."
//!
//! # The ruling
//!
//! Anthony, 2026-10-13, on item 23 of the judgement pass ("Go with your
//! recommendations"): "merge survivor: the engine proposes and a person picks,
//! after a root-cause pass".
//!
//! # Why these drive JSON
//!
//! `apply_heal` takes the proposal back as JSON off the MCP surface, so a person's
//! pick arrives as an edited JSON operation. These tests edit the proposal the
//! way a client does, which also lets them compile and FAIL against the build
//! before the fix rather than fail to compile.

use reflow2_core::nodes::{Props, edge, node};
use reflow2_core::{DesignGraph, HealOptions, HealProposal, HealReport};
use serde_json::{Value, json};

fn asserted_duplicate(g: &mut DesignGraph, node_type: &str, a: &str, b: &str) {
    g.create_edge(
        edge::DUPLICATES,
        node_type,
        a,
        node_type,
        b,
        Props::new().set("basis", "asserted"),
    )
    .unwrap();
}

fn proposal_json(g: &DesignGraph) -> Value {
    serde_json::to_value(g.propose_heal(HealOptions::default()).unwrap()).unwrap()
}

/// The proposal's one merge operation.
fn the_merge(p: &Value) -> Value {
    let ops = p["operations"].as_array().expect("operations is a list");
    assert_eq!(ops.len(), 1, "the fixture holds one duplicate pair: {p:#}");
    ops[0].clone()
}

fn apply(g: &mut DesignGraph, p: Value) -> Result<HealReport, String> {
    let proposal: HealProposal =
        serde_json::from_value(p).expect("a client can send this proposal back");
    g.apply_heal(&proposal).map_err(|e| e.to_string())
}

fn exists(g: &DesignGraph, node_type: &str, id: &str) -> bool {
    g.get_node(node_type, id).unwrap().is_some()
}

/// Swap keep and remove in operation `i`, the edit a person makes to keep the
/// other node.
fn swap_survivor(p: &mut Value, i: usize) {
    let m = &mut p["operations"][i]["op"]["Merge"];
    let (keep_id, remove_id) = (m["keep_id"].clone(), m["remove_id"].clone());
    let (keep_type, remove_type) = (m["keep_type"].clone(), m["remove_type"].clone());
    m["keep_id"] = remove_id;
    m["remove_id"] = keep_id;
    m["keep_type"] = remove_type;
    m["remove_type"] = keep_type;
}

/// Two capabilities a person asserted are one thing; `cap:a` carries more of
/// the design (it satisfies a requirement as well), so it is the recommendation.
fn connected_pair() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.add_project("proj:x", "X").unwrap();
    g.add_requirement("req:r", "R", "need r").unwrap();
    g.add_component("cmp:c", "C", "part c", None).unwrap();
    g.add_capability("cap:a", "Cap A", "does a", None).unwrap();
    g.add_capability("cap:b", "Cap B", "also does a", None)
        .unwrap();
    g.satisfies("cap:a", "req:r").unwrap();
    g.allocate("cap:a", "cmp:c").unwrap();
    g.allocate("cap:b", "cmp:c").unwrap();
    asserted_duplicate(&mut g, node::CAPABILITY, "cap:a", "cap:b");
    g
}

/// Two capabilities that differ in nothing but their ids: same provenance,
/// same status, same connections.
fn tied_pair() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.add_project("proj:x", "X").unwrap();
    g.add_component("cmp:c", "C", "part c", None).unwrap();
    g.add_capability("cap:p", "Cap P", "does p", None).unwrap();
    g.add_capability("cap:q", "Cap Q", "does p too", None)
        .unwrap();
    g.allocate("cap:p", "cmp:c").unwrap();
    g.allocate("cap:q", "cmp:c").unwrap();
    asserted_duplicate(&mut g, node::CAPABILITY, "cap:p", "cap:q");
    g
}

// ---- 1 · a person's pick is applied ----------------------------------------

#[test]
fn a_person_who_swaps_the_survivor_keeps_their_choice() {
    let mut g = connected_pair();
    let mut p = proposal_json(&g);
    let merge = the_merge(&p);
    assert_eq!(
        merge["op"]["Merge"]["keep_id"], "cap:a",
        "the engine recommends the better-connected node"
    );

    // The person keeps cap:b instead.
    swap_survivor(&mut p, 0);
    let report = apply(&mut g, p).expect("a person's pick between the offered pair is applied");
    assert!(report.applied, "{report:?}");
    assert!(report.verified, "{report:?}");
    assert!(
        exists(&g, node::CAPABILITY, "cap:b"),
        "the person's choice survives"
    );
    assert!(
        !exists(&g, node::CAPABILITY, "cap:a"),
        "and the other is merged away"
    );
    // The merge still carries the removed node's structure onto the survivor.
    let sats = g.outgoing("cap:b", Some(edge::SATISFIES)).unwrap();
    assert_eq!(sats.len(), 1, "cap:a's SATISFIES moved to cap:b");
    assert_eq!(sats[0].to_id, "req:r");
}

#[test]
fn a_person_can_name_the_survivor_without_reordering_the_operation() {
    let mut g = connected_pair();
    let mut p = proposal_json(&g);
    p["operations"][0]["chosen_survivor"] = json!("cap:b");
    let report = apply(&mut g, p).expect("chosen_survivor names the node to keep");
    assert!(report.verified, "{report:?}");
    assert!(exists(&g, node::CAPABILITY, "cap:b"));
    assert!(!exists(&g, node::CAPABILITY, "cap:a"));
}

#[test]
fn a_chosen_survivor_outside_the_pair_is_refused_before_any_write() {
    let mut g = connected_pair();
    let mut p = proposal_json(&g);
    p["operations"][0]["chosen_survivor"] = json!("req:r");
    let err = apply(&mut g, p).expect_err("only one of the two candidates can survive");
    assert!(
        err.contains("chosen_survivor"),
        "the refusal names the field: {err}"
    );
    assert!(err.contains("Nothing was changed"), "{err}");
    assert!(exists(&g, node::CAPABILITY, "cap:a"));
    assert!(exists(&g, node::CAPABILITY, "cap:b"));
}

#[test]
fn the_recommendation_applied_unchanged_still_keeps_the_recommendation() {
    let mut g = connected_pair();
    let p = proposal_json(&g);
    let report = apply(&mut g, p).expect("the round trip still works");
    assert!(report.verified, "{report:?}");
    assert!(exists(&g, node::CAPABILITY, "cap:a"));
    assert!(!exists(&g, node::CAPABILITY, "cap:b"));
}

// ---- 2 · the engine ranks, explains, and recommends only on a real difference

#[test]
fn a_merge_carries_both_candidates_ranked_with_their_reasons() {
    let g = connected_pair();
    let p = proposal_json(&g);
    let survivor = &the_merge(&p)["survivor"];
    assert_eq!(survivor["recommendation"], "cap:a", "{survivor:#}");
    assert_eq!(survivor["needs_choice"], false, "{survivor:#}");
    assert_eq!(survivor["decided_by"], "connections", "{survivor:#}");
    let candidates = survivor["candidates"].as_array().expect("candidates");
    assert_eq!(candidates.len(), 2, "both candidates are listed");
    assert_eq!(candidates[0]["node_id"], "cap:a", "recommended first");
    assert_eq!(candidates[1]["node_id"], "cap:b");
    assert_eq!(candidates[0]["connections"], 2, "SATISFIES + ALLOCATED_TO");
    assert_eq!(candidates[1]["connections"], 1, "ALLOCATED_TO");
    for c in candidates {
        let reasons = c["reasons"]
            .as_array()
            .expect("each candidate gives its reasons");
        let text = reasons
            .iter()
            .map(|r| r.as_str().unwrap_or_default())
            .collect::<Vec<_>>()
            .join(" | ");
        for signal in ["provenance", "connections", "status", "priority", "newer"] {
            assert!(
                text.contains(signal),
                "{} must say its {signal}: {text}",
                c["node_id"]
            );
        }
    }
    let reason = survivor["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("connections"),
        "the reason names what decided: {reason}"
    );
}

#[test]
fn a_pair_tied_on_everything_needs_a_person_and_is_never_applied_without_one() {
    let mut g = tied_pair();
    let p = proposal_json(&g);
    let merge = the_merge(&p);
    let survivor = &merge["survivor"];
    assert_eq!(survivor["needs_choice"], true, "{survivor:#}");
    assert!(survivor["recommendation"].is_null(), "{survivor:#}");
    let reason = survivor["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("no recommendation: a person must pick"),
        "it says plainly that a person must pick: {reason}"
    );
    assert!(
        p["summary"]
            .as_str()
            .unwrap_or_default()
            .contains("a person"),
        "and the summary says a choice is waiting: {}",
        p["summary"]
    );

    // Applied as proposed: refused, and nothing changes.
    let err = apply(&mut g, p.clone()).expect_err("a merge that needs a choice and got none");
    assert!(err.contains("a person must pick"), "{err}");
    assert!(
        err.contains("chosen_survivor"),
        "the refusal says how to pick: {err}"
    );
    assert!(err.contains("Nothing was changed"), "{err}");
    assert!(exists(&g, node::CAPABILITY, "cap:p"));
    assert!(exists(&g, node::CAPABILITY, "cap:q"));

    // The order of keep and remove is not a choice here: both orders are
    // equally the engine's, so a swap alone is refused too.
    let mut swapped = p.clone();
    swap_survivor(&mut swapped, 0);
    let err = apply(&mut g, swapped).expect_err("reordering is not picking");
    assert!(err.contains("a person must pick"), "{err}");

    // Dropping the marker does not get round it: apply re-derives the tie.
    let mut stripped = p.clone();
    stripped["operations"][0]
        .as_object_mut()
        .unwrap()
        .remove("survivor");
    let err =
        apply(&mut g, stripped).expect_err("the tie is read from the design, not the proposal");
    assert!(err.contains("a person must pick"), "{err}");
    assert!(exists(&g, node::CAPABILITY, "cap:p"));
    assert!(exists(&g, node::CAPABILITY, "cap:q"));

    // The person picks: cap:q survives.
    let mut picked = p;
    picked["operations"][0]["chosen_survivor"] = json!("cap:q");
    let report = apply(&mut g, picked).expect("with a pick, the merge applies");
    assert!(report.verified, "{report:?}");
    assert!(
        exists(&g, node::CAPABILITY, "cap:q"),
        "the person's choice survives"
    );
    assert!(!exists(&g, node::CAPABILITY, "cap:p"));
}

#[test]
fn the_id_never_decides_whichever_way_the_ids_sort() {
    // The same tie with the ids reversed against the edge's direction: still a
    // person's choice. A tiebreak on the id would recommend one of them.
    for (a, b) in [("cap:a", "cap:z"), ("cap:z", "cap:a")] {
        let mut g = DesignGraph::open_in_memory().unwrap();
        g.add_project("proj:x", "X").unwrap();
        g.add_capability("cap:a", "A", "does a", None).unwrap();
        g.add_capability("cap:z", "Z", "does a too", None).unwrap();
        asserted_duplicate(&mut g, node::CAPABILITY, a, b);
        let p = proposal_json(&g);
        let survivor = &the_merge(&p)["survivor"];
        assert_eq!(
            survivor["needs_choice"], true,
            "{a} DUPLICATES {b}: {survivor:#}"
        );
        assert!(survivor["recommendation"].is_null(), "{survivor:#}");
    }
}

#[test]
fn the_alphabet_no_longer_chooses_and_no_longer_says_it_did() {
    let g = tied_pair();
    let p = proposal_json(&g);
    let destroy = p["would_destroy"].as_array().expect("would_destroy");
    assert_eq!(destroy.len(), 1, "{p:#}");
    let note = destroy[0]["reason"].as_str().unwrap_or_default();
    assert!(
        !note.contains("ALPHABET"),
        "the alphabet no longer chooses: {note}"
    );
    assert!(
        note.contains("a person must pick"),
        "the disclosure says the victim is the person's to choose: {note}"
    );
}

#[test]
fn a_withdrawn_node_never_outlives_a_live_one_however_connected() {
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.add_project("proj:x", "X").unwrap();
    g.create_node(
        node::DECISION,
        "dec:a-old",
        Props::new()
            .set("name", "Old")
            .set("decision", "the old way")
            .set("status", "superseded"),
    )
    .unwrap();
    g.create_node(
        node::DECISION,
        "dec:b-live",
        Props::new()
            .set("name", "Live")
            .set("decision", "the way now")
            .set("status", "accepted"),
    )
    .unwrap();
    // The superseded one is the better connected.
    for i in 0..3 {
        let id = format!("req:r{i}");
        g.add_requirement(&id, "R", "need").unwrap();
        g.create_edge(
            edge::GOVERNED_BY,
            node::REQUIREMENT,
            &id,
            node::DECISION,
            "dec:a-old",
            Props::new(),
        )
        .unwrap();
    }
    asserted_duplicate(&mut g, node::DECISION, "dec:a-old", "dec:b-live");
    let p = proposal_json(&g);
    let survivor = &the_merge(&p)["survivor"];
    assert_eq!(survivor["recommendation"], "dec:b-live", "{survivor:#}");
    assert_eq!(survivor["decided_by"], "standing", "{survivor:#}");
    assert_eq!(the_merge(&p)["op"]["Merge"]["keep_id"], "dec:b-live");
}

// ---- 3 · the two recorded cases --------------------------------------------

#[test]
fn the_contributor_case_recommends_who_ajs_for_its_connections() {
    // fact:apply-heal-would-have-destroyed-the-wrong-contributor, in
    // miniature: the alphabet kept ctr:anthony (few edges, no description)
    // and deleted who:ajs (the identity the design uses everywhere).
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.add_project("proj:x", "X").unwrap();
    g.add_contributor(
        "who:ajs",
        "Anthony Sligar",
        Some("person"),
        Some("ajs"),
        Some("the owner"),
    )
    .unwrap();
    g.add_contributor("ctr:anthony", "Anthony", Some("person"), None, None)
        .unwrap();
    for i in 0..12 {
        let id = format!("req:by-ajs-{i:02}");
        g.add_requirement(&id, "R", "need").unwrap();
        g.authored_by(node::REQUIREMENT, &id, "who:ajs", None, Some("2026-08-01"))
            .unwrap();
    }
    for i in 0..2 {
        let id = format!("req:by-anthony-{i}");
        g.add_requirement(&id, "R", "need").unwrap();
        g.authored_by(
            node::REQUIREMENT,
            &id,
            "ctr:anthony",
            None,
            Some("2026-08-09"),
        )
        .unwrap();
    }
    asserted_duplicate(&mut g, node::CONTRIBUTOR, "who:ajs", "ctr:anthony");

    let p = proposal_json(&g);
    let merge = the_merge(&p);
    let survivor = &merge["survivor"];
    assert_eq!(survivor["recommendation"], "who:ajs", "{survivor:#}");
    assert_eq!(survivor["decided_by"], "connections", "{survivor:#}");
    let reason = survivor["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("12") && reason.contains("2"),
        "the reason gives both counts: {reason}"
    );
    assert_eq!(merge["op"]["Merge"]["keep_id"], "who:ajs");
    assert_eq!(merge["op"]["Merge"]["remove_id"], "ctr:anthony");
    assert_eq!(p["would_destroy"][0]["reference"], "ctr:anthony");
}

#[test]
fn the_requirement_case_recommends_the_critical_one_for_its_priority() {
    // fact:defect-merge-survivor-ignores-priority-and-status, in miniature:
    // both authored, so the alphabet kept the medium, accepted requirement and
    // deleted the critical, proposed one. The accepted one is also the better
    // connected here, as in the scratch reproduction (2 against 1).
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.add_project("proj:x", "X").unwrap();
    g.create_node(
        node::REQUIREMENT,
        "req:graph_is_the_content_bible",
        Props::new()
            .set("name", "The graph is the content bible")
            .set("statement", "panels are grounded")
            .set("priority", "critical")
            .set("status", "proposed")
            .set("provenance", "authored"),
    )
    .unwrap();
    g.create_node(
        node::REQUIREMENT,
        "req:backend_as_product",
        Props::new()
            .set("name", "Backend as product")
            .set("statement", "third parties consume the API")
            .set("priority", "medium")
            .set("status", "accepted")
            .set("provenance", "authored"),
    )
    .unwrap();
    g.add_capability("cap:api", "API", "serves the graph", None)
        .unwrap();
    g.satisfies("cap:api", "req:backend_as_product").unwrap();
    asserted_duplicate(
        &mut g,
        node::REQUIREMENT,
        "req:graph_is_the_content_bible",
        "req:backend_as_product",
    );

    let p = proposal_json(&g);
    let merge = the_merge(&p);
    let survivor = &merge["survivor"];
    assert_eq!(
        survivor["recommendation"], "req:graph_is_the_content_bible",
        "{survivor:#}"
    );
    assert_eq!(survivor["decided_by"], "priority", "{survivor:#}");
    let reason = survivor["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("critical"),
        "the reason names the priority: {reason}"
    );
    assert!(
        reason.contains("accepted"),
        "and says what points the other way, so the person sees it: {reason}"
    );
    assert_eq!(
        merge["op"]["Merge"]["keep_id"],
        "req:graph_is_the_content_bible"
    );
    let destroy = &p["would_destroy"][0];
    assert_eq!(destroy["reference"], "req:backend_as_product");
    assert!(
        !destroy["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("ALPHABET"),
        "{destroy:#}"
    );
}
