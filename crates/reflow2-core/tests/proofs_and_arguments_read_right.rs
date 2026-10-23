//! PROOFS AND ARGUMENTS READ RIGHT — PR A1, queued on Anthony's word of
//! 2026-10-22 ("Go with your recommendations (for all the open decisions)").
//!
//! 1. **The root cause the heat rung found**
//!    (`fact:root-cause-proof-statements-fall-through-to-software-predicates-
//!    and-assumed-premises-stopped-counting-2026-10-22`):
//!    - (a) an ASSUMED premise takes part in the reasoning again — 4879171c5
//!      (2026-10-17) narrowed `takes_part_in_reasoning` with the what-if's
//!      `steps_resting_on`;
//!    - (b) a HYPOTHESIS has a truth of its own: it holds only under itself,
//!      never "not delivered";
//!    - (c) `encoding_undecided` never asks a statement for a software `kind`;
//!    - (d) proof-shaped gaps follow from what the design HOLDS, and the
//!      Project's `domain` is only a fallback: the same design reads the same
//!      under `software`, `document` and `proof`, and no proof design is asked
//!      for tests, conventions, a deployment, a structure or a quality target.
//! 2. **Contested, not unproven**
//!    (`req:a-conclusion-whose-every-route-is-contested-reads-contested`): a
//!    reductio whose ⊥ rests on a contested premise is a contested route.
//! 3. **A what-if walks past a falsum**: the Lucas–Penrose trial's "the what-if
//!    walk stops at the falsum and never reaches the reductio or the
//!    conclusion", and the four colour rung's ⊥ that read `refuted` when its
//!    derivation was lost.
//! 4. **A bridge premise kind**
//!    (`dec:idea-a-bridge-premise-says-a-formal-statement-models-a-claim`,
//!    option b).
//! 5. **The circularity check**
//!    (`dec:idea-speculate-the-bridge-that-would-close-a-gap-then-test-it`,
//!    option c).
//! 6. **Proven equivalence**
//!    (`req:two-statements-can-be-recorded-as-proven-equivalent`).
//!
//! The measured cases are the trial designs exported from flo2.io on
//! 2026-10-23 (reflow2 0.90.0): the heat-equation rung (7f5bd325ad4e8c91), the
//! Lucas–Penrose argument (cd9bedc1cb114de6), the Riemann map
//! (5cdc6110fcaa5ad5), the Navier–Stokes map (ad65e4983833f39f) and the four
//! colour rung (7d851f5be7cca068), with the Gettier rung (57e91debfac7c608)
//! and the committed logic rung.
//!
//! MEASURED BEFORE (0.90.0's detect_gaps on each export, by kind):
//! - heat: 11 — artifact_standard_undeclared, build_without_governance,
//!   build_without_verification, change_axis_unstated,
//!   closure_criterion_undeclared, concept_without_design, encoding_undecided,
//!   no_deploy_operate, quality_target_unstated, unconfirmed_reasoning,
//!   unlinked_intent (six hypotheses);
//! - Navier–Stokes: 6 — agent_instructions_unregistered,
//!   build_without_governance, build_without_verification, encoding_undecided,
//!   no_deploy_operate, unconfirmed_reasoning;
//! - Lucas–Penrose: 6 — build_without_governance, build_without_verification,
//!   encoding_undecided, no_deploy_operate, unconfirmed_reasoning,
//!   unlinked_intent;
//! - Riemann: 6 — build_without_governance, encoding_undecided,
//!   no_deploy_operate, reasoning_not_as_stated, unconfirmed_reasoning,
//!   unlinked_intent;
//! - four colour: 10 — build_without_governance, encoding_undecided,
//!   no_deploy_operate, reasoning_not_as_stated ×5, unconfirmed_reasoning,
//!   unlinked_intent.
//!
//! AFTER: heat 2 (change_axis_unstated, unconfirmed_reasoning); Navier–Stokes
//! 1; Lucas–Penrose 1; Riemann 3 (its isolated equivalence theorems still read
//! as unlinked until the map records them EQUIVALENT_TO); four colour 6.

use reflow2_core::DesignGraph;
use reflow2_core::export::GraphExport;
use reflow2_core::foundation::core::Value;
use reflow2_core::nodes::Props;
use reflow2_core::propagate::PropagateOptions;
use serde_json::Value as Json;
use std::collections::BTreeMap;

const HEAT: &str = include_str!("fixtures/proof_trial_heat_equation.json");
const LUCAS_PENROSE: &str = include_str!("fixtures/argument_trial_lucas_penrose.json");
const RIEMANN: &str = include_str!("fixtures/proof_map_riemann.json");
const NAVIER_STOKES: &str = include_str!("fixtures/proof_map_navier_stokes.json");
const FOUR_COLOUR: &str = include_str!("fixtures/proof_trial_four_colour.json");
const LOGIC: &str = include_str!("fixtures/proof_trial_logic_rung.json");
const GETTIER: &str = include_str!("fixtures/argument_trial_gettier.json");
const PROOF_SKILL: &str = include_str!("../../../getting-started/skills/design-a-proof/SKILL.md");
const ARGUMENT_SKILL: &str =
    include_str!("../../../getting-started/skills/design-an-argument/SKILL.md");

const ALL: [(&str, &str); 7] = [
    ("gettier", GETTIER),
    ("heat", HEAT),
    ("lucas-penrose", LUCAS_PENROSE),
    ("riemann", RIEMANN),
    ("navier-stokes", NAVIER_STOKES),
    ("four-colour", FOUR_COLOUR),
    ("logic", LOGIC),
];

/// The software-shaped questions no proof or argument design may be asked.
const SOFTWARE_GAPS: [&str; 10] = [
    "concept_without_design",
    "quality_target_unstated",
    "artifact_standard_undeclared",
    "closure_criterion_undeclared",
    "build_without_verification",
    "build_without_governance",
    "no_deploy_operate",
    "encoding_undecided",
    "agent_instructions_unregistered",
    "unsatisfied_requirement",
];

fn load(doc: &str) -> DesignGraph {
    let doc: GraphExport = serde_json::from_str(doc).expect("fixture parses");
    let mut g = DesignGraph::open_in_memory().expect("in-memory graph");
    g.import_graph(&doc).expect("fixture imports");
    g
}

fn graph() -> DesignGraph {
    DesignGraph::open_in_memory().expect("in-memory graph")
}

fn json<T: serde::Serialize>(v: T) -> Json {
    serde_json::to_value(v).expect("serializes")
}

fn gap_kinds(g: &DesignGraph) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for gap in g.detect_gaps().expect("gaps") {
        *out.entry(gap.gap_source.as_str().to_string()).or_default() += 1;
    }
    out
}

fn gaps_of(g: &DesignGraph, kind: &str) -> Vec<Vec<String>> {
    g.detect_gaps()
        .expect("gaps")
        .into_iter()
        .filter(|x| x.gap_source.as_str() == kind)
        .map(|x| x.affected_ids)
        .collect()
}

fn type_of(g: &DesignGraph, id: &str) -> String {
    let held = g.node_types_holding(id).expect("types holding");
    assert_eq!(held.len(), 1, "{id}: {held:?}");
    held[0].clone()
}

fn set(g: &mut DesignGraph, id: &str, key: &str, value: Value) {
    let ty = type_of(g, id);
    g.upsert_node(&ty, id, Props::new().set(key, value))
        .unwrap_or_else(|e| panic!("{id}.{key}: {e}"));
}

fn statement(g: &mut DesignGraph, id: &str, kind: &str) {
    g.add_requirement(id, &format!("statement {id}"), "A statement.")
        .unwrap();
    set(g, id, "statement_kind", Value::String(kind.to_string()));
}

fn edge(g: &mut DesignGraph, from: &str, rel: &str, to: &str, props: Props) {
    let (ft, tt) = (type_of(g, from), type_of(g, to));
    g.create_edge(rel, &ft, from, &tt, to, props)
        .unwrap_or_else(|e| panic!("{from} {rel} {to}: {e}"));
}

fn step(g: &mut DesignGraph, id: &str, premises: &[&str], assumed: &[&str], conclusion: &str) {
    g.upsert_node(
        "Inference",
        id,
        Props::new().set("name", id).set("rule", "all"),
    )
    .unwrap();
    for p in premises {
        edge(g, p, "PREMISE_OF", id, Props::new());
    }
    for p in assumed {
        edge(g, p, "PREMISE_OF", id, Props::new().set("role", "assumed"));
    }
    edge(g, id, "CONCLUDES", conclusion, Props::new());
}

/// A reductio by hand: a supposition `req:s`, a contradiction `req:bot`
/// derived from it and the ordinary premise `premise`, and the step `inf:raa`
/// that discharges the supposition and concludes `conclusion`.
fn reductio(g: &mut DesignGraph, premise: &str, conclusion: &str) {
    statement(g, "req:s", "hypothesis");
    statement(g, "req:bot", "falsum");
    step(g, "inf:bot", &[premise], &["req:s"], "req:bot");
    g.upsert_node(
        "Inference",
        "inf:raa",
        Props::new().set("name", "raa").set("rule", "all").set(
            "discharges",
            Value::List(vec![Value::String("req:s".into())]),
        ),
    )
    .unwrap();
    edge(g, "req:bot", "PREMISE_OF", "inf:raa", Props::new());
    edge(g, "inf:raa", "CONCLUDES", conclusion, Props::new());
}

fn statement_reads(g: &DesignGraph, id: &str) -> String {
    json(g.statement_reading(id).unwrap())["reads"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

fn proof_status(g: &DesignGraph, id: &str) -> Json {
    json(g.proof_reading(id).unwrap())["status"].clone()
}

fn claim(g: &DesignGraph, id: &str) -> Json {
    let r = json(g.reasoning_report(Some(id), Some(500)).unwrap());
    r["claims"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .cloned()
        .unwrap_or_else(|| panic!("{id} not in its own read-back"))
}

fn strs(v: &Json) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn projects(g: &DesignGraph) -> Vec<String> {
    g.scan_nodes("Project")
        .unwrap()
        .into_iter()
        .map(|n| n.node_id)
        .collect()
}

// ─── 1(a) An assumed premise takes part in the reasoning ──────────────────

/// FAILS ON 4879171c5..v0.90.0: a hypothesis only ever taken up as an
/// `assumed` premise read as outside the reasoning, so `unlinked_intent`
/// asked about it and the design stopped reading as a proof.
#[test]
fn an_assumed_premise_takes_part_in_the_reasoning() {
    let mut g = graph();
    statement(&mut g, "req:a", "axiom");
    statement(&mut g, "req:not-s", "theorem");
    reductio(&mut g, "req:a", "req:not-s");

    assert!(g.takes_part_in_reasoning("req:s").unwrap());
    assert!(
        !g.unlinked_intent().unwrap().contains(&"req:s".to_string()),
        "an assumed-only hypothesis is linked by the step that assumes it"
    );
    assert!(g.is_a_reasoning_design().unwrap());

    // The class, on every trial design: no assumed premise reads unlinked.
    for (name, doc) in ALL {
        let g = load(doc);
        for affected in gaps_of(&g, "unlinked_intent") {
            for id in affected {
                let assumed = g
                    .outgoing(&id, Some("PREMISE_OF"))
                    .unwrap()
                    .iter()
                    .any(|e| e.properties.get("role").and_then(Value::as_str) == Some("assumed"));
                assert!(!assumed, "{name}: {id} is assumed and read as unlinked");
            }
        }
    }
}

// ─── 1(b) A hypothesis has a truth of its own ─────────────────────────────

#[test]
fn a_hypothesis_reads_assumed_never_not_delivered() {
    // The four colour rung: a DISCHARGED hypothesis read "unknown … not
    // delivered, and its status (proposed)".
    let g = load(FOUR_COLOUR);
    let (truth, why) = g.premise_truth("req:hyp-mc5-exists").unwrap();
    assert_eq!(truth.as_str(), "holds", "{why}");
    assert!(why.contains("hypothesis"), "{why}");
    assert!(!why.contains("not delivered"), "{why}");
    assert_eq!(
        g.assumptions_under("req:hyp-mc5-exists").unwrap(),
        vec!["req:hyp-mc5-exists".to_string()],
        "a hypothesis holds only under itself"
    );
    assert_eq!(statement_reads(&g, "req:hyp-mc5-exists"), "discharged");
    let c = claim(&g, "req:hyp-mc5-exists");
    assert_eq!(c["truth"], "conditional", "{c}");

    // The class: no hypothesis in any trial reads "not delivered".
    for (name, doc) in ALL {
        let g = load(doc);
        for r in g.scan_nodes("Requirement").unwrap() {
            if r.properties.get("statement_kind").and_then(Value::as_str) != Some("hypothesis") {
                continue;
            }
            let (_, why) = g.premise_truth(&r.node_id).unwrap();
            assert!(
                !why.contains("not delivered"),
                "{name}: {} reads {why}",
                r.node_id
            );
        }
    }

    // A hypothesis used as an ORDINARY premise makes what rests on it
    // conditional on it, until a step discharges it.
    let mut g = graph();
    statement(&mut g, "req:h", "hypothesis");
    statement(&mut g, "req:c", "lemma");
    step(&mut g, "inf:s", &["req:h"], &[], "req:c");
    assert_eq!(proof_status(&g, "req:c"), "conditional");
    assert_eq!(
        g.assumptions_under("req:c").unwrap(),
        vec!["req:h".to_string()]
    );
}

// ─── 1(c) encoding_undecided never asks a statement for `kind` ────────────

#[test]
fn encoding_undecided_never_asks_a_statement_for_a_software_kind() {
    for (name, doc) in ALL {
        let g = load(doc);
        for affected in gaps_of(&g, "encoding_undecided") {
            for id in affected {
                let ty = type_of(&g, &id);
                let n = g.get_node(&ty, &id).unwrap().unwrap();
                assert!(
                    !n.properties.contains_key("statement_kind"),
                    "{name}: {id} carries statement_kind and is asked for `kind`"
                );
            }
        }
    }
    // A plain requirement population is still asked: the gate is the
    // statement, not the design.
    let mut g = graph();
    for i in 0..3 {
        statement(&mut g, &format!("req:s{i}"), "theorem");
    }
    assert!(gaps_of(&g, "encoding_undecided").is_empty());
    // Mixed encoding among the needs is the sharp signal, and it is asked.
    g.add_requirement("req:n-said", "n", "A software need.")
        .unwrap();
    set(
        &mut g,
        "req:n-said",
        "kind",
        Value::String("functional".into()),
    );
    for i in 0..3 {
        g.add_requirement(&format!("req:n{i}"), "n", "A software need.")
            .unwrap();
    }
    let asked = gaps_of(&g, "encoding_undecided");
    assert_eq!(asked.len(), 1, "{asked:?}");
    assert!(
        asked[0].iter().all(|id| id.starts_with("req:n")),
        "{asked:?}"
    );
}

// ─── 1(d) The structure decides; the domain is never a switch ─────────────

#[test]
fn no_proof_design_is_asked_a_software_question_whatever_its_domain_says() {
    for (name, doc) in ALL {
        let base = gap_kinds(&load(doc));
        for domain in [
            "software",
            "document",
            "proof",
            "mathematics: an open problem",
        ] {
            let mut g = load(doc);
            for p in projects(&g) {
                set(&mut g, &p, "domain", Value::String(domain.to_string()));
            }
            assert!(
                g.is_a_reasoning_design().unwrap(),
                "{name} under domain `{domain}` must read as reasoning by its structure"
            );
            let kinds = gap_kinds(&g);
            assert_eq!(kinds, base, "{name}: domain `{domain}` switched the gaps");
            for software in SOFTWARE_GAPS {
                assert!(
                    !kinds.contains_key(software),
                    "{name} under `{domain}` was asked {software}: {kinds:?}"
                );
            }
        }
    }
    // The measured after-count (see the module documentation).
    assert_eq!(
        gap_kinds(&load(HEAT)),
        BTreeMap::from([
            ("change_axis_unstated".to_string(), 1),
            ("unconfirmed_reasoning".to_string(), 1),
        ])
    );
}

#[test]
fn the_domain_hint_is_only_a_fallback() {
    // A requirement nobody marked as a statement (genesis's own top
    // requirement) outside the reasoning: the structure no longer says
    // reasoning, and the hint still can.
    let mut g = graph();
    statement(&mut g, "req:a", "axiom");
    statement(&mut g, "req:t", "theorem");
    step(&mut g, "inf:s", &["req:a"], &[], "req:t");
    g.add_requirement("req:top", "Prove it", "The owner's ask.")
        .unwrap();
    assert!(!g.is_a_reasoning_design().unwrap());
    g.upsert_node(
        "Project",
        "proj:p",
        Props::new().set("name", "p").set("domain", "a proof"),
    )
    .unwrap();
    assert!(g.is_a_reasoning_design().unwrap());
}

#[test]
fn a_software_design_that_holds_a_proof_reads_its_statements_as_reasoning() {
    let mut g = graph();
    statement(&mut g, "req:a", "axiom");
    statement(&mut g, "req:t", "theorem");
    statement(&mut g, "req:h", "hypothesis");
    step(&mut g, "inf:s", &["req:a"], &["req:h"], "req:t");
    g.add_requirement("req:need", "A need", "The system must log in.")
        .unwrap();
    g.add_capability("cap:c", "Logs in", "It logs in.", None)
        .unwrap();
    edge(&mut g, "cap:c", "SATISFIES", "req:need", Props::new());
    g.add_requirement("req:other", "Another need", "The system must log out.")
        .unwrap();
    assert!(!g.is_a_reasoning_design().unwrap(), "it builds something");
    let unsatisfied: Vec<String> = gaps_of(&g, "unsatisfied_requirement")
        .into_iter()
        .flatten()
        .collect();
    assert_eq!(unsatisfied, vec!["req:other".to_string()]);
    // And its proof reads as a proof.
    assert_eq!(statement_reads(&g, "req:h"), "assumed");
    assert_eq!(proof_status(&g, "req:t"), "conditional");
}

// ─── 2. Contested, not unproven ────────────────────────────────────────────

#[test]
fn a_reductio_through_a_contested_falsum_reads_contested() {
    // Lucas–Penrose: the main conclusion read "unproven, no derivation holds"
    // while every objection against it stood on a translation under the ⊥.
    let g = load(LUCAS_PENROSE);
    let main = "req:mind-is-not-a-machine";
    assert_eq!(proof_status(&g, main), "contested");
    assert_eq!(statement_reads(&g, main), "contested");
    let c = claim(&g, main);
    assert!(
        !strs(&c["contested_by"]).is_empty(),
        "it names what stands against it: {c}"
    );

    // The shape, by hand.
    let mut g = graph();
    statement(&mut g, "req:p", "proposition");
    set(&mut g, "req:p", "taken_as_given", Value::Bool(true));
    statement(&mut g, "req:obj", "objection");
    edge(&mut g, "req:obj", "CONTRADICTS", "req:p", Props::new());
    statement(&mut g, "req:c", "proposition");
    reductio(&mut g, "req:p", "req:c");
    assert_eq!(proof_status(&g, "req:c"), "contested");
    let c = claim(&g, "req:c");
    assert_eq!(strs(&c["contested_by"]), vec!["req:obj".to_string()], "{c}");
}

#[test]
fn a_falsum_carries_no_proof_status() {
    // The four colour rung: ⊥ read proof_status `unproven` beside `false`.
    for (name, doc) in ALL {
        let g = load(doc);
        for r in g.scan_nodes("Requirement").unwrap() {
            if r.properties.get("statement_kind").and_then(Value::as_str) == Some("falsum") {
                assert_eq!(
                    proof_status(&g, &r.node_id),
                    Json::Null,
                    "{name}: {}",
                    r.node_id
                );
                assert!(
                    matches!(
                        statement_reads(&g, &r.node_id).as_str(),
                        "false" | "inconsistent" | "stated"
                    ),
                    "{name}: {}",
                    r.node_id
                );
            }
        }
    }
}

#[test]
fn a_what_if_reads_a_claim_left_only_with_a_contested_route_contested() {
    // The Gettier rung (57e91debfac7c608): dropping the closure principle
    // costs the two case routes of "JTB is not sufficient", and its barn route
    // stands contested by Sosa's objection. The rows read `unsupported`.
    let g = load(GETTIER);
    let br = json(
        g.propagate_from(&["req:g-closure"], PropagateOptions::default())
            .unwrap(),
    );
    let rows: Vec<&Json> = br["inferences"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["conclusion"] == "req:jtb-not-sufficient")
        .collect();
    assert!(!rows.is_empty(), "{br}");
    for r in rows {
        assert_eq!(r["conclusion_reads"], "contested", "{r}");
        assert!(
            strs(&r["untouched_routes"]).contains(&"inf:not-sufficient-via-barn".to_string()),
            "{r}"
        );
    }
}

// ─── 3. A what-if walks past a falsum ──────────────────────────────────────

#[test]
fn a_what_if_walks_through_a_falsum_to_the_reductio_and_its_conclusion() {
    let mut g = graph();
    statement(&mut g, "req:p", "proposition");
    set(&mut g, "req:p", "taken_as_given", Value::Bool(true));
    statement(&mut g, "req:c", "theorem");
    statement(&mut g, "req:d", "corollary");
    reductio(&mut g, "req:p", "req:c");
    step(&mut g, "inf:cor", &["req:c"], &[], "req:d");
    assert_eq!(proof_status(&g, "req:c"), "shown");

    let walk = json(g.inference_walk(&["req:p"]).unwrap());
    let rows = walk.as_array().unwrap();
    let row = |id: &str| {
        rows.iter()
            .find(|r| r["inference"] == id)
            .unwrap_or_else(|| panic!("the walk never reached {id}: {walk}"))
            .clone()
    };
    // The step deriving ⊥ loses it, and reads its conclusion by the
    // derivation — never `refuted` (the four colour rung).
    let bot = row("inf:bot");
    assert_ne!(bot["conclusion_was"], "refuted", "{bot}");
    assert_eq!(bot["conclusion_reads"], "unsupported", "{bot}");
    // Past the ⊥: the reductio, and what rests on its conclusion.
    let raa = row("inf:raa");
    assert_eq!(raa["impact"], "unsupported", "{raa}");
    assert_eq!(raa["conclusion_reads"], "unsupported", "{raa}");
    let cor = row("inf:cor");
    assert_eq!(cor["conclusion_reads"], "unsupported", "{cor}");

    // The trial: dropping "we know our own consistency" reaches the reductio.
    let g = load(LUCAS_PENROSE);
    let walk = json(
        g.inference_walk(&["req:tr-mind-can-know-fm-consistent"])
            .unwrap(),
    );
    assert!(
        walk.as_array()
            .unwrap()
            .iter()
            .any(|r| r["inference"] == "inf:lucas-5-reductio"),
        "the what-if stops at the ⊥: {walk}"
    );
}

// ─── 4. A bridge premise ───────────────────────────────────────────────────

/// Gödel's theorem, cited and checked against a pinned source; the bridge
/// "the mind's output is a formal system"; and the claim about minds.
fn bridged() -> DesignGraph {
    let mut g = graph();
    statement(&mut g, "req:godel", "theorem");
    set(&mut g, "req:godel", "taken_as_given", Value::Bool(true));
    g.upsert_node(
        "Artifact",
        "art:src",
        Props::new()
            .set("name", "the source")
            .set("location", "https://doi.org/10.1007/example")
            .set("checksum", "sha256:0123456789abcdef"),
    )
    .unwrap();
    edge(&mut g, "art:src", "DOCUMENTS", "req:godel", Props::new());
    statement(&mut g, "req:bridge", "bridge");
    statement(&mut g, "req:minds", "proposition");
    step(
        &mut g,
        "inf:carry",
        &["req:godel", "req:bridge"],
        &[],
        "req:minds",
    );
    g
}

#[test]
fn what_rests_on_a_bridge_reads_conditional_on_it() {
    let g = bridged();
    assert_eq!(statement_reads(&g, "req:bridge"), "bridge");
    assert_eq!(statement_reads(&g, "req:godel"), "cited");
    assert_eq!(proof_status(&g, "req:minds"), "conditional");
    assert_eq!(
        g.bridges_under("req:minds").unwrap(),
        vec!["req:bridge".to_string()]
    );
    assert!(g.unverified_under("req:minds").unwrap().is_empty());
    let c = claim(&g, "req:minds");
    assert_eq!(c["truth"], "conditional", "{c}");
    assert_eq!(strs(&c["rests_on_bridges"]), vec!["req:bridge".to_string()]);
    assert!(c["why"].as_str().unwrap().contains("bridge"), "{c}");
    let step = json(g.inference_reading("inf:carry").unwrap().unwrap());
    assert_eq!(step["holds_as"], "conditional", "{step}");
    assert_eq!(
        strs(&step["rests_on_bridges"]),
        vec!["req:bridge".to_string()]
    );
}

#[test]
fn an_objection_to_a_bridge_contests_every_claim_resting_on_it() {
    let mut g = bridged();
    statement(&mut g, "req:c2", "corollary");
    step(&mut g, "inf:c2", &["req:minds"], &[], "req:c2");
    statement(&mut g, "req:obj", "objection");
    edge(&mut g, "req:obj", "CONTRADICTS", "req:bridge", Props::new());
    assert_eq!(statement_reads(&g, "req:bridge"), "contested");
    for id in ["req:minds", "req:c2"] {
        assert_eq!(proof_status(&g, id), "contested", "{id}");
        assert!(
            strs(&claim(&g, id)["contested_by"]).contains(&"req:obj".to_string()),
            "{id}"
        );
    }
    // The theorem under it stands.
    assert_eq!(statement_reads(&g, "req:godel"), "cited");
}

#[test]
fn reasoning_report_lists_bridges_apart_from_citations_and_foundations() {
    let g = bridged();
    let r = json(g.reasoning_report(Some("req:minds"), Some(500)).unwrap());
    assert_eq!(strs(&r["bridges"]), vec!["req:bridge".to_string()], "{r}");
    assert!(!strs(&r["uncited_premises"]).contains(&"req:bridge".to_string()));
    assert!(!strs(&r["stated_foundations"]).contains(&"req:bridge".to_string()));
    let debt = json(g.reasoning_debt().unwrap());
    assert_eq!(
        debt["unverified_citations"], 0,
        "a bridge is no citation: {debt}"
    );
}

#[test]
fn a_what_if_on_a_bridge_leaves_the_theorem_standing() {
    let g = bridged();
    let br = json(
        g.propagate_from(&["req:bridge"], PropagateOptions::default())
            .unwrap(),
    );
    let rows = br["inferences"].as_array().unwrap();
    let row = rows
        .iter()
        .find(|r| r["inference"] == "inf:carry")
        .unwrap_or_else(|| panic!("{br}"));
    assert_eq!(strs(&row["bridges_lost"]), vec!["req:bridge".to_string()]);
    assert_eq!(row["conclusion_reads"], "unsupported", "{row}");
    assert_eq!(statement_reads(&g, "req:godel"), "cited");
}

// ─── 6. Proven equivalence ─────────────────────────────────────────────────

const RH: &str = "req:riemann-hypothesis";
const ROBIN: &str = "req:robin-inequality-all-n";
const BRIDGE_1: &str = "req:bridge-1-robin-on-colossally-abundant-numbers";
const ROBIN_1984: &str = "req:cite-robin-1984-equivalence";

fn equivalent(g: &mut DesignGraph, a: &str, b: &str, by: Option<&str>) {
    let props = match by {
        Some(by) => Props::new().set("by", by),
        None => Props::new(),
    };
    edge(g, a, "EQUIVALENT_TO", b, props);
}

fn mentions_merge(v: &Json) -> bool {
    v.to_string().contains("\"merge\"")
}

#[test]
fn an_equivalence_never_reads_as_a_duplicate_and_is_never_merged() {
    // The Riemann map's shape: RH and Robin's inequality joined by an
    // asserted DUPLICATES for want of a word, which detect_defects offered as
    // a merge (the owner acknowledged it as wrong, so the fixture's own
    // defect list no longer shows it; the shape is rebuilt here).
    let mut g = graph();
    statement(&mut g, "req:rh", "conjecture");
    statement(&mut g, "req:robin", "conjecture");
    statement(&mut g, "req:robin-1984", "theorem");
    edge(
        &mut g,
        "req:robin",
        "DUPLICATES",
        "req:rh",
        Props::new().set("basis", "asserted"),
    );
    assert!(mentions_merge(&json(g.detect_defects().unwrap())));
    equivalent(&mut g, "req:robin", "req:rh", Some("req:robin-1984"));
    let after = json(g.detect_defects().unwrap());
    assert!(!mentions_merge(&after), "{after}");
    assert_eq!(
        after["swept"]["suppressed_as_equivalent"]["duplicate"], 1,
        "counted, never silently dropped: {after}"
    );
    let heal = json(
        g.propose_heal(reflow2_core::heal::HealOptions::default())
            .unwrap(),
    );
    assert!(!mentions_merge(&heal), "{heal}");
    assert!(gaps_of(&g, "possible_duplicate").is_empty());
}

#[test]
fn an_equivalence_leaves_each_side_reading_as_it_did() {
    let plain = load(RIEMANN);
    let mut g = load(RIEMANN);
    equivalent(&mut g, ROBIN, RH, Some(ROBIN_1984));
    for id in [RH, ROBIN] {
        assert_eq!(statement_reads(&g, id), statement_reads(&plain, id), "{id}");
        assert_eq!(proof_status(&g, id), proof_status(&plain, id), "{id}");
        assert_eq!(
            g.premise_truth(id).unwrap().0,
            plain.premise_truth(id).unwrap().0,
            "{id}"
        );
        assert!(g.assumptions_under(id).unwrap().is_empty(), "{id}");
    }
    assert_eq!(
        statement_reads(&g, RH),
        "open",
        "an open conjecture stays open"
    );
    let c = claim(&g, RH);
    let eq = c["equivalent_to"].as_array().unwrap();
    assert_eq!(eq.len(), 1, "{c}");
    assert_eq!(eq[0]["id"], ROBIN);
    assert_eq!(eq[0]["by"], ROBIN_1984);
    // The biconditional is a citation checked against its pinned source, so
    // the equivalence reads as it does.
    assert_eq!(eq[0]["reads"], "holds", "{c}");
}

#[test]
fn a_what_if_on_one_side_reaches_the_other() {
    let mut g = load(RIEMANN);
    equivalent(&mut g, ROBIN, RH, Some(ROBIN_1984));
    let br = json(
        g.propagate_from(&[ROBIN], PropagateOptions::default())
            .unwrap(),
    );
    let reached = br["equivalents_reached"].as_array().unwrap();
    assert!(
        reached.iter().any(|e| e["from"] == ROBIN && e["to"] == RH),
        "{br}"
    );
    assert!(
        br["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["node_id"] == RH),
        "{br}"
    );
}

#[test]
fn an_equivalence_links_two_statements() {
    let mut g = graph();
    statement(&mut g, "req:x", "conjecture");
    statement(&mut g, "req:y", "conjecture");
    assert!(g.unlinked_intent().unwrap().contains(&"req:x".to_string()));
    equivalent(&mut g, "req:x", "req:y", None);
    let unlinked = g.unlinked_intent().unwrap();
    assert!(!unlinked.contains(&"req:x".to_string()), "{unlinked:?}");
    assert!(!unlinked.contains(&"req:y".to_string()), "{unlinked:?}");
}

// ─── 5. The circularity check ──────────────────────────────────────────────

#[test]
fn a_step_whose_premise_is_equivalent_to_its_conclusion_reads_circular() {
    // The map's bridge 1 (Robin on colossally abundant numbers) is
    // equivalent to Robin's inequality, which is equivalent to RH: a step
    // closing RH's hole with it only restates the hole.
    let mut g = load(RIEMANN);
    equivalent(&mut g, ROBIN, RH, Some(ROBIN_1984));
    equivalent(&mut g, BRIDGE_1, ROBIN, None);
    g.upsert_node(
        "Inference",
        "inf:speculated-closing",
        Props::new().set("name", "closing").set("rule", "all"),
    )
    .unwrap();
    edge(
        &mut g,
        BRIDGE_1,
        "PREMISE_OF",
        "inf:speculated-closing",
        Props::new(),
    );
    edge(
        &mut g,
        "inf:speculated-closing",
        "CONCLUDES",
        RH,
        Props::new(),
    );

    let c = g
        .circularity_of("inf:speculated-closing")
        .unwrap()
        .expect("circular");
    assert_eq!(c.premise, BRIDGE_1);
    assert_eq!(c.conclusion, RH);
    let step = json(
        g.inference_reading("inf:speculated-closing")
            .unwrap()
            .unwrap(),
    );
    assert!(
        step["circular"]
            .as_str()
            .unwrap_or("")
            .starts_with("circular"),
        "{step}"
    );
    let r = json(g.reasoning_report(Some(RH), Some(500)).unwrap());
    assert_eq!(
        r["circular_steps"][0]["step"], "inf:speculated-closing",
        "{r}"
    );
    let asked = gaps_of(&g, "reasoning_not_as_stated");
    assert!(
        asked
            .iter()
            .any(|a| a.contains(&"inf:speculated-closing".to_string())),
        "{asked:?}"
    );

    // No other step of the map is circular.
    for s in g.scan_nodes("Inference").unwrap() {
        if s.node_id != "inf:speculated-closing" {
            assert!(
                g.circularity_of(&s.node_id).unwrap().is_none(),
                "{}",
                s.node_id
            );
        }
    }
}

#[test]
fn a_step_whose_premise_is_the_same_formula_reads_circular() {
    let mut g = graph();
    statement(&mut g, "req:c", "lemma");
    statement(&mut g, "req:c-again", "lemma");
    set(
        &mut g,
        "req:c-again",
        "same_formula_as",
        Value::String("req:c".into()),
    );
    step(&mut g, "inf:s", &["req:c-again"], &[], "req:c");
    assert!(g.circularity_of("inf:s").unwrap().is_some());
}

// ─── The skills say how, and agree with the tools ──────────────────────────

#[test]
fn the_skills_say_how_to_record_each() {
    for word in [
        "`bridge`",
        "rests_on_bridges",
        "EQUIVALENT_TO",
        "equivalents_reached",
        "circular_steps",
        "never from the Project's `domain`",
        "`contested`",
    ] {
        assert!(PROOF_SKILL.contains(word), "design-a-proof lacks {word}");
    }
    for word in [
        "`bridge`",
        "rests_on_bridges",
        "bridges_lost",
        "through the ⊥",
    ] {
        assert!(
            ARGUMENT_SKILL.contains(word),
            "design-an-argument lacks {word}"
        );
    }
}
