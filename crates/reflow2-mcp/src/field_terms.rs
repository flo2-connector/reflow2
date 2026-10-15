//! A field's own words for reflow2's types, in `find_tools`' index
//! (`req:the-vocabulary-is-served-in-a-fields-own-terms`).
//!
//! The terms live in `schema/*.yaml` beside each type's hint (`also_called`)
//! and are read through [`reflow2_core::vocabulary::field_terms_in`]. They are
//! served in two places, where an agent looks when it chooses where something
//! goes: `describe_schema`'s answer (`tools/query.rs`) and this index. NEVER in
//! tools/list, which every client pays for on every session, so not one byte
//! of any tool's description or schema changes (`tools/toolsnap.py` proves it).
//!
//! # How a term ranks a tool
//!
//! A term for a NODE type indexes the type's constructors — the writers that
//! take the new node's `id` (`lemma` → `add_requirement`), the same set the
//! PascalCase route in `find_tools` names. A term for an EDGE type indexes
//! every served tool that draws it (`counterexample` → `review_relations`).
//!
//! A query that USES a term — its words, consecutive, whole — adds to each
//! indexed tool what an EXACT NAME match adds: [`TERM_HIT`] times how
//! discriminating the term is across the served surface, inside the same
//! length normalisation. A field's word for the type a tool writes names what
//! the tool makes, which is what a tool's name does. Weighed, not lifted to
//! the top: a word with an ordinary sense too ("approach", "reference") counts
//! by how rare it is on the surface, like any other word.
//!
//! Measured 2026-10-14 on the live binary against the corpus
//! (`tests/fixtures/find_tools_corpus.json`): no query in reflow2's own words
//! uses a term, and all 195 replies are byte-identical to main's. Of the 15
//! mathematical phrasings, main put every named tool in the top 5 for 1 and
//! first for none; now all 15 have every named tool in the top 5 and one of
//! them first. `tools/tool_confusability.py` scores the same way, and
//! `--validate` held it to the live ranker on all 210 at top 1. With the
//! Inference terms (2026-10-15) the phrasings are 25, all pass, and the
//! replica agreed with the live ranker on all 221 at top 1.

use std::collections::BTreeMap;

use reflow2_core::vocabulary::{TermHome, field_terms_in};

/// What a term hit is worth, before discrimination and length: an exact name
/// match's weight in [`crate::service::score_tool`].
pub(crate) const TERM_HIT: f64 = 8.0;

/// One field's term in the index: the words it is matched by, where it lives,
/// and the served tools that write that type.
pub(crate) struct Entry {
    pub words: Vec<String>,
    pub home: TermHome,
    pub tools: Vec<String>,
}

/// The words a term or query is matched by: lowercased, split exactly where
/// `find_tools` splits a query.
pub(crate) fn words_of(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

/// Are `phrase`'s words in `words`, consecutive and whole?
fn uses(words: &[&str], phrase: &[String]) -> bool {
    !phrase.is_empty()
        && words
            .windows(phrase.len())
            .any(|w| w.iter().zip(phrase).all(|(a, b)| *a == b.as_str()))
}

/// Is `phrase` written in a tool's (lowercased) description, as whole words?
/// Split the way [`crate::service::has_word`] splits, so a one-word term is
/// counted exactly as a query word is.
fn written_in(description_lc: &str, phrase: &[String]) -> bool {
    let words: Vec<&str> = description_lc
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    uses(&words, phrase)
}

/// Every field's term, against the tools in `served` that write its type.
/// A term no served tool writes is left out: it still answers through
/// `describe_schema`, and `create_node` / `create_edge` write it.
pub(crate) fn index(served: &[rmcp::model::Tool]) -> Vec<Entry> {
    // The compiled-in schema every graph in this process is opened with. If it
    // failed to parse, no design could be served at all and every write would
    // say so; the catalogue then simply carries no terms.
    let Ok(schema) = reflow2_core::schema::parsed_schema() else {
        return Vec::new();
    };
    field_terms_in(schema)
        .into_iter()
        .filter_map(|home| {
            let tools: Vec<String> = match (&home.node_type, &home.edge_type) {
                (Some(t), _) => crate::revise_route::constructors_of(t, served),
                (None, Some(e)) => edge_tools(schema, e, served),
                (None, None) => Vec::new(),
            };
            let words = words_of(&home.term);
            (!tools.is_empty() && !words.is_empty()).then_some(Entry { words, home, tools })
        })
        .collect()
}

/// The served tools that draw `edge_type`: its typed helpers (writers.json's
/// `draws_edge_type`) or, when it has none, the constructors of the one type
/// its edge is pinned to, which draw it as part of that record.
///
/// `PREMISE_OF` and `CONCLUDES` are the case: no typed helper draws them, and
/// `add_inference` draws both with the step (`premises`, `conclusion`). The
/// shared map lists typed edge helpers only, by its own rule, so it does not
/// name `add_inference` for them, and an agent holding "premise" would
/// otherwise be told nothing writes it. An edge open on both ends with no
/// helper (`*` to `*`) has no record it belongs to, and answers none.
pub(crate) fn edge_tools(
    schema: &reflow2_core::foundation::core::Schema,
    edge_type: &str,
    served: &[rmcp::model::Tool],
) -> Vec<String> {
    let helpers: Vec<String> = crate::writers::edge_type_writers(edge_type)
        .into_iter()
        .filter(|w| served.iter().any(|t| t.name == w.as_str()))
        .collect();
    if !helpers.is_empty() {
        return helpers;
    }
    let Some(def) = schema.edge_types.get(edge_type) else {
        return Vec::new();
    };
    let pinned = |end: &reflow2_core::foundation::core::EdgeEndpoint| match end {
        reflow2_core::foundation::core::EdgeEndpoint::Single(t)
            if t != reflow2_core::foundation::core::EdgeEndpoint::WILDCARD =>
        {
            Some(t.clone())
        }
        _ => None,
    };
    let mut tools: Vec<String> = [pinned(&def.from), pinned(&def.to)]
        .into_iter()
        .flatten()
        .flat_map(|t| crate::revise_route::constructors_of(&t, served))
        .collect();
    tools.sort();
    tools.dedup();
    tools
}

/// What the terms `query_words` use add to each tool, before the length
/// normalisation, and the homes that explain it — keyed by tool name.
///
/// `corpus` is `find_tools`' own: every served tool as (name, description),
/// lowercased. A term's document frequency counts the tools whose description
/// writes it plus the tools it indexes, so a word already common on the
/// surface ranks by how common it is.
pub(crate) fn hits<'a>(
    query_words: &[&str],
    index: &'a [Entry],
    corpus: &[(String, String)],
) -> BTreeMap<String, (f64, Vec<&'a TermHome>)> {
    let n = corpus.len() as f64;
    let mut out: BTreeMap<String, (f64, Vec<&'a TermHome>)> = BTreeMap::new();
    for entry in index.iter().filter(|e| uses(query_words, &e.words)) {
        let df = corpus
            .iter()
            .filter(|(name, desc)| {
                entry.tools.iter().any(|t| t == name) || written_in(desc, &entry.words)
            })
            .count() as f64;
        let weight = ((n + 1.0) / (df + 1.0)).ln().max(0.0);
        for tool in &entry.tools {
            let slot = out.entry(tool.clone()).or_default();
            slot.0 += TERM_HIT * weight;
            slot.1.push(&entry.home);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::ReflowService;

    fn served() -> Vec<rmcp::model::Tool> {
        ReflowService::in_memory()
            .expect("service")
            .tool_router
            .list_all()
            .into_iter()
            .map(crate::arguments::published)
            .collect()
    }

    #[test]
    fn a_term_indexes_the_tools_that_write_its_type() {
        let index = index(&served());
        let tools_of = |term: &str| {
            index
                .iter()
                .find(|e| e.home.term == term)
                .map(|e| e.tools.clone())
                .unwrap_or_default()
        };
        assert_eq!(tools_of("lemma"), ["add_requirement"]);
        assert_eq!(tools_of("hypothesis"), ["add_constraint"]);
        assert_eq!(tools_of("proof strategy"), ["add_decision"]);
        assert_eq!(tools_of("cited result"), ["add_artifact"]);
        assert_eq!(tools_of("counterexample"), ["review_relations"]);
        assert_eq!(tools_of("induction"), ["add_inference"]);
        // No typed helper draws these; the step's constructor does.
        assert_eq!(tools_of("premise"), ["add_inference"]);
        assert_eq!(tools_of("therefore"), ["add_inference"]);
    }

    #[test]
    fn a_term_is_used_only_whole_and_in_order() {
        let phrase = words_of("proof strategy");
        assert!(uses(&["the", "proof", "strategy"], &phrase));
        assert!(!uses(&["strategy", "proof"], &phrase));
        assert!(!uses(&["proof"], &phrase));
        assert!(!uses(&["proofs", "strategy"], &phrase));
        assert!(written_in("a proof-strategy choice", &phrase));
        assert!(!written_in("a proofstrategy", &phrase));
    }

    /// A query that uses no field's term gains nothing here, so its ranking
    /// is the ranking it had before terms existed: asked of every query in
    /// the find_tools corpus, which is written in reflow2's own words.
    #[test]
    fn a_query_in_reflow2s_own_words_gains_nothing() {
        let all = served();
        let index = index(&all);
        assert!(
            !index.is_empty(),
            "no term is indexed, so this compared nothing"
        );
        let corpus: Vec<(String, String)> = all
            .iter()
            .map(|t| {
                (
                    t.name.to_lowercase(),
                    t.description.as_deref().unwrap_or("").to_lowercase(),
                )
            })
            .collect();
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/find_tools_corpus.json"))
                .expect("corpus");
        for (tool, q) in fixture["queries"].as_object().expect("queries") {
            let q = q.as_str().expect("query");
            let words = words_of(q);
            let words: Vec<&str> = words.iter().map(String::as_str).collect();
            let got = hits(&words, &index, &corpus);
            assert!(
                got.is_empty(),
                "{tool}: {q:?} uses a field's term: {:?}",
                got.keys()
            );
        }
    }
}
