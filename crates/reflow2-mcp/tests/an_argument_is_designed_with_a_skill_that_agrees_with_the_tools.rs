//! AN ARGUMENT IS DESIGNED WITH A SERVED SKILL THAT AGREES WITH THE TOOLS —
//! `req:a-served-skill-designs-an-argument-for-and-against` (Anthony
//! 2026-10-19, the trolley trial's item 3), over the MCP surface, with the two
//! readings the skill teaches: an objection aimed at a step
//! (`req:an-objection-aimed-at-a-step-is-read-against-that-step`) and a
//! position held on several lines
//! (`req:a-claim-with-several-lines-reads-each-lines-assumptions`).
//!
//! Measured in the trolley trial on 0.87.0: no served skill covered an
//! argument, and design-a-proof said it was "Not for an argument for and
//! against a choice (that is a Decision)", so the designer followed it anyway.
//! In the cogito trial's AI branch a chat that read no skill drew rival
//! verdicts as one CONTRADICTS and one side silently read refuted.
//!
//! - `find_skills` finds `design-an-argument` from "argument", "line of
//!   reasoning" and "for and against", and design-a-proof still from its own
//!   words; design-a-proof points to it instead of excluding the case.
//! - The skill's recipe, run through the tools a client calls, reads as the
//!   skill says: rival positions with a contradiction each way both read
//!   contested; an objection drawn to a step's own id contests the step and
//!   the read-back lists it; a position two principles support names both.

use reflow2_mcp::service::ReflowService;
use reflow2_mcp::skills::SKILLS;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "argument-probe".to_string();
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
    async fn ok(&self, tool: &str, args: Value) -> Value {
        let shown = args.to_string();
        let arguments = args.as_object().cloned().unwrap_or_default();
        match self
            .client
            .call_tool(
                rmcp::model::CallToolRequestParams::new(tool.to_string()).with_arguments(arguments),
            )
            .await
        {
            Ok(r) if r.is_error.unwrap_or(false) => panic!(
                "`{tool}` {shown} was refused: {}",
                r.content
                    .iter()
                    .filter_map(|c| c.as_text().map(|t| t.text.clone()))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            Ok(r) => r.structured_content.unwrap_or(Value::Null),
            Err(e) => panic!("`{tool}` {shown} failed: {e}"),
        }
    }
}

fn skill(name: &str) -> &'static str {
    SKILLS
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no served skill named {name}"))
        .body
}

#[tokio::test]
async fn find_skills_finds_the_argument_skill_from_the_ways_people_ask() {
    let s = session().await;
    for query in [
        "argument",
        "line of reasoning",
        "for and against",
        "design an argument",
    ] {
        let found = s
            .ok("find_skills", json!({"query": query, "limit": 3}))
            .await;
        assert_eq!(
            found["items"][0]["name"], "design-an-argument",
            "{query}: {found:#}"
        );
    }
    for query in ["design a proof", "prove a theorem"] {
        let found = s
            .ok("find_skills", json!({"query": query, "limit": 3}))
            .await;
        assert_eq!(
            found["items"][0]["name"], "design-a-proof",
            "{query}: {found:#}"
        );
    }
}

#[test]
fn design_a_proof_points_to_the_argument_skill_instead_of_excluding_the_case() {
    let proof = skill("design-a-proof");
    assert!(
        !proof.contains("(that is a Decision)"),
        "design-a-proof no longer sends an argument for and against to a Decision"
    );
    let description = proof
        .lines()
        .find(|l| l.starts_with("description:"))
        .expect("a description");
    assert!(
        description.contains("design-an-argument"),
        "design-a-proof's description names the argument skill: {description}"
    );
    let argument = skill("design-an-argument");
    for taught in [
        // An objection aimed at the step itself, and how the step reads.
        "the step's own id",
        "`objected`",
        "`defeated`",
        // Rival positions: a contradiction each way, never one way.
        "Never draw a single CONTRADICTS",
        "a CONTRADICTS each way",
        // A position held on several lines names every one.
        "`holds_under_any_of`",
        // A read-back per question.
        "`steps_objected`",
    ] {
        assert!(argument.contains(taught), "the skill teaches {taught:?}");
    }
}

/// THE RECIPE THROUGH THE WIRE, the trolley's footbridge in miniature: a case
/// given, two principles chosen, a position per principle, the rival positions
/// kept as a contradiction each way, a third principle supporting the
/// impermissible side, and the closeness objection drawn to the double-effect
/// step's own id.
#[tokio::test]
async fn the_recipe_reads_as_the_skill_says() {
    let s = session().await;
    s.ok(
        "add_constraint",
        json!({"id": "con:case-footbridge", "name": "Footbridge",
               "statement": "A stranger on a footbridge; pushing him stops the trolley.",
               "statement_kind": "proposition", "taken_as_given": true}),
    )
    .await;
    for (id, name) in [
        ("con:pr-act-consequentialism", "Act consequentialism"),
        ("con:pr-double-effect", "Double effect"),
        ("con:pr-mere-means", "Never merely as a means"),
    ] {
        s.ok(
            "add_constraint",
            json!({"id": id, "name": name, "statement": format!("The principle: {name}."),
                   "statement_kind": "proposition"}),
        )
        .await;
    }
    for (id, name) in [
        ("req:v-push-permissible", "Footbridge: pushing permissible"),
        (
            "req:v-push-impermissible",
            "Footbridge: pushing impermissible",
        ),
    ] {
        s.ok(
            "add_requirement",
            json!({"id": id, "name": name, "statement": format!("POSITION. {name}."),
                   "statement_kind": "proposition"}),
        )
        .await;
    }
    for (id, principle, position) in [
        (
            "inf:push-by-act-consequentialism",
            "con:pr-act-consequentialism",
            "req:v-push-permissible",
        ),
        (
            "inf:push-by-double-effect",
            "con:pr-double-effect",
            "req:v-push-impermissible",
        ),
        (
            "inf:push-by-mere-means",
            "con:pr-mere-means",
            "req:v-push-impermissible",
        ),
    ] {
        s.ok(
            "add_inference",
            json!({"id": id, "premises": ["con:case-footbridge"], "assumed": [principle],
                   "conclusion": position, "rule": "all",
                   "rule_name": "application of a principle to a case",
                   "warrant": "The principle gives this verdict on the case."}),
        )
        .await;
    }
    // Rival positions: a contradiction each way.
    for (from, to) in [
        ("req:v-push-permissible", "req:v-push-impermissible"),
        ("req:v-push-impermissible", "req:v-push-permissible"),
    ] {
        s.ok(
            "review_relations",
            json!({"node_id": from, "links": [{"relation": "CONTRADICTS", "other_id": to,
                   "evidence": "rival verdicts on one case, kept as an open dispute"}]}),
        )
        .await;
    }
    // The closeness objection, aimed at the double-effect step itself.
    s.ok(
        "add_requirement",
        json!({"id": "req:obj-closeness", "name": "Closeness",
               "statement": "OBJECTION. The death is too close to the means for double effect to separate them.",
               "statement_kind": "proposition"}),
    )
    .await;
    s.ok(
        "review_relations",
        json!({"node_id": "req:obj-closeness", "links": [{"relation": "CONTRADICTS",
               "other_id": "inf:push-by-double-effect",
               "evidence": "it attacks the step's warrant, not the principle or the position"}]}),
    )
    .await;

    // Both rivals read contested, neither refuted.
    for side in ["req:v-push-permissible", "req:v-push-impermissible"] {
        let n = s.ok("get_node", json!({"id": side})).await;
        assert_eq!(
            n["node"]["proof_status"]["status"], "contested",
            "{side}: {n:#}"
        );
    }
    // The objection to the step contests the step, and the read-back lists it.
    let st = s
        .ok("get_node", json!({"id": "inf:push-by-double-effect"}))
        .await;
    let r = &st["node"]["inference_reading"];
    assert_eq!(r["objected"], "contested", "{st:#}");
    assert_eq!(r["objections"][0]["id"], "req:obj-closeness", "{st:#}");
    let rep = s.ok("reasoning_report", json!({})).await;
    assert!(
        rep["steps_objected"]
            .as_array()
            .is_some_and(|a| a.iter().any(|x| x["step"] == "inf:push-by-double-effect")),
        "{rep:#}"
    );
    // The impermissible side is held by two principles, and names both.
    let rep = s
        .ok(
            "reasoning_report",
            json!({"claim": "req:v-push-impermissible"}),
        )
        .await;
    let c = rep["claims"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "req:v-push-impermissible")
        .cloned()
        .expect("the position in its own read-back");
    let named: Vec<String> = c["holds_under_any_of"]
        .as_array()
        .map(|a| {
            a.iter()
                .flat_map(|l| l["assumes"].as_array().cloned().unwrap_or_default())
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    // The double-effect line is contested by the closeness objection, so only
    // the mere-means line holds: one line, named in `assumes`.
    assert!(named.is_empty(), "{c:#}");
    assert_eq!(c["assumes"], json!(["con:pr-mere-means"]), "{c:#}");

    // Answer the objection: the double-effect step is reinstated, and the
    // position names both of its lines.
    s.ok(
        "add_requirement",
        json!({"id": "req:reply-closeness", "name": "Reply to closeness",
               "statement": "REPLY. The means is the stopping of the trolley, not the death.",
               "statement_kind": "proposition", "taken_as_given": true}),
    )
    .await;
    s.ok(
        "review_relations",
        json!({"node_id": "req:reply-closeness", "links": [{"relation": "CONTRADICTS",
               "other_id": "req:obj-closeness", "evidence": "it answers the objection"}]}),
    )
    .await;
    let st = s
        .ok("get_node", json!({"id": "inf:push-by-double-effect"}))
        .await;
    assert_eq!(
        st["node"]["inference_reading"]["objected"], "answered",
        "{st:#}"
    );
    let n = s
        .ok("get_node", json!({"id": "req:v-push-impermissible"}))
        .await;
    let f = &n["node"]["foundations"]["holds_under_any_of"];
    let named: Vec<String> = f
        .as_array()
        .map(|a| {
            a.iter()
                .flat_map(|l| l["assumes"].as_array().cloned().unwrap_or_default())
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(
        named,
        vec!["con:pr-double-effect", "con:pr-mere-means"],
        "{n:#}"
    );
}
