//! reflow2's derived relations are DECLARED, the declarations are held to the
//! code, and a served read evaluates them.
//!
//! `req:reflow2-declares-its-derived-relations-and-serves-a-read-that-runs-them`,
//! promoted by Anthony on 2026-09-29 ("go with the 7 ideas you'd promote").
//! Root cause it answers:
//! `fact:root-cause-no-tool-can-evaluate-reflow2s-derived-relations-because-reflow2-declares-none-of-them-2026-09-29`
//! — 23 derived relations, hand-written across ~15 files, declared nowhere.
//!
//! THE COUNTER-ARGUMENT THIS FILE EXISTS TO ANSWER
//! (`dec:idea-a-served-read-evaluates-every-derived-relation-reflow2-computes`):
//! declarations kept beside hand-written code are a second copy, honest only if
//! a test checks them against the code. So these tests read the named
//! functions' SOURCE, and fail when:
//!
//! - a named function does not exist;
//! - an edge the declaration says its rule reads appears in none of its
//!   functions (a declaration with no matching code);
//! - a function marked `dedicated` reads an edge the declaration does not list
//!   (code with no declaration);
//! - a `dedicated` function calls a helper that reads edges and no declaration
//!   of this relation, or of one it is computed over, names that helper (code
//!   the declaration cannot see);
//! - a declared edge has no reading, or its reading shares no primitive with
//!   the relation's own reading;
//! - the declaration file and the served read's evaluators name different
//!   relations, in either direction.
//!
//! WHAT THIS CANNOT CHECK: that a relation's RULE, as prose, is what the code
//! does beyond which edges it reads. Checking that needs the rule stated as
//! something a machine evaluates, which is the rule engine this design does
//! not yet have; the edge-level check is the part that can be made mechanical
//! today, and it is the part that drifts first (a new edge read in a helper).
//!
//! OBSERVED FAILING, 2026-09-29, on origin/main 72917f6 with only the new files
//! applied: see the PR. With the evaluators in place and one edge dropped from
//! a declaration, `every_declaration_matches_the_edges_its_code_reads` names
//! the relation and the edge.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use reflow2_core::DesignGraph;
use reflow2_core::derived::{
    DERIVED_COSTS, DERIVED_EVALUATORS, DerivedRelation, INFERENCE_KINDS, STATED_OVER_READINGS,
    declared_derived_relations,
};
use reflow2_core::foundation::core::{READING_FORMS, RELATION_PRIMITIVES, Reading, Value};
use reflow2_core::nodes::{edge, node};

fn graph() -> DesignGraph {
    DesignGraph::open_in_memory().expect("in-memory graph")
}

fn declarations() -> Vec<DerivedRelation> {
    declared_derived_relations().expect("schema/derived/relations.yaml parses")
}

// ─── The source scanner ─────────────────────────────────────────────────────

/// The core crate's source, blanked where it cannot name an edge: comments and
/// string literals are replaced by spaces, except a literal that is all capitals
/// and underscores, which may be an edge type written as a string.
struct Source {
    files: BTreeMap<String, String>,
    /// `edge::CONST` → the edge type it names.
    edge_consts: HashMap<String, String>,
    edge_types: BTreeSet<String>,
    /// fn name → files defining it.
    defs: HashMap<String, Vec<String>>,
}

fn blank(src: &str) -> String {
    let b = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    let spaces = |out: &mut String, s: &str| {
        for ch in s.chars() {
            out.push(if ch == '\n' { '\n' } else { ' ' });
        }
    };
    while i < b.len() {
        let rest = &src[i..];
        if rest.starts_with("//") {
            let end = rest.find('\n').unwrap_or(rest.len());
            spaces(&mut out, &rest[..end]);
            i += end;
        } else if let Some(after) = rest.strip_prefix("/*") {
            let end = after.find("*/").map_or(rest.len(), |e| e + 4);
            spaces(&mut out, &rest[..end]);
            i += end;
        } else if (rest.starts_with("r\"") || rest.starts_with("r#"))
            && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_'))
        {
            let hashes = rest[1..].chars().take_while(|c| *c == '#').count();
            if rest[1 + hashes..].starts_with('"') {
                let close = format!("\"{}", "#".repeat(hashes));
                let start = 2 + hashes;
                let end = rest[start..]
                    .find(&close)
                    .map_or(rest.len(), |e| start + e + close.len());
                spaces(&mut out, &rest[..end]);
                i += end;
            } else {
                out.push('r');
                i += 1;
            }
        } else if rest.starts_with('"') {
            let mut j = 1;
            let rb = rest.as_bytes();
            while j < rb.len() && rb[j] != b'"' {
                j += if rb[j] == b'\\' { 2 } else { 1 };
            }
            let lit = &rest[1..j.min(rest.len())];
            let keep = !lit.is_empty() && lit.bytes().all(|c| c.is_ascii_uppercase() || c == b'_');
            out.push('"');
            if keep {
                out.push_str(lit);
            } else {
                spaces(&mut out, lit);
            }
            out.push('"');
            i += (j + 1).min(rest.len());
        } else if rest.starts_with('\'') {
            // A char literal ('x', '\n', '{') — not a lifetime ('a).
            let rb = rest.as_bytes();
            let len = if rb.len() > 3 && rb[1] == b'\\' && rb[3] == b'\'' {
                4
            } else if rb.len() > 2 && rb[2] == b'\'' {
                3
            } else {
                0
            };
            if len > 0 {
                spaces(&mut out, &rest[..len]);
                i += len;
            } else {
                out.push('\'');
                i += 1;
            }
        } else {
            let ch = rest.chars().next().expect("non-empty");
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

impl Source {
    fn load(g: &DesignGraph) -> Self {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = BTreeMap::new();
        for entry in std::fs::read_dir(&dir).expect("read src/") {
            let p = entry.expect("dir entry").path();
            if p.extension().and_then(|e| e.to_str()) == Some("rs") {
                let name = p.file_name().unwrap().to_string_lossy().to_string();
                let raw = std::fs::read_to_string(&p).expect("read source");
                files.insert(name, blank(&raw));
            }
        }
        // `pub mod edge { pub const NAME: &str = "VALUE"; … }` in nodes.rs.
        let nodes_raw = std::fs::read_to_string(dir.join("nodes.rs")).expect("read nodes.rs");
        let edge_mod = &nodes_raw[nodes_raw.find("pub mod edge").expect("pub mod edge")..];
        let mut edge_consts = HashMap::new();
        for line in edge_mod.lines() {
            let l = line.trim();
            if let Some(rest) = l.strip_prefix("pub const ")
                && let (Some(colon), Some(q1)) = (rest.find(':'), rest.find('"'))
            {
                let name = rest[..colon].trim().to_string();
                let value: String = rest[q1 + 1..].chars().take_while(|c| *c != '"').collect();
                edge_consts.insert(name, value);
            }
            if l == "}" {
                break;
            }
        }
        let edge_types: BTreeSet<String> = g.schema().edge_types.keys().cloned().collect();
        let mut defs: HashMap<String, Vec<String>> = HashMap::new();
        for (f, s) in &files {
            let mut at = 0;
            while let Some(p) = s[at..].find("fn ") {
                let start = at + p;
                at = start + 3;
                if start > 0 && is_ident(s[..start].chars().last().unwrap()) {
                    continue;
                }
                let name: String = s[start + 3..]
                    .chars()
                    .take_while(|c| is_ident(*c))
                    .collect();
                if !name.is_empty() {
                    defs.entry(name).or_default().push(f.clone());
                }
            }
        }
        Source {
            files,
            edge_consts,
            edge_types,
            defs,
        }
    }

    /// The body of `fn name` in `file`, or None when there is no such fn with a body.
    fn body(&self, file: &str, name: &str) -> Option<&str> {
        let s = self.files.get(file)?;
        let needle = format!("fn {name}");
        let mut at = 0;
        while let Some(p) = s[at..].find(&needle) {
            let start = at + p;
            at = start + needle.len();
            let before_ok = start == 0 || !is_ident(s[..start].chars().last().unwrap());
            let after = s[start + needle.len()..].chars().next();
            if !before_ok || !matches!(after, Some('(') | Some('<')) {
                continue;
            }
            let sig_end = s[start..].find(['{', ';']).map(|e| start + e)?;
            if s.as_bytes()[sig_end] == b';' {
                continue;
            }
            let mut depth = 0usize;
            for (k, ch) in s[sig_end..].char_indices() {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(&s[sig_end + 1..sig_end + k]);
                        }
                    }
                    _ => {}
                }
            }
            return None;
        }
        None
    }

    /// The edge types a body names: `edge::CONST` and quoted edge-type literals.
    fn edges_in(&self, body: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut at = 0;
        while let Some(p) = body[at..].find("edge::") {
            let start = at + p + "edge::".len();
            at = start;
            let name: String = body[start..].chars().take_while(|c| is_ident(*c)).collect();
            if let Some(v) = self.edge_consts.get(&name) {
                out.insert(v.clone());
            }
        }
        for (k, _) in body.match_indices('"') {
            let lit: String = body[k + 1..].chars().take_while(|c| *c != '"').collect();
            if self.edge_types.contains(&lit) {
                out.insert(lit);
            }
        }
        out
    }

    /// The methods a body calls on `self`.
    fn self_calls(&self, body: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for (k, _) in body.match_indices("self.") {
            let name: String = body[k + 5..].chars().take_while(|c| is_ident(*c)).collect();
            let next = body[k + 5 + name.len()..].trim_start().chars().next();
            if !name.is_empty() && next == Some('(') {
                out.insert(name);
            }
        }
        out
    }
}

// ─── The declarations are well-formed ──────────────────────────────────────

fn primitives_of(r: &Reading) -> BTreeSet<String> {
    let mut s: BTreeSet<String> = r.components.iter().cloned().collect();
    if let Some(p) = &r.primitive {
        s.insert(p.clone());
    }
    s
}

#[test]
fn every_declaration_is_well_formed() {
    let g = graph();
    let decls = declarations();
    assert_eq!(decls.len(), 23, "the study's 23 relations are all declared");
    let ids: BTreeSet<&str> = decls.iter().map(|d| d.id.as_str()).collect();
    for d in &decls {
        let id = &d.id;
        assert!(
            READING_FORMS.contains(&d.reading.form.as_str()),
            "{id}: reading form {:?}",
            d.reading.form
        );
        assert_eq!(
            d.reading.form, "composite",
            "{id}: a derived relation is a composition"
        );
        assert!(
            d.reading.composition.is_some(),
            "{id}: composition is stated"
        );
        assert!(
            !d.reading.components.is_empty(),
            "{id}: components are listed"
        );
        for c in &d.reading.components {
            assert!(
                RELATION_PRIMITIVES.contains(&c.as_str()),
                "{id}: component {c:?} is not one of the sixteen primitives"
            );
        }
        assert!(
            STATED_OVER_READINGS.contains(&d.stated_over_readings.as_str()),
            "{id}: stated_over_readings {:?}",
            d.stated_over_readings
        );
        assert_eq!(
            d.not_yet_stated.is_some(),
            d.stated_over_readings != "full",
            "{id}: not_yet_stated says what is missing exactly when the rule is not fully stated"
        );
        assert!(
            INFERENCE_KINDS.contains(&d.inference.as_str()),
            "{id}: inference"
        );
        assert!(DERIVED_COSTS.contains(&d.cost.as_str()), "{id}: cost");
        assert!(!d.code.is_empty(), "{id}: names where the code computes it");
        assert_eq!(
            d.edges.is_empty(),
            d.stated_over_readings == "none",
            "{id}: a rule that reads no edge is stated over no reading, and only that one"
        );
        for o in &d.over {
            assert!(ids.contains(o.as_str()), "{id}: over {o:?} is not declared");
        }
        for e in &d.edges {
            assert!(
                g.schema().edge_types.contains_key(e),
                "{id}: edge {e} is not a schema edge type"
            );
        }
    }
}

#[test]
fn the_declarations_and_the_evaluators_name_the_same_relations() {
    let declared: BTreeSet<String> = declarations().into_iter().map(|d| d.id).collect();
    let evaluated: BTreeSet<String> = DERIVED_EVALUATORS.iter().map(|s| s.to_string()).collect();
    let undeclared: Vec<_> = evaluated.difference(&declared).collect();
    let unevaluated: Vec<_> = declared.difference(&evaluated).collect();
    assert!(
        undeclared.is_empty() && unevaluated.is_empty(),
        "evaluators with no declaration: {undeclared:?}; declarations with no evaluator: {unevaluated:?}"
    );
}

#[test]
fn every_declared_edge_is_read_in_primitives_the_relation_names() {
    let g = graph();
    for d in declarations() {
        let components = primitives_of(&d.reading);
        for e in &d.edges {
            let def = &g.schema().edge_types[e];
            let reading = def
                .reading
                .as_ref()
                .unwrap_or_else(|| panic!("{}: edge {e} has no reading", d.id));
            let mut prims = primitives_of(&reading.reading);
            for split in &reading.splits {
                for r in split.values.values() {
                    prims.extend(primitives_of(r));
                }
            }
            assert!(
                !prims.is_disjoint(&components),
                "{}: edge {e} reads as {prims:?}, which shares no primitive with the relation's own reading {components:?}",
                d.id
            );
        }
    }
}

// ─── The declarations are held to the code ────────────────────────────────

#[test]
fn every_declaration_matches_the_edges_its_code_reads() {
    let g = graph();
    let src = Source::load(&g);
    let decls = declarations();
    let by_id: HashMap<&str, &DerivedRelation> = decls.iter().map(|d| (d.id.as_str(), d)).collect();
    let mut problems = Vec::new();
    for d in &decls {
        let over: Vec<&DerivedRelation> = d.over.iter().map(|o| by_id[o.as_str()]).collect();
        let sites: BTreeSet<(String, String)> = d
            .code
            .iter()
            .chain(over.iter().flat_map(|o| o.code.iter()))
            .map(|c| (c.file.clone(), c.function.clone()))
            .collect();
        let mut allowed: BTreeSet<String> = d.edges.iter().cloned().collect();
        for o in &over {
            allowed.extend(o.edges.iter().cloned());
        }
        let mut read = BTreeSet::new();
        for c in &d.code {
            let Some(body) = src.body(&c.file, &c.function) else {
                problems.push(format!("{}: fn {} is not in {}", d.id, c.function, c.file));
                continue;
            };
            let edges = src.edges_in(body);
            read.extend(edges.iter().cloned());
            if !c.dedicated {
                continue;
            }
            for e in edges.difference(&allowed) {
                problems.push(format!(
                    "{}: {}::{} reads {e}, which the declaration does not list",
                    d.id, c.file, c.function
                ));
            }
            for call in src.self_calls(body) {
                for f in src.defs.get(&call).into_iter().flatten() {
                    let reads_edges = src
                        .body(f, &call)
                        .is_some_and(|b| !src.edges_in(b).is_empty());
                    if reads_edges && !sites.contains(&(f.clone(), call.clone())) {
                        problems.push(format!(
                            "{}: {}::{} calls {f}::{call}, which reads edges and no declaration here names",
                            d.id, c.file, c.function
                        ));
                    }
                }
            }
        }
        for e in d.edges.iter().filter(|e| !read.contains(*e)) {
            problems.push(format!(
                "{}: declares {e}, but none of its functions reads it",
                d.id
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "declarations out of step with the code:\n{}",
        problems.join("\n")
    );
}

/// The scanner itself: blanking keeps an edge literal, drops prose, and does
/// not mistake a lifetime for a char literal.
#[test]
fn the_scanner_reads_what_it_should() {
    let s = blank(
        "fn f<'a>(x: &'a str) { let y = \"REALIZES\"; let z = \"not { an edge\"; // edge::VERIFIES\n self.g(edge::SATISFIES) }",
    );
    assert!(s.contains("\"REALIZES\""));
    assert!(!s.contains("VERIFIES"));
    assert!(!s.contains("not"));
    assert!(s.contains("edge::SATISFIES"));
}

// ─── The served read counts what the code computes ────────────────────────

fn s(v: &str) -> Value {
    Value::String(v.to_string())
}

fn props(pairs: &[(&str, &str)]) -> HashMap<String, Value> {
    pairs.iter().map(|(k, v)| (k.to_string(), s(v))).collect()
}

fn fixture() -> DesignGraph {
    let mut g = graph();
    let n = |g: &mut DesignGraph, t: &str, id: &str, extra: &[(&str, &str)]| {
        let mut p = props(&[("name", id)]);
        p.extend(props(extra));
        g.create_node(t, id, p)
            .unwrap_or_else(|e| panic!("create {id}: {e}"));
    };
    n(&mut g, node::REQUIREMENT, "req:a", &[("statement", "a")]);
    n(&mut g, node::REQUIREMENT, "req:b", &[("statement", "b")]);
    for c in ["cap:a", "cap:b", "cap:c"] {
        n(&mut g, node::CAPABILITY, c, &[("description", c)]);
    }
    for k in ["cmp:x", "cmp:y"] {
        n(&mut g, node::COMPONENT, k, &[("purpose", k)]);
    }
    n(&mut g, node::ARTIFACT, "art:x", &[]);
    n(
        &mut g,
        node::VERIFICATION,
        "ver:a",
        &[("status", "passing")],
    );
    n(
        &mut g,
        node::DECISION,
        "dec:o",
        &[("decision", "retire cap:c"), ("status", "accepted")],
    );
    n(&mut g, node::INTERFACE, "ifc:i", &[]);
    let e = |g: &mut DesignGraph, t: &str, ft: &str, f: &str, tt: &str, to: &str| {
        g.create_edge(t, ft, f, tt, to, HashMap::new())
            .unwrap_or_else(|err| panic!("{t} {f}->{to}: {err}"));
    };
    e(
        &mut g,
        edge::REALIZES,
        node::ARTIFACT,
        "art:x",
        node::CAPABILITY,
        "cap:a",
    );
    e(
        &mut g,
        edge::REALIZES,
        node::ARTIFACT,
        "art:x",
        node::COMPONENT,
        "cmp:x",
    );
    e(
        &mut g,
        edge::VERIFIES,
        node::VERIFICATION,
        "ver:a",
        node::CAPABILITY,
        "cap:a",
    );
    e(
        &mut g,
        edge::SATISFIES,
        node::CAPABILITY,
        "cap:a",
        node::REQUIREMENT,
        "req:a",
    );
    e(
        &mut g,
        edge::SATISFIES,
        node::CAPABILITY,
        "cap:b",
        node::REQUIREMENT,
        "req:b",
    );
    e(
        &mut g,
        edge::ALLOCATED_TO,
        node::CAPABILITY,
        "cap:b",
        node::COMPONENT,
        "cmp:x",
    );
    e(
        &mut g,
        edge::OBSOLETES,
        node::DECISION,
        "dec:o",
        node::CAPABILITY,
        "cap:c",
    );
    e(
        &mut g,
        edge::PROVIDES,
        node::COMPONENT,
        "cmp:x",
        node::INTERFACE,
        "ifc:i",
    );
    e(
        &mut g,
        edge::CONSUMES,
        node::COMPONENT,
        "cmp:y",
        node::INTERFACE,
        "ifc:i",
    );
    e(
        &mut g,
        edge::DEPENDS_ON,
        node::COMPONENT,
        "cmp:y",
        node::COMPONENT,
        "cmp:x",
    );
    g
}

#[test]
fn the_served_read_counts_what_the_code_computes() {
    let g = fixture();
    let r = g.derived_report(&[], 3).expect("derived_report");
    let count = |id: &str| {
        r.relations
            .iter()
            .find(|t| t.id == id)
            .unwrap_or_else(|| panic!("{id} in the report"))
            .count
    };
    assert_eq!(r.declared, 23);
    assert_eq!(r.relations.len(), 23, "every declared relation is reported");

    // By hand: cap:a is realized directly, cap:b through cmp:x; cap:c is withdrawn.
    assert_eq!(count("realized"), Some(2));
    assert_eq!(count("checked"), Some(1), "only cap:a has a passing check");
    assert_eq!(
        count("delivered"),
        Some(1),
        "req:a; req:b's capability is realized but unchecked"
    );
    assert_eq!(count("discontinued"), Some(1));
    assert_eq!(count("contract_pair"), Some(1));
    assert_eq!(count("coupling"), Some(1));

    // And by the code path each declaration names.
    assert_eq!(
        count("delivered"),
        Some(g.delivery_coverage().unwrap().delivered)
    );
    assert_eq!(
        count("discontinued"),
        Some(g.discontinued_ids().unwrap().len())
    );
    let seams = g.seam_coverage(None).unwrap();
    assert_eq!(count("contract_pair"), Some(seams.declared));
    assert_eq!(count("coupling"), Some(seams.couplings));
    assert_eq!(
        count("rerun_owed"),
        Some(g.invalidated_findings().unwrap().len())
    );
    assert_eq!(
        count("level_mismatch"),
        Some(g.hierarchy_issues().unwrap().len())
    );

    // The closure is kept as a rule and not counted, and says why.
    let impact = r.relations.iter().find(|t| t.id == "impact").unwrap();
    assert_eq!(impact.count, None);
    assert!(impact.not_counted.is_some());
    assert_eq!(r.counted, 22);

    // Nothing is stored: reading twice changes nothing and asserted facts hold.
    let again = g.derived_report(&[], 3).expect("second read");
    assert_eq!(again.asserted_facts, r.asserted_facts);
    assert_eq!(again.derived_facts, r.derived_facts);
    assert!(r.asserted_facts > 0 && r.derived_per_asserted.is_some());
}

#[test]
fn a_relation_nobody_declared_is_refused_not_answered_empty() {
    let g = fixture();
    let err = g
        .derived_report(&["no_such_relation".to_string()], 3)
        .expect_err("an undeclared id is refused");
    assert!(err.to_string().contains("no_such_relation"));
    let one = g
        .derived_report(&["realized".to_string()], 3)
        .expect("a declared id narrows the read");
    assert_eq!(one.relations.len(), 1);
    assert_eq!(one.relations[0].count, Some(2));
}
