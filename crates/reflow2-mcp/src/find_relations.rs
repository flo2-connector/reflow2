//! `find_tools`' second half: the schema's RELATIONS, ranked beside the tools
//! (`dec:find-tools-also-finds-relations-and-points-to-what-joins-x-to-y`,
//! Anthony 2026-10-25).
//!
//! # Why find_tools ranks relations at all
//!
//! reflow2's vocabulary has two halves: the typed tools, and the schema's node
//! and edge types. Until 2026-10-25 `find_tools` reached only the first. A
//! relation with no typed tool of its own could not be found by describing the
//! job: a Grok chat asking for "link actor to capability performed by operated
//! by uses" searched six times and found `INTERACTS_WITH` only through
//! `describe_schema` with `from`/`to`
//! (`fact:root-cause-a-relation-with-no-typed-tool-cannot-be-found-by-find-tools-2026-10-25`).
//!
//! So each schema edge type is scored against the query too, and the best few
//! come back under `relations`, apart from `items`. They are kept apart on
//! purpose: a relation is not a tool, and a tool's place in `items` must not
//! move because a relation scored well (the tool corpus,
//! `tests/fixtures/find_tools_corpus.json`, holds every tool to its rank).
//! No tool is added and no tool's text changes.
//!
//! # What a relation is scored on
//!
//! Its name, split at the underscores (`INTERACTS_WITH` → `interacts`,
//! `with`); its hint (what the schema says the edge asserts) and any field's
//! word for it (`also_called`); its property names; and the types it may join,
//! where one is named rather than `*`. The weights are `score_tool`'s (an exact
//! name, a name word, a hint word, a property), plus one for an endpoint, and
//! each is multiplied by how discriminating the query word is across tools AND
//! relations together.
//!
//! Words match by [`same_word`], not exact spelling: a person writes
//! "capabilities", "operated", "shapes", and the schema says "capability",
//! "operates", "shaped". The tools' scorer keeps its exact matching, so no
//! tool's ranking moves.
//!
//! # Measured
//!
//! On the relation corpus (`tests/fixtures/find_tools_relation_corpus.json`):
//! see the test that holds it, `a_relation_is_found_by_a_query_in_a_persons_words`.

use reflow2_core::vocabulary::EdgeTypeSpec;
use serde_json::{Value as JsonValue, json};

use crate::service::{length_norm, term_weights};

/// How many relations a reply carries at most. Small, like the tools' five:
/// the point is to name the relation worth drawing, not to list the schema.
pub(crate) const RELATIONS_SHOWN: usize = 3;

/// A relation is shown only if it scores at least this share of the best one,
/// so a relation that clearly answers the query is not padded out with two
/// that merely share a word with it.
const SHARE_OF_BEST: f64 = 1.0 / 3.0;

/// The weight of a query word that names a type the relation may join.
const ENDPOINT_HIT: f64 = 2.0;

/// What every find_tools reply says about the question it cannot rank: which
/// relation may join two KINDS of thing. That question has an exact answer,
/// and `describe_schema` gives it; the chat that searched six times found its
/// relation only there.
pub(crate) const WHAT_JOINS: &str = "To ask what may join an X to a Y (which relation links an \
     Actor to a Capability, say), call `describe_schema` with `from` and `to` set to the two node \
     types: it lists every relation the schema allows between them, best first, and the call that \
     draws each.";

/// Two spellings of one word: equal, or sharing a prefix of at least four
/// letters that leaves no more than three over on the longer
/// (`capabilities`/`capability`, `operated`/`operates`, `risk`/`risks`), so
/// `uses` never matches `causes` and `part` never matches `participates`.
pub(crate) fn same_word(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let shorter = a.len().min(b.len());
    if shorter < 4 {
        return false;
    }
    let shared = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    shared >= 4.max(a.len().max(b.len()).saturating_sub(3))
}

fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

/// The prose a relation is matched on: its hint and every field's word for it.
fn prose(edge: &EdgeTypeSpec) -> String {
    let mut text = edge.hint.clone().unwrap_or_default();
    for term in edge.also_called.values().flatten() {
        text.push(' ');
        text.push_str(&term.term);
    }
    text.to_lowercase()
}

fn score_relation(edge: &EdgeTypeSpec, prose: &str, weighted: &[(&str, f64)]) -> f64 {
    let name = edge.edge_type.to_lowercase();
    let parts: Vec<&str> = name.split('_').collect();
    let text = words(prose);
    let props: Vec<String> = edge
        .properties
        .iter()
        .flat_map(|p| words(&p.name))
        .collect();
    let ends: Vec<String> = edge
        .from
        .iter()
        .chain(&edge.to)
        .filter(|t| t.as_str() != "*")
        .map(|t| t.to_lowercase())
        .collect();
    let mut score = 0.0;
    for (term, weight) in weighted {
        let term = *term;
        if name == term {
            score += 8.0 * weight;
        } else if parts.iter().any(|p| same_word(term, p)) {
            score += 5.0 * weight;
        }
        if text.iter().any(|w| same_word(term, w)) {
            score += 2.0 * weight;
        }
        if props.iter().any(|w| same_word(term, w)) {
            score += weight;
        }
        if ends.iter().any(|e| same_word(term, e)) {
            score += ENDPOINT_HIT * weight;
        }
    }
    score / length_norm(prose.len())
}

/// The call that draws `edge_type`: its typed tool where one exists, and
/// `create_edge` with that `edge_type` always (it draws every relation).
fn draw_with(edge: &EdgeTypeSpec) -> JsonValue {
    let typed = crate::writers::edge_type_writers(&edge.edge_type);
    let generic = format!("create_edge with edge_type \"{}\"", edge.edge_type);
    let how = match typed.as_slice() {
        [] => format!("{generic} (no typed tool draws it)"),
        [one] => format!("{one}, or {generic}"),
        many => format!("{} (any one), or {generic}", many.join(", ")),
    };
    json!({
        "how": how,
        "typed_tools": typed,
        "create_edge": {
            "edge_type": edge.edge_type,
            "from_id": "<id of the source node>",
            "to_id": "<id of the target node>",
        },
    })
}

/// The relations a query names, best first, at most [`RELATIONS_SHOWN`].
/// `tool_corpus` is the served tools as find_tools scores them (lowercased
/// name, lowercased description), so a word's weight counts tools and
/// relations together.
pub(crate) fn rank(
    edges: &[EdgeTypeSpec],
    terms: &[&str],
    tool_corpus: &[(String, String)],
) -> Vec<JsonValue> {
    let prose: Vec<String> = edges.iter().map(prose).collect();
    let mut corpus: Vec<(String, String)> = tool_corpus.to_vec();
    corpus.extend(
        edges
            .iter()
            .zip(&prose)
            .map(|(e, p)| (e.edge_type.to_lowercase(), p.clone())),
    );
    let weighted = term_weights(terms, &corpus);
    let mut scored: Vec<(f64, &EdgeTypeSpec)> = edges
        .iter()
        .zip(&prose)
        .map(|(e, p)| (score_relation(e, p, &weighted), e))
        .filter(|(s, _)| *s > 0.0)
        .collect();
    // Ties broken by name, as the tools are: the same query answers the same
    // way twice.
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.edge_type.cmp(&b.1.edge_type))
    });
    let best = scored.first().map(|(s, _)| *s).unwrap_or(0.0);
    scored
        .into_iter()
        .filter(|(s, _)| *s >= best * SHARE_OF_BEST)
        .take(RELATIONS_SHOWN)
        .map(|(score, e)| {
            json!({
                "relation": e.edge_type,
                "kind": "relation: an edge type in the schema, not a tool",
                "joins": { "from": e.from, "to": e.to },
                "means": e.hint.as_deref().map(crate::service::trim_summary),
                "draw_with": draw_with(e),
                "score": score,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::same_word;

    #[test]
    fn same_word_takes_a_persons_spelling_and_no_more() {
        for (a, b) in [
            ("capabilities", "capability"),
            ("operated", "operates"),
            ("shapes", "shaped"),
            ("risk", "risks"),
            ("interacts", "interacts"),
            ("performs", "performed"),
        ] {
            assert!(same_word(a, b), "{a} ~ {b}");
        }
        for (a, b) in [
            ("uses", "causes"),
            ("part", "participates"),
            ("use", "user"),
            ("data", "date"),
            ("in", "interacts"),
        ] {
            assert!(!same_word(a, b), "{a} !~ {b}");
        }
    }
}
