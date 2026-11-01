//! A MEASUREMENT INSTRUMENT, NOT A GATE: replay the near-match guard's query
//! over every capture node of a saved design, and write the score window each
//! one would have met, so cut options and signals can be compared offline
//! against a real design rather than chosen by taste.
//!
//! Made 2026-11-01 for
//! `req:the-near-match-guard-names-a-reworded-duplicate-of-a-verified-capability`.
//! Its first use measured four cuts on today's score and found none that
//! catches a reworded duplicate without new false alarms at every design size
//! (`fact:no-cut-tried-for-the-near-match-guard-catches-a-reworded-duplicate-
//! without-new-false-alarms-2026-11-01`). It is `#[ignore]` and reads its
//! inputs from the environment, so an ordinary run never touches it.
//!
//! # Running it
//!
//! ```text
//! REFLOW2_MEASURE_EXPORT=docs/design/reflow2/ \
//! REFLOW2_MEASURE_OUT=/tmp/replay.jsonl \
//!   cargo test -p reflow2-mcp --no-default-features --test in_memory \
//!   near_match_replay -- --ignored --nocapture
//! ```
//!
//! - `REFLOW2_MEASURE_EXPORT` (required): a saved design, the item layout's
//!   directory or a single-file export. A design's text stays out of git:
//!   export it under `docs/design/`, which is ignored.
//! - `REFLOW2_MEASURE_OUT` (required): where the JSONL goes.
//! - `REFLOW2_MEASURE_EXTRA` (optional): a JSON list of nodes to write before
//!   the replay, each `{"type", "id", "props": {name: text}}`, for a probe the
//!   saved design does not hold yet.
//! - `REFLOW2_MEASURE_KEEP_MOD=N` (optional): keep one node in N, by a stable
//!   hash of its id, and only the edges between kept nodes, to replay the same
//!   design at a smaller size. `REFLOW2_MEASURE_ALWAYS` names ids, comma
//!   separated, that are kept regardless.
//!
//! # What it writes
//!
//! One line per probe: its `id`, `type`, `words` (of the text the guard
//! searches), `kind`, `statement_kind`, `distinct_from`, and `hits`, the
//! search window of [`WINDOW`] for that text, best first, the probe itself
//! included. Each hit is `[id, type, score, edges, kind]`, where `edges` lists
//! every edge between the probe and the hit (`>TYPE` out of the probe, `<TYPE`
//! into it) and is empty when nothing links them: the false-alarm proxy.
//!
//! The probe is already in the index, exactly as it is when `search_first`
//! runs (the write lands first), so its own score is the same baseline the
//! guard reads. What differs from write time is that the design has grown
//! since, so absolute rates run higher than at write time; comparisons between
//! options read the same windows and are like for like.

use reflow2_core::{DesignGraph, ImportOptions};
use std::io::Write;

/// How many hits each probe's window holds. Wider than the guard's own window
/// (12) so that options reading the score DISTRIBUTION have one to read.
const WINDOW: usize = 40;

/// The text the guard searches for each capture type: the same `format!`s as
/// the `search_first` call sites in `src/tools/capture.rs`. Keep them in step.
fn text_of(node_type: &str, p: &std::collections::HashMap<String, reflow2_core::Value>) -> String {
    let s = |k: &str| {
        p.get(k)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    };
    let name = s("name");
    match node_type {
        "Requirement" | "DesignRule" | "Constraint" => format!("{name} {}", s("statement")),
        "Capability" => format!("{name} {}", s("description")),
        "Component" => format!("{name} {}", s("purpose")),
        "Interface" => name,
        "Decision" => format!("{name} {}", s("decision")),
        _ => name,
    }
}

#[test]
#[ignore = "measurement, not a gate — see the module header"]
fn replay_search_first_over_a_saved_design() {
    let Ok(path) = std::env::var("REFLOW2_MEASURE_EXPORT") else {
        eprintln!("REFLOW2_MEASURE_EXPORT unset; nothing measured");
        return;
    };
    let out_path = std::env::var("REFLOW2_MEASURE_OUT").expect("REFLOW2_MEASURE_OUT");
    let mut doc = reflow2_mcp::saved_design::read_export(&path).expect("read export");
    // Subsample: keep one node in N (by a stable hash of its id), plus the
    // probes named in REFLOW2_MEASURE_ALWAYS, and only edges between kept nodes.
    if let Ok(m) = std::env::var("REFLOW2_MEASURE_KEEP_MOD") {
        let m: u64 = m.parse().expect("mod");
        let always: Vec<String> = std::env::var("REFLOW2_MEASURE_ALWAYS")
            .unwrap_or_default()
            .split(',')
            .map(str::to_string)
            .collect();
        let h = |s: &str| {
            s.bytes().fold(1469598103934665603u64, |a, b| {
                (a ^ b as u64).wrapping_mul(1099511628211)
            })
        };
        doc.nodes.retain(|n| {
            always.contains(&n.node_id)
                || h(&n.node_id) % m == 0
                || n.node_type == "Project"
                || n.node_type == "Contributor"
        });
        let kept: std::collections::HashSet<String> =
            doc.nodes.iter().map(|n| n.node_id.clone()).collect();
        doc.edges
            .retain(|e| kept.contains(&e.from_id) && kept.contains(&e.to_id));
        doc.content_hash = None;
        eprintln!(
            "subsampled to {} nodes, {} edges",
            doc.nodes.len(),
            doc.edges.len()
        );
    }
    let mut g = DesignGraph::open_in_memory_as(&doc.graph_id).expect("graph");
    g.import_graph_with(
        &doc,
        ImportOptions {
            accept_newer: true,
            merge: false,
        },
    )
    .expect("import");
    // Extra probes: (type, id, name, body) written into the design first, as a
    // capture tool would write them.
    if let Ok(extra) = std::env::var("REFLOW2_MEASURE_EXTRA") {
        let raw = std::fs::read_to_string(extra).expect("extra");
        let v: serde_json::Value = serde_json::from_str(&raw).expect("extra json");
        for e in v.as_array().expect("array") {
            let ty = e["type"].as_str().unwrap();
            let id = e["id"].as_str().unwrap();
            let mut props = std::collections::HashMap::new();
            for (k, val) in e["props"].as_object().unwrap() {
                props.insert(
                    k.clone(),
                    reflow2_core::Value::from(val.as_str().unwrap().to_string()),
                );
            }
            g.upsert_node(ty, id, props).expect("upsert extra");
        }
    }
    let mut out = std::fs::File::create(&out_path).expect("out");
    let types = [
        "Requirement",
        "DesignRule",
        "Capability",
        "Component",
        "Interface",
        "Constraint",
        "Decision",
    ];
    let mut n = 0usize;
    for ty in types {
        for node in g.scan_nodes(ty).expect("scan") {
            let text = text_of(ty, &node.properties);
            let words = text.split_whitespace().count();
            let r = match g.search_design(&text, None, WINDOW) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("{}: {e}", node.node_id);
                    continue;
                }
            };
            let mut linked: std::collections::HashMap<String, Vec<String>> = Default::default();
            for e in g.outgoing(&node.node_id, None).unwrap_or_default() {
                linked
                    .entry(e.to_id.clone())
                    .or_default()
                    .push(format!(">{}", e.edge_type));
            }
            for e in g.incoming(&node.node_id, None).unwrap_or_default() {
                linked
                    .entry(e.from_id.clone())
                    .or_default()
                    .push(format!("<{}", e.edge_type));
            }
            let distinct: Vec<String> = match node.properties.get("distinct_from") {
                Some(reflow2_core::Value::List(l)) => l
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect(),
                _ => Vec::new(),
            };
            let hits: Vec<serde_json::Value> = r
                .hits
                .iter()
                .map(|h| {
                    serde_json::json!([
                        h.node_id,
                        h.node_type,
                        h.score,
                        linked
                            .get(&h.node_id)
                            .cloned()
                            .unwrap_or_default()
                            .join(","),
                        h.kind.clone().unwrap_or_default()
                    ])
                })
                .collect();
            let line = serde_json::json!({
                "id": node.node_id,
                "type": ty,
                "words": words,
                "kind": node.properties.get("kind").and_then(|v| v.as_str()).unwrap_or_default(),
                "statement_kind": node.properties.get("statement_kind").and_then(|v| v.as_str()).unwrap_or_default(),
                "distinct_from": distinct,
                "hits": hits,
            });
            writeln!(out, "{line}").unwrap();
            n += 1;
        }
    }
    eprintln!("probes written: {n}");
}
