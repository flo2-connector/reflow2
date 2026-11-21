//! EVERY FLOW THAT COULD BE RECORDED BEFORE A STEP COULD NAME ITS PARTS READS
//! BACK BYTE FOR BYTE AS IT DID.
//!
//! `req:a-step-of-a-process-names-the-parts-it-puts-together` gave a step two
//! new edges (PUTS_TOGETHER, CLOSES) and `flow_report` new fields for an
//! `assembly` flow: who performs each step, its checks and its limits. A flow
//! of any other type already carries the edges those fields read — a
//! Verification VERIFIES a step, a Constraint or DesignRule CONSTRAINS one, an
//! Actor INTERACTS_WITH one, and `interaction` was free text, so "performs"
//! could already be written — so a report that read them for every flow would
//! have changed every existing flow's answer. This pins that it does not.
//!
//! The scenario holds every shape a flow could take before the change, one
//! flow per existing `flow_type`: ordered and unordered steps, steps sharing a
//! position, a capability in two flows, transitions with and without a role,
//! a loop and a self-trigger, a flow with no steps; and on its steps every
//! edge the new fields read, including an actor that "performs" a step.
//!
//! THE GOLDEN WAS TAKEN ON origin/main 3133057e6, BEFORE THE CHANGE, with
//! `REFLOW2_BLESS_FLOW_GOLDEN=1`, which is only ever right on code that cannot
//! yet record an assembly. It holds each report as the exact string
//! `serde_json` writes, so a reordered or added field fails, not only a
//! changed value.

use std::collections::BTreeMap;

use reflow2_core::graph::DesignGraph;
use reflow2_core::nodes::{Props, edge, node};

const GOLDEN: &str = "tests/fixtures/flow_reports_before_a_step_named_its_parts.json";

fn cap(g: &mut DesignGraph, id: &str, name: &str) {
    g.add_capability(id, name, &format!("{name}, a step"), None)
        .expect("capability");
}

fn triggers(g: &mut DesignGraph, from: &str, to: &str, role: Option<&str>) {
    g.create_edge(
        edge::TRIGGERS,
        node::CAPABILITY,
        from,
        node::CAPABILITY,
        to,
        Props::new().set_opt("role", role),
    )
    .expect("triggers");
}

fn actor(g: &mut DesignGraph, id: &str, name: &str) {
    g.create_node(node::ACTOR, id, Props::new().set("name", name))
        .expect("actor");
}

fn interacts(g: &mut DesignGraph, actor_id: &str, target_id: &str, interaction: Option<&str>) {
    g.create_edge(
        edge::INTERACTS_WITH,
        node::ACTOR,
        actor_id,
        node::CAPABILITY,
        target_id,
        Props::new().set_opt("interaction", interaction),
    )
    .expect("interacts");
}

/// One design holding a flow of every type that existed before `assembly`,
/// with every edge the instruction fields read drawn onto their steps.
fn design() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("open");
    g.add_project("proj:flows", "Flows of every kind")
        .expect("project");

    // A process with a loop, an unroled transition and an unordered step.
    g.add_flow(
        "flow:process",
        "Make and check",
        Some("a process with a feedback loop"),
        Some("process"),
    )
    .expect("flow");
    for (id, name, order) in [
        ("cap:prepare", "Prepare", Some(1)),
        ("cap:make", "Make", Some(2)),
        ("cap:check", "Check", Some(3)),
        ("cap:tidy", "Tidy", None),
    ] {
        cap(&mut g, id, name);
        g.part_of_flow(id, "flow:process", order).expect("step");
    }
    triggers(&mut g, "cap:prepare", "cap:make", Some("feeds"));
    triggers(&mut g, "cap:make", "cap:check", None);
    triggers(&mut g, "cap:check", "cap:make", Some("forces a redo"));

    // Every edge an assembly's instructions read, on a process's steps.
    g.add_verification(
        "ver:make-check",
        "The made thing is checked",
        Some("inspection"),
        None,
        None,
    )
    .expect("verification");
    g.set_verification_status("ver:make-check", "passing", None, None)
        .expect("status");
    g.verifies("ver:make-check", node::CAPABILITY, "cap:make")
        .expect("verifies");
    g.add_verification(
        "ver:prepare-check",
        "Preparation is reviewed",
        None,
        None,
        None,
    )
    .expect("verification");
    g.verifies("ver:prepare-check", node::CAPABILITY, "cap:prepare")
        .expect("verifies");
    g.add_constraint(
        "con:time",
        "Time to make",
        "making takes at most an hour",
        None,
        Some("time_min"),
        Some(60.0),
        None,
        None,
    )
    .expect("constraint");
    g.constrains(
        "con:time",
        node::CAPABILITY,
        "cap:make",
        Some(40.0),
        None,
        None,
        None,
    )
    .expect("constrains");
    g.add_design_rule(
        "rule:clean",
        "Clean as you go",
        "every step leaves the bench clean",
        None,
        None,
    )
    .expect("rule");
    g.constrains_by(
        node::DESIGN_RULE,
        "rule:clean",
        node::CAPABILITY,
        "cap:tidy",
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .expect("constrains");
    actor(&mut g, "act:maker", "The maker");
    actor(&mut g, "act:checker", "The checker");
    actor(&mut g, "act:owner", "The owner");
    interacts(&mut g, "act:maker", "cap:make", Some("performs"));
    interacts(&mut g, "act:checker", "cap:check", Some("triggers"));
    interacts(&mut g, "act:owner", "cap:prepare", None);
    // Structure beside the steps: an allocation and a contract between parts.
    g.add_component("cmp:bench", "Bench", "where the making happens", None)
        .expect("component");
    g.add_component("cmp:tool", "Tool", "what the making uses", None)
        .expect("component");
    g.allocate("cap:make", "cmp:bench").expect("allocate");
    g.add_interface("ifc:bench-tool", "Bench to tool")
        .expect("interface");
    g.provides("cmp:bench", "ifc:bench-tool").expect("provides");
    g.consumes("cmp:tool", "ifc:bench-tool").expect("consumes");

    // A data flow whose first two steps share a position.
    g.add_flow("flow:data", "Readings in", None, Some("data_flow"))
        .expect("flow");
    for (id, name, order) in [
        ("cap:read-a", "Read A", Some(1)),
        ("cap:read-b", "Read B", Some(1)),
        ("cap:merge", "Merge", Some(2)),
    ] {
        cap(&mut g, id, name);
        g.part_of_flow(id, "flow:data", order).expect("step");
    }
    interacts(&mut g, "act:owner", "cap:read-a", Some("reads"));

    // A control flow of one step that triggers itself.
    g.add_flow("flow:control", "Retry", None, Some("control_flow"))
        .expect("flow");
    cap(&mut g, "cap:retry", "Retry");
    g.part_of_flow("cap:retry", "flow:control", Some(1))
        .expect("step");
    triggers(&mut g, "cap:retry", "cap:retry", Some("retries"));

    // A decision flow with no steps at all.
    g.add_flow("flow:decision", "Choose", None, Some("decision_flow"))
        .expect("flow");

    // Capture, retrieval and generation, reusing a step from the process.
    g.add_flow("flow:capture", "Capture", None, Some("capture"))
        .expect("flow");
    g.part_of_flow("cap:make", "flow:capture", Some(1))
        .expect("step");
    g.part_of_flow("cap:check", "flow:capture", None)
        .expect("step");
    g.add_flow("flow:retrieval", "Retrieve", None, Some("retrieval"))
        .expect("flow");
    g.part_of_flow("cap:read-a", "flow:retrieval", None)
        .expect("step");
    g.part_of_flow("cap:read-b", "flow:retrieval", None)
        .expect("step");
    g.add_flow(
        "flow:generation",
        "Generate",
        Some("made up"),
        Some("generation"),
    )
    .expect("flow");
    g.part_of_flow("cap:merge", "flow:generation", Some(7))
        .expect("step");

    // A flow whose type nobody stated: the schema's default applies.
    g.add_flow("flow:unstated", "Unstated", None, None)
        .expect("flow");
    g.part_of_flow("cap:tidy", "flow:unstated", Some(1))
        .expect("step");
    g
}

const FLOWS: &[&str] = &[
    "flow:process",
    "flow:data",
    "flow:control",
    "flow:decision",
    "flow:capture",
    "flow:retrieval",
    "flow:generation",
    "flow:unstated",
];

#[test]
fn every_flow_of_an_existing_type_reads_back_byte_for_byte_as_it_did() {
    let g = design();
    let now: BTreeMap<&str, String> = FLOWS
        .iter()
        .map(|f| {
            let report = g.flow_report(f).expect("report");
            (*f, serde_json::to_string(&report).expect("serializes"))
        })
        .collect();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(GOLDEN);
    if std::env::var_os("REFLOW2_BLESS_FLOW_GOLDEN").is_some() {
        std::fs::write(&path, serde_json::to_string_pretty(&now).unwrap() + "\n").unwrap();
        return;
    }
    let golden: BTreeMap<String, String> =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("the golden exists"))
            .expect("the golden parses");
    assert_eq!(
        golden.keys().map(String::as_str).collect::<Vec<_>>(),
        now.keys().copied().collect::<Vec<_>>(),
        "every flow is in the golden"
    );
    for (flow, report) in &now {
        assert_eq!(
            report, &golden[*flow],
            "{flow}: the report moved from what it read before a step could name its parts"
        );
    }
}
