//! `propagate_from` with `arriving_from`: a design works out for itself where a
//! ripple from another design enters it, from its own pins, so the hub skill
//! carries a change from member to member knowing none of their ids
//! (`cap:a-cross-design-ripple-follows-each-members-relation`).

use reflow2_mcp::service::*;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

macro_rules! j {
    ($call:expr) => {
        $call
            .await
            .expect("tool ok")
            .structured_content
            .expect("structured content present")
    };
}

macro_rules! call {
    ($s:expr, $tool:ident, $args:expr) => {
        j!($s.$tool(Parameters(serde_json::from_value($args).unwrap())))
    };
}

/// B provides interface I; A consumes it and pins B as a peer it uses across I.
async fn two_designs() -> (ReflowService, ReflowService) {
    let b = ReflowService::in_memory_as("design-b").unwrap();
    call!(b, add_project, json!({"id": "proj:b", "name": "B"}));
    call!(b, add_interface, json!({"id": "ifc:i", "name": "I"}));
    call!(
        b,
        add_component,
        json!({"id": "cmp:b-engine", "name": "B's engine", "description": "serves I"})
    );
    call!(
        b,
        provides,
        json!({"from_id": "cmp:b-engine", "to_id": "ifc:i"})
    );

    let a = ReflowService::in_memory_as("design-a").unwrap();
    call!(a, add_project, json!({"id": "proj:a", "name": "A"}));
    call!(
        a,
        add_interface,
        json!({"id": "ifc:i", "name": "I, as A uses it"})
    );
    call!(
        a,
        add_component,
        json!({"id": "cmp:a-client", "name": "A's client", "description": "calls I"})
    );
    call!(
        a,
        consumes,
        json!({"from_id": "cmp:a-client", "to_id": "ifc:i"})
    );
    call!(
        a,
        external_dependency,
        json!({"id": "dep:b", "name": "B", "source": "https://example.org/b", "version": "v1",
               "graph_id": "design-b", "relation": ["uses"], "interfaces": ["ifc:i"]})
    );
    (a, b)
}

fn ids(radius: &Value) -> Vec<String> {
    radius["direct_ring"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|n| n["node_id"].as_str().map(str::to_string))
        .collect()
}

#[tokio::test]
async fn a_member_that_uses_the_changed_design_takes_the_ripple_across_its_interface() {
    let (a, b) = two_designs().await;
    let in_b = call!(b, propagate_from, json!({"seed_ids": ["ifc:i"]}));
    assert_eq!(in_b["design"], json!("design-b"));
    assert_eq!(in_b["interfaces_reached"], json!(["ifc:i"]));

    let in_a = call!(
        a,
        propagate_from,
        json!({"arriving_from": "design-b", "interfaces": in_b["interfaces_reached"]})
    );
    assert_eq!(in_a["arrived"], json!(true), "{in_a}");
    assert_eq!(in_a["design"], json!("design-a"));
    assert_eq!(in_a["arrived_via"][0]["direction"], json!("across"));
    assert_eq!(in_a["arrived_via"][0]["seeds"], json!(["ifc:i"]));
    assert!(ids(&in_a).contains(&"cmp:a-client".to_string()), "{in_a}");
    assert!(
        in_a.get("continue_in").is_none(),
        "the radius does not send the ripple back where it came from: {in_a}"
    );
}

#[tokio::test]
async fn a_design_with_no_recorded_way_in_says_so_and_walks_nothing() {
    let (_a, b) = two_designs().await;
    // B names no other design, so nothing arrives from A.
    let out = call!(
        b,
        propagate_from,
        json!({"arriving_from": "design-a", "interfaces": ["ifc:i"]})
    );
    assert_eq!(out["arrived"], json!(false), "{out}");
    assert!(
        out["note"].as_str().is_some_and(|n| n.contains("design-a")),
        "{out}"
    );
}

#[tokio::test]
async fn seeds_and_arriving_from_together_or_neither_are_refused() {
    let (a, _b) = two_designs().await;
    let both = a
        .propagate_from(Parameters(
            serde_json::from_value(json!({"seed_ids": ["ifc:i"], "arriving_from": "design-b"}))
                .unwrap(),
        ))
        .await;
    assert!(both.is_err(), "both is ambiguous and refused");
    let neither = a
        .propagate_from(Parameters(serde_json::from_value(json!({})).unwrap()))
        .await;
    assert!(neither.is_err(), "neither has nothing to walk from");
}

#[tokio::test]
async fn a_row_for_an_interface_a_uses_pin_names_carries_the_members_design() {
    let (a, _b) = two_designs().await;
    let r = call!(a, propagate_from, json!({"seed_ids": ["cmp:a-client"]}));
    let row = r["direct_ring"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["node_id"] == json!("ifc:i"))
        .unwrap_or_else(|| panic!("ifc:i is one hop from its consumer: {r}"));
    assert_eq!(row["design"], json!("design-b"), "{row}");
    assert_eq!(r["continue_in"][0]["design"], json!("design-b"), "{r}");
    assert_eq!(r["continue_in"][0]["direction"], json!("across"));
}
