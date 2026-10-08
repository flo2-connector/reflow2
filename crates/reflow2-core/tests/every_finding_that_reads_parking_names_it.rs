//! Every finding whose rule reads a `parks` ruling says so, in the one shared
//! sentence, and no rule can start reading parking without being counted.
//!
//! THE FOURTH INSTANCE IS WHY THIS WALKS THE SOURCE. Parking tells a design an
//! unattached or unsatisfied state is deliberate. Four times a person was stuck
//! at a finding that reads it and nothing in that finding's words said so:
//! dev_storyflow on 2026-08-04 and 08-15, reflow2 on 08-19, and the dev_reflow2
//! two-agent exercise on 2026-09-29 (I12,
//! `fact:root-cause-parking-is-still-not-named-where-an-unsatisfied-requirement-is-read-2026-09-29`).
//! Each fix reached the one finding reported. A per-finding test would pin
//! today's four and leave the fifth open, so this reads the crate:
//!
//! 1. every call that reads parking sits in a function this file maps to the
//!    finding it feeds — an unmapped reader fails here, not in the field;
//! 2. the mapped findings are exactly `PARKING_READERS`, the list the shared
//!    sentence names;
//! 3. every one of them is reached by that sentence — the heal rule in its
//!    repair note, each gap through the reply that marks its row.

use reflow2_core::heal::PARKING_READERS;
use std::collections::{BTreeMap, BTreeSet};

/// The function a parking read sits in → the finding it feeds, or why it
/// feeds none. A new reader must be added here, which is the point.
const READING_FUNCTIONS: &[(&str, Option<&str>)] = &[
    // The predicate's own set form, delegating to `is_parked`.
    ("parked_nodes", None),
    // Counts the parked set into `swept.parked`; renders no finding.
    ("sweep_scope", None),
    ("all_defects", Some("orphan_node")),
    (
        "detect_unsatisfied_requirements",
        Some("unsatisfied_requirement"),
    ),
    (
        "detect_unallocated_components",
        Some("unallocated_component"),
    ),
    (
        "detect_fix_without_recorded_cause",
        Some("fix_without_recorded_cause"),
    ),
    (
        "detect_defect_overtaken_by_change",
        Some("defect_overtaken_by_change"),
    ),
    (
        "detect_decision_overtaken_by_promotion",
        Some("decision_overtaken_by_promotion"),
    ),
    ("detect_prohibitions_in_prose", Some("prohibition_in_prose")),
    (
        "detect_settled_decision_named_open",
        Some("settled_decision_named_open"),
    ),
    // Feeds `detect_unreviewed_ideas`, which renders the gap.
    ("unreviewed_ideas", Some("unreviewed_ideas")),
    // Feeds `detect_unlinked_intent`, which renders the gap.
    ("unlinked_intent", Some("unlinked_intent")),
    // closure_report's traceability leg counts a parked requirement in
    // `parked`, never as a hole; it renders no finding
    // (req:closure-reads-design-done-separately-from-build-done).
    ("leg_traceability", None),
];

fn sources() -> BTreeMap<String, String> {
    fn walk(dir: &std::path::Path, out: &mut BTreeMap<String, String>) {
        for entry in std::fs::read_dir(dir).expect("src is readable") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.insert(
                    path.display().to_string(),
                    std::fs::read_to_string(&path).expect("source is readable"),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut out,
    );
    out
}

/// The name of the innermost `fn` a line sits in, by the nearest preceding
/// `fn name(` line. Crude and sufficient: these readers are all methods.
fn enclosing_fn(lines: &[&str], at: usize) -> Option<String> {
    for line in lines[..=at].iter().rev() {
        let t = line.trim_start();
        let t = t
            .strip_prefix("pub(crate) ")
            .or_else(|| t.strip_prefix("pub "))
            .unwrap_or(t);
        if let Some(rest) = t.strip_prefix("fn ") {
            return rest
                .split('(')
                .next()
                .map(|n| n.split('<').next().unwrap_or(n).to_string());
        }
    }
    None
}

#[test]
fn every_rule_that_reads_parking_is_mapped_to_the_finding_it_feeds() {
    let mapped: BTreeMap<&str, Option<&str>> = READING_FUNCTIONS.iter().copied().collect();
    let mut unmapped = Vec::new();
    let mut seen_keys = BTreeSet::new();
    for (file, text) in sources() {
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            let reads = code.contains("is_parked(")
                || code.contains("parked.contains(")
                || code.contains("parked_nodes()");
            if !reads || code.contains("fn is_parked(") || code.contains("fn parked_nodes(") {
                continue;
            }
            match enclosing_fn(&lines, i).as_deref() {
                Some(f) if mapped.contains_key(f) => {
                    if let Some(k) = mapped[f] {
                        seen_keys.insert(k);
                    }
                }
                other => unmapped.push(format!("{file}:{} in fn {other:?}", i + 1)),
            }
        }
    }
    assert!(
        unmapped.is_empty(),
        "a rule reads parking and nothing says which finding it feeds — map it in \
         READING_FUNCTIONS, add its key to PARKING_READERS, and render parks_route in its \
         words: {unmapped:#?}"
    );
    let listed: BTreeSet<&str> = PARKING_READERS.iter().copied().collect();
    assert_eq!(
        seen_keys, listed,
        "the findings that read parking and the list the shared sentence names must be the \
         same set"
    );
}

#[test]
fn every_finding_that_reads_parking_is_one_the_shared_sentence_reaches() {
    let all: String = sources().into_values().collect::<Vec<_>>().join("\n");
    let mut unreached = Vec::new();
    for key in PARKING_READERS {
        // `orphan_node` carries the sentence in its repair note; every other
        // reader is a gap, and the MCP reply marks each row whose key is on
        // the list and sends the sentence once. So a key must be one of the
        // two — a real gap source, or the heal rule whose note renders it.
        let reached = if *key == "orphan_node" {
            all.contains("ORPHAN_REPAIR") && all.contains("parks_route()")
        } else {
            all.contains(&format!("=> \"{key}\""))
        };
        if !reached {
            unreached.push(*key);
        }
    }
    assert!(
        unreached.is_empty(),
        "these keys read parking and are neither a gap source nor the note that renders it: \
         {unreached:?}"
    );
}

#[test]
fn the_shared_sentence_names_the_mechanism_and_every_reader() {
    let text = reflow2_core::heal::parks_route();
    assert!(text.contains("ruling:") && text.contains("parks") && text.contains("ACCEPTED"));
    for key in PARKING_READERS {
        assert!(
            text.contains(key),
            "{key} missing from the sentence: {text}"
        );
    }
}
