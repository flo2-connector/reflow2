//! An agent that wants to UPDATE a node finds the route that does it.
//!
//! # The finding (2026-10-05)
//!
//! `fact:a-constructors-revise-role-never-reaches-the-find-tools-summary-so-update-an-existing-node-misses-it-2026-10-05`.
//! reflow2 changes an existing node by calling its constructor again with the
//! same id: `add_artifact` with `art:x` and a new `status` revises `art:x` in
//! place and keeps every field it is not passed. Twenty-two tools work that way,
//! and until 2026-10-05 none said so in its first sentence, which is the whole
//! summary `find_tools` shows; no served tool is NAMED for updating; and a
//! guessed `update_node` was refused with no word of the route. So "update
//! properties of an existing node" ranked `add_artifact` outside the first 40,
//! and the field agent found the route only by trying it.
//!
//! # What this module carries
//!
//! The ONE sentence that names the route, so every refused tool name that
//! guesses at an update — through a session or through the shell door — says
//! the same thing; each constructor's own first sentence now says "Create or
//! revise", which is the line `find_tools` shows
//! (`dec:idea-an-agent-that-wants-to-update-a-node-finds-the-revise-path`).
//! No setter tool is added: the route exists and works, and a second way to do
//! the same write would be one more thing to keep in step.

/// How to change a node that already exists, in the words every door can act
/// on.
pub const REVISE_ROUTE: &str = "No tool is named for updating, because the constructors do it: \
     to change a node that already exists, call the add_* tool that creates its type \
     (add_requirement, add_decision, add_artifact, add_component, …) again with the same id and \
     only the fields you are changing — it revises the node in place and keeps every field you \
     do not pass, and its reply's `revision` block says what moved. A relation is revised the \
     same way, by calling its tool again for the same pair. For a type with no constructor of \
     its own, create_node with the same id does the same. Each such tool's summary begins \
     \"Create or revise\".";

/// Is a tool name that nothing serves a guess at an update tool?
pub fn guesses_an_update(name: &str) -> bool {
    let lower = name.to_lowercase();
    let first = lower.split(['_', '-']).next().unwrap_or("");
    matches!(
        first,
        "update" | "edit" | "modify" | "patch" | "upsert" | "revise" | "amend"
    ) || matches!(
        lower.as_str(),
        "set"
            | "set_node"
            | "set_nodes"
            | "set_property"
            | "set_properties"
            | "set_props"
            | "set_field"
            | "set_fields"
            | "set_node_properties"
            | "set_node_property"
    )
}

/// The constructors of `node_type` among `served`: a tool that writes the type
/// (writers.json) and takes the new node's `id`, which sets it apart from a
/// setter of the same type. One rule, read by `find_tools` (a query naming
/// the type ranks them first) and by the refusal below.
pub fn constructors_of(node_type: &str, served: &[rmcp::model::Tool]) -> Vec<String> {
    crate::writers::node_type_writers(node_type)
        .into_iter()
        .filter(|tool| {
            served.iter().any(|t| {
                t.name == tool.as_str()
                    && t.input_schema
                        .get("required")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|r| r.iter().any(|f| f == "id"))
            })
        })
        .collect()
}

/// The words of a type's name: `TemporalFact` → `temporal`, `fact`.
fn type_words(node_type: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in node_type.chars() {
        if c.is_uppercase() || out.is_empty() {
            out.push(String::new());
        }
        if let Some(w) = out.last_mut() {
            w.extend(c.to_lowercase());
        }
    }
    out
}

/// The node types a guessed tool name names, with the tools that create each:
/// `add_temporal_fact` and `record_fact` name `TemporalFact`. The name's words,
/// less the verbs a guess wraps them in, must be the type's words or the end
/// of them.
pub fn types_named(name: &str, served: &[rmcp::model::Tool]) -> Vec<(String, Vec<String>)> {
    const WRAPPERS: &[&str] = &[
        "add", "create", "new", "record", "make", "write", "set", "update", "put", "insert",
        "upsert", "edit", "get", "node", "nodes", "entry", "item",
    ];
    let rest: Vec<String> = name
        .to_lowercase()
        .split(['_', '-'])
        .filter(|w| !w.is_empty() && !WRAPPERS.contains(w))
        .map(|w| {
            w.strip_suffix('s')
                .filter(|s| s.len() > 2)
                .unwrap_or(w)
                .to_string()
        })
        .collect();
    if rest.is_empty() {
        return Vec::new();
    }
    crate::writers::node_types_written()
        .into_iter()
        .filter(|ty| {
            let words = type_words(ty);
            words.len() >= rest.len() && words[words.len() - rest.len()..] == rest[..]
        })
        .map(|ty| {
            let tools = constructors_of(&ty, served);
            (ty, tools)
        })
        .filter(|(_, tools)| !tools.is_empty())
        .collect()
}

/// The refusal for a tool name nothing serves: the tool that creates a node
/// type the name names, the revise route when the name guesses at an update,
/// and the nearest served names.
pub fn unknown_tool(name: &str, served: &[rmcp::model::Tool]) -> String {
    let names: std::collections::BTreeSet<String> =
        served.iter().map(|t| t.name.to_string()).collect();
    let near = crate::lessons::nearest(name, &names);
    let nearest = if near.is_empty() {
        String::new()
    } else {
        format!(" The nearest served names: {}.", near.join(", "))
    };
    let route = if guesses_an_update(name) {
        format!(" {REVISE_ROUTE}")
    } else {
        String::new()
    };
    let typed: String = types_named(name, served)
        .into_iter()
        .map(|(ty, tools)| {
            format!(
                " It names a node type: `{ty}` is created by {}.",
                tools
                    .iter()
                    .map(|t| format!("`{t}`"))
                    .collect::<Vec<_>>()
                    .join(" or ")
            )
        })
        .collect();
    format!("no tool named `{name}` is served.{typed}{route}{nearest}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_type_name_splits_into_its_words() {
        assert_eq!(type_words("TemporalFact"), vec!["temporal", "fact"]);
        assert_eq!(type_words("Decision"), vec!["decision"]);
    }

    #[test]
    fn the_guesses_the_field_made_are_recognised() {
        for g in [
            "update_node",
            "update",
            "set_node",
            "edit_node",
            "update_artifact",
        ] {
            assert!(guesses_an_update(g), "{g}");
        }
        for g in ["get_nodes", "settle", "set_requirement_status", "add_thing"] {
            assert!(!guesses_an_update(g), "{g}");
        }
    }
}
