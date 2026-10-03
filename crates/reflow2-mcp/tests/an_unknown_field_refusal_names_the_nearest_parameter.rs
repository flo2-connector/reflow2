//! An unknown-field refusal names the nearest served parameter FIRST and
//! offers the stale-client possibility second. flo2, 2026-09-18: eight
//! rejected calls were filed upstream as schema drift because the refusal led
//! with "your client's tool list may predate the server", when five of the six
//! names had never existed in any release — the agent had guessed.
//!
//! Since 2026-10-02 an unknown key is refused by the argument check
//! (`reflow2_mcp::arguments`) before serde sees it, so the nearest name comes
//! from [`nearest_name`] over the schema's names, and the deserialiser's own
//! refusal — a call the schema accepted — uses the same function over the
//! names serde listed. These pin the rule and the order of the two sentences.

use reflow2_mcp::arguments::{Transport, check, deserializer_refusal, nearest_name, refusal};
use serde_json::json;

/// What a caller is told about `unknown`, given the names a tool takes in
/// the order serde lists them — the deserialiser's refusal, as it is worded.
fn stale_client_hint(serde_message: &str) -> String {
    deserializer_refusal(
        "t",
        &format!("failed to deserialize parameters: {serde_message}"),
        Transport::Session,
    )
    .expect("a deserialiser refusal is rewritten")
}

fn serde_message(unknown: &str, legal: &[&str]) -> String {
    let list = legal
        .iter()
        .map(|l| format!("`{l}`"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("unknown field `{unknown}`, expected one of {list}")
}

#[test]
fn node_id_reaches_id_and_parent_id_reaches_project_id() {
    let m = stale_client_hint(&serde_message(
        "node_id",
        &["id", "name", "decision", "rationale"],
    ));
    assert!(m.contains("Nearest served parameter: `id`."), "{m}");
    let m = stale_client_hint(&serde_message(
        "parent_id",
        &["from_id", "node_id", "project_id", "to_id"],
    ));
    assert!(m.contains("Nearest served parameter: `project_id`."), "{m}");
    let m = stale_client_hint(&serde_message(
        "authored_at",
        &[
            "from_type",
            "node_type",
            "from_id",
            "acted_at",
            "contributor_id",
        ],
    ));
    assert!(m.contains("Nearest served parameter: `acted_at`."), "{m}");
}

#[test]
fn the_same_concept_under_a_sibling_tools_name_is_found_by_its_shared_token_not_by_letters() {
    // flo2 F10, 2026-09-19: four of seven rejections were one concept named
    // differently by an adjacent tool. `id` is closer to `status` by letters.
    let m = stale_client_hint(&serde_message(
        "id",
        &["decision_id", "status", "approver", "acted_at"],
    ));
    assert!(
        m.contains("Nearest served parameter: `decision_id`."),
        "{m}"
    );
    let m = stale_client_hint(&serde_message(
        "node_id",
        &[
            "target_id",
            "target_type",
            "epoch_id",
            "change_type",
            "action",
        ],
    ));
    assert!(m.contains("Nearest served parameter: `target_id`."), "{m}");
    let m = stale_client_hint(&serde_message(
        "properties",
        &[
            "edge_type",
            "from_type",
            "from_id",
            "to_type",
            "to_id",
            "props",
        ],
    ));
    assert!(m.contains("Nearest served parameter: `props`."), "{m}");
}

#[test]
fn a_field_that_resembles_nothing_served_gets_no_nearest_name() {
    // `acted_at` on add_capability: no sibling field is the same concept, and
    // offering `id` as "nearest" would be a wrong answer dressed as help.
    let m = stale_client_hint(&serde_message(
        "acted_at",
        &["id", "name", "description", "status", "satisfies", "tier"],
    ));
    assert!(!m.contains("Nearest served parameter"), "{m}");
    let m = stale_client_hint(&serde_message(
        "kind",
        &["id", "name", "statement", "status", "priority", "concern"],
    ));
    assert!(!m.contains("Nearest served parameter"), "{m}");
}

#[test]
fn the_stale_client_line_comes_after_the_nearest_name_and_is_a_possibility() {
    let m = stale_client_hint(&serde_message("node_id", &["id", "name"]));
    let nearest = m.find("Nearest served parameter").expect("nearest first");
    let stale = m.find("may predate this server").expect("stale second");
    assert!(nearest < stale, "{m}");

    // And the same order in the argument check's own refusal.
    let schema = json!({"type": "object", "additionalProperties": false,
        "required": ["decision_id"],
        "properties": {"decision_id": {"type": "string"}, "status": {"type": "string"}}});
    let args = json!({"decision_id": "dec:x", "id": "dec:x"});
    let v = check(schema.as_object().unwrap(), args.as_object().unwrap());
    let m = refusal("set_decision_status", &v, Transport::Session);
    let nearest = m
        .find("Nearest served parameter: `decision_id`")
        .expect("nearest first");
    let stale = m.find("may predate this server").expect("stale second");
    assert!(nearest < stale, "{m}");
    assert!(m.contains("If nothing listed is what you meant"), "{m}");
}

#[test]
fn the_check_and_the_deserialiser_use_the_same_rule() {
    assert_eq!(
        nearest_name("id", &["decision_id", "status", "approver"]).as_deref(),
        Some("decision_id")
    );
    assert_eq!(nearest_name("kind", &["id", "name", "statement"]), None);
}
