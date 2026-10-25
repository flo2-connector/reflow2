//! REASONING IS EVALUATED ONCE PER REPORT, AND A CIRCLE IS WORKED OUT ONCE.
//!
//! `dec:reasoning-evaluation-is-memoized-across-cycles-and-shared-across-a-report`
//! (Anthony, 2026-10-25), from the root cause of the Shop stall on flo2.io
//! (`fact:root-cause-the-shop-stall-was-reasoning-evaluation-that-grew-past-the-gateways-30-s-and-nothing-stops-or-reports-it-honestly-2026-10-25`):
//! a design of about 600 nodes and 56 reasoning steps took reasoning_report
//! 13.3 s (91 ms at about 34 steps) and loop_status 20-30 s, and the engine
//! stalled for 35 minutes. Two limits were in the evaluation: each reader made
//! its own memo, so nothing was shared across the nodes of one report, and a
//! truth computed while a circle was cut beneath it was never memoized, so it
//! was computed again every time it was asked.
//!
//! Three things are pinned here:
//!
//! 1. A GENERATED DESIGN SHAPED LIKE THE SHOP ([`shop_shaped`]): design
//!    choices concluded by one step each, whose premises are requirements the
//!    decisions govern, constraints and other decisions (circles among the
//!    decisions), plus chains of steps concluding requirements that rest on
//!    each other (one long circle) and objection circles.
//! 2. ITS COST: reasoning_report, loop_status and detect_gaps each within a
//!    stated budget ([`BUDGET`]), so the cost cannot grow back.
//! 3. EVERY READING IS AS IT WAS: the readings of every proof, argument and
//!    verification fixture, and of the generated design, equal a golden taken
//!    on the code before the change (main 7e9209515).
//!
//! `cargo test -p reflow2-core --no-default-features --release --test in_memory
//! reasoning_is_evaluated_once_per_report::time_ -- --ignored --nocapture`
//! prints the timings.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use reflow2_core::DesignGraph;
use reflow2_core::export::GraphExport;
use reflow2_core::nodes::Props;
use serde_json::Value as Json;

fn to_json<T: serde::Serialize>(v: T) -> Json {
    serde_json::to_value(v).expect("serializes")
}

// ─── 1: a design shaped like the Shop ─────────────────────────────────────

fn edge(g: &mut DesignGraph, rel: &str, ft: &str, from: &str, tt: &str, to: &str) {
    g.create_edge(rel, ft, from, tt, to, Props::new())
        .unwrap_or_else(|e| panic!("{from} {rel} {to}: {e}"));
}

fn node(g: &mut DesignGraph, ty: &str, id: &str, props: Props) {
    g.upsert_node(ty, id, props)
        .unwrap_or_else(|e| panic!("{ty} {id}: {e}"));
}

fn step(g: &mut DesignGraph, id: &str, premises: &[(&str, &str)], ct: &str, conclusion: &str) {
    node(
        g,
        "Inference",
        id,
        Props::new().set("name", id).set("rule", "all"),
    );
    let mut seen = BTreeSet::new();
    for (ty, p) in premises {
        if seen.insert(*p) {
            edge(g, "PREMISE_OF", ty, p, "Inference", id);
        }
    }
    edge(g, "CONCLUDES", "Inference", id, ct, conclusion);
}

/// A design shaped like the Shop: `decisions` design choices, each concluded
/// by one step, and `chains` steps concluding requirements, so
/// `decisions + chains` steps in all (24 + 32 = 56 for the Shop's size).
///
/// - Each decision's step rests on two requirements (read as intent), one of
///   them governed by that same decision (the circle a requirement read as
///   intent refuses), a constraint, and two other decisions: decision `i`
///   rests on `i+1` and `i+5`, so the decisions form circles.
/// - Each chain step concludes a requirement and rests on two other
///   concluded requirements (`j+1` and `j+7`, read by delivery, so the
///   requirements form one long circle), a decision and a constraint.
/// - Six constraints object to each other in pairs (objection circles).
/// - Capabilities, components and checks fill the design out to its size;
///   they take no part in the reasoning.
pub(crate) fn shop_shaped(decisions: usize, chains: usize) -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("in-memory graph");
    let reqs = 2 * decisions + chains + 8;
    let cons = 20;
    for i in 0..decisions {
        let status = match i % 6 {
            4 => "proposed",
            _ => "accepted",
        };
        node(
            &mut g,
            "Decision",
            &format!("dec:d{i:02}"),
            Props::new()
                .set("name", format!("design choice {i}"))
                .set("decision", "A choice.")
                .set("status", status),
        );
    }
    for r in 0..reqs {
        let id = format!("req:r{r:02}");
        g.add_requirement(&id, &format!("requirement {r}"), "The owner's need.")
            .unwrap();
        let status = if r % 9 == 5 { "proposed" } else { "accepted" };
        node(
            &mut g,
            "Requirement",
            &id,
            Props::new().set("status", status),
        );
        let d = format!("dec:d{:02}", r % decisions);
        edge(&mut g, "GOVERNED_BY", "Requirement", &id, "Decision", &d);
    }
    for c in 0..cons {
        node(
            &mut g,
            "Constraint",
            &format!("con:c{c:02}"),
            Props::new()
                .set("name", format!("constraint {c}"))
                .set("statement", "A limit the design keeps."),
        );
    }
    for k in 0..3 {
        let (a, b) = (
            format!("con:c{:02}", 2 * k),
            format!("con:c{:02}", 2 * k + 1),
        );
        edge(&mut g, "CONTRADICTS", "Constraint", &a, "Constraint", &b);
        edge(&mut g, "CONTRADICTS", "Constraint", &b, "Constraint", &a);
    }
    for i in 0..decisions {
        let own = format!("req:r{i:02}");
        let other = format!("req:r{:02}", decisions + i);
        let con = format!("con:c{:02}", i % cons);
        let d1 = format!("dec:d{:02}", i + 1);
        let d5 = format!("dec:d{:02}", i + 5);
        let mut premises = vec![
            ("Requirement", own.as_str()),
            ("Requirement", other.as_str()),
            ("Constraint", con.as_str()),
        ];
        if i + 1 < decisions {
            premises.push(("Decision", &d1));
        }
        if i + 5 < decisions {
            premises.push(("Decision", &d5));
        }
        step(
            &mut g,
            &format!("inf:d{i:02}"),
            &premises,
            "Decision",
            &format!("dec:d{i:02}"),
        );
    }
    let base = 2 * decisions;
    for j in 0..chains {
        let concl = format!("req:r{:02}", base + j);
        let n1 = format!("req:r{:02}", base + j + 1);
        let n7 = format!("req:r{:02}", base + j + 7);
        let d = format!("dec:d{:02}", j % decisions);
        let con = format!("con:c{:02}", (j + 3) % cons);
        let mut premises = vec![("Decision", d.as_str()), ("Constraint", con.as_str())];
        // Past the last chain step, the premise is a requirement no step
        // concludes: the chain's ground.
        premises.push(("Requirement", &n1));
        premises.push(("Requirement", &n7));
        step(
            &mut g,
            &format!("inf:r{j:02}"),
            &premises,
            "Requirement",
            &concl,
        );
    }
    // The rest of the design: no part in the reasoning.
    let caps = 150;
    for c in 0..caps {
        let id = format!("cap:k{c:03}");
        node(
            &mut g,
            "Capability",
            &id,
            Props::new()
                .set("name", format!("capability {c}"))
                .set("description", "What a part does.")
                .set("status", if c % 3 == 0 { "realized" } else { "planned" }),
        );
        let r = format!("req:r{:02}", c % reqs);
        edge(&mut g, "SATISFIES", "Capability", &id, "Requirement", &r);
    }
    for p in 0..150 {
        let id = format!("comp:p{p:03}");
        node(
            &mut g,
            "Component",
            &id,
            Props::new()
                .set("name", format!("part {p}"))
                .set("purpose", "A part of the design."),
        );
        let c = format!("cap:k{:03}", p % caps);
        edge(&mut g, "ALLOCATED_TO", "Capability", &c, "Component", &id);
    }
    for v in 0..100 {
        let id = format!("ver:v{v:03}");
        node(
            &mut g,
            "Verification",
            &id,
            Props::new()
                .set("name", format!("check {v}"))
                .set("status", if v % 4 == 0 { "passing" } else { "planned" }),
        );
        let c = format!("cap:k{:03}", (3 * v) % caps);
        edge(&mut g, "VERIFIES", "Verification", &id, "Capability", &c);
    }
    g
}

// ─── 2: what it costs ─────────────────────────────────────────────────────

/// Run `f` on a thread with a 64 MiB stack. The Shop-shaped design's chains
/// are about 56 steps deep, and a DEBUG build's frames overflow the 2 MiB a
/// test thread gets well before that, on the code before this change as on
/// the code after it (measured on 7e9209515 and 367cd86d8); a release build
/// reads it on 2 MiB.
fn on_a_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(f)
        .expect("a thread")
        .join()
        .unwrap_or_else(|e| std::panic::resume_unwind(e))
}

fn timed<T>(f: impl FnOnce() -> T) -> (T, Duration) {
    let t = Instant::now();
    let out = f();
    (out, t.elapsed())
}

/// The Shop's size, timed: each reader alone, and the sizes either side of
/// it so the growth shows. Prints; asserts nothing (the budget test does).
#[test]
#[ignore = "a measurement: run with --ignored --nocapture"]
fn time_the_shop_shaped_design() {
    on_a_big_stack(time_it);
}

fn time_it() {
    // A design of your own, timed the same way: an export's path.
    if let Ok(path) = std::env::var("REFLOW2_TIME_DESIGN") {
        let doc: GraphExport =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let fresh = || {
            let mut g = DesignGraph::open_in_memory().unwrap();
            g.import_graph(&doc).unwrap();
            g
        };
        let g = fresh();
        let (_, report) = timed(|| g.reasoning_report(None, None).unwrap());
        let g = fresh();
        let (_, status) = timed(|| g.loop_status().unwrap());
        let g = fresh();
        let (_, gaps) = timed(|| g.detect_gaps().unwrap());
        println!(
            "{path}: reasoning_report {report:.1?}  loop_status {status:.1?}  detect_gaps {gaps:.1?}"
        );
    }
    for (decisions, chains) in [(14, 20), (18, 26), (24, 32), (30, 40)] {
        let g = shop_shaped(decisions, chains);
        let (_, report) = timed(|| g.reasoning_report(None, None).unwrap());
        let g = shop_shaped(decisions, chains);
        let (_, status) = timed(|| g.loop_status().unwrap());
        let g = shop_shaped(decisions, chains);
        let (_, gaps) = timed(|| g.detect_gaps().unwrap());
        println!(
            "steps {:>3}: reasoning_report {:>10.1?}  loop_status {:>10.1?}  detect_gaps {:>10.1?}",
            decisions + chains,
            report,
            status,
            gaps
        );
    }
}

/// WHAT EACH READER MAY SPEND on the Shop-sized design, in a DEBUG build on
/// CI's hardware. Measured on 7e9209515 (before the change, a release build on
/// an 8-core desktop): reasoning_report took longer than the gateway's 30 s.
/// After it, see the pull request; the budget leaves a margin of about ten
/// times the debug-build time measured, so a slow CI runner passes and a
/// return of the explosive cost (seconds to minutes) fails.
const BUDGET: Duration = Duration::from_secs(2);

#[test]
fn a_shop_sized_design_with_circles_reads_within_its_budget() {
    on_a_big_stack(within_its_budget);
}

fn within_its_budget() {
    let g = shop_shaped(24, 32);
    let (report, took) = timed(|| g.reasoning_report(None, None).unwrap());
    assert!(
        report.claims.len() > 56,
        "the whole reasoning is read: {}",
        report.claims.len()
    );
    assert!(
        took < BUDGET,
        "reasoning_report took {took:?} on the Shop-sized design, past its budget of {BUDGET:?}"
    );
    let (_, took) = timed(|| g.loop_status().unwrap());
    assert!(
        took < BUDGET,
        "loop_status took {took:?} on the Shop-sized design, past its budget of {BUDGET:?}"
    );
    let (_, took) = timed(|| g.detect_gaps().unwrap());
    assert!(
        took < BUDGET,
        "detect_gaps took {took:?} on the Shop-sized design, past its budget of {BUDGET:?}"
    );
}

// ─── 3: every reading as it was ───────────────────────────────────────────

/// Every reader of a design's reasoning, over every node of it: each step,
/// premise, conclusion and objection read alone (truth, statement, chain,
/// objections, foundations, proof, lines, check conditions, step), each
/// conclusion's own report, the whole report, the debt, what follows and does
/// not, the assumptions, independence, generalizations, single points,
/// foundations, loop_status, detect_gaps, detect_defects, and the what-if walk
/// from every premise. Whole JSON, keyed by reader.
fn readings(g: &DesignGraph, doc: &GraphExport, every_node: bool) -> BTreeMap<String, Json> {
    let mut ids: BTreeSet<String> = BTreeSet::new();
    let mut steps: BTreeSet<String> = BTreeSet::new();
    let mut premises: BTreeSet<String> = BTreeSet::new();
    let mut conclusions: BTreeSet<String> = BTreeSet::new();
    for n in &doc.nodes {
        if n.node_type == "Inference" {
            steps.insert(n.node_id.clone());
            ids.insert(n.node_id.clone());
        } else if g.takes_part_in_reasoning(&n.node_id).unwrap() {
            ids.insert(n.node_id.clone());
        }
    }
    for e in &doc.edges {
        match e.edge_type.as_str() {
            "CONTRADICTS" => {
                ids.insert(e.from_id.clone());
                ids.insert(e.to_id.clone());
            }
            "PREMISE_OF" => {
                premises.insert(e.from_id.clone());
            }
            "CONCLUDES" => {
                conclusions.insert(e.to_id.clone());
            }
            _ => {}
        }
    }
    let mut out: BTreeMap<String, Json> = BTreeMap::new();
    let mut per = |reader: &str, f: &dyn Fn(&str) -> Json, over: &BTreeSet<String>| {
        if !every_node {
            return;
        }
        let m: BTreeMap<String, Json> = over.iter().map(|id| (id.clone(), f(id))).collect();
        out.insert(reader.to_string(), to_json(m));
    };
    per(
        "premise_truth",
        &|id| to_json(g.premise_truth(id).unwrap()),
        &ids,
    );
    per(
        "statement_reading",
        &|id| to_json(g.statement_reading(id).unwrap()),
        &ids,
    );
    per(
        "objections_to",
        &|id| to_json(g.objections_to(id).unwrap()),
        &ids,
    );
    per(
        "chain_reading",
        &|id| to_json(g.chain_reading(id).unwrap()),
        &ids,
    );
    per(
        "foundations_reading",
        &|id| to_json(g.foundations_reading(id).unwrap()),
        &ids,
    );
    per(
        "proof_reading",
        &|id| to_json(g.proof_reading(id).unwrap()),
        &ids,
    );
    per(
        "lines_under",
        &|id| to_json(g.lines_under(id).unwrap()),
        &ids,
    );
    per(
        "check_conditions",
        &|id| to_json(g.check_conditions(id).unwrap()),
        &ids,
    );
    per(
        "inference_reading",
        &|id| to_json(g.inference_reading(id).unwrap()),
        &steps,
    );
    per(
        "reasoning_report_under",
        &|id| to_json(g.reasoning_report(Some(id), Some(500)).unwrap()),
        &conclusions,
    );
    per(
        "what_if",
        &|id| to_json(g.inference_walk_full(&[id]).unwrap().rows),
        &premises,
    );
    out.insert(
        "reasoning_report".into(),
        to_json(g.reasoning_report(None, Some(500)).unwrap()),
    );
    out.insert(
        "reasoning_debt".into(),
        to_json(g.reasoning_debt().unwrap()),
    );
    out.insert(
        "conclusions_that_do_not_follow".into(),
        to_json(g.conclusions_that_do_not_follow().unwrap()),
    );
    out.insert(
        "conclusions_that_follow".into(),
        to_json(g.conclusions_that_follow().unwrap()),
    );
    out.insert("assumptions".into(), to_json(g.assumptions().unwrap()));
    out.insert(
        "stated_independence".into(),
        to_json(g.stated_independence().unwrap()),
    );
    out.insert(
        "generalizations_that_do_not_hold".into(),
        to_json(g.generalizations_that_do_not_hold().unwrap()),
    );
    out.insert(
        "reasoning_single_points".into(),
        to_json(g.reasoning_single_points().unwrap()),
    );
    out.insert(
        "design_foundations".into(),
        to_json(g.design_foundations().unwrap()),
    );
    out.insert("loop_status".into(), to_json(g.loop_status().unwrap()));
    out.insert("detect_gaps".into(), to_json(g.detect_gaps().unwrap()));
    out.insert(
        "detect_defects".into(),
        to_json(g.detect_defects().unwrap()),
    );
    canonical(&mut out);
    out
}

/// The one reading whose ORDER is not stable from run to run, on the code
/// before this change as after it: two CONTRADICTS edges between the same
/// pair (A against B, B against A) are one defect id, and which of the two
/// is listed first follows the store's iteration order. Sorted here, so the
/// golden pins what is found, not the order of a tie.
fn canonical(out: &mut BTreeMap<String, Json>) {
    if let Some(Json::Array(defects)) = out
        .get_mut("detect_defects")
        .and_then(|d| d.get_mut("defects"))
    {
        defects.sort_by_key(|d| d.to_string());
    }
}

/// FNV-1a over the JSON text, as the golden keeps it: the whole readings would
/// be megabytes. `REFLOW2_DUMP_READINGS=<dir>` writes them whole, to diff.
fn digest(v: &Json) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in v.to_string().bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

const GOLDEN: &str = "tests/fixtures/readings_before_the_shared_evaluation.json";

/// EVERY READING IS AS IT WAS: each fixture of the proof, argument and
/// verification trials, and the Shop-shaped design, read by every reader in
/// [`readings`], equals the golden taken on main 7e9209515, before the
/// evaluation was shared or a circle memoized. A difference is a bug in the
/// change. `REFLOW2_BLESS_READINGS_GOLDEN=1` rewrites the golden, which is
/// only ever right on the code before the change.
#[test]
fn every_reading_is_as_it_was_before_the_evaluation_was_shared() {
    on_a_big_stack(every_reading_as_it_was);
}

fn every_reading_as_it_was() {
    let mut designs: Vec<(String, DesignGraph, GraphExport)> = Vec::new();
    for (name, doc) in super::a_premise_says_whether_it_rests_on_intent_or_delivery::FIXTURES {
        let doc: GraphExport = serde_json::from_str(doc).expect("fixture parses");
        let mut g = DesignGraph::open_in_memory().expect("in-memory graph");
        g.import_graph(&doc).expect("fixture imports");
        designs.push((name.to_string(), g, doc));
    }
    // The Shop's shape at two sizes: small enough that the code before the
    // change reads every node of it in seconds, and the Shop's own size,
    // read by the whole-design readers (the code before the change took
    // minutes over it; every node of it would take hours).
    let g = shop_shaped(8, 10);
    let doc = g.export_graph().unwrap();
    designs.push(("shop-shaped-small".to_string(), g, doc));
    let g = shop_shaped(24, 32);
    let doc = g.export_graph().unwrap();
    designs.push(("shop-shaped".to_string(), g, doc));

    let dump = std::env::var("REFLOW2_DUMP_READINGS").ok();
    let mut now: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for (name, g, doc) in &designs {
        let reads = readings(g, doc, name != "shop-shaped");
        if let Some(dir) = &dump {
            std::fs::create_dir_all(dir).unwrap();
            std::fs::write(
                std::path::Path::new(dir).join(format!("{name}.json")),
                serde_json::to_string_pretty(&reads).unwrap(),
            )
            .unwrap();
        }
        now.insert(
            name.clone(),
            reads.iter().map(|(k, v)| (k.clone(), digest(v))).collect(),
        );
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(GOLDEN);
    // THE GOLDEN FROM READINGS DUMPED BEFORE: the code before the change
    // takes minutes over the Shop-sized design, so its readings are dumped
    // once (REFLOW2_DUMP_READINGS) and digested here.
    if let Ok(dir) = std::env::var("REFLOW2_BLESS_READINGS_FROM") {
        let mut was: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        for name in now.keys() {
            let file = std::path::Path::new(&dir).join(format!("{name}.json"));
            let mut reads: BTreeMap<String, Json> =
                serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
            canonical(&mut reads);
            was.insert(
                name.clone(),
                reads.iter().map(|(k, v)| (k.clone(), digest(v))).collect(),
            );
        }
        std::fs::write(&path, serde_json::to_string_pretty(&was).unwrap() + "\n").unwrap();
        return;
    }
    if std::env::var("REFLOW2_BLESS_READINGS_GOLDEN").is_ok() {
        std::fs::write(&path, serde_json::to_string_pretty(&now).unwrap() + "\n").unwrap();
        return;
    }
    let golden: BTreeMap<String, BTreeMap<String, String>> =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("the golden exists"))
            .expect("the golden parses");
    assert_eq!(
        golden.keys().collect::<Vec<_>>(),
        now.keys().collect::<Vec<_>>(),
        "every design is in the golden"
    );
    let mut moved = Vec::new();
    for (name, reads) in &now {
        for (reader, d) in reads {
            if golden[name].get(reader) != Some(d) {
                moved.push(format!("{name}: {reader}"));
            }
        }
    }
    assert!(
        moved.is_empty(),
        "readings moved (REFLOW2_DUMP_READINGS=<dir> writes them whole, to diff against the \
         code before the change): {moved:#?}"
    );
}
