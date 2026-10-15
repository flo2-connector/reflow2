//! An agent holding only a FIELD's words finds the type that holds them.
//!
//! `req:the-vocabulary-is-served-in-a-fields-own-terms`, accepted 2026-10-14.
//! Its SHOWN WHEN: "an agent given only mathematical wording ('record Lemma 2,
//! which follows from the hypothesis by induction') finds the right types
//! through find_tools and describe_schema without being told reflow2's names".
//!
//! Two doors, both additive, and NOTHING in tools/list:
//!
//! - `describe_schema` answers `node_type` given a field's word as well as a
//!   type's name. `{"node_type": "lemma"}` was refused ("Unknown node type")
//!   and now answers Requirement, with `found_by_term` saying what the mapping
//!   means. A declared type's name answers exactly as it did. Every type's
//!   answer carries its `also_called` list.
//! - `find_tools` indexes each term against the tools that write its type, so
//!   "record a lemma" ranks `add_requirement`, and the item says why
//!   (`also_called`).
//!
//! These ran on main before the terms existed and failed there: the lookups
//! were refused and the phrasings ranked nothing of the kind.

use reflow2_mcp::service::ReflowService;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

const CORPUS: &str = include_str!("fixtures/find_tools_corpus.json");

async fn describe(s: &ReflowService, args: Value) -> Result<Value, String> {
    s.describe_schema(Parameters(serde_json::from_value(args).expect("args")))
        .await
        .map(|r| r.structured_content.expect("structured"))
        .map_err(|e| e.message.to_string())
}

async fn find(s: &ReflowService, query: &str) -> Vec<Value> {
    let v: Value = s
        .find_tools(Parameters(
            serde_json::from_value(json!({"query": query, "limit": 5})).unwrap(),
        ))
        .await
        .expect("find_tools")
        .structured_content
        .expect("structured");
    v["items"].as_array().expect("items").clone()
}

fn names(items: &[Value]) -> Vec<String> {
    items
        .iter()
        .filter_map(|i| i["tool"].as_str().map(String::from))
        .collect()
}

/// The `found_by_term` entries of a describe_schema answer, as
/// (field, term, home) where home is the node or edge type.
fn found(v: &Value) -> Vec<(String, String, String)> {
    v["found_by_term"]
        .as_array()
        .unwrap_or_else(|| panic!("no found_by_term in {v}"))
        .iter()
        .map(|h| {
            assert!(
                h["means"].as_str().is_some_and(|m| !m.trim().is_empty()),
                "a found term must say what it means: {h}"
            );
            let home = h["node_type"]
                .as_str()
                .or_else(|| h["edge_type"].as_str())
                .unwrap_or_default();
            (
                h["field"].as_str().unwrap_or_default().to_string(),
                h["term"].as_str().unwrap_or_default().to_string(),
                home.to_string(),
            )
        })
        .collect()
}

#[tokio::test]
async fn describe_schema_finds_a_node_type_by_a_fields_word() {
    let s = ReflowService::in_memory().expect("service");
    for (asked, term, home) in [
        ("lemma", "lemma", "Requirement"),
        ("Lemma", "lemma", "Requirement"),
        ("  THEOREM ", "theorem", "Requirement"),
        ("hypothesis", "hypothesis", "Constraint"),
        ("axiom", "axiom", "Constraint"),
        ("proof   strategy", "proof strategy", "Decision"),
        ("cited result", "cited result", "Artifact"),
    ] {
        let v = describe(&s, json!({"node_type": asked}))
            .await
            .unwrap_or_else(|e| panic!("{asked:?} was refused: {e}"));
        assert_eq!(
            v["node_type"], home,
            "{asked:?} must answer as {home}, the type that holds it: {v}"
        );
        assert_eq!(
            found(&v),
            [(
                "mathematics".to_string(),
                term.to_string(),
                home.to_string()
            )],
            "{asked:?}"
        );
        assert!(
            v["written_by"].is_array() && v["properties"].is_array(),
            "the answer is {home}'s own, writers and properties included: {v}"
        );
    }
    // required_only narrows the same answer, as it does for a type's name.
    let v = describe(&s, json!({"node_type": "lemma", "required_only": true}))
        .await
        .expect("lemma, required only");
    assert_eq!(v["node_type"], "Requirement");
    assert!(
        v.get("outgoing").is_none(),
        "required_only drops the edges: {v}"
    );
    assert_eq!(found(&v).len(), 1);
}

#[tokio::test]
async fn describe_schema_finds_an_edge_type_by_a_fields_word() {
    let s = ReflowService::in_memory().expect("service");
    for asked in ["counterexample", "refutes"] {
        let v = describe(&s, json!({"node_type": asked}))
            .await
            .unwrap_or_else(|e| panic!("{asked:?} was refused: {e}"));
        assert_eq!(v["edge_type"], "CONTRADICTS", "{asked:?}: {v}");
        assert_eq!(
            found(&v),
            [(
                "mathematics".to_string(),
                asked.to_string(),
                "CONTRADICTS".to_string()
            )]
        );
        assert!(
            v["drawn_by"]
                .as_array()
                .is_some_and(|d| d.iter().any(|t| t == "review_relations")),
            "the edge's answer names the tool that draws it: {v}"
        );
        assert!(v["reading"].is_object(), "and its whole reading: {v}");
    }
}

/// A declared type's name answers as it always did, and a word that is no
/// type and no field's term is refused as it always was.
#[tokio::test]
async fn a_types_own_name_answers_as_before() {
    let s = ReflowService::in_memory().expect("service");
    let v = describe(&s, json!({"node_type": "Requirement"}))
        .await
        .expect("Requirement");
    assert!(v.get("found_by_term").is_none(), "{v}");
    let e = describe(&s, json!({"node_type": "Relese"}))
        .await
        .expect_err("an unknown name stays refused");
    assert!(e.contains("Relese"), "{e}");
}

/// Each type's answer carries its list — the whole vocabulary and one type's
/// focus alike — so an agent reading either sees the field's words.
#[tokio::test]
async fn each_types_answer_carries_its_also_called_list() {
    let s = ReflowService::in_memory().expect("service");
    let terms = |spec: &Value| -> Vec<String> {
        spec["also_called"]["mathematics"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|t| t["term"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    let all = describe(&s, json!({})).await.expect("full vocabulary");
    let req = all["node_types"]
        .as_array()
        .expect("node_types")
        .iter()
        .find(|n| n["node_type"] == "Requirement")
        .expect("Requirement listed")
        .clone();
    for t in ["theorem", "lemma", "corollary", "conjecture", "proposition"] {
        assert!(terms(&req).contains(&t.to_string()), "{t}: {req}");
    }
    let contradicts = all["edge_types"]
        .as_array()
        .expect("edge_types")
        .iter()
        .find(|e| e["edge_type"] == "CONTRADICTS")
        .expect("CONTRADICTS listed")
        .clone();
    assert!(
        terms(&contradicts).contains(&"counterexample".to_string()),
        "{contradicts}"
    );
    let one = describe(&s, json!({"node_type": "Constraint"}))
        .await
        .expect("Constraint");
    assert!(terms(&one).contains(&"hypothesis".to_string()), "{one}");
}

/// The corpus's mathematical phrasings: every named tool is in the default
/// top 5, the first result is one of them, and each named tool's item says
/// which field's word found it.
#[tokio::test]
async fn find_tools_ranks_the_typed_tool_for_a_fields_phrasing() {
    let corpus: Value = serde_json::from_str(CORPUS).expect("corpus");
    let section = &corpus["in_a_fields_own_terms"]["mathematics"];
    let cases = section
        .as_array()
        .expect("the corpus carries mathematical phrasings");
    assert!(cases.len() >= 10, "too few phrasings to mean anything");
    let s = ReflowService::in_memory().expect("service");
    let mut wrong = Vec::new();
    for case in cases {
        let q = case["query"].as_str().expect("query");
        let want: Vec<&str> = case["tools"]
            .as_array()
            .expect("tools")
            .iter()
            .filter_map(Value::as_str)
            .collect();
        let items = find(&s, q).await;
        let got = names(&items);
        let first_ok = got.first().is_some_and(|f| want.contains(&f.as_str()));
        let all_in = want.iter().all(|w| got.iter().any(|g| g == w));
        let said = want.iter().all(|w| {
            items
                .iter()
                .find(|i| i["tool"] == *w)
                .is_some_and(|i| i["also_called"].as_array().is_some_and(|a| !a.is_empty()))
        });
        if !(first_ok && all_in && said) {
            wrong.push(format!("{q:?}\n      want {want:?}\n      top5 {got:?}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {} mathematical phrasings do not find their typed tool:\n  {}",
        wrong.len(),
        cases.len(),
        wrong.join("\n  ")
    );
}

/// THE SHOWN WHEN, as the requirement words it. The agent is given one
/// sentence of mathematics and no reflow2 name, and must reach the types.
#[tokio::test]
async fn record_lemma_two_which_follows_from_the_hypothesis_by_induction() {
    const SENTENCE: &str = "record Lemma 2, which follows from the hypothesis by induction";
    let s = ReflowService::in_memory().expect("service");

    // find_tools, on the sentence alone.
    let items = find(&s, SENTENCE).await;
    let got = names(&items);
    for (tool, term, home) in [
        ("add_requirement", "lemma", "Requirement"),
        ("add_constraint", "hypothesis", "Constraint"),
    ] {
        let item = items
            .iter()
            .find(|i| i["tool"] == tool)
            .unwrap_or_else(|| panic!("{tool} is not in the top 5: {got:?}"));
        let named = item["also_called"].as_array().is_some_and(|a| {
            a.iter()
                .any(|h| h["term"] == term && h["node_type"] == home && h["field"] == "mathematics")
        });
        assert!(named, "{tool} must say {term:?} → {home}: {item}");
    }

    // describe_schema, on the sentence's own words.
    for (word, home) in [("Lemma", "Requirement"), ("hypothesis", "Constraint")] {
        let v = describe(&s, json!({"node_type": word}))
            .await
            .unwrap_or_else(|e| panic!("{word:?} was refused: {e}"));
        assert_eq!(v["node_type"], home, "{word:?}: {v}");
    }
}

/// A query in reflow2's own words is answered as before: no item carries a
/// field's term. (The ranking itself is pinned unchanged by
/// `field_terms::tests` and by tools/tool_confusability.py --validate.)
#[tokio::test]
async fn a_query_in_reflow2s_own_words_carries_no_field_term() {
    let corpus: Value = serde_json::from_str(CORPUS).expect("corpus");
    let s = ReflowService::in_memory().expect("service");
    let mut carried = Vec::new();
    for (tool, q) in corpus["queries"].as_object().expect("queries") {
        let q = q.as_str().expect("query");
        for item in find(&s, q).await {
            if item.get("also_called").is_some() {
                carried.push(format!("{tool}: {q:?} → {}", item["tool"]));
            }
        }
    }
    assert!(carried.is_empty(), "{carried:#?}");
}
