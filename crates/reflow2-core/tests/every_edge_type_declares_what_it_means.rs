//! Every edge type declares what it MEANS, and the vocabulary serves it.
//!
//! `req:every-edge-type-declares-what-it-means`, promoted by Anthony on
//! 2026-09-28. The meaning of an edge used to live in at least six private
//! lists — the impact table, the structural sweep's traceability set,
//! `RISK_EDGES`, `COMMITMENT_EDGES`, `ARTIFACT_BOOKKEEPING`, and the five
//! hand-written projections of `dec:a-projection-is-named-and-the-battery-runs-over-it`
//! — with nothing keeping them in agreement, and one edge was already used
//! against its meaning (a DriftEvent's link to its Anchor drawn as DEPENDS_ON).
//! A declared reading is the one place those can move onto.
//!
//! WHAT THIS CHECKS: that every edge type HAS a reading, that the reading is
//! well-formed against the declared vocabulary, that it names every property
//! the edge declares as a modifier, that describe_schema and the
//! select-by-reading function both serve it — and that ONE EDGE is read by its
//! own values where its type is several relations under one name (`CAUSES` at
//! its default `basis` is a correlation, not a cause).
//!
//! WHAT IT DOES NOT CHECK, and cannot: whether any reading is RIGHT. Every
//! reading starts as `basis: classified` — one model's reading from the
//! 2026-09-28 three-graph classification — and becomes `reviewed` only on the
//! owner's word. That is a judgement, not a property a test can assert.
//!
//! OBSERVED FAILING, 2026-09-28: with the schema's readings stashed and the
//! code change in place, the first three tests below failed — the first on
//! ABOUT_ENTITY, the alphabetically first edge type — and all three passed once
//! the readings were restored. With the code for splits in place but no split
//! yet declared, the four tests that read one — every one below except the
//! well-formedness check — failed, the selector listing `CAUSES` as ALWAYS a
//! cause and an edge at its default `basis` reading as one; all passed once
//! the four edges declared their splits. Two schema mutations (a split value
//! outside its enum, a split on a property the edge does not declare) were
//! each caught by the well-formedness check, naming the edge. Eleven more
//! splits followed the same evening, from the classification's own records;
//! with those eleven and the `observed` reading stripped, the selector listed
//! `CHANGED` and `DEPENDS_ON` as ALWAYS a cause and a CAUSES edge `observed`
//! read as none, and both behaviour tests failed on exactly that.

use std::collections::{BTreeSet, HashMap};

use reflow2_core::DesignGraph;
use reflow2_core::foundation::core::{
    MODIFIER_KINDS, PropertyType, READING_BASES, READING_FORMS, RELATION_PRIMITIVES, Reading, Value,
};

fn graph() -> DesignGraph {
    DesignGraph::open_in_memory().expect("in-memory graph")
}

/// One reading is well-formed: a known form, and the primitive(s) it names
/// are in the declared set. Applied to what a type is named for and to every
/// split value's reading alike.
fn check_form(name: &str, r: &Reading) {
    assert!(
        READING_FORMS.contains(&r.form.as_str()),
        "{name}: form {:?} is not one of {READING_FORMS:?}",
        r.form
    );
    match r.form.as_str() {
        "primitive" => {
            let p = r
                .primitive
                .as_deref()
                .unwrap_or_else(|| panic!("{name}: form is `primitive` but no primitive is named"));
            assert!(
                RELATION_PRIMITIVES.contains(&p),
                "{name}: primitive {p:?} is not in the declared set {RELATION_PRIMITIVES:?}"
            );
        }
        "composite" => {
            assert!(
                r.composition
                    .as_deref()
                    .is_some_and(|c| !c.trim().is_empty()),
                "{name}: form is `composite` but no composition is written"
            );
            assert!(
                !r.components.is_empty(),
                "{name}: a composition must list the primitives it uses"
            );
            for c in &r.components {
                assert!(
                    RELATION_PRIMITIVES.contains(&c.as_str()),
                    "{name}: component {c:?} is not in the declared set {RELATION_PRIMITIVES:?}"
                );
            }
            // The components ARE the primitives the composition writes, no
            // more and no fewer — a list kept beside the formula and checked
            // against nothing drifted into padding on the derived declarations
            // (fact:root-cause-the-derived-declaration-check-draws-every-population-from-the-declarations-it-checks-2026-09-29).
            let listed: BTreeSet<&str> = r.components.iter().map(String::as_str).collect();
            let written: BTreeSet<&str> = r.composition_primitives().into_iter().collect();
            assert_eq!(
                listed, written,
                "{name}: components {listed:?} are not the primitives its composition writes {written:?}"
            );
        }
        "leftover" => assert!(
            r.note.as_deref().is_some_and(|n| !n.trim().is_empty()),
            "{name}: a leftover reading must say why no primitive fits"
        ),
        _ => unreachable!("form already checked"),
    }
}

#[test]
fn every_edge_type_declares_a_well_formed_reading() {
    let g = graph();
    let schema = g.schema();
    let mut names: Vec<&String> = schema.edge_types.keys().collect();
    names.sort();
    assert!(
        !names.is_empty(),
        "the schema declares no edge types — this test checked nothing"
    );

    for name in names {
        let def = &schema.edge_types[name];
        let r = def.reading.as_ref().unwrap_or_else(|| {
            panic!(
                "{name} declares no reading. Every edge type says what it means in primitive \
                 terms, so a rule can select edges by meaning instead of keeping a private list \
                 of names (req:every-edge-type-declares-what-it-means)"
            )
        });

        assert!(
            READING_BASES.contains(&r.basis.as_str()),
            "{name}: basis {:?} is not one of {READING_BASES:?} — a reading must say who says so",
            r.basis
        );
        check_form(name, &r.reading);
        for split in &r.splits {
            let prop = def.properties.get(&split.property).unwrap_or_else(|| {
                panic!(
                    "{name}: a split on {:?}, which the edge does not declare — no edge could \
                     ever carry the value that selects it",
                    split.property
                )
            });
            assert!(
                !split.values.is_empty(),
                "{name}.{}: a split with no values splits nothing",
                split.property
            );
            if prop.prop_type == PropertyType::Enum {
                let allowed: BTreeSet<&str> =
                    prop.values.iter().flatten().map(String::as_str).collect();
                for value in split.values.keys() {
                    assert!(
                        allowed.contains(value.as_str()),
                        "{name}.{}: split value {value:?} is not one of the enum's {allowed:?}",
                        split.property
                    );
                }
            }
            for (value, reading) in &split.values {
                check_form(&format!("{name}.{}={value}", split.property), reading);
            }
        }

        // Every declared property is a modifier of the relation, and nothing
        // is called a modifier that the edge does not declare.
        let declared: BTreeSet<&str> = def.properties.keys().map(String::as_str).collect();
        let named: BTreeSet<&str> = r.modifiers.keys().map(String::as_str).collect();
        assert_eq!(
            declared, named,
            "{name}: the reading's modifiers must name exactly the properties the edge declares \
             (missing = declared but unclassified; extra = named but not declared)"
        );
        for (prop, kind) in &r.modifiers {
            assert!(
                MODIFIER_KINDS.contains(&kind.as_str()),
                "{name}.{prop}: modifier kind {kind:?} is not one of {MODIFIER_KINDS:?}"
            );
        }
    }
}

#[test]
fn describe_schema_serves_each_edges_reading() {
    let g = graph();

    // The whole-vocabulary read lists every edge, so it carries each reading
    // as ONE LINE: that listing was already past its reply budget with all
    // prose withheld (45,818 characters against 30,000, measured 2026-09-28),
    // and it is never shortened, so the full object on 65 edges would only
    // have grown an overflowing answer by about 10,000 characters.
    let vocab = serde_json::to_value(g.describe_vocabulary()).expect("serializable");
    let edges = vocab["edge_types"]
        .as_array()
        .expect("edge_types is a list");
    assert!(
        !edges.is_empty(),
        "the vocabulary serves no edge types — this test checked nothing"
    );
    for e in edges {
        assert!(
            e["reads_as"].as_str().is_some_and(|s| !s.is_empty()),
            "describe_schema lists {} with no reading — a caller choosing between edges that \
             all validate cannot compare what they mean",
            e["edge_type"]
        );
    }
    // WHO SAYS SO stays visible on the listing, as a count.
    let by_basis: u64 = vocab["edge_readings_by_basis"]
        .as_object()
        .expect("the listing counts readings by basis")
        .values()
        .map(|n| n.as_u64().expect("a count"))
        .sum();
    assert_eq!(
        by_basis as usize,
        edges.len(),
        "every listed edge's reading is counted by basis"
    );

    // A node type's listing of the edges it can carry: one line each, too.
    let detail = serde_json::to_value(g.describe_node_type("Capability").expect("exists"))
        .expect("serializable");
    for side in ["outgoing", "incoming"] {
        for e in detail[side].as_array().expect("a list") {
            assert!(
                e["reads_as"].as_str().is_some_and(|s| !s.is_empty()),
                "Capability's {side} {} carries no reading",
                e["edge_type"]
            );
        }
    }

    // The pair query carries the WHOLE reading: that is where a caller is
    // actually choosing between edges that all validate.
    let pair = serde_json::to_value(
        g.edge_types_between("Capability", "Requirement")
            .expect("both types exist"),
    )
    .expect("serializable");
    let satisfies = pair["matches"]
        .as_array()
        .and_then(|m| m.iter().find(|x| x["edge_type"] == "SATISFIES"))
        .expect("SATISFIES joins a Capability to a Requirement");
    assert_eq!(satisfies["reading"]["primitive"], "norm");
    assert_eq!(satisfies["reading"]["polarity"], "+");
    assert_eq!(satisfies["reading"]["basis"], "classified");
    // A `planned` SATISFIES is a promise, not a fact, so the one line says
    // the reading splits, and the whole reading says on what.
    assert_eq!(satisfies["reads_as"], "norm(+) · meets | by coverage");
    assert_eq!(satisfies["reading"]["splits"][0]["property"], "coverage");
    assert_eq!(
        satisfies["reading"]["splits"][0]["values"]["planned"]["modality"],
        "intended"
    );

    // …and a type that is several relations carries how it splits.
    let causes = pair["matches"]
        .as_array()
        .and_then(|m| m.iter().find(|x| x["edge_type"] == "CAUSES"))
        .expect("CAUSES joins any two types");
    assert_eq!(causes["reading"]["splits"][0]["property"], "basis");
    assert_eq!(
        causes["reading"]["splits"][0]["values"]["correlational"]["primitive"],
        "compares"
    );
}

#[test]
fn the_one_line_form_says_the_primitive_its_direction_and_sign() {
    let g = graph();
    let vocab = serde_json::to_value(g.describe_vocabulary()).expect("serializable");
    let reads = |name: &str| {
        vocab["edge_types"]
            .as_array()
            .and_then(|e| e.iter().find(|x| x["edge_type"] == name))
            .map(|x| x["reads_as"].as_str().unwrap_or_default().to_string())
            .unwrap_or_else(|| panic!("{name} is not listed"))
    };
    // CONTAINS runs whole → part, the inverse of part-of.
    assert_eq!(reads("CONTAINS"), "part-of⁻¹");
    assert_eq!(reads("BLOCKS"), "causes(-)");
    // A composition lists the primitives it is built from.
    assert_eq!(reads("SPECIFIES"), "about ∧ norm");
    // A type that is several relations says which properties tell them apart,
    // so a caller skimming the listing knows not to take the line whole.
    assert_eq!(reads("CAUSES"), "causes(+) | by basis, validation_status");
}

#[test]
fn edges_are_selected_by_their_declared_reading() {
    let g = graph();

    let part_of = g.edge_types_read_as("part-of", None);
    for expected in ["CONTAINS", "DECOMPOSES"] {
        assert!(
            part_of.always.iter().any(|n| n == expected),
            "{expected} reads as part-of and was not selected: {part_of:?}"
        );
    }
    assert!(
        !part_of
            .always
            .iter()
            .chain(&part_of.per_edge)
            .any(|n| n == "CAUSES"),
        "CAUSES does not read as part-of and was selected: {part_of:?}"
    );

    // Narrowed by sign: BLOCKS and RISKS are negative causes; no CAUSES edge
    // is, whatever its values.
    let negative_causes = g.edge_types_read_as("causes", Some("-"));
    for expected in ["BLOCKS", "RISKS"] {
        assert!(
            negative_causes.always.iter().any(|n| n == expected),
            "{expected}: {negative_causes:?}"
        );
    }
    assert!(
        !negative_causes
            .always
            .iter()
            .chain(&negative_causes.per_edge)
            .any(|n| n == "CAUSES"),
        "{negative_causes:?}"
    );

    // ⚠️ THE CASE THE SPLIT EXISTS FOR. BLOCKS is always a cause; CAUSES is a
    // cause only on the edges whose `basis` says so, INTERACTS_WITH only where
    // the actor `triggers`, DEPENDS_ON not where it is a data flow, CHANGED
    // not where it made the node. A rule that took the NAME as the meaning
    // would count every correlation as a cause.
    let causes = g.edge_types_read_as("causes", None);
    assert!(causes.always.iter().any(|n| n == "BLOCKS"), "{causes:?}");
    assert!(causes.always.iter().any(|n| n == "TRIGGERS"), "{causes:?}");
    for per_edge in ["CAUSES", "CHANGED", "DEPENDS_ON", "INTERACTS_WITH"] {
        assert!(
            causes.per_edge.iter().any(|n| n == per_edge),
            "{per_edge} means causes only for some values, so it is decided per edge: {causes:?}"
        );
        assert!(
            !causes.always.iter().any(|n| n == per_edge),
            "{per_edge} is not ALWAYS a cause: {causes:?}"
        );
    }

    // Identity is decided per edge too: only an `asserted` duplicate licenses
    // a merge, and a merged extraction resolves a mention into a node.
    let same_as = g.edge_types_read_as("same-as", None);
    for per_edge in ["DUPLICATES", "YIELDED"] {
        assert!(
            same_as.per_edge.iter().any(|n| n == per_edge),
            "{per_edge}: {same_as:?}"
        );
    }

    // Deterministic: sorted, so a rule written over it gives one answer.
    for list in [&part_of.always, &causes.per_edge] {
        let mut sorted = list.clone();
        sorted.sort();
        assert_eq!(list, &sorted, "selection must be sorted by name");
    }

    // A primitive nothing reads as is an empty answer, not an error.
    let none = g.edge_types_read_as("not-a-primitive", None);
    assert!(none.always.is_empty() && none.per_edge.is_empty());
}

fn props(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

fn s(v: &str) -> Value {
    Value::String(v.to_string())
}

#[test]
fn one_edge_is_read_by_its_own_values() {
    let g = graph();
    let is_cause = |p: &[(&str, Value)]| {
        g.edge_is_read_as("CAUSES", &props(p), "causes", None)
            .expect("CAUSES is declared")
    };

    // UNSET IS THE SCHEMA DEFAULT, `correlational`: a correlation, not a cause.
    assert!(
        !is_cause(&[]),
        "a CAUSES edge that never set `basis` is correlational by default and must not read as \
         a cause"
    );
    let unset = g
        .edge_reads_as("CAUSES", &HashMap::new())
        .expect("declared");
    assert_eq!(unset.len(), 1);
    assert_eq!(unset[0].primitive.as_deref(), Some("compares"));

    assert!(is_cause(&[("basis", s("causal"))]));
    // Seen to bring it about, with no mechanism: still a causal claim, on
    // weaker grounds (Anthony, 2026-09-28) — unlike `correlational`.
    assert!(is_cause(&[("basis", s("observed"))]));
    // Judged spurious: asserted NOT to hold — a negated reading matches nothing.
    assert!(!is_cause(&[("basis", s("spurious"))]));
    let spurious = g
        .edge_reads_as("CAUSES", &props(&[("basis", s("spurious"))]))
        .expect("declared");
    assert!(spurious[0].negated && spurious[0].summary() == "¬causes");
    // A later split overrides an earlier one: a refuted test outranks the
    // `causal` it was asserted on…
    assert!(!is_cause(&[
        ("basis", s("causal")),
        ("validation_status", s("refuted"))
    ]));
    // …and a status with no reading of its own leaves `basis`'s answer alone.
    assert!(is_cause(&[
        ("basis", s("causal")),
        ("validation_status", s("validated"))
    ]));

    // CONTRADICTS: the default is conflict; `supporting` is corroboration.
    let contradicts = |p: &[(&str, Value)]| {
        g.edge_reads_as("CONTRADICTS", &props(p))
            .expect("declared")
            .iter()
            .map(Reading::summary)
            .collect::<Vec<_>>()
    };
    assert_eq!(contradicts(&[]), ["compares(-) · conflicts"]);
    assert_eq!(
        contradicts(&[("alignment", s("supporting"))]),
        ["compares(+) · corroborates"]
    );

    // A LIST property is one relation per value: authored AND approved.
    let two = g
        .edge_reads_as(
            "AUTHORED_BY",
            &props(&[("roles", Value::List(vec![s("author"), s("approver")]))]),
        )
        .expect("declared");
    let lines: Vec<String> = two.iter().map(Reading::summary).collect();
    assert_eq!(lines, ["source-of⁻¹ · made", "stance⁻¹ · settles"]);
    // A legacy edge with no roles keeps what the type is named for.
    let legacy = g
        .edge_reads_as("AUTHORED_BY", &HashMap::new())
        .expect("declared");
    assert_eq!(legacy[0].summary(), "source-of⁻¹ · made");

    // An umbrella: unset says only that SOME relation exists; each value reads.
    assert!(
        g.edge_is_read_as(
            "INTERACTS_WITH",
            &props(&[("interaction", s("triggers"))]),
            "causes",
            None
        )
        .expect("declared")
    );
    assert!(
        g.edge_is_read_as(
            "INTERACTS_WITH",
            &props(&[("interaction", s("writes"))]),
            "flows-to",
            None
        )
        .expect("declared")
    );
    let bare = g
        .edge_reads_as("INTERACTS_WITH", &HashMap::new())
        .expect("declared");
    assert_eq!(bare[0].form, "leftover");

    // DEPENDS_ON: what kind of dependency decides the relation. A data flow
    // passes something across; a call brings something about; unset keeps
    // "needs it to function".
    let depends = |p: &[(&str, Value)]| {
        g.edge_reads_as("DEPENDS_ON", &props(p))
            .expect("declared")
            .iter()
            .map(Reading::summary)
            .collect::<Vec<_>>()
    };
    assert_eq!(depends(&[]), ["causes⁻¹(+)"]);
    assert_eq!(
        depends(&[("dependency_type", s("data_flow"))]),
        ["flows-to⁻¹"]
    );
    assert_eq!(
        depends(&[("dependency_type", s("function_call"))]),
        ["causes(+)"]
    );

    // CHANGED: adding a node made it; removing it ended it.
    let changed = |action: &str| {
        g.edge_reads_as("CHANGED", &props(&[("action", s(action))]))
            .expect("declared")[0]
            .summary()
    };
    assert_eq!(changed("added"), "source-of · made");
    assert_eq!(changed("removed"), "causes(-) · ends");

    // DUPLICATES: unset is `suspected` — a comparison, never a licence to merge.
    let merge_licensed = |p: &[(&str, Value)]| {
        g.edge_is_read_as("DUPLICATES", &props(p), "same-as", None)
            .expect("declared")
    };
    assert!(!merge_licensed(&[]));
    assert!(merge_licensed(&[("basis", s("asserted"))]));

    // A value can change only the POSSIBILITY: a planned SATISFIES is a
    // promise, and an unset one is the fact that counts toward delivery.
    let satisfies = |p: &[(&str, Value)]| {
        g.edge_reads_as("SATISFIES", &props(p)).expect("declared")[0]
            .modality
            .clone()
    };
    assert_eq!(satisfies(&[]), None);
    assert_eq!(
        satisfies(&[("coverage", s("planned"))]).as_deref(),
        Some("intended")
    );
    // …and a confirmed rule violation is a permitted one — a waiver.
    let waiver = g
        .edge_reads_as("VIOLATES_RULE", &props(&[("status", s("confirmed"))]))
        .expect("declared");
    assert_eq!(waiver[0].modality.as_deref(), Some("permitted"));

    // A type with no splits reads the same whatever its values.
    assert!(
        g.edge_is_read_as("BLOCKS", &HashMap::new(), "causes", Some("-"))
            .expect("declared")
    );

    // An undeclared edge type is an error, never "means nothing".
    assert!(g.edge_reads_as("NOT_AN_EDGE", &HashMap::new()).is_err());
}
