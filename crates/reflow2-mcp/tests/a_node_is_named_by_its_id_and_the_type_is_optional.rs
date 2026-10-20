//! Convention (i), Anthony 2026-09-06: wherever a tool names an EXISTING node by
//! a type+id pair, the type is optional — resolved from the id, since the id
//! prefix names the type by convention — and a COLLISION (one id under two
//! types) is refused naming both, never guessed. Constructors keep the type
//! required (you cannot create a thing without saying what it is).
//!
//! `delete_node` kept the type required on purpose (ruling #3b, 2026-09-06)
//! until Anthony reopened it for delete_node alone on 2026-10-19
//! (`req:delete-node-takes-the-id-prefix-as-the-callers-stated-type`, option
//! (b) of `dec:idea-delete-node-says-why-it-asks-for-the-type-or-takes-the-id-prefix-as-the-callers-word`):
//! omitted, the id's PREFIX is the caller's stated type, and the delete goes
//! ahead only when the one type holding the id is the type that prefix names.
//! fact:defect-sibling-constructors-disagree-on-how-to-name-a-node-and-what-status-it-lands-in
//! fact:defect-typed-tool-parameter-names-are-inconsistent
//! dec:idea-one-way-to-name-which-node-across-the-tool-surface
//!
//! Written first and observed failing to compile: the type fields were `String`.
use reflow2_mcp::service::*;
use rmcp::handler::server::wrapper::Parameters;

macro_rules! j {
    ($call:expr) => {
        $call
            .await
            .expect("tool ok")
            .structured_content
            .expect("structured content present")
    };
}

async fn seeded() -> ReflowService {
    let s = ReflowService::in_memory().expect("service");
    j!(s.add_requirement(Parameters(RequirementReq {
        source: None,
        provenance: None,
        id: "req:fast".into(),
        name: Some("Fast".into()),
        statement: Some("Answer in under a second.".into()),
        distinct_from: None,
        replaces: None,
        status: None,
        approver: None,
        acted_at: None,
        priority: None,
        concern: None,
        kind: None,
        taken_as_given: None,
        statement_kind: None,
        source_reliability: None,
        information_credibility: None,
        reported_at: None,
        confidence: None,
        variables: None,
        fixes: None,
    })));
    j!(s.add_verification(Parameters(
        serde_json::from_value(serde_json::json!({
            "id": "ver:fast", "name": "Fast check", "description": "Times the answer."
        }))
        .unwrap()
    )));
    j!(s.add_decision(Parameters(serde_json::from_value(serde_json::json!({
        "id": "dec:fast", "name": "Fast is a hard budget", "decision": "One second, p50.", "rationale": "Users leave."
    })).unwrap())));
    s
}

#[tokio::test]
async fn verifies_resolves_the_target_type_from_the_id() {
    let s = seeded().await;
    let out = j!(s.verifies(Parameters(
        serde_json::from_value(serde_json::json!({
            "verification_id": "ver:fast", "target_id": "req:fast"
        }))
        .unwrap()
    )));
    assert_eq!(out["to_id"], "req:fast", "{out}");
}

#[tokio::test]
async fn governed_by_resolves_both_ends_from_their_ids() {
    let s = seeded().await;
    let out = j!(s.governed_by(Parameters(
        serde_json::from_value(serde_json::json!({
            "from_id": "req:fast", "to_id": "dec:fast"
        }))
        .unwrap()
    )));
    assert_eq!(out["edge_type"], "GOVERNED_BY", "{out}");
}

#[tokio::test]
async fn a_collision_is_refused_and_names_both_types() {
    let s = seeded().await;
    // The same id under a second type: a convention violation, but writable.
    j!(s.create_node(Parameters(serde_json::from_value(serde_json::json!({
        "node_type": "Capability", "id": "req:fast", "props": {"name": "Impostor", "description": "same id"}
    })).unwrap())));
    let e = s
        .verifies(Parameters(
            serde_json::from_value(serde_json::json!({
                "verification_id": "ver:fast", "target_id": "req:fast"
            }))
            .unwrap(),
        ))
        .await
        .expect_err("two types hold req:fast");
    let m = format!("{e:?}");
    assert!(
        m.contains("Requirement") && m.contains("Capability") && m.contains("target_type"),
        "names both and the field to pass: {m}"
    );
    // Passing the type still works — that is the way through.
    let out = j!(s.verifies(Parameters(
        serde_json::from_value(serde_json::json!({
            "verification_id": "ver:fast", "target_type": "Requirement", "target_id": "req:fast"
        }))
        .unwrap()
    )));
    assert_eq!(out["to_id"], "req:fast");
}

#[tokio::test]
async fn an_unknown_id_says_so_rather_than_guessing_a_type() {
    let s = seeded().await;
    let e = s
        .verifies(Parameters(
            serde_json::from_value(serde_json::json!({
                "verification_id": "ver:fast", "target_id": "req:nope"
            }))
            .unwrap(),
        ))
        .await
        .expect_err("no such node");
    let m = format!("{e:?}");
    assert!(
        m.contains("req:nope") && m.to_lowercase().contains("no node"),
        "{m}"
    );
}

async fn delete(s: &ReflowService, args: serde_json::Value) -> Result<serde_json::Value, String> {
    s.delete_node(Parameters(
        serde_json::from_value(args).expect("delete_node's arguments"),
    ))
    .await
    .map(|r| r.structured_content.expect("structured content present"))
    .map_err(|e| format!("{e:?}"))
}

async fn holds(s: &ReflowService, node_type: &str, id: &str) -> bool {
    let out = j!(s.get_node(Parameters(
        serde_json::from_value(serde_json::json!({"id": id, "node_type": node_type})).unwrap()
    )));
    !out["node"].is_null()
}

async fn create(s: &ReflowService, node_type: &str, id: &str) {
    j!(s.create_node(Parameters(serde_json::from_value(serde_json::json!({
        "node_type": node_type, "id": id, "props": {"name": "Stand-in", "description": "for the delete rule"}
    })).unwrap())));
}

/// THE RULE THAT REPLACED "delete_node still requires the type on purpose":
/// with the type omitted, a holder whose type is not the one the id's prefix
/// names is REFUSED, nothing is deleted, and the refusal names what the id
/// holds and the way through. Field limitation 23, 2026-10-19.
#[tokio::test]
async fn delete_node_refuses_an_id_whose_prefix_names_another_type_and_the_node_survives() {
    let s = seeded().await;
    create(&s, "Capability", "req:impostor").await;
    let m = delete(&s, serde_json::json!({"id": "req:impostor"}))
        .await
        .expect_err("a Capability under a req: id is not what the caller said");
    assert!(
        m.contains("Capability") && m.contains("Requirement") && m.contains("node_type"),
        "names the holder, the prefix's type and the way through: {m}"
    );
    assert!(m.contains("Nothing was deleted"), "{m}");
    assert!(
        holds(&s, "Capability", "req:impostor").await,
        "it was deleted"
    );
    // A passed type behaves exactly as before: that is the way through.
    let out = delete(
        &s,
        serde_json::json!({"id": "req:impostor", "node_type": "Capability"}),
    )
    .await
    .expect("the stated type deletes it");
    assert_eq!(out["deleted"], true, "{out}");
    assert!(!holds(&s, "Capability", "req:impostor").await);
}

/// The id's prefix names the one type holding it: the delete goes ahead with no
/// type passed, and the reply says which type was deleted.
#[tokio::test]
async fn delete_node_with_a_bare_id_deletes_when_the_prefix_names_the_holders_type() {
    let s = seeded().await;
    let out = delete(&s, serde_json::json!({"id": "req:fast"}))
        .await
        .expect("req:fast is a Requirement");
    assert_eq!(out["deleted"], true, "{out}");
    assert_eq!(out["node_type"], "Requirement", "{out}");
    assert!(!holds(&s, "Requirement", "req:fast").await);
}

/// One id under two types: refused naming both, and both survive.
#[tokio::test]
async fn delete_node_with_a_bare_id_held_by_two_types_refuses_and_both_survive() {
    let s = seeded().await;
    create(&s, "Capability", "req:fast").await;
    let m = delete(&s, serde_json::json!({"id": "req:fast"}))
        .await
        .expect_err("two types hold req:fast");
    assert!(
        m.contains("Requirement") && m.contains("Capability") && m.contains("node_type"),
        "{m}"
    );
    assert!(holds(&s, "Requirement", "req:fast").await);
    assert!(holds(&s, "Capability", "req:fast").await);
}

/// A prefix that names no type by convention states no belief: refused, named,
/// and the node survives.
#[tokio::test]
async fn delete_node_with_a_bare_id_whose_prefix_names_no_type_refuses() {
    let s = seeded().await;
    create(&s, "Capability", "thing:x").await;
    let m = delete(&s, serde_json::json!({"id": "thing:x"}))
        .await
        .expect_err("thing: names no type");
    assert!(
        m.contains("Capability") && m.contains("thing:") && m.contains("node_type"),
        "{m}"
    );
    assert!(holds(&s, "Capability", "thing:x").await);
}

/// A missing id is reported, not an error, with or without the type.
#[tokio::test]
async fn delete_node_with_a_bare_id_that_names_nothing_reports_it() {
    let s = seeded().await;
    let out = delete(&s, serde_json::json!({"id": "req:nope"}))
        .await
        .expect("nothing to delete is not an error");
    assert_eq!(out["deleted"], false, "{out}");
}
