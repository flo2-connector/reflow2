//! REASONING CROSSES DESIGNS WITHIN A HUB
//! (`req:reasoning-crosses-designs-within-a-hub`, B of Anthony's 2026-10-18
//! build list, phase 4b).
//!
//! Measured 2026-10-18 on 0.87.0 (flo2.io), case 3 of the reasoning proof of
//! concept: flo2-calc's step citing a theorem proved in design
//! ef749c7f6e03a853 read its premise, a reference `xref:<F>:<theorem>`, as
//! "reflow2 computes no truth for a Resource", so its conclusion read
//! unproven although the theorem is shown where it lives; and breaking the
//! theorem's ground in its own design walked a radius with
//! `boundary_crossings: []` that never reached calc, directly or through the
//! hub.
//!
//! WHAT IS ASSERTED, over the real binary serving a hub H with members F (a
//! proof) and C (a design citing it) behind a trusted gateway — the shape of
//! that case:
//!   · (1) A REFERENCE USED AS A PREMISE CARRIES THE FAR NODE'S TRUTH AND PROOF
//!     STATUS: through H, C's claim reads `shown` (F's theorem holds there and
//!     its step is unconfirmed), and the premise's reading names F and its
//!     proof status there
//!   · (2) A CHANGE WALK CONTINUES INTO DESIGNS THAT HOLD A REFERENCE TO WHAT IT
//!     REACHED: a walk from F's axiom through H names C's reference in
//!     `boundary_crossings` and carries a radius in C under `across_designs`
//!   · (3) STALE OR UNREACHABLE READS AS SUCH, NEVER AS HOLDING: a reference to
//!     a node F does not hold reads `stale`; with F outside what the caller
//!     may read, C's premise reads unreachable, its claim unproven, and the
//!     reply reveals nothing F holds
//!
//! Written first and run against main 62e9c84f3, where each of the three
//! fails (the premise reads "computes no truth for a Resource").

use serde_json::{Value, json};

use super::a_hubs_address_serves_a_view_over_its_members::{
    GATEWAY, Session, mint_holding, ok, serve, tmp_root,
};

/// H, F and C under one root. F holds an axiom taken as given and a theorem a
/// step concludes from it; C holds a claim a step concludes from a reference
/// to F's theorem, and a second claim resting on a reference to a node F does
/// not hold. Returns (root, H, F, C).
fn proof_and_citer(tag: &str) -> (std::path::PathBuf, String, String, String) {
    let root = tmp_root(tag);
    let mut ids = Vec::new();
    for (dir, reqs) in [
        ("hub", vec![("req:hub", "the hub")]),
        (
            "fermat",
            vec![
                ("req:theorem", "a Fermat test failure proves n composite"),
                ("req:axiom", "Fermat's little theorem"),
            ],
        ),
        (
            "calc",
            vec![
                ("req:claim", "a failed Fermat test answered false is proven"),
                ("req:other-claim", "a claim resting on something gone"),
            ],
        ),
    ] {
        let d = root.join(dir);
        std::fs::create_dir_all(&d).unwrap();
        let reqs: Vec<(String, String)> = reqs
            .into_iter()
            .map(|(i, s)| (i.to_string(), s.to_string()))
            .collect();
        ids.push(mint_holding(&d, &reqs));
    }
    (root, ids[0].clone(), ids[1].clone(), ids[2].clone())
}

/// Write the reasoning into F and C, each on its own address.
fn write_reasoning(port: u16, f: &str, c: &str) {
    let mut on_f = Session::open(port, f);
    ok(
        &on_f.call(
            "add_requirement",
            json!({"id": "req:axiom", "taken_as_given": true}),
            None,
            None,
        ),
        "the axiom is taken as given in F",
    );
    ok(
        &on_f.call(
            "add_inference",
            json!({"id": "inf:theorem", "rule": "all", "premises": ["req:axiom"],
                   "conclusion": "req:theorem"}),
            None,
            None,
        ),
        "F's step concludes the theorem",
    );
    let mut on_c = Session::open(port, c);
    for (claim, far) in [
        ("req:claim", "req:theorem"),
        ("req:other-claim", "req:gone"),
    ] {
        ok(
            &on_c.call(
                "review_relations",
                json!({"node_id": claim, "links": [{"relation": "DEPENDS_ON",
                       "other_design": f, "other_id": far,
                       "evidence": "cites the result proved there"}]}),
                None,
                None,
            ),
            "C links its claim to F's node",
        );
        ok(
            &on_c.call(
                "add_inference",
                json!({"id": format!("inf:{}", &claim[4..]), "rule": "all",
                       "premises": [format!("xref:{f}:{far}")], "conclusion": claim}),
                None,
                None,
            ),
            "C's step rests on the reference",
        );
    }
}

fn premise_of(node: &Value) -> Value {
    node["node"]["inference_reading"]["premises"][0].clone()
}

#[test]
fn a_reference_used_as_a_premise_carries_the_far_nodes_truth_and_proof_status() {
    let (root, h, f, c) = proof_and_citer("far-truth");
    let server = serve(&root, Some(GATEWAY));
    write_reasoning(server.port, &f, &c);
    let mut on_h = Session::open(server.port, &h);
    let readable = [h.as_str(), f.as_str(), c.as_str()];
    let view = [f.as_str(), c.as_str()];

    let claim = ok(
        &on_h.call(
            "get_node",
            json!({"id": format!("{c}:req:claim")}),
            Some(&readable),
            Some(&view),
        ),
        "C's claim through H",
    );
    assert_eq!(
        claim["node"]["proof_status"]["status"], "shown",
        "C's claim rests on F's theorem, which holds there by an unconfirmed step: {claim}"
    );
    let step = ok(
        &on_h.call(
            "get_node",
            json!({"id": format!("{c}:inf:claim")}),
            Some(&readable),
            Some(&view),
        ),
        "C's step through H",
    );
    let premise = premise_of(&step);
    assert_eq!(premise["truth"], "holds", "{step}");
    let why = premise["why"].as_str().unwrap_or_default();
    assert!(
        why.contains(&f) && why.contains("shown"),
        "the premise says where it holds and its proof status there: {why}"
    );
}

#[test]
fn a_stale_or_unreachable_far_end_reads_as_such_and_never_as_holding() {
    let (root, h, f, c) = proof_and_citer("far-stale");
    let server = serve(&root, Some(GATEWAY));
    write_reasoning(server.port, &f, &c);
    let mut on_h = Session::open(server.port, &h);
    let view = [f.as_str(), c.as_str()];

    // STALE: F holds no req:gone.
    let readable = [h.as_str(), f.as_str(), c.as_str()];
    let step = ok(
        &on_h.call(
            "get_node",
            json!({"id": format!("{c}:inf:other-claim")}),
            Some(&readable),
            Some(&view),
        ),
        "C's second step through H",
    );
    let premise = premise_of(&step);
    assert_eq!(premise["truth"], "unknown", "{step}");
    assert!(
        premise["why"]
            .as_str()
            .unwrap_or_default()
            .contains("stale"),
        "a far node that is gone reads stale: {step}"
    );

    // UNREACHABLE: the caller may not read F.
    let readable = [h.as_str(), c.as_str()];
    let view = [c.as_str()];
    let reply = on_h.call(
        "get_node",
        json!({"id": format!("{c}:inf:claim")}),
        Some(&readable),
        Some(&view),
    );
    let step = ok(&reply, "C's step with F unreadable");
    let premise = premise_of(&step);
    assert_eq!(premise["truth"], "unknown", "{step}");
    assert!(
        premise["why"]
            .as_str()
            .unwrap_or_default()
            .contains("unreachable"),
        "{step}"
    );
    assert!(
        !reply.raw.contains("a Fermat test failure proves"),
        "nothing F holds is revealed to a caller who may not read it: {}",
        reply.raw
    );
    let claim = ok(
        &on_h.call(
            "get_node",
            json!({"id": format!("{c}:req:claim")}),
            Some(&readable),
            Some(&view),
        ),
        "C's claim with F unreadable",
    );
    assert_eq!(
        claim["node"]["proof_status"]["status"], "unproven",
        "{claim}"
    );
}

#[test]
fn a_change_walk_continues_into_a_design_that_holds_a_reference_to_what_it_reached() {
    let (root, h, f, c) = proof_and_citer("far-walk");
    let server = serve(&root, Some(GATEWAY));
    write_reasoning(server.port, &f, &c);
    let mut on_h = Session::open(server.port, &h);
    let readable = [h.as_str(), f.as_str(), c.as_str()];
    let view = [f.as_str(), c.as_str()];

    let radius = ok(
        &on_h.call(
            "propagate_from",
            json!({"seed_ids": [format!("{f}:req:axiom")]}),
            Some(&readable),
            Some(&view),
        ),
        "a walk from F's axiom through H",
    );
    let reference = format!("{c}:xref:{f}:req:theorem");
    let crossings: Vec<String> = radius["boundary_crossings"]
        .as_array()
        .unwrap_or_else(|| panic!("a radius names its crossings: {radius}"))
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    assert!(
        crossings.contains(&reference),
        "the walk crosses into C at its reference to F's theorem: {radius}"
    );
    let across = radius["across_designs"]
        .as_array()
        .unwrap_or_else(|| panic!("the radius carries the designs it continued into: {radius}"));
    let into_c = across
        .iter()
        .find(|a| a["design"] == c.as_str())
        .unwrap_or_else(|| panic!("the walk continued into C: {radius}"));
    assert!(
        into_c["entered_at"].to_string().contains(&reference),
        "{into_c}"
    );
    assert!(
        into_c["radius"]["inferences"]
            .to_string()
            .contains("inf:claim"),
        "C's step resting on the reference is in C's radius: {into_c}"
    );

    // And with C outside what the caller may read, the walk does not enter it.
    let readable = [h.as_str(), f.as_str()];
    let view = [f.as_str()];
    let radius = ok(
        &on_h.call(
            "propagate_from",
            json!({"seed_ids": [format!("{f}:req:axiom")]}),
            Some(&readable),
            Some(&view),
        ),
        "the walk with C unreadable",
    );
    assert!(
        !radius.to_string().contains(&c),
        "nothing of C is revealed to a caller who may not read it: {radius}"
    );
}
