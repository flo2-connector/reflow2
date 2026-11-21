//! AN ASSEMBLY FLOW READS BACK AS INSTRUCTIONS, over the MCP surface.
//! The core's `an_assembly_flow_reads_back_as_instructions.rs` pins the
//! reading; this file pins what a client writes and reads through the real
//! tools (req:a-step-of-a-process-names-the-parts-it-puts-together):
//!
//! - `part_of_flow` takes `puts_together` (Component ids) and `closes`
//!   (Interface ids) and names every edge it drew in `edges_drawn`; a step
//!   sent again names what was already there apart;
//! - a call naming a wrong id is refused WHOLE, naming each, and writes
//!   nothing — on its own and as an item of `draw_edges`;
//! - `draw_edges` runs the same body, and its `drawn` names the step's parts
//!   and joins too;
//! - `flow_report` reads the assembly as instructions, and a bare step's reply
//!   is the edge it always was;
//! - `propagate_from` on a join reaches the step that closes it.
//!
//! The case is the core's: a hibiscus flower ring cast in sterling silver in
//! one pour.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "assembly-probe".to_string();
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

    /// The ring's parts, joins, people, flow and steps, through the tools a
    /// client uses. Each step is recorded with its parts and joins in ONE call.
    async fn ring(&self) {
        for (id, name, purpose) in [
            ("cmp:band", "Band", "the ring that goes round the finger"),
            ("cmp:cup", "Cup", "the flower's cup, on the band"),
            ("cmp:petals", "Petals", "the five petals, round the cup"),
            (
                "cmp:stamen",
                "Stamen",
                "the stamen, with its bezel at the tip",
            ),
            ("cmp:stone", "Stone", "a 2 mm white sapphire, bought"),
        ] {
            self.ok(
                "add_component",
                json!({"id": id, "name": name, "description": purpose}),
            )
            .await;
        }
        for (id, name, holds, provider, consumer) in [
            (
                "ifc:cup-to-band",
                "Cup to band",
                "fused in the casting",
                "cmp:band",
                "cmp:cup",
            ),
            (
                "ifc:petals-to-cup",
                "Petals to cup",
                "fused in the casting",
                "cmp:cup",
                "cmp:petals",
            ),
            (
                "ifc:stamen-to-cup",
                "Stamen to cup",
                "fused in the casting",
                "cmp:cup",
                "cmp:stamen",
            ),
            (
                "ifc:stone-in-bezel",
                "Stone in bezel",
                "the stone sits in the bezel seat with 0.05 mm a side; the lip is pushed over it",
                "cmp:stamen",
                "cmp:stone",
            ),
        ] {
            self.ok(
                "add_interface",
                json!({"id": id, "name": name, "description": holds}),
            )
            .await;
            self.ok("provides", json!({"from_id": provider, "to_id": id}))
                .await;
            self.ok("consumes", json!({"from_id": consumer, "to_id": id}))
                .await;
        }
        self.ok(
            "add_flow",
            json!({
                "id": "flow:make-the-ring",
                "name": "Make the hibiscus ring",
                "flow_type": "assembly",
            }),
        )
        .await;
        for (id, name, who, order, parts, joins) in [
            (
                "cap:print-the-pattern",
                "Print the pattern",
                "act:print-service",
                1,
                json!(["cmp:band", "cmp:cup", "cmp:petals", "cmp:stamen"]),
                json!([]),
            ),
            (
                "cap:cast-it",
                "Cast it",
                "act:caster",
                2,
                json!([]),
                json!(["ifc:cup-to-band", "ifc:petals-to-cup", "ifc:stamen-to-cup"]),
            ),
            (
                "cap:clean-up-and-polish",
                "Clean up and polish",
                "act:owner",
                3,
                json!([]),
                json!([]),
            ),
            (
                "cap:plate-it-rose-gold",
                "Plate it rose gold",
                "act:plater",
                4,
                json!([]),
                json!([]),
            ),
            (
                "cap:set-the-stone",
                "Set the stone",
                "act:jeweler",
                5,
                json!(["cmp:stone"]),
                json!(["ifc:stone-in-bezel"]),
            ),
        ] {
            self.ok(
                "add_capability",
                json!({"id": id, "name": name, "description": format!("{name}, a step")}),
            )
            .await;
            self.ok(
                "part_of_flow",
                json!({
                    "capability_id": id,
                    "flow_id": "flow:make-the-ring",
                    "step_order": order,
                    "puts_together": parts,
                    "closes": joins,
                }),
            )
            .await;
            self.ok("add_actor", json!({"id": who, "name": who})).await;
            self.ok(
                "create_edge",
                json!({
                    "edge_type": "INTERACTS_WITH",
                    "from_id": who,
                    "to_id": id,
                    "props": {"interaction": "performs"},
                }),
            )
            .await;
        }
        self.ok(
            "add_verification",
            json!({
                "id": "ver:casting-check",
                "name": "The casting came out whole",
                "method": "analysis",
                "status": "passing",
                "verifies": [{"target_id": "cap:cast-it"}],
            }),
        )
        .await;
        self.ok(
            "add_verification",
            json!({
                "id": "ver:setting-inspection",
                "name": "The stone is held and does not turn",
                "method": "inspection",
                "verifies": [{"target_id": "cap:set-the-stone"}],
            }),
        )
        .await;
    }

    async fn report(&self) -> Value {
        self.ok("flow_report", json!({"flow_id": "flow:make-the-ring"}))
            .await
    }
}

fn step<'a>(report: &'a Value, id: &str) -> &'a Value {
    report["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .find(|s| s["capability_id"] == id)
        .unwrap_or_else(|| panic!("{id} in {report}"))
}

fn ids(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|x| x["id"].as_str().unwrap_or_default().to_string())
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn the_ring_written_through_the_tools_reads_back_as_instructions() {
    let s = session().await;
    s.ring().await;
    let r = s.report().await;
    assert_eq!(r["flow_type"], "assembly");
    let order: Vec<&str> = r["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["capability_id"].as_str().unwrap())
        .collect();
    assert_eq!(
        order,
        [
            "cap:print-the-pattern",
            "cap:cast-it",
            "cap:clean-up-and-polish",
            "cap:plate-it-rose-gold",
            "cap:set-the-stone",
        ]
    );
    let print = step(&r, "cap:print-the-pattern");
    assert_eq!(
        ids(&print["puts_together"]),
        ["cmp:band", "cmp:cup", "cmp:petals", "cmp:stamen"]
    );
    assert_eq!(ids(&print["performed_by"]), ["act:print-service"]);
    assert!(print.get("closes").is_none() && print.get("checks").is_none());
    let cast = step(&r, "cap:cast-it");
    assert_eq!(
        ids(&cast["closes"]),
        ["ifc:cup-to-band", "ifc:petals-to-cup", "ifc:stamen-to-cup"]
    );
    assert_eq!(cast["closes"][0]["what_must_hold"], "fused in the casting");
    assert_eq!(cast["checks"][0]["id"], "ver:casting-check");
    assert_eq!(cast["checks"][0]["method"], "analysis");
    assert_eq!(cast["checks"][0]["status"], "passing");
    let set = step(&r, "cap:set-the-stone");
    assert_eq!(ids(&set["puts_together"]), ["cmp:stone"]);
    assert_eq!(
        set["closes"][0]["what_must_hold"],
        "the stone sits in the bezel seat with 0.05 mm a side; the lip is pushed over it"
    );
    assert_eq!(ids(&set["performed_by"]), ["act:jeweler"]);
    assert_eq!(set["checks"][0]["method"], "inspection");
    // The plating step is its name, its place and who does it.
    let plate = step(&r, "cap:plate-it-rose-gold");
    let mut keys: Vec<&str> = plate
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["capability_id", "name", "performed_by", "step_order"]
    );

    let confessions: Vec<&str> = r["confessions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    assert_eq!(confessions.len(), 2, "{confessions:#?}");
    assert!(confessions[0].starts_with("2 of 5 step(s) put together no part and close no join"));
    assert!(confessions[1].starts_with(
        "3 of 5 step(s) have no check — cap:print-the-pattern, cap:clean-up-and-polish, \
         cap:plate-it-rose-gold:"
    ));
}

#[tokio::test]
async fn a_step_names_every_edge_it_drew_and_apart_what_was_there() {
    let s = session().await;
    s.ring().await;
    let again = s
        .ok(
            "part_of_flow",
            json!({
                "capability_id": "cap:set-the-stone",
                "flow_id": "flow:make-the-ring",
                "puts_together": ["cmp:stone"],
                "closes": ["ifc:stone-in-bezel"],
            }),
        )
        .await;
    assert_eq!(again["edge_type"], "PART_OF_FLOW");
    assert_eq!(again["properties"]["step_order"], 5, "the place is kept");
    assert_eq!(again["edges_drawn"], json!([]));
    assert_eq!(
        again["edges_already_present"],
        json!([
            "cap:set-the-stone PUTS_TOGETHER cmp:stone",
            "cap:set-the-stone CLOSES ifc:stone-in-bezel",
        ])
    );

    // A new step names what it drew.
    s.ok(
        "add_capability",
        json!({"id": "cap:check-the-setting", "name": "Check the setting", "description": "a look"}),
    )
    .await;
    let drawn = s
        .ok(
            "part_of_flow",
            json!({
                "capability_id": "cap:check-the-setting",
                "flow_id": "flow:make-the-ring",
                "step_order": 6,
                "closes": ["ifc:stone-in-bezel"],
            }),
        )
        .await;
    assert_eq!(
        drawn["edges_drawn"],
        json!(["cap:check-the-setting CLOSES ifc:stone-in-bezel"])
    );

    // A bare step is the edge it always was, with no block.
    s.ok(
        "add_capability",
        json!({"id": "cap:wrap-it", "name": "Wrap it", "description": "for the box"}),
    )
    .await;
    let bare = s
        .ok(
            "part_of_flow",
            json!({"capability_id": "cap:wrap-it", "flow_id": "flow:make-the-ring", "step_order": 7}),
        )
        .await;
    assert!(bare.get("edges_drawn").is_none(), "{bare}");
    assert!(bare.get("edges_already_present").is_none(), "{bare}");
    assert_eq!(bare["edge_type"], "PART_OF_FLOW");
}

#[tokio::test]
async fn a_step_naming_a_wrong_part_or_join_is_refused_whole_and_writes_nothing() {
    let s = session().await;
    s.ring().await;
    s.ok(
        "add_capability",
        json!({"id": "cap:polish-again", "name": "Polish again", "description": "once more"}),
    )
    .await;
    let err = s
        .call(
            "part_of_flow",
            json!({
                "capability_id": "cap:polish-again",
                "flow_id": "flow:make-the-ring",
                "step_order": 6,
                "puts_together": ["cmp:band", "ifc:cup-to-band", "cmp:nothing"],
                "closes": ["cmp:stone"],
            }),
        )
        .await
        .expect_err("refused");
    for said in [
        "NOTHING WAS WRITTEN",
        "puts_together: 'ifc:cup-to-band' is an Interface, not a Component",
        "puts_together: 'cmp:nothing' names nothing in this design",
        "closes: 'cmp:stone' is a Component, not an Interface",
    ] {
        assert!(err.contains(said), "{said} — in: {err}");
    }
    let r = s.report().await;
    assert!(
        r["steps"]
            .as_array()
            .unwrap()
            .iter()
            .all(|st| st["capability_id"] != "cap:polish-again"),
        "the refused step was written: {r}"
    );
    let node = s.ok("get_node", json!({"id": "cap:polish-again"})).await;
    let text = node.to_string();
    for e in ["PART_OF_FLOW", "PUTS_TOGETHER", "CLOSES"] {
        assert!(
            !text.contains(e),
            "{e} was written by a refused call: {text}"
        );
    }

    // The same item in a batch refuses the batch, and the batch writes nothing.
    let batch = s
        .ok(
            "draw_edges",
            json!({"edges": [
                {"tool": "part_of_flow", "arguments": {
                    "capability_id": "cap:polish-again",
                    "flow_id": "flow:make-the-ring",
                    "puts_together": ["cmp:band"],
                }},
                {"tool": "part_of_flow", "arguments": {
                    "capability_id": "cap:polish-again",
                    "flow_id": "flow:make-the-ring",
                    "closes": ["cmp:stone"],
                }},
            ]}),
        )
        .await;
    assert_eq!(batch["applied"], false, "{batch}");
    assert!(
        batch["failures"][0]["error"]
            .as_str()
            .unwrap()
            .contains("closes: 'cmp:stone' is a Component, not an Interface"),
        "{batch}"
    );
    let text = s
        .ok("get_node", json!({"id": "cap:polish-again"}))
        .await
        .to_string();
    assert!(!text.contains("PUTS_TOGETHER"), "{text}");
}

#[tokio::test]
async fn draw_edges_draws_a_step_with_its_parts_and_names_them_all() {
    let s = session().await;
    s.ring().await;
    s.ok(
        "add_capability",
        json!({"id": "cap:size-it", "name": "Size it", "description": "to the finger"}),
    )
    .await;
    let batch = s
        .ok(
            "draw_edges",
            json!({"edges": [{"tool": "part_of_flow", "arguments": {
                "capability_id": "cap:size-it",
                "flow_id": "flow:make-the-ring",
                "step_order": 6,
                "puts_together": ["cmp:band"],
                "closes": ["ifc:cup-to-band"],
            }}]}),
        )
        .await;
    assert_eq!(batch["applied"], true, "{batch}");
    assert_eq!(
        batch["drawn"],
        json!([
            "cap:size-it PART_OF_FLOW flow:make-the-ring",
            "cap:size-it PUTS_TOGETHER cmp:band",
            "cap:size-it CLOSES ifc:cup-to-band",
        ])
    );
    let r = s.report().await;
    let size = step(&r, "cap:size-it");
    assert_eq!(ids(&size["puts_together"]), ["cmp:band"]);
    assert_eq!(ids(&size["closes"]), ["ifc:cup-to-band"]);
}

#[tokio::test]
async fn a_changed_join_reaches_the_step_that_closes_it() {
    let s = session().await;
    s.ring().await;
    let radius = s
        .ok(
            "propagate_from",
            json!({"seed_ids": ["ifc:stone-in-bezel"], "full": true}),
        )
        .await;
    let text = radius.to_string();
    assert!(text.contains("cap:set-the-stone"), "{text}");
    let radius = s
        .ok(
            "propagate_from",
            json!({"seed_ids": ["cmp:band"], "full": true}),
        )
        .await;
    assert!(
        radius.to_string().contains("cap:print-the-pattern"),
        "{radius}"
    );
}
