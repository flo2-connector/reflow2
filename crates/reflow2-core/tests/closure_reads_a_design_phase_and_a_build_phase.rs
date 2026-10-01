//! Closure reads a DESIGN phase beside the BUILD phase
//! (`req:closure-reads-design-done-separately-from-build-done`, Anthony
//! 2026-09-29; root cause
//! `fact:root-cause-closure-cannot-read-design-done-because-traceability-was-bound-to-the-delivery-line-2026-09-29`).
//!
//! The case these pin was measured on a real design: a finished design with
//! nothing built, every requirement satisfied by an allocated capability with
//! a check planned, read traceability 0/7, because the only traceability
//! predicate closure had was the delivery line. Its first hole was a
//! requirement the design had PARKED under an accepted decision, because
//! closure did not read parking either. And its owner's rule, "design done
//! may close a budget on an estimate; build done needs measurements", could
//! not be computed, because the budgets leg never read `basis`.
//!
//! The assertions read the SERIALIZED report, which is what every consumer
//! (the MCP reply, release_report) receives — and which lets these compile
//! against the code they were written to fail on.

use reflow2_core::DesignGraph;
use reflow2_core::nodes::node;
use serde_json::Value as Json;

fn json<T: serde::Serialize>(t: &T) -> Json {
    serde_json::to_value(t).expect("the report serializes")
}

/// A design finished on paper: the requirement is satisfied by a capability
/// that is allocated to a part and has a check planned. Nothing is built.
fn designed() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.add_project("proj:p", "P").unwrap();
    g.add_requirement("req:ship", "It ships", "The thing must ship.")
        .unwrap();
    g.set_requirement_status("req:ship", "accepted").unwrap();
    g.add_component("cmp:engine", "Engine", "does the work", None)
        .unwrap();
    g.add_capability("cap:ship", "Ship it", "ships the thing", Some("planned"))
        .unwrap();
    g.satisfies("cap:ship", "req:ship").unwrap();
    g.allocate("cap:ship", "cmp:engine").unwrap();
    g.add_verification("ver:ship", "ship test", Some("test"), None, None)
        .unwrap();
    g.verifies("ver:ship", node::CAPABILITY, "cap:ship")
        .unwrap();
    g
}

/// The same thread, built and checked: realized, and its check passing.
fn built() -> DesignGraph {
    let mut g = designed();
    g.add_capability("cap:ship", "Ship it", "ships the thing", Some("realized"))
        .unwrap();
    g.add_artifact(
        "art:engine",
        "engine.rs",
        Some("code"),
        Some("src/engine.rs"),
    )
    .unwrap();
    g.realizes("art:engine", node::CAPABILITY, "cap:ship", None, None)
        .unwrap();
    g.set_verification_status("ver:ship", "passing", None, None)
        .unwrap();
    g
}

fn leg<'a>(reading: &'a Json, name: &str) -> &'a Json {
    reading["legs"]
        .as_array()
        .expect("a reading has legs")
        .iter()
        .find(|l| l["leg"] == name)
        .unwrap_or_else(|| panic!("no {name} leg in {reading}"))
}

#[test]
fn a_design_allocated_traced_and_checked_on_paper_closes_in_the_design_phase_and_not_in_the_build_phase()
 {
    let mut g = designed();
    g.set_closure_criterion("proj:p", &["traceability"], 100.0)
        .unwrap();
    let r = json(&g.closure_report().unwrap());

    // The top level stays the BUILD reading — never replaced — and says so.
    assert_eq!(r["phase"], "build", "{r}");
    assert_eq!(r["verdict"], "does_not_close", "{r}");

    // The design reading is carried beside it, and closes.
    let d = &r["design"];
    assert_eq!(d["phase"], "design", "{r}");
    assert_eq!(d["verdict"], "closes", "{r}");
    assert_eq!(leg(d, "traceability")["closed"], 1, "{d}");
}

#[test]
fn the_design_phase_needs_the_allocation_and_the_planned_check() {
    // No planned check: traced and allocated is not enough.
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.add_project("proj:p", "P").unwrap();
    g.add_requirement("req:ship", "It ships", "The thing must ship.")
        .unwrap();
    g.add_component("cmp:engine", "Engine", "does the work", None)
        .unwrap();
    g.add_capability("cap:ship", "Ship it", "ships the thing", Some("planned"))
        .unwrap();
    g.satisfies("cap:ship", "req:ship").unwrap();
    g.allocate("cap:ship", "cmp:engine").unwrap();
    g.set_closure_criterion("proj:p", &["traceability"], 100.0)
        .unwrap();
    let r = json(&g.closure_report().unwrap());
    let d = &r["design"];
    assert_eq!(d["verdict"], "does_not_close", "{r}");
    assert_eq!(d["first_hole"]["id"], "req:ship", "{r}");
    let why = d["first_hole"]["why"].as_str().unwrap_or_default();
    assert!(
        why.contains("check"),
        "the hole names what is missing: {why}"
    );

    // A planned check but no allocation: nobody owns delivering it.
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.add_project("proj:p", "P").unwrap();
    g.add_requirement("req:ship", "It ships", "The thing must ship.")
        .unwrap();
    g.add_capability("cap:ship", "Ship it", "ships the thing", Some("planned"))
        .unwrap();
    g.satisfies("cap:ship", "req:ship").unwrap();
    g.add_verification("ver:ship", "ship test", Some("test"), None, None)
        .unwrap();
    g.verifies("ver:ship", node::CAPABILITY, "cap:ship")
        .unwrap();
    g.set_closure_criterion("proj:p", &["traceability"], 100.0)
        .unwrap();
    let r = json(&g.closure_report().unwrap());
    let d = &r["design"];
    assert_eq!(d["verdict"], "does_not_close", "{r}");
    let why = d["first_hole"]["why"].as_str().unwrap_or_default();
    assert!(
        why.contains("allocated"),
        "the hole names what is missing: {why}"
    );
}

#[test]
fn a_budget_closing_on_estimates_closes_at_design_and_not_at_build() {
    let mut g = built();
    g.add_constraint(
        "con:mass",
        "Mass",
        "Under 100 kg.",
        Some("budget"),
        Some("mass"),
        Some(100.0),
        None,
        Some("maximum"),
    )
    .unwrap();
    g.set_constraint_unit("con:mass", "kg").unwrap();
    g.set_constraint_provenance("con:mass", Some("asserted"), Some("who:ajs"), None)
        .unwrap();
    g.constrains_in(
        "con:mass",
        "Component",
        "cmp:engine",
        Some(60.0),
        Some("kg"),
        Some("estimated"),
        Some("who:ajs"),
        None,
        None,
    )
    .unwrap();
    g.set_closure_criterion("proj:p", &["budgets"], 100.0)
        .unwrap();
    let r = json(&g.closure_report().unwrap());

    // BUILD: an estimate inside the limit is not a measured close.
    assert_eq!(r["verdict"], "does_not_close", "{r}");
    assert_eq!(r["first_hole"]["id"], "con:mass", "{r}");
    let why = r["first_hole"]["why"].as_str().unwrap_or_default();
    assert!(why.contains("measured"), "the hole says why: {why}");
    // DESIGN: the same estimate, carrying its basis, closes.
    assert_eq!(r["design"]["verdict"], "closes", "{r}");

    // Measured, it closes at build too.
    g.add_component("cmp:frame", "Frame", "holds it", None)
        .unwrap();
    g.constrains_in(
        "con:mass",
        "Component",
        "cmp:frame",
        Some(30.0),
        Some("kg"),
        Some("measured"),
        Some("scale"),
        Some("2026-09-29"),
        None,
    )
    .unwrap();
    g.constrains_in(
        "con:mass",
        "Component",
        "cmp:engine",
        Some(60.0),
        Some("kg"),
        Some("measured"),
        Some("scale"),
        Some("2026-09-29"),
        None,
    )
    .unwrap();
    let r = json(&g.closure_report().unwrap());
    assert_eq!(r["verdict"], "closes", "{r}");
    assert_eq!(r["design"]["verdict"], "closes", "{r}");
}

#[test]
fn a_parked_requirement_is_counted_as_parked_never_as_the_first_hole() {
    let mut g = built();
    g.add_requirement(
        "req:buy-hardware",
        "The owner buys the hardware",
        "A person's act; nothing in the design can satisfy it.",
    )
    .unwrap();
    g.add_decision(
        "dec:the-owner-acts-for-hardware",
        "Hardware is the owner's act",
        "Anything that spends money is left for the owner.",
        None,
    )
    .unwrap();
    g.set_decision_status("dec:the-owner-acts-for-hardware", "accepted")
        .unwrap();
    g.governed_by(
        node::REQUIREMENT,
        "req:buy-hardware",
        node::DECISION,
        "dec:the-owner-acts-for-hardware",
        Some("parks"),
        None,
    )
    .unwrap();
    g.set_closure_criterion("proj:p", &["traceability"], 100.0)
        .unwrap();
    let r = json(&g.closure_report().unwrap());

    for reading in [&r, &r["design"]] {
        assert_eq!(reading["verdict"], "closes", "{r}");
        let t = leg(reading, "traceability");
        assert_eq!(
            t["swept"], 1,
            "the parked requirement leaves the population: {t}"
        );
        assert_eq!(t["parked"], 1, "and is counted as parked: {t}");
        let note = t["swept_note"].as_str().unwrap_or_default();
        assert!(note.contains("parked"), "the note says so: {note}");
    }
}

#[test]
fn a_proposed_ruling_parks_nothing() {
    // A musing must not suppress a hole — the same rule every parking
    // reader keeps.
    let mut g = built();
    g.add_requirement("req:later", "Later", "Something for later.")
        .unwrap();
    g.add_decision("dec:maybe", "Maybe", "Thinking about it.", None)
        .unwrap();
    g.governed_by(
        node::REQUIREMENT,
        "req:later",
        node::DECISION,
        "dec:maybe",
        Some("parks"),
        None,
    )
    .unwrap();
    g.set_closure_criterion("proj:p", &["traceability"], 100.0)
        .unwrap();
    let r = json(&g.closure_report().unwrap());
    assert_eq!(r["verdict"], "does_not_close", "{r}");
    assert_eq!(r["first_hole"]["id"], "req:later", "{r}");
    assert_eq!(r["design"]["first_hole"]["id"], "req:later", "{r}");
}

#[test]
fn every_reading_names_its_phase_so_design_done_cannot_be_quoted_as_done() {
    let mut g = designed();
    g.set_closure_criterion("proj:p", &["traceability"], 100.0)
        .unwrap();
    let r = json(&g.closure_report().unwrap());
    for l in r["legs"].as_array().expect("legs") {
        assert_eq!(l["phase"], "build", "{l}");
    }
    for l in r["design"]["legs"].as_array().expect("design legs") {
        assert_eq!(l["phase"], "design", "{l}");
    }
    let build_note = r["note"].as_str().unwrap_or_default();
    let design_note = r["design"]["note"].as_str().unwrap_or_default();
    assert!(build_note.starts_with("BUILD"), "{build_note}");
    assert!(design_note.starts_with("DESIGN"), "{design_note}");
    assert!(
        design_note.contains("nothing about whether"),
        "a design verdict disclaims the build: {design_note}"
    );

    // The release report carries both verdicts, each named.
    g.add_release("rel:1", "1.0", Some("1.0"), Some("binary"))
        .unwrap();
    let rr = json(&g.release_report("rel:1").unwrap());
    assert_eq!(rr["closure"]["phase"], "build", "{rr}");
    assert_eq!(rr["closure"]["verdict"], "does_not_close", "{rr}");
    assert_eq!(rr["closure"]["design_verdict"], "closes", "{rr}");
}
