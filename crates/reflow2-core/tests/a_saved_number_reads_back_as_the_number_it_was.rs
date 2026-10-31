//! A NUMBER SAVED IN A DESIGN READS BACK AS THE SAME NUMBER, to the last bit —
//! through the single-file export and through the item layout.
//!
//! FOUND 2026-10-30, by the generated design that replaced the committed copy
//! of reflow2's own design (`a_generated_design_stands_in_for_a_real_one.rs`).
//! Its round trip through the item layout reported 81 of 68,000 edge files as
//! "do not match their own content_hash — edited outside reflow2 or
//! corrupted", every one carrying a float such as `confidence: 0.11458333333333333`
//! (11/96). Nothing had edited them. MEASURED IN ISOLATION, with serde_json
//! 1.0.151 alone: writing 11/96 gives `0.11458333333333333`, the shortest text
//! that names it, and parsing that text gives `0.11458333333333331`, one unit
//! in the last place away; 13 of 291 such values came back different. Parsing
//! exactly is serde_json's `float_roundtrip` feature, which was off. With it,
//! 0 of 291.
//!
//! THE CLASS, NOT THE INSTANCE: any f64 a design holds could change on its way
//! through a saved file, and then a design whose files nobody touched reads as
//! tampered (item layout), fails its own `content_hash` (single file), or
//! comes back from a round trip a different design. reflow2's own design never
//! happened to hold one of the unlucky values, which is why the committed copy
//! never showed it. This pins every value of a sweep that, without the feature,
//! included 13 that changed — it FAILED before `float_roundtrip` was enabled in
//! crates/reflow2-core/Cargo.toml, and was watched failing.

use std::collections::BTreeMap;

use reflow2_core::GraphExport;
use reflow2_core::item_layout::{self, Anchor, DesignStamp, OnDisk};
use serde_json::json;

/// The sweep: 97 points on three ranges, the shape a generated `float` takes.
fn floats() -> Vec<f64> {
    let mut out = Vec::new();
    for k in 0..97u32 {
        for (lo, hi) in [(0.0f64, 1.0f64), (0.0, 100.0), (-1.0, 1.0)] {
            out.push(lo + (hi - lo) * (f64::from(k) / 96.0));
        }
    }
    out.push(11.0 / 96.0);
    out
}

fn design() -> GraphExport {
    let nodes: Vec<_> = (0..floats().len())
        .map(|i| {
            json!({"node_type": "Requirement", "node_id": format!("req:r{i}"),
                        "properties": {"name": "r", "statement": "s"}})
        })
        .chain([json!({"node_type": "Decision", "node_id": "dec:d", "properties": {"name": "d"}})])
        .collect();
    let edges: Vec<_> = floats()
        .iter()
        .enumerate()
        .map(|(i, x)| {
            json!({"edge_type": "EVOLVES_INTO", "from_id": "dec:d",
                             "to_id": format!("req:r{i}"), "properties": {"confidence": x}})
        })
        .collect();
    serde_json::from_value(json!({"graph_id": "numbers", "nodes": nodes, "edges": edges}))
        .expect("a design document")
}

#[test]
fn every_saved_float_reads_back_bit_for_bit_from_the_single_file() {
    let doc = design();
    let text = serde_json::to_string(&doc).expect("serializes");
    let back: GraphExport = serde_json::from_str(&text).expect("parses");
    let changed: Vec<String> = doc
        .edges
        .iter()
        .zip(&back.edges)
        .filter(|(a, b)| {
            let (x, y) = (
                a.properties["confidence"].as_f64(),
                b.properties["confidence"].as_f64(),
            );
            x.map(f64::to_bits) != y.map(f64::to_bits)
        })
        .map(|(a, b)| {
            format!(
                "{:?} -> {:?}",
                a.properties["confidence"], b.properties["confidence"]
            )
        })
        .collect();
    assert!(
        changed.is_empty(),
        "{} of {} floats changed through the export text: {:?}",
        changed.len(),
        doc.edges.len(),
        &changed[..changed.len().min(4)]
    );
    assert_eq!(
        back.compute_content_hash(),
        doc.compute_content_hash(),
        "the design read back is not the design written"
    );
}

#[test]
fn every_saved_float_reads_back_intact_from_the_item_layout() {
    let doc = design();
    let plan = item_layout::plan_write(&doc, &BTreeMap::<String, OnDisk>::new(), Anchor::Disk);
    let parsed: Vec<_> = plan
        .writes
        .iter()
        .map(|(rel, text)| item_layout::parse_item(rel, text.as_bytes()).expect("item parses"))
        .collect();
    let tampered: Vec<&str> = parsed
        .iter()
        .filter(|p| !p.intact())
        .map(|p| p.rel_path.as_str())
        .collect();
    assert!(
        tampered.is_empty(),
        "{} item file(s) reflow2 just wrote read back as edited: {:?}",
        tampered.len(),
        &tampered[..tampered.len().min(4)]
    );
    let back = item_layout::assemble(
        DesignStamp {
            graph_id: doc.graph_id.clone(),
            schema_version: 1,
            migrated_from: None,
        },
        parsed,
    )
    .expect("one design");
    // The layout assembles in its own order, so compare by edge, bit for bit.
    let read: BTreeMap<&str, u64> = back
        .export
        .edges
        .iter()
        .filter_map(|e| {
            Some((
                e.to_id.as_str(),
                e.properties.get("confidence")?.as_f64()?.to_bits(),
            ))
        })
        .collect();
    for e in &doc.edges {
        let x = e.properties["confidence"].as_f64().expect("a float");
        assert_eq!(
            read.get(e.to_id.as_str()),
            Some(&x.to_bits()),
            "{x:?} on {} did not come back from the layout as itself",
            e.to_id
        );
    }
}
