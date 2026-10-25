//! A CHANGED QUANTITY NAMES EVERY KEPT COMPUTATION THAT USED IT, AND A CHANGE
//! RETURNS ITS REDO LIST — over the MCP surface, as a client calls it.
//!
//! The web of rules @ajs accepted on 2026-10-24
//! (dec:idea-reflow2-becomes-the-web-of-rules-for-a-buildings-ifc-model, options
//! (b) and (c)), and its two requirements:
//!
//! - req:a-changed-quantity-names-every-kept-computation-that-used-it;
//! - req:a-change-to-a-physical-designs-part-returns-its-redo-list-by-trade.
//!
//! THE CASE, with the same shape as the Shop sample and none of its data
//! (fact:root-cause-a-changed-requirement-does-not-flow-because-the-accepted-
//! web-of-rules-was-never-built-2026-10-25): a household changed from four to
//! five. The food calc's input `persons` cited the requirement but held its own
//! copy of "4", so changing the requirement marked nothing owed; loop_status
//! read 0 stale and propagate_from reached two calcs and no model or drawing.
//!
//! Here the household is a DESIGN QUANTITY (a setpoint Constraint: 4 people),
//! the food calc is registered with that quantity as an input, the electricity
//! calc with the food calc as its input, and a drawing with the model it was
//! drawn from. Then the household becomes 5.
//!
//! THE DIVIDING LINE (2026-10-13) holds throughout: reflow2 never reads a
//! record and never recomputes. Whoever registers the record says which inputs
//! it took and what it used; the design compares that with what it holds now.
//!
//! OBSERVED FAILING FIRST on main 7e9209515: `add_artifact` refused `inputs`
//! as an unknown field, so every test below failed at its first registration.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "web-of-rules-probe".to_string();
        cfg
    }
}

struct Session {
    client: rmcp::service::RunningService<rmcp::service::RoleClient, Probe>,
}

async fn session() -> Session {
    let svc = ReflowService::in_memory().expect("in-memory service");
    let (server_rx, client_tx) = tokio::io::duplex(1 << 22);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 22);
    tokio::spawn(async move {
        if let Ok(running) = svc.serve((server_rx, server_tx)).await {
            let _ = running.waiting().await;
        }
    });
    let client = Probe
        .serve((client_rx, client_tx))
        .await
        .expect("in-process handshake");
    Session { client }
}

impl Session {
    async fn call(&self, tool: &str, args: Value) -> Result<Value, String> {
        let arguments = args.as_object().cloned().unwrap_or_default();
        match self
            .client
            .call_tool(
                rmcp::model::CallToolRequestParams::new(tool.to_string()).with_arguments(arguments),
            )
            .await
        {
            Ok(r) if r.is_error.unwrap_or(false) => Err(r
                .content
                .iter()
                .filter_map(|c| c.as_text().map(|t| t.text.clone()))
                .collect::<Vec<_>>()
                .join("\n")),
            Ok(r) => Ok(r.structured_content.unwrap_or(Value::Null)),
            Err(rmcp::service::ServiceError::McpError(e)) => Err(e.message.to_string()),
            Err(e) => Err(format!("transport failure: {e}")),
        }
    }

    async fn ok(&self, tool: &str, args: Value) -> Value {
        let shown = args.to_string();
        self.call(tool, args)
            .await
            .unwrap_or_else(|e| panic!("`{tool}` {shown} was refused: {e}"))
    }
}

/// The four-person household, as a design holds it once the number is a
/// quantity and not prose; two kept computations, the decisions and limit
/// that rest on them, the part one decision sizes, the model that realizes
/// the part, and a drawing of the model.
async fn the_household_of_four() -> Session {
    let s = session().await;
    s.ok(
        "add_project",
        json!({"id": "proj:farm", "name": "A small farm (fixture)"}),
    )
    .await;
    s.ok(
        "add_constraint",
        json!({"id": "cst:household", "name": "The household is four people",
               "statement": "The farm feeds the household that lives on it.",
               "quantity": "household", "unit": "people", "limit": 4.0,
               "direction": "setpoint", "limit_basis": "asserted",
               "limit_source": "who:owner"}),
    )
    .await;
    s.ok(
        "add_decision",
        json!({"id": "dec:growing-area", "name": "The grow loop is sized for the household",
               "decision": "Growing area and fish stock sized from the food calc."}),
    )
    .await;
    s.ok(
        "add_decision",
        json!({"id": "dec:solar-modules", "name": "The solar array is sized from the electricity calc",
               "decision": "Module count from the electricity calc."}),
    )
    .await;
    s.ok(
        "add_component",
        json!({"id": "cmp:grow-room", "name": "Grow room", "description": "The aquaponics room."}),
    )
    .await;
    s.ok(
        "add_component",
        json!({"id": "cmp:roof-array", "name": "Roof solar array", "description": "The PV modules."}),
    )
    .await;
    s.ok(
        "governed_by",
        json!({"from_id": "cmp:grow-room", "from_type": "Component",
               "to_id": "dec:growing-area", "to_type": "Decision"}),
    )
    .await;
    s.ok(
        "governed_by",
        json!({"from_id": "cmp:roof-array", "from_type": "Component",
               "to_id": "dec:solar-modules", "to_type": "Decision"}),
    )
    .await;
    // The food calc: its input `persons` is the household, and it used 4.
    s.ok(
        "add_artifact",
        json!({"id": "art:food-calc", "name": "food.calc.json", "artifact_type": "data",
               "location": "food.calc.json", "checksum": "sha256:food-1",
               "inputs": [{"node_id": "cst:household", "input": "persons",
                           "value": 4.0, "unit": "people"}]}),
    )
    .await;
    s.ok(
        "documents",
        json!({"artifact_id": "art:food-calc", "target_id": "dec:growing-area",
               "target_type": "Decision"}),
    )
    .await;
    // The electricity calc took the food calc's pump hours as an input; its
    // version is the one the design holds now, so nobody need state it.
    s.ok(
        "add_artifact",
        json!({"id": "art:power-calc", "name": "power.calc.json", "artifact_type": "data",
               "location": "power.calc.json", "checksum": "sha256:power-1",
               "inputs": [{"node_id": "art:food-calc", "input": "pump_hours"}]}),
    )
    .await;
    s.ok(
        "documents",
        json!({"artifact_id": "art:power-calc", "target_id": "dec:solar-modules",
               "target_type": "Decision"}),
    )
    .await;
    // A limit whose number the electricity calc computed.
    s.ok(
        "add_constraint",
        json!({"id": "cst:array-kw", "name": "The array delivers at least 6.2 kW",
               "statement": "Peak array output.", "quantity": "array_power", "unit": "kW",
               "limit": 6.2, "direction": "minimum", "limit_basis": "computed",
               "limit_source": "art:power-calc"}),
    )
    .await;
    // The building model realizes the grow room; the plan is drawn from it.
    s.ok(
        "add_artifact",
        json!({"id": "art:model", "name": "farm.ifc", "artifact_type": "model",
               "location": "farm.ifc", "checksum": "sha256:model-1"}),
    )
    .await;
    s.ok(
        "realizes",
        json!({"artifact_id": "art:model", "target_id": "cmp:grow-room",
               "target_type": "Component"}),
    )
    .await;
    s.ok(
        "add_artifact",
        json!({"id": "art:plan", "name": "ground-floor-plan.svg", "artifact_type": "drawing",
               "location": "ground-floor-plan.svg", "checksum": "sha256:plan-1",
               "inputs": [{"node_id": "art:model", "checksum": "sha256:model-1"}]}),
    )
    .await;
    s
}

fn ids(rows: &Value) -> Vec<String> {
    rows.as_array()
        .unwrap_or_else(|| panic!("expected a list, got {rows}"))
        .iter()
        .map(|r| {
            r.get("node_id")
                .or_else(|| r.get("artifact_id"))
                .and_then(Value::as_str)
                .unwrap_or_else(|| panic!("a row with no id: {r}"))
                .to_string()
        })
        .collect()
}

fn row<'a>(rows: &'a Value, id: &str) -> &'a Value {
    rows.as_array()
        .and_then(|a| {
            a.iter().find(|r| {
                r.get("node_id")
                    .or_else(|| r.get("artifact_id"))
                    .and_then(Value::as_str)
                    == Some(id)
            })
        })
        .unwrap_or_else(|| panic!("{id} is not listed in {rows}"))
}

#[tokio::test]
async fn registering_a_record_draws_an_edge_per_input_and_says_what_it_used() {
    let s = the_household_of_four().await;
    let food = s
        .ok(
            "get_node",
            json!({"id": "art:food-calc", "include_edges": true}),
        )
        .await;
    let inputs: Vec<&Value> = food["edges"]["items"]
        .as_array()
        .expect("edges")
        .iter()
        .filter(|e| e["edge_type"] == "DEPENDS_ON" && e["direction"] == "out")
        .collect();
    assert_eq!(inputs.len(), 1, "one edge per input: {food}");
    let e = inputs[0];
    assert_eq!(e["to_id"], "cst:household");
    assert_eq!(e["properties"]["input"], "persons");
    assert_eq!(e["properties"]["value_used"], 4.0);
    assert_eq!(e["properties"]["unit_used"], "people");
    assert_eq!(e["properties"]["used_basis"], "stated");

    // The upstream computation's version nobody stated is the one the design
    // held when the record was registered, and the edge says so.
    let power = s
        .ok(
            "get_node",
            json!({"id": "art:power-calc", "include_edges": true}),
        )
        .await;
    let up = power["edges"]["items"]
        .as_array()
        .expect("edges")
        .iter()
        .find(|e| e["edge_type"] == "DEPENDS_ON" && e["to_id"] == "art:food-calc")
        .unwrap_or_else(|| panic!("no input edge to the food calc: {power}"));
    assert_eq!(up["properties"]["checksum_used"], "sha256:food-1");
    assert_eq!(up["properties"]["used_basis"], "current");
}

#[tokio::test]
async fn nothing_is_owed_while_every_record_used_what_the_design_holds() {
    let s = the_household_of_four().await;
    let ls = s.ok("loop_status", json!({})).await;
    assert_eq!(ls["computations_owed"], 0, "{ls}");
    assert!(
        ls["owed_computations"]
            .as_array()
            .is_some_and(Vec::is_empty),
        "{ls}"
    );
}

#[tokio::test]
async fn four_to_five_marks_the_food_calc_owed_and_the_electricity_calc_behind_it() {
    let s = the_household_of_four().await;
    s.ok(
        "add_constraint",
        json!({"id": "cst:household", "limit": 5.0}),
    )
    .await;
    let ls = s.ok("loop_status", json!({})).await;

    assert_eq!(ls["computations_owed"], 2, "{ls}");
    let owed = &ls["owed_computations"];
    assert_eq!(ids(owed), vec!["art:food-calc", "art:power-calc"], "{ls}");
    let food = row(owed, "art:food-calc");
    assert_eq!(food["distance"], 1);
    let why = &food["because"][0];
    assert_eq!(why["input_id"], "cst:household");
    assert_eq!(why["reason"], "quantity_changed");
    assert_eq!(why["was"], "4 people");
    assert_eq!(why["now"], "5 people");
    let power = row(owed, "art:power-calc");
    assert_eq!(power["distance"], 2, "transitive, one step further: {ls}");
    assert_eq!(power["because"][0]["reason"], "input_owed");

    // Every decision or limit resting on an owed number reads as resting on
    // a stale one.
    let resting = &ls["resting_on_stale_numbers"];
    let mut r = ids(resting);
    r.sort();
    assert_eq!(
        r,
        vec!["cst:array-kw", "dec:growing-area", "dec:solar-modules"],
        "{ls}"
    );
    assert_eq!(
        row(resting, "dec:growing-area")["rests_on"][0],
        "art:food-calc"
    );
    assert_eq!(row(resting, "cst:array-kw")["distance"], 3);

    assert_eq!(ls["clean"], false, "an owed re-run is debt: {ls}");
    let next = ls["next"].to_string();
    assert!(next.contains("re-run"), "`next` says what to do: {next}");
}

#[tokio::test]
async fn a_rerun_clears_its_record_and_owes_the_one_downstream() {
    let s = the_household_of_four().await;
    s.ok(
        "add_constraint",
        json!({"id": "cst:household", "limit": 5.0}),
    )
    .await;
    // The calculator re-ran the food calc with 5: a new version, registered
    // again with what it used.
    s.ok(
        "set_artifact_checksum",
        json!({"artifact_id": "art:food-calc", "checksum": "sha256:food-2",
               "disposition": "design_holds", "change_type": "resync"}),
    )
    .await;
    s.ok(
        "add_artifact",
        json!({"id": "art:food-calc",
               "inputs": [{"node_id": "cst:household", "input": "persons",
                           "value": 5.0, "unit": "people"}]}),
    )
    .await;
    let ls = s.ok("loop_status", json!({})).await;
    assert_eq!(ls["computations_owed"], 1, "{ls}");
    let power = row(&ls["owed_computations"], "art:power-calc");
    assert_eq!(power["distance"], 1, "{ls}");
    assert_eq!(power["because"][0]["reason"], "input_re_run");
    assert_eq!(power["because"][0]["was"], "sha256:food-1");
    assert_eq!(power["because"][0]["now"], "sha256:food-2");

    s.ok(
        "set_artifact_checksum",
        json!({"artifact_id": "art:power-calc", "checksum": "sha256:power-2",
               "disposition": "design_holds", "change_type": "resync"}),
    )
    .await;
    s.ok(
        "add_artifact",
        json!({"id": "art:power-calc",
               "inputs": [{"node_id": "art:food-calc", "input": "pump_hours"}]}),
    )
    .await;
    let ls = s.ok("loop_status", json!({})).await;
    assert_eq!(ls["computations_owed"], 0, "{ls}");
}

#[tokio::test]
async fn the_redo_list_of_the_household_reaches_the_math_the_part_the_model_and_the_drawing() {
    let s = the_household_of_four().await;
    let r = s
        .ok("propagate_from", json!({"seed_ids": ["cst:household"]}))
        .await;
    let redo = &r["redo_list"];
    assert!(redo.is_object(), "propagate_from carries a redo list: {r}");
    assert_eq!(
        ids(&redo["computations"]),
        vec!["art:food-calc", "art:power-calc"],
        "{r}"
    );
    assert_eq!(row(&redo["computations"], "art:power-calc")["distance"], 2);
    let mut d = ids(&redo["decisions_and_limits"]);
    d.sort();
    assert_eq!(
        d,
        vec!["cst:array-kw", "dec:growing-area", "dec:solar-modules"],
        "{r}"
    );
    let mut p = ids(&redo["parts"]);
    p.sort();
    assert_eq!(p, vec!["cmp:grow-room", "cmp:roof-array"], "{r}");
    assert_eq!(ids(&redo["models"]), vec!["art:model"], "{r}");
    assert_eq!(ids(&redo["drawings"]), vec!["art:plan"], "{r}");
}

#[tokio::test]
async fn a_changed_part_returns_the_parts_that_depend_on_it_its_model_and_its_drawings() {
    let s = the_household_of_four().await;
    s.ok(
        "add_component",
        json!({"id": "cmp:foundation", "name": "Foundation", "description": "Slab and footings."}),
    )
    .await;
    s.ok(
        "depends_on",
        json!({"from_id": "cmp:grow-room", "from_type": "Component",
               "to_id": "cmp:foundation", "to_type": "Component"}),
    )
    .await;
    let r = s
        .ok("propagate_from", json!({"seed_ids": ["cmp:foundation"]}))
        .await;
    let redo = &r["redo_list"];
    assert_eq!(ids(&redo["parts"]), vec!["cmp:grow-room"], "{r}");
    assert_eq!(ids(&redo["models"]), vec!["art:model"], "{r}");
    assert_eq!(ids(&redo["drawings"]), vec!["art:plan"], "{r}");
}

#[tokio::test]
async fn an_input_that_is_not_a_quantity_or_a_kept_record_is_refused_by_name() {
    let s = the_household_of_four().await;
    let refused = s
        .call(
            "add_artifact",
            json!({"id": "art:food-calc",
                   "inputs": [{"node_id": "dec:growing-area", "input": "persons"}]}),
        )
        .await
        .expect_err("a Decision holds no number a record can have used");
    assert!(
        refused.contains("dec:growing-area") && refused.contains("limit"),
        "the refusal names the input and what would work: {refused}"
    );
    let refused = s
        .call(
            "add_artifact",
            json!({"id": "art:food-calc",
                   "inputs": [{"node_id": "cst:household", "value": 4.0, "unit": "adults"}]}),
        )
        .await
        .expect_err("a value in another unit than the design holds it in");
    assert!(refused.contains("people"), "{refused}");
    let refused = s
        .call(
            "add_artifact",
            json!({"id": "art:food-calc", "inputs": [{"node_id": "cst:nothing-here"}]}),
        )
        .await
        .expect_err("an input naming nothing");
    assert!(refused.contains("cst:nothing-here"), "{refused}");
}
