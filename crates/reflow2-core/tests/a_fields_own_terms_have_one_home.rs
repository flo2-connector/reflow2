//! A field's own words for reflow2's types — the TERM TABLE, held to its rules.
//!
//! `req:the-vocabulary-is-served-in-a-fields-own-terms` (accepted 2026-10-14):
//! every node type and edge type may carry an "also called" list per field,
//! beside its hint in `schema/*.yaml`, and each term says what the mapping
//! means and does not mean. Mathematics is filled first.
//!
//! # Why the table is a test and not a file somebody keeps
//!
//! The objection recorded against a term table
//! (`dec:idea-the-term-table-is-the-residue-of-a-capture-procedure-not-a-maintained-artifact`)
//! is that a stored mapping keeps an early mistake where nobody revisits it.
//! These terms live in the code, so a change to one is a reviewed diff here,
//! against [`MATHEMATICS`], and the rules below fail the build:
//!
//! - ONE TERM, ONE HOME WITHIN A FIELD — two types claiming one field's word
//!   would make "where does a lemma go?" a coin toss;
//! - every term says what it MEANS;
//! - every term sits on a type that is served with it;
//! - no term is spelled as a type's own name, so finding a type by a field's
//!   word can never shadow the type a caller named.
//!
//! These tests read the schema and the served vocabulary as JSON, so they ran
//! (and failed) on main before the table existed — a check over nothing is
//! refused rather than passed.

use std::collections::{BTreeMap, BTreeSet};

use reflow2_core::DesignGraph;
use serde_json::Value;

/// THE MATHEMATICS TABLE: term → the type that holds it. Reviewed against the
/// schema on 2026-10-14; a change here is a change to what an agent is told.
///
/// - Requirement holds what is to be SHOWN: theorem, proposition, lemma,
///   corollary, and a conjecture (one not yet shown).
/// - Constraint holds what is TAKEN: hypothesis, axiom, definition.
/// - Decision holds the choice of method: proof strategy, approach.
/// - Artifact holds the cited source: cited result, reference.
/// - CONTRADICTS joins a counterexample to what it refutes.
/// - Inference holds a step and the method it applies: the methods of proof
///   from "Mathematical proof" (Wikipedia, "Methods of proof"), each as the
///   step it is recorded as, and the two-column proof as rows of steps.
/// - PREMISE_OF joins a premise to its step. CONCLUDES carries no term (see
///   NO_MATHEMATICS_TERM).
const MATHEMATICS: &[(&str, &str)] = &[
    ("approach", "Decision"),
    ("axiom", "Constraint"),
    ("case analysis", "Inference"),
    ("cited result", "Artifact"),
    ("conjecture", "Requirement"),
    ("contraposition", "Inference"),
    ("corollary", "Requirement"),
    ("counterexample", "CONTRADICTS"),
    ("definition", "Constraint"),
    ("direct proof", "Inference"),
    ("follows from", "Inference"),
    ("hypothesis", "Constraint"),
    ("inference rule", "Inference"),
    ("inference step", "Inference"),
    ("lemma", "Requirement"),
    ("mathematical induction", "Inference"),
    ("modus ponens", "Inference"),
    ("premise", "PREMISE_OF"),
    ("probabilistic method", "Inference"),
    ("proof by construction", "Inference"),
    ("proof by contradiction", "Inference"),
    ("proof by double counting", "Inference"),
    ("proof by exhaustion", "Inference"),
    ("proof by induction", "Inference"),
    ("proof step", "Inference"),
    ("proof strategy", "Decision"),
    ("proposition", "Requirement"),
    ("reference", "Artifact"),
    ("refutes", "CONTRADICTS"),
    ("rule of inference", "Inference"),
    ("theorem", "Requirement"),
    ("two-column proof", "Inference"),
];

/// Judged and LEFT OUT of mathematics, each for a reason:
///
/// - `assumption`: `req:assumptions-are-first-class` keeps open whether an
///   assumption is its own type; a term here would decide it by the back door.
/// - `given`: in mathematical prose it is a participle ("given x > 0"), and
///   its noun ("the givens") is what `hypothesis` already names. In ordinary
///   English it is a preposition — the find_tools corpus asks "when can this
///   release ship GIVEN the technology maturity" — so as an index term it would
///   route ordinary sentences to add_constraint.
/// - `used in`: offered for PREMISE_OF, but it is ordinary English ("used in
///   production") and names no mathematical relation; `premise` is the word.
/// - `combinatorial proof`: the article uses it for double counting, and
///   Hughes's combinatorial proofs are a different formalism — one word, two
///   meanings.
/// - `contradiction`, `construction`, `double counting` and `induction` BARE:
///   each also means something else a design says — two claims in conflict
///   (which is CONTRADICTS), a building, a cost counted twice, an induction
///   motor, heating coil or loop — so each method is named whole, `proof by …`
///   (and `mathematical induction`). Measured 2026-10-15 on the replica:
///   "avoid double counting the mass in the budget" put add_inference above
///   budget_report, and "record the induction motor power budget" put it first.
///   The corpus's `a_fields_terms_misroute_nothing` guards hold the induction
///   cases.
/// - `therefore` and `hence`: offered for CONCLUDES, and ordinary connectives
///   in an engineer's sentence. Measured 2026-10-15 on the live binary: "the
///   load is high, therefore add a requirement for cooling" ranked
///   add_inference first and add_requirement second, and "the bus is shared,
///   hence add an interface for arbitration" ranked add_inference first and
///   add_interface second. Without the terms add_requirement and add_interface
///   rank first; both sentences are corpus guards. The noun `conclusion`
///   misroutes the same way ("record the conclusion of the review").
const LEFT_OUT_OF_MATHEMATICS: &[&str] = &[
    "assumption",
    "given",
    "used in",
    "combinatorial proof",
    "contradiction",
    "construction",
    "double counting",
    "induction",
    "therefore",
    "hence",
    "conclusion",
];

/// Types that carry NO mathematics term, by judgement rather than oversight:
///
/// - `DEPENDS_ON` is functional need ("the source needs the target to
///   function"), drawn between capabilities and components. A statement
///   resting on a lemma is a premise relation, not a functional dependency,
///   and the Inference record is where a premise goes.
/// - `DECOMPOSES` is the nearest to a case split — satisfying every child
///   satisfies the parent — but a proof by cases is an INFERENCE whose
///   exhaustiveness is itself a premise, and DECOMPOSES has no place for that
///   premise or for the rule. `case analysis` and `proof by exhaustion` are on
///   Inference, where the rule and that premise can be said.
/// - `CONCLUDES`: its words ("therefore", "hence", "conclusion") are everyday
///   connectives that took engineers' sentences (see LEFT_OUT_OF_MATHEMATICS);
///   "follows from" already finds the Inference whose conclusion it draws.
const NO_MATHEMATICS_TERM: &[&str] = &["DEPENDS_ON", "DECOMPOSES", "CONCLUDES"];

/// One declared term: (field, term, home type, means).
type Declared = (String, String, String, String);

fn schema_json() -> Value {
    serde_json::to_value(reflow2_core::schema::load_schema().expect("schema loads"))
        .expect("schema serializes")
}

/// Every term the merged schema declares, from node types and edge types.
fn declared() -> Vec<Declared> {
    let schema = schema_json();
    let mut out = Vec::new();
    for kind in ["node_types", "edge_types"] {
        for (ty, def) in schema[kind].as_object().expect(kind) {
            let Some(fields) = def.get("also_called").and_then(Value::as_object) else {
                continue;
            };
            for (field, terms) in fields {
                for t in terms.as_array().expect("a field's terms are a list") {
                    out.push((
                        field.clone(),
                        t["term"].as_str().unwrap_or_default().to_string(),
                        ty.clone(),
                        t["means"].as_str().unwrap_or_default().to_string(),
                    ));
                }
            }
        }
    }
    out.sort();
    out
}

/// How a term is matched: case and runs of whitespace do not count, and an
/// underscore reads as a space, so `PREMISE_OF` and "premise of" compare alike.
fn norm(s: &str) -> String {
    s.replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn nonempty(all: &[Declared]) {
    assert!(
        !all.is_empty(),
        "the schema declares no field's term at all, so this rule held over nothing — \
         req:the-vocabulary-is-served-in-a-fields-own-terms asks for mathematics first"
    );
}

#[test]
fn the_mathematics_terms_are_the_reviewed_table() {
    let got: BTreeSet<(String, String)> = declared()
        .into_iter()
        .filter(|d| d.0 == "mathematics")
        .map(|d| (d.1, d.2))
        .collect();
    let want: BTreeSet<(String, String)> = MATHEMATICS
        .iter()
        .map(|(t, ty)| (t.to_string(), ty.to_string()))
        .collect();
    let missing: Vec<_> = want.difference(&got).collect();
    let extra: Vec<_> = got.difference(&want).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "the schema's mathematics terms differ from the reviewed table.\n  missing: \
         {missing:?}\n  not in the table: {extra:?}\nA term is what an agent is told a \
         field's word means: change the table here in the same pull request, so the \
         change is reviewed."
    );
}

#[test]
fn one_term_has_one_home_within_a_field() {
    let all = declared();
    nonempty(&all);
    let mut homes: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    for (field, term, ty, _) in &all {
        homes
            .entry((field.clone(), norm(term)))
            .or_default()
            .insert(ty.clone());
    }
    let twice: Vec<_> = homes.iter().filter(|(_, tys)| tys.len() > 1).collect();
    assert!(
        twice.is_empty(),
        "a field's word has more than one home, so an agent holding it cannot tell \
         where it goes: {twice:?}. A term that fits two types badly goes on neither."
    );
    // The same word twice on ONE type is a copy-paste, and the second copy's
    // `means` would contradict or repeat the first.
    let mut seen = BTreeSet::new();
    for (field, term, ty, _) in &all {
        assert!(
            seen.insert((field.clone(), norm(term), ty.clone())),
            "{ty} lists the {field} term {term:?} twice"
        );
    }
}

#[test]
fn every_term_says_what_it_means() {
    let all = declared();
    nonempty(&all);
    let bare: Vec<_> = all
        .iter()
        .filter(|(_, term, _, means)| {
            let m = means.trim();
            m.is_empty() || norm(m) == norm(term) || !m.contains(' ')
        })
        .map(|(f, t, ty, _)| format!("{f}/{t} on {ty}"))
        .collect();
    assert!(
        bare.is_empty(),
        "every term must say what the mapping means AND does not mean, in a sentence: {bare:?}"
    );
    let blank: Vec<_> = all
        .iter()
        .filter(|(f, t, _, _)| f.trim().is_empty() || t.trim().is_empty())
        .collect();
    assert!(blank.is_empty(), "a blank field or term: {blank:?}");
}

/// Every term is served ON its type's own answer: the vocabulary an agent
/// reads carries it beside the hint, on a type that exists.
#[test]
fn every_term_is_served_on_its_types_answer() {
    let all = declared();
    nonempty(&all);
    let g = DesignGraph::open_in_memory().expect("graph");
    let vocab = serde_json::to_value(g.describe_vocabulary()).expect("vocabulary");
    let mut served: BTreeSet<(String, String, String)> = BTreeSet::new();
    for (list, key) in [("node_types", "node_type"), ("edge_types", "edge_type")] {
        for spec in vocab[list].as_array().expect(list) {
            let ty = spec[key].as_str().expect("type name").to_string();
            let Some(fields) = spec.get("also_called").and_then(Value::as_object) else {
                continue;
            };
            for (field, terms) in fields {
                for t in terms.as_array().expect("list") {
                    served.insert((
                        field.clone(),
                        t["term"].as_str().unwrap_or_default().to_string(),
                        ty.clone(),
                    ));
                }
            }
        }
    }
    let unserved: Vec<_> = all
        .iter()
        .filter(|(f, t, ty, _)| !served.contains(&(f.clone(), t.clone(), ty.clone())))
        .map(|(f, t, ty, _)| format!("{f}/{t} on {ty}"))
        .collect();
    assert!(
        unserved.is_empty(),
        "declared in the schema but not served on the type's answer: {unserved:?}"
    );
}

/// A term spelled like a type's own name would make "find a type by a
/// field's word" ambiguous with "describe this type", and the lookup could
/// shadow a name a caller meant.
#[test]
fn no_term_is_spelled_as_a_types_own_name() {
    let all = declared();
    nonempty(&all);
    let schema = schema_json();
    let names: BTreeSet<String> = ["node_types", "edge_types"]
        .iter()
        .flat_map(|k| schema[*k].as_object().expect(k).keys().map(|n| norm(n)))
        .collect();
    let clash: Vec<_> = all
        .iter()
        .filter(|(_, t, _, _)| names.contains(&norm(t)))
        .collect();
    assert!(
        clash.is_empty(),
        "a term spelled as a type's name: {clash:?}"
    );
}

#[test]
fn what_was_judged_out_stays_out() {
    let all = declared();
    nonempty(&all);
    for word in LEFT_OUT_OF_MATHEMATICS {
        let homes: Vec<_> = all
            .iter()
            .filter(|(f, t, _, _)| f == "mathematics" && norm(t) == *word)
            .collect();
        assert!(
            homes.is_empty(),
            "{word:?} was judged out of mathematics (see LEFT_OUT_OF_MATHEMATICS) and \
             has a home: {homes:?}"
        );
    }
    for ty in NO_MATHEMATICS_TERM {
        let on: Vec<_> = all
            .iter()
            .filter(|(f, _, home, _)| f == "mathematics" && home == ty)
            .collect();
        assert!(
            on.is_empty(),
            "{ty} was judged to carry no mathematics term (see NO_MATHEMATICS_TERM): {on:?}"
        );
    }
}

/// A field name is one of the declared fields: `mathmatics` is a typo the
/// table must not quietly grow a second field from.
#[test]
fn every_terms_field_is_a_declared_field() {
    let all = declared();
    nonempty(&all);
    let known = reflow2_core::foundation::core::TERM_FIELDS;
    let unknown: Vec<_> = all
        .iter()
        .filter(|(f, ..)| !known.contains(&f.as_str()))
        .map(|(f, t, ty, _)| format!("{f}/{t} on {ty}"))
        .collect();
    assert!(
        unknown.is_empty(),
        "terms under a field TERM_FIELDS does not declare ({known:?}): {unknown:?}"
    );
}

/// The lookup an agent reaches through `describe_schema`: each mathematics
/// term finds its one home, whatever its case and spacing.
#[test]
fn every_mathematics_term_finds_its_home() {
    let g = DesignGraph::open_in_memory().expect("graph");
    for (term, home) in MATHEMATICS {
        for asked in [
            term.to_string(),
            term.to_uppercase(),
            format!("  {}  ", term.replace(' ', "   ")),
        ] {
            let found = g.types_called(&asked);
            assert_eq!(found.len(), 1, "{asked:?}: {found:?}");
            let h = &found[0];
            assert_eq!(
                h.node_type.as_deref().or(h.edge_type.as_deref()),
                Some(*home),
                "{asked:?}"
            );
            assert_eq!(h.field, "mathematics");
            assert!(!h.means.is_empty());
        }
    }
}
