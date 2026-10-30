//! `find_tools` finds a RELATION from a question in a person's words, and
//! every reply says how to ask what joins an X to a Y
//! (`dec:find-tools-also-finds-relations-and-points-to-what-joins-x-to-y`,
//! Anthony 2026-10-25: "a. find_tools also searches relation types and
//! answers \"use create_edge with INTERACTS_WITH\". Adds no tools. b.
//! find_tools' reply points to describe_schema with from/to for \"what joins
//! X to Y?\"").
//!
//! # Why
//!
//! A Grok chat asking how to link an actor to a capability searched
//! find_tools six times; INTERACTS_WITH has no typed tool, find_tools ranked
//! tools only, and the chat found the relation through describe_schema
//! (`fact:root-cause-a-relation-with-no-typed-tool-cannot-be-found-by-find-tools-2026-10-25`).
//! The tool corpus could not have caught it: it held tool queries only. This
//! is the relation half, `fixtures/find_tools_relation_corpus.json`, with
//! four of Alex's queries in it word for word.
//!
//! # Measured
//!
//! OBSERVED FAILING on main (68df7252d), 2026-10-25: no reply carried a
//! relation, so every query in the corpus missed, Alex's four included, and
//! no reply carried the from/to pointer. With the relation half: 24 of 24 pass
//! and the 5 known misses still miss; the tool corpus is unchanged (its own
//! tests, `every_tool_is_found_by_a_query_in_the_users_words`).

use reflow2_mcp::service::ReflowService;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

const CORPUS: &str = include_str!("fixtures/find_tools_relation_corpus.json");

async fn find(s: &ReflowService, query: &str) -> Value {
    s.find_tools(Parameters(
        serde_json::from_value(json!({ "query": query })).unwrap(),
    ))
    .await
    .expect("find_tools")
    .structured_content
    .expect("structured")
}

fn relations_of(reply: &Value) -> Vec<String> {
    reply["relations"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|r| r["relation"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn entries(key: &str) -> Vec<(String, Vec<String>)> {
    let v: Value = serde_json::from_str(CORPUS).expect("corpus parses");
    v[key]
        .as_array()
        .expect(key)
        .iter()
        .map(|e| {
            (
                e["query"].as_str().expect("query").to_string(),
                e["relations"]
                    .as_array()
                    .expect("relations")
                    .iter()
                    .map(|r| r.as_str().expect("relation").to_string())
                    .collect(),
            )
        })
        .collect()
}

/// THE ACCEPTANCE TEST: each query's relation is among those returned.
#[tokio::test]
async fn every_relation_query_finds_its_relation() {
    let s = ReflowService::in_memory().expect("service");
    let queries = entries("queries");
    let mut missed = Vec::new();
    for (query, accepted) in &queries {
        let got = relations_of(&find(&s, query).await);
        if !got.iter().any(|r| accepted.contains(r)) {
            missed.push(format!(
                "{accepted:?} ← {query:?}\n      relations: {got:?}"
            ));
        }
    }
    assert!(
        missed.is_empty(),
        "{} of {} relation questions did not find their relation:\n  {}",
        missed.len(),
        queries.len(),
        missed.join("\n  ")
    );
}

/// A known miss must still be one: a query that now finds its relation moves
/// into `queries`, so the corpus only ever records what is true.
#[tokio::test]
async fn every_known_miss_still_misses() {
    let s = ReflowService::in_memory().expect("service");
    for (query, accepted) in entries("known_misses") {
        let got = relations_of(&find(&s, &query).await);
        assert!(
            !got.iter().any(|r| accepted.contains(r)),
            "{query:?} now finds {accepted:?} ({got:?}): move it from known_misses to queries"
        );
    }
}

/// A relation hit is marked as a relation, never as a tool, and names the
/// call that draws it: the typed tool where one exists, and create_edge with
/// that edge_type always.
#[tokio::test]
async fn a_relation_hit_names_the_call_that_draws_it() {
    let s = ReflowService::in_memory().expect("service");

    let reply = find(&s, "link actor to capability performed by operated by uses").await;
    let hit = reply["relations"]
        .as_array()
        .expect("relations")
        .iter()
        .find(|r| r["relation"] == "INTERACTS_WITH")
        .expect("INTERACTS_WITH is returned")
        .clone();
    assert!(hit.get("tool").is_none(), "a relation is not a tool: {hit}");
    assert!(
        hit["kind"]
            .as_str()
            .is_some_and(|k| k.starts_with("relation")),
        "marked as a relation: {hit}"
    );
    assert_eq!(hit["joins"]["from"], json!(["Actor"]), "{hit}");
    assert_eq!(hit["draw_with"]["typed_tools"], json!([]), "{hit}");
    assert_eq!(
        hit["draw_with"]["create_edge"]["edge_type"], "INTERACTS_WITH",
        "{hit}"
    );
    let how = hit["draw_with"]["how"].as_str().unwrap_or_default();
    assert!(
        how.contains("create_edge") && how.contains("INTERACTS_WITH"),
        "{how}"
    );
    // No relation ever reaches the tools' own list.
    for item in reply["items"].as_array().expect("items") {
        assert!(item["tool"].is_string(), "items holds tools only: {item}");
    }

    let reply = find(&s, "a rule that limits what a component may do").await;
    let hit = reply["relations"]
        .as_array()
        .expect("relations")
        .iter()
        .find(|r| r["relation"] == "CONSTRAINS")
        .expect("CONSTRAINS is returned")
        .clone();
    assert_eq!(
        hit["draw_with"]["typed_tools"],
        json!(["constrains"]),
        "{hit}"
    );
    let how = hit["draw_with"]["how"].as_str().unwrap_or_default();
    assert!(how.starts_with("constrains"), "the typed tool first: {how}");
}

/// Every reply, whatever it ranked, says that "what joins an X to a Y?" is
/// describe_schema with `from` and `to`.
#[tokio::test]
async fn every_reply_points_to_describe_schema_for_what_joins_x_to_y() {
    let s = ReflowService::in_memory().expect("service");
    for query in [
        "link actor to capability",
        "what shows me the blast radius",
        "zzzz no word here matches anything",
    ] {
        let reply = find(&s, query).await;
        let said = reply["what_joins"].as_str().unwrap_or_default();
        assert!(
            said.contains("describe_schema") && said.contains("`from`") && said.contains("`to`"),
            "{query:?}: what_joins = {said:?}"
        );
    }
}

/// THE FOLLOW-UP TO #798: relations come back only when one scores close to
/// the top tool (`find_relations::CLOSE_TO_THE_TOP_TOOL`), so a plain tool
/// search does not carry them every time. Held over the tool corpus (one query
/// per served tool, each a TOOL job in a person's words): the share of
/// replies carrying any relation. The relation corpus above stays whole.
#[tokio::test]
async fn a_plain_tool_search_rarely_carries_relations() {
    const TOOL_CORPUS: &str = include_str!("fixtures/find_tools_corpus.json");
    let corpus: Value = serde_json::from_str(TOOL_CORPUS).expect("tool corpus parses");
    let queries = corpus["queries"].as_object().expect("queries");
    let s = ReflowService::in_memory().expect("service");
    let mut carrying = Vec::new();
    for (tool, query) in queries {
        let reply = find(&s, query.as_str().expect("query")).await;
        if !relations_of(&reply).is_empty() {
            carrying.push(tool.clone());
        }
    }
    let share = carrying.len() as f64 / queries.len() as f64;
    eprintln!(
        "relation share over the tool corpus: {} of {} ({:.1}%)",
        carrying.len(),
        queries.len(),
        share * 100.0
    );
    assert!(
        share <= RELATION_SHARE_CEILING,
        "{} of {} plain tool searches carry relations ({:.1}%, ceiling {:.1}%): {carrying:?}",
        carrying.len(),
        queries.len(),
        share * 100.0,
        RELATION_SHARE_CEILING * 100.0
    );
}

/// The most a plain tool search may carry relations, as a share of the tool
/// corpus — a ratchet: lower it when the share falls, never raise it.
///
/// MEASURED 2026-10-25. On main at 614487215, before this: 198 of 198 (100%),
/// every reply carried relations. With relations only close to the top tool:
/// 95 of 199 (47.7%), the corpus by then holding record_the_build's query too.
/// Every question in the relation corpus still finds its relation (26 of 26,
/// Alex's six among them).
///
/// RE-MEASURED 2026-10-30, NOT LOOSENED: the fifteen counted refusing stubs
/// (crates/reflow2-mcp/src/deprecation.rs) left the tool corpus, six of whose
/// queries carried relations. The other 184 queries carry relations exactly as
/// on main 4bc0811d5 — the same 89 tools, measured on both builds — so the
/// share over the corpus moved from 95/199 to 89/184 with no reply changing.
/// The ratchet holds from here: lower it when the share falls.
const RELATION_SHARE_CEILING: f64 = 0.485;
