//! THE REASONING WALKS KEEP THEIR OWN STACK
//! (`dec:deep-reasoning-walks-keep-their-own-stack-and-the-runtime-gets-headroom`,
//! Anthony, 2026-10-26: "Go with 1 + 3").
//!
//! Measured on a release build of main 0c567349c
//! (`fact:a-release-server-overflows-its-2-mib-worker-stack-on-a-reasoning-chain-about-2000-steps-deep-and-the-whole-process-aborts-2026-10-25`):
//! `foundations_walk` and `proof_chain_walk` recursed once per step of a
//! reasoning chain, with no bound, and a server's 2 MiB worker stack ran out at
//! about 3,900 steps (get_node) and 2,000 (loop_status on a chain whose steps
//! hold). A stack overflow aborts the whole process, so on flo2.io one deep
//! design took the engine down for every design it serves.
//!
//! Each test here pins the CLASS, not the measured depth: the walk is called
//! directly on a chain [`STEPS`] steps deep, on a thread whose stack is set to
//! [`WORKER_STACK`], the size a tokio worker got before the fix. A recursive
//! walk needs that many frames, which no stack a server would choose holds; a
//! walk with its own work stack needs one.
//!
//! A STACK OVERFLOW ABORTS THE WHOLE TEST PROGRAM, not one test, so each walk
//! runs in a CHILD PROCESS: the test runs this same test binary again with
//! [`CHILD`] set, filtered to itself, and asserts the child exited cleanly and
//! said what it walked. Without [`CHILD`] a test only spawns its child.

use super::*;
use std::process::Command;

/// How deep the chain is: far past anything a recursive walk survives on a
/// 2 MiB stack (about 3,900 steps in release, far fewer in debug).
const STEPS: usize = 100_000;

/// The stack a tokio worker had before the fix: std's default for a spawned
/// thread, 2 MiB. Set explicitly, so RUST_MIN_STACK cannot change it.
const WORKER_STACK: usize = 2 << 20;

/// Set in the child process: the test does the walk instead of spawning.
const CHILD: &str = "REFLOW2_DEEP_WALK_CHILD";

/// The id of step `k` and of the claim it concludes, zero-padded so the ids
/// sort in chain order.
fn step(k: usize) -> String {
    format!("inf:s{k:06}")
}
fn claim(k: usize) -> String {
    format!("req:c{k:06}")
}

/// A linear chain `steps` deep: `con:ground` → `inf:s000000` → `req:c000001`
/// → `inf:s000001` → … → `req:c{steps}`. Each step rests on the claim before
/// it, and every step but the deepest, `inf:s000000`, is confirmed by a
/// person. Built in chain order, so the circle check each link write makes
/// looks only one link ahead.
///
/// WRITTEN IN ONE WRITE UNIT, the way a served call writes. Outside a unit
/// every node write commits the full-text index and reloads its reader, and
/// `cargo test --workspace` builds this crate with `fulltext` on (reflow2-mcp
/// asks for it). Measured 2026-10-26 on a debug build: 1,000 steps took 0.59 s
/// to build with the feature off and 118.5 s with it on (4,000: 1.96 s and
/// 534.8 s), while the walk itself took the same either way. Unbatched, this
/// chain kept PR #802's workspace job past its 60-minute limit.
fn chain(steps: usize) -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("an in-memory graph");
    g.begin_unit();
    g.upsert_node(
        node::CONSTRAINT,
        "con:ground",
        Props::new()
            .set("name", "the ground")
            .set("statement", "A limit the design keeps."),
    )
    .expect("the ground");
    for k in 1..=steps {
        g.upsert_node(
            node::REQUIREMENT,
            &claim(k),
            Props::new()
                .set("name", format!("claim {k}"))
                .set("statement", "The owner's need.")
                .set("status", "accepted"),
        )
        .expect("a claim");
    }
    for k in 0..steps {
        let s = step(k);
        let mut props = Props::new().set("name", s.as_str()).set("rule", "all");
        if k > 0 {
            props = props.set("confirmed_how", "person");
        }
        g.upsert_node(node::INFERENCE, &s, props).expect("a step");
        let (premise_type, premise) = if k == 0 {
            (node::CONSTRAINT, "con:ground".to_string())
        } else {
            (node::REQUIREMENT, claim(k))
        };
        g.create_edge(
            edge::PREMISE_OF,
            premise_type,
            &premise,
            node::INFERENCE,
            &s,
            Props::new(),
        )
        .expect("a premise");
        g.create_edge(
            edge::CONCLUDES,
            node::INFERENCE,
            &s,
            node::REQUIREMENT,
            &claim(k + 1),
            Props::new(),
        )
        .expect("a conclusion");
    }
    g.commit_unit().expect("the chain is written");
    g
}

/// Run `f` on a thread with exactly [`WORKER_STACK`] of stack.
fn on_a_worker_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .name("deep-walk".into())
        .stack_size(WORKER_STACK)
        .spawn(f)
        .expect("a thread")
        .join()
        .unwrap_or_else(|e| std::panic::resume_unwind(e))
}

/// In the parent: run THIS test again in a child process, and assert it
/// exited cleanly and ran (a filter that matched nothing also exits 0).
/// Returns true in the child, where the caller does the work.
fn in_a_child(test: &str) -> bool {
    if std::env::var_os(CHILD).is_some() {
        return true;
    }
    let path = format!(
        "{}::{test}",
        module_path!()
            .split_once("::")
            .map_or(module_path!(), |(_, rest)| rest)
    );
    let out = Command::new(std::env::current_exe().expect("this test binary"))
        .args([path.as_str(), "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD, "1")
        .output()
        .expect("the child test process runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "{path} in a child process did not exit cleanly ({}): a walk {STEPS} steps deep on a \
         {} MiB stack.\n--- stdout\n{stdout}\n--- stderr\n{stderr}",
        out.status,
        WORKER_STACK >> 20
    );
    assert!(
        stdout.contains("1 passed"),
        "{path}: the child ran no test.\n--- stdout\n{stdout}\n--- stderr\n{stderr}"
    );
    false
}

/// `foundations_walk`, called through `foundations_of` (which does nothing
/// else before it) on the chain's last claim: it reaches the ground, the one
/// axiom, through every step.
#[test]
fn the_foundations_walk_reaches_the_ground_of_a_chain_100000_steps_deep_on_a_2_mib_stack() {
    if !in_a_child(
        "the_foundations_walk_reaches_the_ground_of_a_chain_100000_steps_deep_on_a_2_mib_stack",
    ) {
        return;
    }
    let g = chain(STEPS);
    let top = claim(STEPS);
    let axioms = on_a_worker_stack(move || {
        g.foundations_of(&top)
            .expect("the foundations read")
            .into_iter()
            .map(|f| f.id)
            .collect::<Vec<_>>()
    });
    assert_eq!(axioms, vec!["con:ground".to_string()]);
}

/// `proof_chain_walk`, called directly from the chain's top step: every step
/// is under the claim, every confirmed one is counted, and the one unconfirmed
/// step, the deepest, leaves the line unconfirmed.
#[test]
fn the_proof_chain_walk_counts_every_step_of_a_chain_100000_steps_deep_on_a_2_mib_stack() {
    if !in_a_child(
        "the_proof_chain_walk_counts_every_step_of_a_chain_100000_steps_deep_on_a_2_mib_stack",
    ) {
        return;
    }
    let g = chain(STEPS);
    let (chain_confirmed, under, confirmed_under) = on_a_worker_stack(move || {
        let mut chain_confirmed = true;
        let mut under = BTreeSet::new();
        let mut confirmed_under = BTreeSet::new();
        let mut visited = BTreeSet::from([claim(STEPS)]);
        g.proof_chain_walk(
            &step(STEPS - 1),
            &mut chain_confirmed,
            &mut under,
            &mut confirmed_under,
            &mut visited,
        )
        .expect("the line is walked");
        (chain_confirmed, under, confirmed_under)
    });
    assert_eq!(under.len(), STEPS, "every step is under the claim");
    assert_eq!(
        confirmed_under.len(),
        STEPS - 1,
        "every confirmed step counts"
    );
    assert!(!confirmed_under.contains(&step(0)));
    assert!(!chain_confirmed, "the deepest step is unconfirmed");
}

// ─── Every answer is the one the recursion gave ───────────────────────────
//
// The walks as they were on main 0c567349c, recursion and all, kept here only
// to compare against: on every fixture this crate and the MCP crate hold, and
// on generated designs with shared steps, diamonds, several lines to one
// claim and assumed premises, each walk must leave exactly the state the
// recursion left — the axioms with how each was read (which step reached an
// axiom first decides that, so it witnesses the visiting order), and what was
// visited, counted and confirmed.

/// `foundations_walk` as it was on main 0c567349c.
fn recursive_foundations_walk(
    g: &DesignGraph,
    steps: &[String],
    axioms: &mut BTreeMap<String, Foundation>,
    visited: &mut BTreeSet<String>,
) -> Result<(), DynoError> {
    for step_id in steps {
        if !visited.insert(step_id.clone()) {
            continue;
        }
        for (premise_id, assumed) in g.premise_links(step_id)? {
            if assumed {
                continue;
            }
            if !visited.insert(premise_id.clone()) {
                continue;
            }
            let concluding = g.inferences_concluding(&premise_id)?;
            if concluding.is_empty() {
                if !axioms.contains_key(&premise_id) {
                    let (truth, why, reads_as) = g.premise_truth_in_step(step_id, &premise_id)?;
                    let node_type = g.type_holding(&premise_id)?;
                    axioms.insert(
                        premise_id.clone(),
                        Foundation {
                            id: premise_id,
                            node_type,
                            truth,
                            why,
                            reads_as: reads_as.and_then(ReadsAs::served),
                        },
                    );
                }
            } else {
                recursive_foundations_walk(g, &concluding, axioms, visited)?;
            }
        }
    }
    Ok(())
}

/// `proof_chain_walk` as it was on main 0c567349c.
fn recursive_proof_chain_walk(
    g: &DesignGraph,
    step_id: &str,
    chain_confirmed: &mut bool,
    under: &mut BTreeSet<String>,
    confirmed_under: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
) -> Result<(), DynoError> {
    if !visited.insert(step_id.to_string()) {
        return Ok(());
    }
    if let Some(rec) = g.step_record(step_id)? {
        under.insert(step_id.to_string());
        if rec.confirmation.is_confirmed() {
            confirmed_under.insert(step_id.to_string());
        } else {
            *chain_confirmed = false;
        }
    }
    for premise_id in g.premises_of(step_id)? {
        if let Some(far) = g.far_premise(&premise_id)?
            && far
                .proof_status
                .as_deref()
                .is_some_and(|status| status != "proven")
        {
            *chain_confirmed = false;
        }
        let concluding = g.inferences_concluding(&premise_id)?;
        for sub_step in &concluding {
            recursive_proof_chain_walk(
                g,
                sub_step,
                chain_confirmed,
                under,
                confirmed_under,
                visited,
            )?;
        }
    }
    Ok(())
}

/// A foundations walk: the steps to walk, the axioms found, what was visited.
type FoundationsWalk<'a> = dyn Fn(&[String], &mut BTreeMap<String, Foundation>, &mut BTreeSet<String>) -> Result<(), DynoError>
    + 'a;

/// A proof walk: whether the line is confirmed, the steps under it, those
/// confirmed, and what was visited.
type ProofChainWalk<'a> = dyn Fn(
        &mut bool,
        &mut BTreeSet<String>,
        &mut BTreeSet<String>,
        &mut BTreeSet<String>,
    ) -> Result<(), DynoError>
    + 'a;

/// Both foundations walks from every concluded node, and both proof walks
/// from every step, compared state for state. Returns how many walks of each
/// it compared.
fn same_as_the_recursion(g: &DesignGraph, design: &str) -> (usize, usize) {
    let mut concluded: BTreeSet<String> = BTreeSet::new();
    let steps: Vec<String> = g
        .scan_nodes(node::INFERENCE)
        .expect("the steps")
        .into_iter()
        .map(|n| n.node_id)
        .collect();
    for s in &steps {
        if let Some(c) = g.conclusion_of(s).expect("a conclusion") {
            concluded.insert(c);
        }
    }
    let foundations = |walk: &FoundationsWalk, id: &str| {
        let mut axioms = BTreeMap::new();
        let mut visited = BTreeSet::from([id.to_string()]);
        let steps = g.inferences_concluding(id).expect("its steps");
        let r = walk(&steps, &mut axioms, &mut visited).map_err(|e| e.to_string());
        let axioms = serde_json::to_value(axioms.into_values().collect::<Vec<_>>()).unwrap();
        (r, axioms, visited)
    };
    for id in &concluded {
        let new = foundations(&|s, a, v| g.foundations_walk(s, a, v), id);
        let old = foundations(&|s, a, v| recursive_foundations_walk(g, s, a, v), id);
        assert_eq!(
            new, old,
            "{design}: the foundations of {id} differ from the recursion's"
        );
    }
    for s in &steps {
        let conclusion = g.conclusion_of(s).expect("a conclusion");
        let line = |walk: &ProofChainWalk| {
            let mut confirmed = true;
            let (mut under, mut confirmed_under) = (BTreeSet::new(), BTreeSet::new());
            let mut visited: BTreeSet<String> = conclusion.iter().cloned().collect();
            let r = walk(
                &mut confirmed,
                &mut under,
                &mut confirmed_under,
                &mut visited,
            )
            .map_err(|e| e.to_string());
            (r, confirmed, under, confirmed_under, visited)
        };
        let new = line(&|c, u, cu, v| g.proof_chain_walk(s, c, u, cu, v));
        let old = line(&|c, u, cu, v| recursive_proof_chain_walk(g, s, c, u, cu, v));
        assert_eq!(
            new, old,
            "{design}: the line from {s} differs from the recursion's"
        );
    }
    (concluded.len(), steps.len())
}

/// Every fixture of reasoning both crates hold, walked both ways.
#[test]
fn every_fixtures_walks_are_the_recursions() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut compared = (0, 0, 0);
    for dir in ["tests/fixtures", "../reflow2-mcp/tests/fixtures"] {
        let mut paths: Vec<_> = std::fs::read_dir(root.join(dir))
            .expect("the fixtures")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        paths.sort();
        for path in paths {
            let text = std::fs::read_to_string(&path).expect("a fixture");
            let Ok(doc) = serde_json::from_str::<crate::export::GraphExport>(&text) else {
                continue; // not a design export
            };
            let mut g = DesignGraph::open_in_memory().expect("an in-memory graph");
            if g.import_graph(&doc).is_err() {
                continue;
            }
            let (c, s) = same_as_the_recursion(&g, &path.display().to_string());
            compared = (compared.0 + 1, compared.1 + c, compared.2 + s);
        }
    }
    assert!(
        compared.0 >= 10 && compared.2 >= 100,
        "too few fixtures read to mean anything: {compared:?}"
    );
}

/// Generated designs, walked both ways: claims concluded by one step or
/// several, steps sharing premises (diamonds), assumed premises, confirmed
/// and unconfirmed steps, and links the write path refuses as circles left
/// out.
#[test]
fn generated_designs_walks_are_the_recursions() {
    let mut seed: u64 = 0x5eed_0c56_7349;
    let mut next = |n: u64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed % n
    };
    let mut steps_compared = 0;
    for design in 0..200 {
        let mut g = DesignGraph::open_in_memory().expect("an in-memory graph");
        let nodes = 4 + next(14) as usize;
        let mut ids = Vec::new();
        for i in 0..nodes {
            let (ty, id) = match next(3) {
                0 => (node::CONSTRAINT, format!("con:n{i:02}")),
                1 => (node::DECISION, format!("dec:n{i:02}")),
                _ => (node::REQUIREMENT, format!("req:n{i:02}")),
            };
            let props = Props::new()
                .set("name", id.as_str())
                .set("statement", "A statement.")
                .set("decision", "A choice.")
                .set("status", if next(3) == 0 { "proposed" } else { "accepted" });
            g.upsert_node(ty, &id, props).expect("a node");
            ids.push((ty, id));
        }
        let steps = 2 + next(12) as usize;
        for k in 0..steps {
            let s = format!("inf:g{k:02}");
            let mut props = Props::new().set("name", s.as_str()).set("rule", "all");
            if next(2) == 0 {
                props = props.set("confirmed_how", "person");
            }
            g.upsert_node(node::INFERENCE, &s, props).expect("a step");
            let (ct, c) = ids[next(nodes as u64) as usize].clone();
            if g.create_edge(edge::CONCLUDES, node::INFERENCE, &s, ct, &c, Props::new())
                .is_err()
            {
                continue;
            }
            for _ in 0..1 + next(4) {
                let (pt, p) = ids[next(nodes as u64) as usize].clone();
                let props = if next(5) == 0 {
                    Props::new().set("role", "assumed")
                } else {
                    Props::new()
                };
                // A circle, or a premise that is the conclusion, is refused.
                let _ = g.create_edge(edge::PREMISE_OF, pt, &p, node::INFERENCE, &s, props);
            }
        }
        steps_compared += same_as_the_recursion(&g, &format!("generated design {design}")).1;
    }
    assert!(steps_compared >= 1000, "{steps_compared} steps compared");
}
