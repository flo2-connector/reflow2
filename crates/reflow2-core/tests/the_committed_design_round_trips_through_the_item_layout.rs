//! REFLOW2'S OWN COMMITTED DESIGN round-trips through the item layout with
//! nothing lost: import → export → item files → import → export gives the same
//! design, byte for byte, and every accepted checksum comes back.
//!
//! The item layout (`dec:how-the-saved-design-is-laid-out-so-git-merges-it`)
//! and the checksum move (`dec:item-13-checksums-move-to-change-edges-and-main-converts-in-one-pr`,
//! decision 3) each change how the record is written: one file per node and
//! per edge, and an Artifact's `checksum` left off its node while the change
//! that accepted it carries it. The small designs in
//! `the_design_is_saved_one_file_per_item.rs` pin the rules. This pins them on
//! the one design that has every node type, every property shape and every
//! odd id this project has ever written — 6,600 nodes and 43,000 edges — so a
//! property the rules mishandle shows up here before it shows up as a design
//! that quietly lost something.
//!
//! It reads whichever form is committed: `docs/design/reflow2/` once the
//! conversion lands, `docs/design/reflow2.json` before it. In-memory backend,
//! no RocksDB, so it runs in the core job.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use reflow2_core::export::current_acceptances;
use reflow2_core::graph::DesignGraph;
use reflow2_core::item_layout::{self, Anchor, DesignStamp, OnDisk, ParsedItem};
use reflow2_core::nodes::node;
use reflow2_core::{GraphExport, Value};

const SINGLE: &str = "../../docs/design/reflow2.json";
const ITEMS: &str = "../../docs/design/reflow2";

/// The committed design as one document, which form it came from, and the
/// whole-design hash the committed record states (the single file) or
/// computes to (the layout).
fn committed() -> (GraphExport, &'static str, String) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let items = root.join(ITEMS);
    if items.is_dir() {
        let stamp: DesignStamp = serde_json::from_slice(
            &std::fs::read(items.join(item_layout::DESIGN_FILE)).expect("design.json"),
        )
        .expect("a design stamp");
        let parsed = read_dir_items(&items);
        let assembled = item_layout::assemble(stamp, parsed).expect("one design");
        assert!(
            assembled.integrity_note().is_none(),
            "the committed layout is not intact: {:?}",
            assembled.integrity_note()
        );
        let hash = assembled.export.compute_content_hash();
        return (assembled.export, "item layout", hash);
    }
    let path = root.join(SINGLE);
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let doc: GraphExport = serde_json::from_str(&raw).expect("the committed export parses");
    let computed = doc.compute_content_hash();
    if let Some(stated) = &doc.content_hash {
        assert_eq!(
            stated, &computed,
            "the committed single file does not match its own content_hash"
        );
    }
    (doc, "single file", computed)
}

fn read_dir_items(dir: &Path) -> Vec<ParsedItem> {
    let mut parsed = Vec::new();
    let mut stack: Vec<PathBuf> = vec![
        dir.join(item_layout::NODES_DIR),
        dir.join(item_layout::EDGES_DIR),
    ];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).expect("list the layout") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "json") {
                let rel = path
                    .strip_prefix(dir)
                    .expect("inside the layout")
                    .to_string_lossy()
                    .replace('\\', "/");
                let bytes = std::fs::read(&path).expect("read an item");
                parsed.push(item_layout::parse_item(&rel, &bytes).expect("an item"));
            }
        }
    }
    parsed
}

/// Write `export` over the in-memory directory `dir` exactly as the server's
/// writer plans it; returns the new directory and how many files it wrote.
fn write(
    export: &GraphExport,
    dir: &BTreeMap<String, String>,
) -> (BTreeMap<String, String>, usize) {
    let parsed: Vec<ParsedItem> = dir
        .iter()
        .map(|(rel, text)| item_layout::parse_item(rel, text.as_bytes()).expect("item parses"))
        .collect();
    let on_disk: BTreeMap<String, OnDisk> = parsed
        .iter()
        .map(|p| (p.rel_path.clone(), OnDisk::from(p)))
        .collect();
    let plan = item_layout::plan_write(export, &on_disk, Anchor::Disk);
    let written = plan.writes.len() + plan.deletes.len();
    let mut out = dir.clone();
    for (rel, text) in plan.writes {
        out.insert(rel, text);
    }
    for rel in plan.deletes {
        out.remove(&rel);
    }
    (out, written)
}

fn import(doc: &GraphExport) -> DesignGraph {
    let mut g = DesignGraph::open_in_memory_as(&doc.graph_id).expect("in-memory graph");
    g.import_graph(doc).expect("the design imports");
    g
}

/// Every Artifact's checksum as the STORE holds it — the value reconcile and
/// the gate compare against the file on disk.
fn stored_checksums(g: &DesignGraph) -> BTreeMap<String, String> {
    g.scan_nodes(node::ARTIFACT)
        .expect("scan artifacts")
        .into_iter()
        .filter_map(|n| {
            let c = n.properties.get("checksum")?.as_str()?.to_string();
            Some((n.node_id.clone(), c))
        })
        .collect()
}

/// The checksum each Artifact carries in a DOCUMENT: stated on the node, or —
/// when the export left it off because its accepting change carries it —
/// derived from the current acceptance, the import's own rule.
fn document_checksums(doc: &GraphExport) -> BTreeMap<String, String> {
    let derived = current_acceptances(&doc.edges);
    doc.nodes
        .iter()
        .filter(|n| n.node_type == node::ARTIFACT)
        .filter_map(|n| {
            let stated = n
                .properties
                .get("checksum")
                .and_then(Value::as_str)
                .map(str::to_string);
            let c = stated.or_else(|| derived.get(&n.node_id).map(|(c, _)| c.clone()))?;
            Some((n.node_id.clone(), c))
        })
        .collect()
}

/// The first few items two documents disagree on, for a failure message that
/// says WHAT was lost rather than only that the hashes differ.
fn first_differences(a: &GraphExport, b: &GraphExport) -> String {
    let key_n = |n: &reflow2_core::export::ExportedNode| format!("{} {}", n.node_type, n.node_id);
    let key_e = |e: &reflow2_core::export::ExportedEdge| {
        format!("{} {} -> {}", e.edge_type, e.from_id, e.to_id)
    };
    let as_map = |d: &GraphExport| -> BTreeMap<String, String> {
        d.nodes
            .iter()
            .map(|n| (key_n(n), serde_json::to_string(n).unwrap()))
            .chain(
                d.edges
                    .iter()
                    .map(|e| (key_e(e), serde_json::to_string(e).unwrap())),
            )
            .collect()
    };
    let (ma, mb) = (as_map(a), as_map(b));
    let mut out = Vec::new();
    for (k, v) in &ma {
        match mb.get(k) {
            None => out.push(format!("only in the first: {k}")),
            Some(w) if w != v => out.push(format!("differs: {k}\n  {v}\n  {w}")),
            _ => {}
        }
    }
    for k in mb.keys() {
        if !ma.contains_key(k) {
            out.push(format!("only in the second: {k}"));
        }
    }
    out.truncate(8);
    out.join("\n")
}

#[test]
fn reflow2s_own_design_round_trips_through_the_item_layout_with_nothing_lost() {
    let (doc, form, committed_hash) = committed();
    assert!(
        doc.nodes.len() > 1_000 && doc.edges.len() > 1_000,
        "this must run on the real design, not a stub: {} nodes, {} edges in the {form}",
        doc.nodes.len(),
        doc.edges.len()
    );

    // ① The committed record imports and exports back as itself: no node,
    // edge or property dropped or invented on the way through the store.
    let a = import(&doc);
    let ea = a.export_graph().expect("export");
    assert_eq!(
        ea.compute_content_hash(),
        committed_hash,
        "import → export of the committed {form} is not the committed design:\n{}",
        first_differences(&doc, &ea)
    );

    // ② Written as item files and read back, it is the same document — the
    // same items in the same order, and the same computed whole-design hash.
    let (files, written) = write(&ea, &BTreeMap::new());
    assert_eq!(
        files.len(),
        ea.nodes.len() + ea.edges.len(),
        "one file per node and per edge, no two items sharing a file"
    );
    assert_eq!(written, files.len());
    let items: Vec<ParsedItem> = files
        .iter()
        .map(|(rel, text)| item_layout::parse_item(rel, text.as_bytes()).expect("item parses"))
        .collect();
    let back = item_layout::assemble(
        DesignStamp {
            graph_id: ea.graph_id.clone(),
            schema_version: 1,
            migrated_from: None,
        },
        items,
    )
    .expect("the layout assembles");
    assert!(
        back.integrity_note().is_none(),
        "the layout the writer just wrote is not intact: {:?}",
        back.integrity_note()
    );
    assert_eq!(
        serde_json::to_string(&back.export.nodes).unwrap(),
        serde_json::to_string(&ea.nodes).unwrap(),
        "the nodes read back from the layout differ:\n{}",
        first_differences(&ea, &back.export)
    );
    assert_eq!(
        serde_json::to_string(&back.export.edges).unwrap(),
        serde_json::to_string(&ea.edges).unwrap(),
        "the edges read back from the layout differ:\n{}",
        first_differences(&ea, &back.export)
    );
    assert_eq!(back.export.compute_content_hash(), committed_hash);

    // ③ The layout imports into a fresh store, which exports the identical
    // design, and writing that over the layout changes not one file.
    let b = import(&back.export);
    let eb = b.export_graph().expect("export");
    assert_eq!(
        eb.compute_content_hash(),
        committed_hash,
        "layout → import → export lost or added something:\n{}",
        first_differences(&ea, &eb)
    );
    let (again, rewritten) = write(&eb, &files);
    assert_eq!(
        rewritten, 0,
        "an unchanged design re-exported over its layout writes nothing"
    );
    assert_eq!(again, files);

    // ④ Every accepted checksum comes back into the store, where reconcile
    // reads it — whether the record states it on the node or derives it from
    // the change that accepted it.
    let in_record = document_checksums(&doc);
    assert!(
        in_record.len() > 100,
        "the record holds artifact checksums ({} found) — otherwise this checks nothing",
        in_record.len()
    );
    let (sa, sb) = (stored_checksums(&a), stored_checksums(&b));
    assert_eq!(
        sa, in_record,
        "importing the committed record did not restore every accepted checksum"
    );
    assert_eq!(
        sb, in_record,
        "importing the layout did not restore every accepted checksum"
    );
}
