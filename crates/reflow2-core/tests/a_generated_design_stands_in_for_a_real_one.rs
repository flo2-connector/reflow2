//! A GENERATED DESIGN OF REALISTIC SIZE, made from the schema: what the tests
//! that need a LARGE, VARIED design read since the committed copy of reflow2's
//! own design retired (2026-10-30).
//!
//! Until then `the_committed_design_round_trips_through_the_item_layout.rs`
//! and `hydrate_cost_from_the_document_alone.rs` read `docs/design/reflow2/`,
//! because it was "the one design that has every node type, every property
//! shape and every odd id this project has ever written". The design is
//! hosted on flo2.io and private now, and a public repository does not carry
//! it. So this makes one that has at least as much of each:
//!
//! - **the size and the mix** of reflow2's own design on 2026-10-30: how many
//!   nodes of each type (8,500 in all), and the 61 (edge type, from, to)
//!   triples that carried 67,725 of its 70,189 edges, scaled by `scale`;
//! - **every node type the schema declares** (the real design used 24 of 28),
//!   with every declared property somewhere on its type;
//! - **every property shape** the schema allows: string, enum, bool, int,
//!   float and list, prose with quotes, newlines and non-ASCII text, a long
//!   value, and `null` on a property that may be null;
//! - **every edge type** on endpoints the schema allows, every declared edge
//!   property somewhere;
//! - **odd ids**: mixed case, a pair differing only in case, non-ASCII, an
//!   emoji, spaces, slashes, a backslash, `%` and `#`, and ids longer than a
//!   file name may be, which the item layout must escape or hash;
//! - **accepted checksums carried by the change that accepted them**
//!   (`checksum_after` on CHANGED edges into Artifacts), which an import
//!   derives back onto the Artifact.
//!
//! What it cannot keep is MEANING: its text is generated. A test that needs
//! what reflow2's design SAYS has no business on this; none of its readers do.
//!
//! The generator is deterministic: one `scale`, one document, byte for byte.
//! `tools/generated_design.py` is the Python twin the surface gates seed a
//! server from; both read the same mix.

use std::collections::{BTreeMap, BTreeSet};

use reflow2_core::foundation::core::{EdgeEndpoint, PropertyDef, PropertyType};
use reflow2_core::schema::parsed_schema;
use reflow2_core::{DesignGraph, GraphExport};
use serde_json::{Map, Value as J, json};

/// Nodes of each type in reflow2's own design on 2026-10-30. A type the schema
/// declares and this does not name gets [`FLOOR`].
const NODE_MIX: &[(&str, usize)] = &[
    ("ChangeEvent", 1761),
    ("TemporalFact", 1111),
    ("Decision", 1066),
    ("Snapshot", 928),
    ("Artifact", 811),
    ("Fragment", 542),
    ("Verification", 534),
    ("Capability", 491),
    ("Requirement", 484),
    ("DesignEpoch", 338),
    ("Release", 110),
    ("Component", 110),
    ("Question", 65),
    ("DriftEvent", 40),
    ("DesignRule", 36),
    ("Interface", 23),
    ("Resource", 20),
    ("Contributor", 9),
    ("Constraint", 9),
    ("Actor", 3),
    ("Environment", 3),
    ("Inference", 3),
    ("Project", 1),
    ("Flow", 1),
];
const FLOOR: usize = 3;

/// The (edge type, from, to) triples that carried 67,725 of its 70,189 edges.
const EDGE_MIX: &[(&str, &str, &str, usize)] = &[
    ("INCLUDES", "Release", "Artifact", 37457),
    ("INCLUDES", "Release", "Component", 9034),
    ("CHANGED", "ChangeEvent", "Artifact", 5319),
    ("AT_EPOCH", "ChangeEvent", "DesignEpoch", 1574),
    ("AUTHORED_BY", "Decision", "Contributor", 992),
    ("REALIZES", "Artifact", "Capability", 988),
    ("AT_EPOCH", "Snapshot", "DesignEpoch", 916),
    ("GOVERNED_BY", "Capability", "Decision", 812),
    ("VERIFIES", "Verification", "Capability", 634),
    ("CHANGED", "ChangeEvent", "Capability", 629),
    ("SATISFIES", "Capability", "Requirement", 539),
    ("YIELDED", "Fragment", "Artifact", 528),
    ("AUTHORED_BY", "Artifact", "Contributor", 454),
    ("HAS_TEMPORAL_FACT", "Capability", "TemporalFact", 437),
    ("AUTHORED_BY", "ChangeEvent", "Contributor", 428),
    ("ALLOCATED_TO", "Capability", "Component", 415),
    ("AUTHORED_BY", "Requirement", "Contributor", 396),
    ("IMPLEMENTS", "Artifact", "Verification", 339),
    ("CHANGED", "ChangeEvent", "Verification", 327),
    ("DEPLOYED_TO", "Release", "Environment", 267),
    ("GOVERNED_BY", "ChangeEvent", "Decision", 264),
    ("HAS_SNAPSHOT", "Decision", "Snapshot", 251),
    ("INVALIDATES", "ChangeEvent", "TemporalFact", 235),
    ("CHANGED", "ChangeEvent", "Decision", 214),
    ("CHANGED", "ChangeEvent", "Requirement", 210),
    ("CHANGED", "ChangeEvent", "Component", 185),
    ("GOVERNED_BY", "Requirement", "Decision", 184),
    ("HAS_SNAPSHOT", "TemporalFact", "Snapshot", 178),
    ("DEPENDS_ON", "Decision", "Decision", 174),
    ("HAS_SNAPSHOT", "Capability", "Snapshot", 172),
    ("ANTICIPATES", "Decision", "Decision", 155),
    ("AUTHORED_BY", "Capability", "Contributor", 153),
    ("EVOLVES_INTO", "Decision", "Requirement", 153),
    ("CAUSES", "Artifact", "TemporalFact", 151),
    ("HAS_TEMPORAL_FACT", "Component", "TemporalFact", 150),
    ("PRECEDES", "DesignEpoch", "DesignEpoch", 148),
    ("HAS_TEMPORAL_FACT", "Decision", "TemporalFact", 146),
    ("AUTHORED_BY", "TemporalFact", "Contributor", 141),
    ("HAS_TEMPORAL_FACT", "Project", "TemporalFact", 125),
    ("REALIZES", "Artifact", "Component", 125),
    ("AUTHORED_BY", "Verification", "Contributor", 111),
    ("AT_EPOCH", "Release", "DesignEpoch", 110),
    ("SCHEDULED_FOR", "Requirement", "DesignEpoch", 103),
    ("GOVERNED_BY", "Component", "Decision", 98),
    ("HAS_TEMPORAL_FACT", "Requirement", "TemporalFact", 98),
    ("CONTAINS", "Component", "Component", 97),
    ("HAS_SNAPSHOT", "Requirement", "Snapshot", 91),
    ("MITIGATES", "Decision", "TemporalFact", 89),
    ("DEPENDS_ON", "Requirement", "Requirement", 86),
    ("ASKS_ABOUT", "Question", "Capability", 83),
    ("ABOUT_ENTITY", "TemporalFact", "Component", 77),
    ("HAS_TEMPORAL_FACT", "Artifact", "TemporalFact", 76),
    ("VALID_FROM", "TemporalFact", "DesignEpoch", 76),
    ("DEPENDS_ON", "Component", "Component", 72),
    ("CONTAINS", "Project", "Requirement", 68),
    ("HAS_SNAPSHOT", "Artifact", "Snapshot", 68),
    ("CHANGED", "ChangeEvent", "TemporalFact", 67),
    ("DEPENDS_ON", "Decision", "Requirement", 67),
    ("EVOLVES_INTO", "Decision", "Decision", 64),
    ("EVOLVES_INTO", "Decision", "Capability", 64),
    ("HAS_SNAPSHOT", "Release", "Snapshot", 61),
];
/// Every edge type the schema declares gets at least this many.
const EDGE_FLOOR: usize = 4;

/// The CHANGED edge's acceptance of a file: set only on a change INTO an
/// Artifact, where the import derives the Artifact's checksum from it.
const ACCEPTANCE: &[&str] = &["checksum_after", "accepted_seq", "checksum_basis"];

/// SplitMix64: a deterministic stream with no dependency.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }
}

const WORDS: &[&str] = &[
    "design",
    "record",
    "change",
    "review",
    "owner",
    "reason",
    "file",
    "check",
    "build",
    "release",
    "store",
    "query",
    "export",
    "import",
    "edge",
    "node",
    "budget",
    "margin",
    "epoch",
    "lineage",
    "merge",
    "layout",
    "schema",
    "surface",
    "reply",
    "bound",
    "gate",
    "trial",
    "finding",
    "cause",
    "fix",
    "écart",
    "Grenzwert",
    "\"quoted\"",
    "it's",
];

fn prose(r: &mut Rng, n: usize) -> String {
    let len = 6 + r.below(40);
    let mut words: Vec<&str> = (0..len).map(|_| WORDS[r.below(WORDS.len())]).collect();
    if n.is_multiple_of(11) {
        words.push("\nA second line,\twith a tab.");
    }
    format!("{} ({n}).", words.join(" "))
}

fn value(r: &mut Rng, prop: &str, spec: &PropertyDef, i: usize) -> J {
    match spec.prop_type {
        PropertyType::Enum => {
            let values = spec.values.clone().unwrap_or_else(|| vec!["x".into()]);
            json!(values[i % values.len()])
        }
        PropertyType::Bool => json!(i.is_multiple_of(2)),
        PropertyType::Int => {
            let (lo, hi) = spec.range.unwrap_or((0.0, 1000.0));
            let span = ((hi - lo) as i64).max(0) + 1;
            json!(lo as i64 + (i as i64 % span))
        }
        PropertyType::Float => {
            let (lo, hi) = spec.range.unwrap_or((0.0, 100.0));
            json!(lo + (hi - lo) * ((i % 97) as f64 / 96.0))
        }
        PropertyType::ListString => J::Array(
            (0..=(i % 3))
                .map(|k| json!(format!("{prop}-{i}-{k}")))
                .collect(),
        ),
        PropertyType::Datetime => json!(format!(
            "2026-{:02}-{:02}T00:00:00Z",
            1 + i % 12,
            1 + i % 28
        )),
        PropertyType::String if spec.fulltext => {
            if i == 1 {
                // One long value per property: the shape a long rationale has.
                json!(prose(r, i).repeat(60))
            } else {
                json!(prose(r, i))
            }
        }
        // `PropertyType` is non-exhaustive: a kind added later reads as a short string.
        _ => json!(format!("{prop} {i}")),
    }
}

fn prefix(node_type: &str) -> String {
    reflow2_core::nodes::node::ID_PREFIXES
        .iter()
        .find(|(_, t)| *t == node_type)
        .map(|(p, _)| p.to_string())
        .unwrap_or_else(|| node_type.to_lowercase())
}

/// The odd ids every type also carries.
fn odd_ids(p: &str) -> Vec<String> {
    vec![
        format!("{p}:Mixed.Case_ID-1"),
        format!("{p}:case"),
        format!("{p}:CASE"),
        format!("{p}:ünïcödé-ñame"),
        format!("{p}:emoji-🚀"),
        format!("{p}:spaces and/slashes\\back%percent#hash"),
        format!("{p}:{}", "long-segment-".repeat(20)),
        format!("{p}:{}", "ü".repeat(90)),
    ]
}

fn endpoint_types(e: &EdgeEndpoint, all: &[String]) -> Vec<String> {
    all.iter().filter(|t| e.accepts(t)).cloned().collect()
}

/// The generated design as a document, ready to import.
pub fn generated_design(scale: f64) -> GraphExport {
    let schema = parsed_schema().expect("the schema parses");
    let mut r = Rng(7);
    let all: Vec<String> = schema
        .node_types
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mix: BTreeMap<&str, usize> = NODE_MIX.iter().copied().collect();

    let mut nodes: Vec<J> = Vec::new();
    let mut by_type: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut null_placed = false;
    for t in &all {
        let def = &schema.node_types[t];
        let props: BTreeMap<&String, &PropertyDef> = def.properties.iter().collect();
        let count = match mix.get(t.as_str()) {
            _ if t == "Project" => 1,
            Some(n) => ((*n as f64 * scale).round() as usize).max(1),
            None => FLOOR,
        };
        let p = prefix(t);
        let mut ids: Vec<String> = (0..count)
            .map(|i| format!("{p}:{}-{i:05}", t.to_lowercase()))
            .collect();
        if t != "Project" {
            ids.extend(odd_ids(&p));
        }
        for (i, id) in ids.iter().enumerate() {
            let mut body = Map::new();
            for (name, spec) in &props {
                // The first node of a type carries EVERY property, so each is
                // somewhere; the rest carry what they must and about half the rest.
                if spec.required || i == 0 || r.chance(50) {
                    let v = if !null_placed && spec.nullable && !spec.required {
                        null_placed = true;
                        J::Null
                    } else {
                        value(&mut r, name, spec, i)
                    };
                    body.insert((*name).clone(), v);
                }
            }
            if t == "Contributor" && i.is_multiple_of(3) && props.contains_key(&"kind".to_string())
            {
                body.insert("kind".into(), json!("person"));
            }
            nodes.push(json!({"node_type": t, "node_id": id, "properties": body}));
        }
        by_type.insert(t.clone(), ids);
    }

    let mut edges: Vec<J> = Vec::new();
    let mut seen: BTreeSet<(String, String, String)> = BTreeSet::new();
    let mut seq: i64 = 0;
    // REASONING STAYS GROUNDED: an Inference concludes exactly one node, and a
    // premise may not lead back to itself. Every premise is kept apart from
    // every conclusion, so no chain drawn here can close a circle.
    let mut concluding: BTreeSet<String> = BTreeSet::new();
    let mut conclusions: BTreeSet<String> = BTreeSet::new();
    let mut premises: BTreeSet<String> = BTreeSet::new();
    let mut add = |r: &mut Rng, et: &str, from: &str, to: &str, to_type: &str| -> bool {
        if from == to || seen.contains(&(et.to_string(), from.to_string(), to.to_string())) {
            return false;
        }
        match et {
            "CONCLUDES" if concluding.contains(from) || premises.contains(to) => return false,
            "CONCLUDES" => {
                concluding.insert(from.into());
                conclusions.insert(to.into());
            }
            "PREMISE_OF" if conclusions.contains(from) => return false,
            "PREMISE_OF" => {
                premises.insert(from.into());
            }
            _ => {}
        }
        seen.insert((et.into(), from.into(), to.into()));
        let def = &schema.edge_types[et];
        let mut props = Map::new();
        let n = seen.len();
        let declared: BTreeMap<&String, &PropertyDef> = def.properties.iter().collect();
        for (name, spec) in &declared {
            if et == "CHANGED" && ACCEPTANCE.contains(&name.as_str()) {
                continue;
            }
            if spec.required || n.is_multiple_of(7) || r.chance(30) {
                props.insert((*name).clone(), value(r, name, spec, n));
            }
        }
        if et == "CHANGED" && to_type == "Artifact" && r.chance(50) {
            seq += 1;
            props.insert("checksum_after".into(), json!(format!("sha256:{seq:064x}")));
            props.insert("accepted_seq".into(), json!(seq));
            if let Some(basis) = declared
                .get(&"checksum_basis".to_string())
                .and_then(|s| s.values.clone())
            {
                props.insert(
                    "checksum_basis".into(),
                    json!(basis[seq as usize % basis.len()]),
                );
            }
        }
        edges.push(json!({"edge_type": et, "from_id": from, "to_id": to, "properties": props}));
        true
    };
    for (et, ft, tt, n) in EDGE_MIX {
        let (Some(froms), Some(tos)) = (by_type.get(*ft), by_type.get(*tt)) else {
            continue;
        };
        if !schema.edge_types.contains_key(*et) {
            continue;
        }
        // A pair drawn twice is skipped, so draw until the count is met or the
        // pairs run thin (four tries an edge), as the Python twin does.
        let want = ((*n as f64 * scale).round() as usize).max(1);
        let (mut made, mut tries) = (0, 0);
        while made < want && tries < want * 4 {
            tries += 1;
            let (a, b) = (
                froms[r.below(froms.len())].clone(),
                tos[r.below(tos.len())].clone(),
            );
            if add(&mut r, et, &a, &b, tt) {
                made += 1;
            }
        }
    }
    let edge_names: BTreeSet<&String> = schema.edge_types.keys().collect();
    for et in edge_names {
        let def = &schema.edge_types[et];
        let froms = endpoint_types(&def.from, &all);
        let tos = endpoint_types(&def.to, &all);
        if froms.is_empty() || tos.is_empty() {
            continue;
        }
        for k in 0..EDGE_FLOOR {
            let (ft, tt) = (&froms[k % froms.len()], &tos[(k * 7) % tos.len()]);
            let (fs, ts) = (&by_type[ft], &by_type[tt]);
            let (a, b) = (fs[r.below(fs.len())].clone(), ts[r.below(ts.len())].clone());
            add(&mut r, et, &a, &b, tt);
        }
    }
    serde_json::from_value(json!({"graph_id": "generated", "nodes": nodes, "edges": edges}))
        .expect("the generated design is a design document")
}

/// The generated design as an EXPORT written by reflow2: generated, imported
/// into an in-memory store and exported, so it carries every default the
/// import materializes and reads exactly as a saved design reads.
pub fn generated_export(scale: f64) -> GraphExport {
    let doc = generated_design(scale);
    let mut g = DesignGraph::open_in_memory_as(&doc.graph_id).expect("in-memory graph");
    g.import_graph(&doc).expect("the generated design imports");
    g.export_graph().expect("export")
}

#[test]
fn the_generated_design_has_every_type_every_shape_and_every_edge_type() {
    let schema = parsed_schema().expect("schema");
    let doc = generated_design(0.05);
    let types: BTreeSet<&str> = doc.nodes.iter().map(|n| n.node_type.as_str()).collect();
    for t in schema.node_types.keys() {
        assert!(types.contains(t.as_str()), "no {t} node");
        let carried: BTreeSet<&String> = doc
            .nodes
            .iter()
            .filter(|n| &n.node_type == t)
            .flat_map(|n| n.properties.keys())
            .collect();
        for p in schema.node_types[t].properties.keys() {
            assert!(carried.contains(p), "no {t} carries {p}");
        }
    }
    let edge_types: BTreeSet<&str> = doc.edges.iter().map(|e| e.edge_type.as_str()).collect();
    let missing: Vec<&String> = schema
        .edge_types
        .keys()
        .filter(|e| !edge_types.contains(e.as_str()))
        .collect();
    assert!(missing.is_empty(), "edge types with no edge: {missing:?}");
    let shapes: BTreeSet<&str> = doc
        .nodes
        .iter()
        .flat_map(|n| n.properties.values())
        .map(|v| match v {
            reflow2_core::Value::Null => "null",
            reflow2_core::Value::Bool(_) => "bool",
            reflow2_core::Value::Int(_) => "int",
            reflow2_core::Value::Float(_) => "float",
            reflow2_core::Value::String(_) => "string",
            reflow2_core::Value::List(_) => "list",
            reflow2_core::Value::Map(_) => "map",
            _ => "other",
        })
        .collect();
    for shape in ["bool", "int", "float", "string", "list"] {
        assert!(shapes.contains(shape), "no {shape} value: {shapes:?}");
    }
    assert!(
        doc.nodes.iter().any(|n| n.node_id.len() > 150)
            && doc.nodes.iter().any(|n| !n.node_id.is_ascii()),
        "the odd ids are there"
    );
    // And it is a design reflow2 accepts: every node and edge validates.
    let mut g = DesignGraph::open_in_memory_as("generated").expect("graph");
    g.import_graph(&doc).expect("the generated design imports");
}

#[test]
fn the_generated_design_is_the_size_of_the_real_one() {
    let doc = generated_design(1.0);
    assert!(
        doc.nodes.len() > 8_000 && doc.edges.len() > 60_000,
        "{} nodes, {} edges",
        doc.nodes.len(),
        doc.edges.len()
    );
    // Deterministic: the same document twice.
    let again = generated_design(1.0);
    assert_eq!(
        doc.compute_content_hash(),
        again.compute_content_hash(),
        "one scale, one document"
    );
}
