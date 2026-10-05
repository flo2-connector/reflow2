//! Which served tool writes which part of the vocabulary — read from ONE map,
//! `crates/reflow2-mcp/writers.json`, which `tools/vocabulary_reach.py` reads too.
//!
//! # Why a shared file and not a table here
//!
//! Until 2026-09-29 the node-type half of this lived only in that script, as a
//! CI instrument. The schema could not tell an agent asking about a type that
//! `record_finding` writes a `TemporalFact`, so the dev_reflow2 designer learned
//! the type's fields by reading an export and wrote each fact with raw
//! `create_node` (I14b,
//! `fact:root-cause-describe-schema-never-names-the-typed-tool-that-writes-a-type-and-the-map-lives-in-a-repo-script-2026-09-29`).
//! A second copy here would be the hand-kept duplicate that drifts; one file
//! read by both cannot.
//!
//! # What reads it
//!
//! - `describe_schema` on a node type answers `written_by`, and on a pair each
//!   edge answers `drawn_by`.
//! - Every invalid-edge refusal names the typed tool that draws each edge the
//!   schema DOES model for the pair ([`invalid_pair_detail`]). A typed helper
//!   refused `verifies(Verification → Decision)` with the bare sentence while
//!   GOVERNED_BY, drawn by `governed_by`, was the modelled route (I14a).

use std::collections::BTreeMap;
use std::sync::LazyLock;

use reflow2_core::vocabulary::{EdgeQuery, EndpointMatch};

#[derive(serde::Deserialize)]
struct WritersFile {
    writes_node_type: BTreeMap<String, Vec<String>>,
    draws_edge_type: BTreeMap<String, Vec<String>>,
}

struct Writers {
    /// Node type → the served tools that write it, sorted.
    by_node_type: BTreeMap<String, Vec<String>>,
    /// Edge type → the served tools that draw it, sorted.
    by_edge_type: BTreeMap<String, Vec<String>>,
}

static WRITERS: LazyLock<Writers> = LazyLock::new(|| {
    // Compiled in, so a malformed file is a build-time test failure
    // (`the_shared_map_names_only_served_tools_and_declared_types`) rather
    // than a runtime surprise; an unparseable file degrades to an empty map
    // instead of taking a refusal down with it.
    let file: WritersFile =
        serde_json::from_str(include_str!("../writers.json")).unwrap_or(WritersFile {
            writes_node_type: BTreeMap::new(),
            draws_edge_type: BTreeMap::new(),
        });
    let invert = |m: BTreeMap<String, Vec<String>>| {
        let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (tool, names) in m {
            for n in names {
                out.entry(n).or_default().push(tool.clone());
            }
        }
        for tools in out.values_mut() {
            tools.sort();
            tools.dedup();
        }
        out
    };
    Writers {
        by_node_type: invert(file.writes_node_type),
        by_edge_type: invert(file.draws_edge_type),
    }
});

/// Every node type some typed tool writes, sorted.
pub fn node_types_written() -> Vec<String> {
    WRITERS.by_node_type.keys().cloned().collect()
}

/// The served tools that write `node_type`, sorted. Empty means only the
/// generic `create_node` does.
pub fn node_type_writers(node_type: &str) -> Vec<String> {
    WRITERS
        .by_node_type
        .get(node_type)
        .cloned()
        .unwrap_or_default()
}

/// The served tools that draw `edge_type`, sorted. Empty means only the
/// generic `create_edge` does.
pub fn edge_type_writers(edge_type: &str) -> Vec<String> {
    WRITERS
        .by_edge_type
        .get(edge_type)
        .cloned()
        .unwrap_or_default()
}

/// The sentence `describe_schema` carries when no typed tool writes a type.
pub const NO_TYPED_WRITER: &str =
    "No typed tool writes this type; `create_node` does, validated against its properties.";

/// How many alternatives a refusal lists before deferring to `describe_schema`.
const MAX_ALTERNATIVES: usize = 12;

/// What DOES accept `from_type` → `to_type`, and the typed tool that draws each
/// — the tail every invalid-edge refusal carries, whichever tool raised it.
///
/// `None` only when the schema cannot answer (an unknown type name); the
/// caller then says less, never something false.
pub fn invalid_pair_detail(query: Result<EdgeQuery, reflow2_core::DynoError>) -> String {
    let q = match query {
        Ok(q) => q,
        // The endpoint types are themselves unknown, which is a better
        // diagnosis than a list of edges would be.
        Err(inner) => {
            return format!("\n\n{inner}\nCall `describe_schema` to list the valid node types.");
        }
    };
    let mut s = format!("\n\n{}", q.note);
    if !q.matches.is_empty() {
        s.push_str("\n\nEdge types that accept this pair, the modelled fits first:");
        for m in q.matches.iter().take(MAX_ALTERNATIVES) {
            let basis = if m.is_exact() {
                "exact"
            } else if m.declared_for_this_pair {
                "declared for this pair"
            } else if m.from_match == EndpointMatch::Exact || m.to_match == EndpointMatch::Exact {
                "one end named, the other open by design"
            } else {
                "via * on both ends — tolerated, not modelled"
            };
            let tools = edge_type_writers(&m.spec.edge_type);
            let how = if tools.is_empty() {
                "draw it with `create_edge`".to_string()
            } else {
                format!(
                    "draw it with {}",
                    tools
                        .iter()
                        .map(|t| format!("`{t}`"))
                        .collect::<Vec<_>>()
                        .join(" or ")
                )
            };
            s.push_str(&format!(
                "\n  {} ({}) — {} -> {} — {how}",
                m.spec.edge_type,
                basis,
                m.spec.from.join("|"),
                m.spec.to.join("|")
            ));
            if let Some(h) = &m.spec.hint {
                // The hint is what lets the caller pick on meaning rather than
                // on whatever validates first.
                s.push_str(&format!("\n      {}", h.lines().next().unwrap_or(h)));
            }
        }
        // No silent truncation (AGENTS.md rule 4).
        if q.matches.len() > MAX_ALTERNATIVES {
            s.push_str(&format!(
                "\n  … and {} more — call `describe_schema`.",
                q.matches.len() - MAX_ALTERNATIVES
            ));
        }
    }
    s.push_str("\n\nCall `describe_schema` for the full vocabulary.");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_map_is_read_and_names_the_writers_the_reports_needed() {
        assert!(
            node_type_writers("TemporalFact").contains(&"record_finding".to_string()),
            "I14b: the one writer the designer could not find"
        );
        assert!(
            edge_type_writers("GOVERNED_BY").contains(&"governed_by".to_string()),
            "I14a: the route a check on a ruling takes"
        );
    }
}
