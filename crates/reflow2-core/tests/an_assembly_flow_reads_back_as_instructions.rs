//! AN ASSEMBLY FLOW READS BACK AS INSTRUCTIONS: each step names the parts it
//! puts together and the joins it closes, who performs it, its checks and its
//! limits — and the report says what the instructions lack.
//!
//! `req:a-step-of-a-process-names-the-parts-it-puts-together`
//! (`cap:a-step-names-the-parts-it-puts-together-and-the-joins-it-closes`),
//! settled by the owner on 2026-11-12. A Flow could say a ring is printed,
//! cast, polished, plated and set, and nothing could say which parts each
//! step puts together or which joins it makes, so how a ring is made and put
//! together could not be recorded.
//!
//! THE CASE: a hibiscus flower ring cast in sterling silver in one pour. Five
//! parts — the band, the cup, the petals, the stamen (with its bezel) and the
//! stone, a 2 mm white sapphire, bought. Four joins, each provided by one part
//! and consumed by the other: cup-to-band, petals-to-cup and stamen-to-cup,
//! each fused in the casting, and stone-in-bezel. Five people: a print
//! service, a caster, the owner (who polishes), a plater and a jeweler. Five
//! steps, in order: print the pattern, cast it, clean up and polish, plate it
//! rose gold, set the stone.

use reflow2_core::foundation::core::DynoError;
use reflow2_core::graph::DesignGraph;
use reflow2_core::nodes::{Props, edge, node};
use reflow2_core::propagate::PropagateOptions;
use reflow2_core::{FlowReport, FlowStep};

const FLOW: &str = "flow:make-the-ring";
const PRINT: &str = "cap:print-the-pattern";
const CAST: &str = "cap:cast-it";
const POLISH: &str = "cap:clean-up-and-polish";
const PLATE: &str = "cap:plate-it-rose-gold";
const SET: &str = "cap:set-the-stone";

const FUSED: &str = "fused in the casting";
const SEATED: &str =
    "the stone sits in the bezel seat with 0.05 mm a side; the lip is pushed over it";

fn ids(list: &[String]) -> Vec<String> {
    list.to_vec()
}

fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// A step of the ring's flow: its id, name, place, parts and joins.
type Step = (
    &'static str,
    &'static str,
    i64,
    &'static [&'static str],
    &'static [&'static str],
);

/// How one join reads: its id, the parts that provide and consume it, and the
/// steps that close it.
type JoinRead<'a> = (&'a str, Vec<String>, Vec<String>, Vec<String>);

fn interface(g: &mut DesignGraph, id: &str, name: &str, what_must_hold: &str) {
    g.add_interface(id, name).expect("interface");
    g.upsert_node(
        node::INTERFACE,
        id,
        Props::new().set("description", what_must_hold),
    )
    .expect("description");
}

fn actor(g: &mut DesignGraph, id: &str, name: &str) {
    g.create_node(node::ACTOR, id, Props::new().set("name", name))
        .expect("actor");
}

fn performs(g: &mut DesignGraph, actor_id: &str, step_id: &str) {
    g.create_edge(
        edge::INTERACTS_WITH,
        node::ACTOR,
        actor_id,
        node::CAPABILITY,
        step_id,
        Props::new().set("interaction", "performs"),
    )
    .expect("performs");
}

fn check(g: &mut DesignGraph, id: &str, name: &str, method: &str, status: &str, step_id: &str) {
    g.add_verification(id, name, Some(method), None, None)
        .expect("verification");
    g.set_verification_status(id, status, None, None)
        .expect("status");
    g.verifies(id, node::CAPABILITY, step_id).expect("verifies");
}

/// The hibiscus ring, as a design, with its assembly flow of `flow_type`.
fn ring(flow_type: &str) -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("open");
    g.add_project("proj:ring", "Hibiscus flower ring")
        .expect("project");

    for (id, name, purpose) in [
        ("cmp:band", "Band", "the ring that goes round the finger"),
        ("cmp:cup", "Cup", "the flower's cup, on the band"),
        ("cmp:petals", "Petals", "the five petals, round the cup"),
        (
            "cmp:stamen",
            "Stamen",
            "the stamen, with its bezel at the tip",
        ),
        ("cmp:stone", "Stone", "a 2 mm white sapphire, bought"),
    ] {
        g.add_component(id, name, purpose, None).expect("part");
    }
    // Each join is provided by one part and consumed by the other.
    for (id, name, holds, provider, consumer) in [
        (
            "ifc:cup-to-band",
            "Cup to band",
            FUSED,
            "cmp:band",
            "cmp:cup",
        ),
        (
            "ifc:petals-to-cup",
            "Petals to cup",
            FUSED,
            "cmp:cup",
            "cmp:petals",
        ),
        (
            "ifc:stamen-to-cup",
            "Stamen to cup",
            FUSED,
            "cmp:cup",
            "cmp:stamen",
        ),
        (
            "ifc:stone-in-bezel",
            "Stone in bezel",
            SEATED,
            "cmp:stamen",
            "cmp:stone",
        ),
    ] {
        interface(&mut g, id, name, holds);
        g.provides(provider, id).expect("provides");
        g.consumes(consumer, id).expect("consumes");
    }
    for (id, name) in [
        ("act:print-service", "The print service"),
        ("act:caster", "The caster"),
        ("act:owner", "The owner"),
        ("act:plater", "The plater"),
        ("act:jeweler", "The jeweler"),
    ] {
        actor(&mut g, id, name);
    }

    g.add_flow(
        FLOW,
        "Make the hibiscus ring",
        Some("printed, cast in sterling silver in one pour, finished and set"),
        Some(flow_type),
    )
    .expect("flow");
    let steps: [Step; 5] = [
        (
            PRINT,
            "Print the pattern",
            1,
            &["cmp:band", "cmp:cup", "cmp:petals", "cmp:stamen"],
            &[],
        ),
        (
            CAST,
            "Cast it",
            2,
            &[],
            &["ifc:cup-to-band", "ifc:petals-to-cup", "ifc:stamen-to-cup"],
        ),
        (POLISH, "Clean up and polish", 3, &[], &[]),
        (PLATE, "Plate it rose gold", 4, &[], &[]),
        (
            SET,
            "Set the stone",
            5,
            &["cmp:stone"],
            &["ifc:stone-in-bezel"],
        ),
    ];
    for (id, name, order, parts, joins) in steps {
        g.add_capability(
            id,
            name,
            &format!("{name}, a step of making the ring"),
            None,
        )
        .expect("step");
        g.draw_flow_step(id, FLOW, Some(order), &strings(parts), &strings(joins))
            .expect("the step, its parts and its joins");
    }
    performs(&mut g, "act:print-service", PRINT);
    performs(&mut g, "act:caster", CAST);
    performs(&mut g, "act:owner", POLISH);
    performs(&mut g, "act:plater", PLATE);
    performs(&mut g, "act:jeweler", SET);
    // An actor who only WATCHES a step does not perform it.
    g.create_edge(
        edge::INTERACTS_WITH,
        node::ACTOR,
        "act:owner",
        node::CAPABILITY,
        CAST,
        Props::new().set("interaction", "reads"),
    )
    .expect("reads");

    check(
        &mut g,
        "ver:casting-check",
        "The casting came out whole",
        "analysis",
        "passing",
        CAST,
    );
    check(
        &mut g,
        "ver:polish-inspection",
        "The surface is clean and polished",
        "inspection",
        "planned",
        POLISH,
    );
    check(
        &mut g,
        "ver:setting-inspection",
        "The stone is held and does not turn",
        "inspection",
        "planned",
        SET,
    );

    g.add_constraint(
        "con:one-pour",
        "One pour",
        "the whole ring is cast in sterling silver in one pour",
        None,
        None,
        None,
        None,
        None,
    )
    .expect("constraint");
    g.constrains(
        "con:one-pour",
        node::CAPABILITY,
        CAST,
        None,
        None,
        None,
        None,
    )
    .expect("limits the cast");
    g
}

fn report(g: &DesignGraph) -> FlowReport {
    g.flow_report(FLOW).expect("report")
}

fn step<'a>(r: &'a FlowReport, id: &str) -> &'a FlowStep {
    r.steps
        .iter()
        .find(|s| s.capability_id == id)
        .unwrap_or_else(|| panic!("{id} is a step: {r:#?}"))
}

fn named(list: &[reflow2_core::StepNamed]) -> Vec<(String, String)> {
    list.iter()
        .map(|n| (n.id.clone(), n.name.clone()))
        .collect()
}

fn pairs(v: &[(&str, &str)]) -> Vec<(String, String)> {
    v.iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

fn confession<'a>(r: &'a FlowReport, starts: &str) -> Option<&'a String> {
    r.confessions.iter().find(|c| c.contains(starts))
}

#[test]
fn the_ring_reads_back_as_instructions_in_order_with_every_field() {
    let g = ring("assembly");
    let r = report(&g);
    assert_eq!(r.flow_type.as_deref(), Some("assembly"));
    let order: Vec<&str> = r.steps.iter().map(|s| s.capability_id.as_str()).collect();
    assert_eq!(order, [PRINT, CAST, POLISH, PLATE, SET]);
    assert_eq!(r.entry_points, [PRINT]);
    assert_eq!(r.exit_points, [SET]);

    // 1 — print the pattern: the four printed parts, by the print service.
    let s = step(&r, PRINT);
    assert_eq!(s.step_order, Some(1));
    assert_eq!(
        named(&s.puts_together),
        pairs(&[
            ("cmp:band", "Band"),
            ("cmp:cup", "Cup"),
            ("cmp:petals", "Petals"),
            ("cmp:stamen", "Stamen"),
        ])
    );
    assert!(s.closes.is_empty());
    assert_eq!(
        named(&s.performed_by),
        pairs(&[("act:print-service", "The print service")])
    );
    assert!(s.checks.is_empty() && s.limits.is_empty());

    // 2 — cast it: the three joins fused in the casting, by the caster,
    // checked by analysis, limited to one pour. The owner only reads it.
    let s = step(&r, CAST);
    assert!(s.puts_together.is_empty());
    let joins: Vec<(&str, &str, Option<&str>)> = s
        .closes
        .iter()
        .map(|j| (j.id.as_str(), j.name.as_str(), j.what_must_hold.as_deref()))
        .collect();
    assert_eq!(
        joins,
        [
            ("ifc:cup-to-band", "Cup to band", Some(FUSED)),
            ("ifc:petals-to-cup", "Petals to cup", Some(FUSED)),
            ("ifc:stamen-to-cup", "Stamen to cup", Some(FUSED)),
        ]
    );
    assert_eq!(
        named(&s.performed_by),
        pairs(&[("act:caster", "The caster")])
    );
    assert_eq!(s.checks.len(), 1);
    assert_eq!(s.checks[0].id, "ver:casting-check");
    assert_eq!(s.checks[0].name, "The casting came out whole");
    assert_eq!(s.checks[0].method.as_deref(), Some("analysis"));
    assert_eq!(s.checks[0].status.as_deref(), Some("passing"));
    assert_eq!(named(&s.limits), pairs(&[("con:one-pour", "One pour")]));

    // 3 — clean up and polish: the owner, an inspection.
    let s = step(&r, POLISH);
    assert!(s.puts_together.is_empty() && s.closes.is_empty() && s.limits.is_empty());
    assert_eq!(named(&s.performed_by), pairs(&[("act:owner", "The owner")]));
    assert_eq!(
        s.checks
            .iter()
            .map(|c| (c.id.as_str(), c.method.as_deref(), c.status.as_deref()))
            .collect::<Vec<_>>(),
        [("ver:polish-inspection", Some("inspection"), Some("planned"))]
    );

    // 4 — plate it rose gold: the plater, and nothing else.
    let s = step(&r, PLATE);
    assert!(s.puts_together.is_empty() && s.closes.is_empty());
    assert!(s.checks.is_empty() && s.limits.is_empty());
    assert_eq!(
        named(&s.performed_by),
        pairs(&[("act:plater", "The plater")])
    );

    // 5 — set the stone: the stone, in its bezel, by the jeweler, inspected.
    let s = step(&r, SET);
    assert_eq!(named(&s.puts_together), pairs(&[("cmp:stone", "Stone")]));
    assert_eq!(s.closes.len(), 1);
    assert_eq!(s.closes[0].id, "ifc:stone-in-bezel");
    assert_eq!(s.closes[0].name, "Stone in bezel");
    assert_eq!(s.closes[0].what_must_hold.as_deref(), Some(SEATED));
    assert_eq!(
        named(&s.performed_by),
        pairs(&[("act:jeweler", "The jeweler")])
    );
    assert_eq!(s.checks.len(), 1);
    assert_eq!(s.checks[0].id, "ver:setting-inspection");
    assert_eq!(s.checks[0].method.as_deref(), Some("inspection"));

    // What the instructions lack, said in plain words: the two finishing
    // steps name no part and no join, and two steps have no check. Every
    // step has a performer, and every join between the ring's parts is closed.
    assert_eq!(r.confessions.len(), 2, "{:#?}", r.confessions);
    assert_eq!(
        r.confessions[0],
        format!(
            "2 of 5 step(s) put together no part and close no join — {POLISH}, {PLATE}. Read \
             as instructions, such a step gives only its name: which parts it handles and which \
             joins it makes were never recorded."
        )
    );
    assert_eq!(
        r.confessions[1],
        format!(
            "2 of 5 step(s) have no check — {PRINT}, {PLATE}: no Verification VERIFIES them, so \
             nothing says how what they do is confirmed."
        )
    );

    // The joins between the ring's own parts, and the steps that close them.
    let joins = g.assembly_joins(FLOW).expect("joins");
    let read: Vec<JoinRead> = joins
        .iter()
        .map(|j| {
            (
                j.id.as_str(),
                ids(&j.provided_by),
                ids(&j.consumed_by),
                ids(&j.closed_by),
            )
        })
        .collect();
    assert_eq!(
        read,
        [
            (
                "ifc:cup-to-band",
                strings(&["cmp:band"]),
                strings(&["cmp:cup"]),
                strings(&[CAST])
            ),
            (
                "ifc:petals-to-cup",
                strings(&["cmp:cup"]),
                strings(&["cmp:petals"]),
                strings(&[CAST])
            ),
            (
                "ifc:stamen-to-cup",
                strings(&["cmp:cup"]),
                strings(&["cmp:stamen"]),
                strings(&[CAST])
            ),
            (
                "ifc:stone-in-bezel",
                strings(&["cmp:stamen"]),
                strings(&["cmp:stone"]),
                strings(&[SET])
            ),
        ]
    );
}

/// On the wire, a step writes each list only when it has something in it, so
/// the plating step reads as its name, its place and who does it.
#[test]
fn a_step_writes_only_the_lists_it_has() {
    let g = ring("assembly");
    let v = serde_json::to_value(report(&g)).expect("serializes");
    let plate = v["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .find(|s| s["capability_id"] == PLATE)
        .expect("the plating step");
    let mut keys: Vec<&str> = plate
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["capability_id", "name", "performed_by", "step_order"]
    );
    let set = v["steps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["capability_id"] == SET)
        .unwrap();
    assert_eq!(set["closes"][0]["what_must_hold"], SEATED);
    assert_eq!(set["checks"][0]["method"], "inspection");
    assert_eq!(set["puts_together"][0]["name"], "Stone");
}

#[test]
fn a_step_nobody_performs_is_confessed() {
    let mut g = ring("assembly");
    assert!(confession(&report(&g), "name no one who performs them").is_none());
    assert!(
        g.delete_edge(edge::INTERACTS_WITH, "act:plater", PLATE)
            .expect("delete")
    );
    let r = report(&g);
    assert!(step(&r, PLATE).performed_by.is_empty());
    assert_eq!(
        confession(&r, "name no one who performs them").map(String::as_str),
        Some(
            format!(
                "1 of 5 step(s) name no one who performs them — {PLATE}: no Actor INTERACTS_WITH \
                 them with `interaction: performs`, so the instructions cannot say who does each."
            )
            .as_str()
        )
    );
}

#[test]
fn a_join_between_the_flows_own_parts_that_no_step_closes_is_confessed() {
    let mut g = ring("assembly");
    assert!(confession(&report(&g), "no step of the flow closes it").is_none());
    assert!(
        g.delete_edge(edge::CLOSES, SET, "ifc:stone-in-bezel")
            .expect("delete")
    );
    let r = report(&g);
    assert!(step(&r, SET).closes.is_empty());
    // The step still puts the stone in, so it is not a step that names nothing.
    assert!(
        !confession(&r, "put together no part")
            .unwrap()
            .contains(SET)
    );
    assert_eq!(
        confession(&r, "no step of the flow closes it").map(String::as_str),
        Some(
            "join 'ifc:stone-in-bezel' (Stone in bezel) is between 'cmp:stamen' (which provides \
             it) and 'cmp:stone' (which consumes it), parts this flow puts together, and no step \
             of the flow closes it — the instructions never say when it is made, or check that \
             what it must hold holds."
        )
    );
    let unclosed: Vec<String> = g
        .assembly_joins(FLOW)
        .expect("joins")
        .into_iter()
        .filter(|j| j.closed_by.is_empty())
        .map(|j| j.id)
        .collect();
    assert_eq!(unclosed, ["ifc:stone-in-bezel"]);
}

#[test]
fn a_step_with_no_check_is_confessed() {
    let mut g = ring("assembly");
    assert!(
        !confession(&report(&g), "have no check")
            .unwrap()
            .contains(CAST)
    );
    assert!(
        g.delete_edge(edge::VERIFIES, "ver:casting-check", CAST)
            .expect("delete")
    );
    let r = report(&g);
    assert!(step(&r, CAST).checks.is_empty());
    assert!(
        confession(&r, "have no check")
            .unwrap()
            .starts_with(&format!(
                "3 of 5 step(s) have no check — {PRINT}, {CAST}, {PLATE}:"
            )),
        "{:#?}",
        r.confessions
    );
}

#[test]
fn a_step_that_puts_together_nothing_and_closes_nothing_is_confessed() {
    let mut g = ring("assembly");
    for part in ["cmp:band", "cmp:cup", "cmp:petals", "cmp:stamen"] {
        assert!(
            g.delete_edge(edge::PUTS_TOGETHER, PRINT, part)
                .expect("delete")
        );
    }
    let r = report(&g);
    assert!(step(&r, PRINT).puts_together.is_empty());
    assert!(
        confession(&r, "put together no part")
            .unwrap()
            .starts_with(&format!("3 of 5 step(s) put together no part and close no join — {PRINT}, {POLISH}, {PLATE}.")),
        "{:#?}",
        r.confessions
    );
    // With the printed parts no longer the flow's, the joins among them are
    // not the flow's to ask about: only the stone is put together now.
    assert!(confession(&r, "no step of the flow closes it").is_none());
}

/// Any other flow reads as it always did: the parts and joins a step names
/// are listed, and the flow says it is not being read as instructions.
#[test]
fn a_flow_of_another_type_is_not_read_as_instructions_and_says_so() {
    let g = ring("process");
    let r = report(&g);
    for s in &r.steps {
        assert!(
            s.performed_by.is_empty() && s.checks.is_empty() && s.limits.is_empty(),
            "{s:#?}"
        );
    }
    assert_eq!(step(&r, PRINT).puts_together.len(), 4);
    assert_eq!(step(&r, CAST).closes.len(), 3);
    assert_eq!(
        r.confessions,
        [
            "3 of 5 step(s) put together parts or close joins, but this flow's flow_type is \
             `process`, not `assembly`, so it is not read as instructions: who performs each \
             step, its checks and its limits are not listed, and no join between its parts is \
             asked about. Recorded with flow_type `assembly` (add_flow), it is."
        ]
    );
}

/// ALL OR NOTHING: a part that is not a Component, a join that is not an
/// Interface, and an id naming nothing are each named in one refusal, and
/// nothing — not the step, not the parts that were right — is written.
#[test]
fn a_step_naming_a_wrong_part_or_join_is_refused_whole_and_writes_nothing() {
    let mut g = ring("assembly");
    g.add_capability("cap:polish-again", "Polish again", "a second polish", None)
        .expect("step");
    let err = g
        .draw_flow_step(
            "cap:polish-again",
            FLOW,
            Some(6),
            &strings(&["cmp:band", "ifc:cup-to-band", "cmp:nothing"]),
            &strings(&["ifc:stone-in-bezel", "cmp:stone"]),
        )
        .expect_err("refused");
    let DynoError::EdgeValidation {
        edge_type,
        property,
        message,
    } = &err
    else {
        panic!("an edge validation refusal: {err:?}");
    };
    assert_eq!(edge_type, "PART_OF_FLOW");
    assert_eq!(property, "puts_together, closes");
    assert!(message.starts_with("NOTHING WAS WRITTEN"), "{message}");
    for said in [
        "puts_together: 'ifc:cup-to-band' is an Interface, not a Component",
        "puts_together: 'cmp:nothing' names nothing in this design (it must be a Component)",
        "closes: 'cmp:stone' is a Component, not an Interface",
    ] {
        assert!(message.contains(said), "{said} — in: {message}");
    }
    assert!(
        !message.contains("'cmp:band'"),
        "a right id is not named: {message}"
    );
    assert!(
        !message.contains("'ifc:stone-in-bezel'"),
        "a right id is not named: {message}"
    );
    for e in [edge::PART_OF_FLOW, edge::PUTS_TOGETHER, edge::CLOSES] {
        assert!(
            g.outgoing("cap:polish-again", Some(e))
                .expect("read")
                .is_empty(),
            "{e} was written by a refused call"
        );
    }

    // A step or a flow of the wrong type is named the same way.
    let err = g
        .draw_flow_step("cmp:band", "cap:cast-it", None, &strings(&["cmp:cup"]), &[])
        .expect_err("refused");
    let text = err.to_string();
    assert!(
        text.contains("capability_id: 'cmp:band' is a Component, not a Capability"),
        "{text}"
    );
    assert!(
        text.contains("flow_id: 'cap:cast-it' is a Capability, not a Flow"),
        "{text}"
    );
}

/// The single-edge constructors refuse a wrong end by name, and a join handed
/// in as a part (or a part as a join) is told which edge takes it.
#[test]
fn each_edge_refuses_a_wrong_end_by_name() {
    let mut g = ring("assembly");
    let err = g
        .puts_together(SET, "ifc:stone-in-bezel")
        .expect_err("a join is not a part")
        .to_string();
    assert!(
        err.contains("'ifc:stone-in-bezel' is an Interface, not a Component")
            && err.contains("A join between two parts is CLOSES"),
        "{err}"
    );
    let err = g
        .closes(SET, "cmp:stone")
        .expect_err("a part is not a join")
        .to_string();
    assert!(
        err.contains("'cmp:stone' is a Component, not an Interface")
            && err.contains("is PUTS_TOGETHER"),
        "{err}"
    );
    let err = g
        .puts_together("act:jeweler", "cmp:stone")
        .expect_err("an actor is not a step")
        .to_string();
    assert!(
        err.contains("'act:jeweler' is an Actor, not a Capability"),
        "{err}"
    );
    let err = g
        .closes(SET, "ifc:nothing")
        .expect_err("names nothing")
        .to_string();
    assert!(
        err.contains("'ifc:nothing' names nothing in this design"),
        "{err}"
    );
}

/// Re-drawing a step keeps what the call leaves out: its parts, its joins and
/// its place, as every typed edge re-draw does.
#[test]
fn a_redrawn_step_keeps_what_the_call_leaves_out() {
    let mut g = ring("assembly");
    let drawn = g
        .draw_flow_step(SET, FLOW, None, &[], &[])
        .expect("redraw bare");
    assert_eq!(
        drawn
            .step
            .properties
            .get("step_order")
            .and_then(|v| v.as_i64()),
        Some(5),
        "the place is kept"
    );
    let drawn = g
        .draw_flow_step(SET, FLOW, None, &strings(&["cmp:stone", "cmp:stone"]), &[])
        .expect("redraw with the part once more");
    assert_eq!(
        drawn.puts_together.len(),
        1,
        "a part named twice is drawn once"
    );
    let r = report(&g);
    let s = step(&r, SET);
    assert_eq!(s.step_order, Some(5));
    assert_eq!(named(&s.puts_together), pairs(&[("cmp:stone", "Stone")]));
    assert_eq!(s.closes.len(), 1, "the join the call left out is kept");
}

/// Impact: a changed join reaches the step that closes it, and a changed part
/// the step that puts it together.
#[test]
fn a_changed_join_or_part_reaches_the_step_that_makes_it() {
    let g = ring("assembly");
    let reached = |seed: &str| -> Vec<(String, String)> {
        g.propagate_from(&[seed], PropagateOptions::default())
            .expect("radius")
            .impacted
            .into_iter()
            .filter(|n| n.distance == 1)
            .map(|n| {
                (
                    n.node_id,
                    n.via
                        .last()
                        .map(|h| h.edge_type.clone())
                        .unwrap_or_default(),
                )
            })
            .collect()
    };
    let from_join = reached("ifc:stone-in-bezel");
    assert!(
        from_join.contains(&(SET.to_string(), "CLOSES".to_string())),
        "{from_join:?}"
    );
    let from_part = reached("cmp:band");
    assert!(
        from_part.contains(&(PRINT.to_string(), "PUTS_TOGETHER".to_string())),
        "{from_part:?}"
    );
    // And the direction: from a part or a join, the step that makes it is
    // downstream — shaped by it, as a realization or a check is.
    let radius = g
        .propagate_from(&["ifc:stone-in-bezel"], PropagateOptions::default())
        .expect("radius");
    let set = radius
        .impacted
        .iter()
        .find(|n| n.node_id == SET)
        .expect("the setting step");
    assert_eq!(set.direction.as_str(), "downstream");
}

/// The served read counts the joins between an assembly's own parts, and
/// names the assembly with how many no step closes.
#[test]
fn the_served_read_counts_the_joins_between_an_assemblys_parts() {
    let mut g = ring("assembly");
    let only = ["assembly_instructions".to_string()];
    let r = g.derived_report(&only, 3).expect("derived_report");
    let t = &r.relations[0];
    assert_eq!(t.id, "assembly_instructions");
    assert_eq!(t.count, Some(4));
    assert_eq!(
        t.sample,
        [format!("{FLOW}: 4 join(s), 0 closed by no step")]
    );
    g.delete_edge(edge::CLOSES, SET, "ifc:stone-in-bezel")
        .expect("delete");
    let r = g.derived_report(&only, 3).expect("derived_report");
    assert_eq!(
        r.relations[0].sample,
        [format!("{FLOW}: 4 join(s), 1 closed by no step")]
    );
    // A flow of another type is not an assembly, and is not counted.
    let r = ring("process")
        .derived_report(&only, 3)
        .expect("derived_report");
    assert_eq!(r.relations[0].count, Some(0));
}
