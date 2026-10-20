//! The typed-id convention is ONE table, `reflow2_core::nodes::node::ID_PREFIXES`,
//! and `delete_node` reads it to take an id's prefix as the caller's stated type
//! when none is passed (`req:delete-node-takes-the-id-prefix-as-the-callers-stated-type`,
//! Anthony 2026-10-19). The conventions are STATED in the served descriptions
//! (`` `req:<slug>` by convention ``, `` conventionally prefixed `fact:` ``), so a
//! constructor added with a new prefix and no row would leave delete_node
//! refusing every bare id of that type as "a prefix that names no type". This
//! holds the two together: every prefix a served description states is a row.
use reflow2_core::nodes::node::ID_PREFIXES;
use reflow2_mcp::service::ReflowService;

/// The prefixes a text states by convention: `` `p:<…>` by convention `` and
/// `` conventionally prefixed `p:` ``, and the `` (`p:` / `q:` …) `` that
/// follows one of them in the same sentence.
fn stated_prefixes(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find('`') {
        let after = &rest[at + 1..];
        let Some(end) = after.find('`') else { break };
        let quoted = &after[..end];
        let tail = &after[end + 1..];
        if let Some((prefix, _)) = quoted.split_once(':') {
            let is_prefix = !prefix.is_empty()
                && prefix.chars().all(|c| c.is_ascii_lowercase())
                && !quoted.contains(' ');
            let stated = tail.trim_start().starts_with("by convention")
                || rest[..at].trim_end().ends_with("conventionally prefixed");
            if is_prefix && stated {
                out.push(prefix.to_string());
            }
        }
        rest = tail;
    }
    out
}

#[test]
fn every_prefix_a_served_description_states_is_a_row_of_the_table() {
    let mut stated: Vec<(String, String)> = Vec::new();
    for tool in ReflowService::served_tools() {
        let schema = serde_json::to_string(&tool.input_schema).unwrap();
        let text = format!(
            "{} {}",
            tool.description.as_deref().unwrap_or_default(),
            schema
        );
        for p in stated_prefixes(&text) {
            stated.push((tool.name.to_string(), p));
        }
    }
    assert!(
        stated.len() >= 15,
        "the reader found too few stated conventions to be reading the surface: {stated:?}"
    );
    let missing: Vec<_> = stated
        .iter()
        .filter(|(_, p)| !ID_PREFIXES.iter().any(|(row, _)| row == p))
        .collect();
    assert!(
        missing.is_empty(),
        "a served description states a prefix the table does not hold: {missing:?}"
    );
}

#[test]
fn the_reader_finds_a_stated_prefix_and_ignores_the_rest() {
    assert_eq!(
        stated_prefixes("The id — `req:<slug>` by convention, so … `xref:a:b` is another."),
        vec!["req"]
    );
    assert_eq!(
        stated_prefixes("Stable id, conventionally prefixed `fact:`."),
        vec!["fact"]
    );
}
