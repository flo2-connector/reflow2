//! A Question is put to a named person, travels to them as a batch with its
//! evidence, and records who answered it, in one call.
//!
//! `req:a-question-is-addressed-to-a-person-and-records-who-answered`
//! (accepted, Anthony 2026-09-29, idea 2 of the two-agent exercise), built as
//! `cap:a-question-names-whom-it-was-put-to-and-who-answered`.
//!
//! # The failure this pins
//!
//! A designer agent put numbered question batches to a separate owner agent. A
//! coordinator carried every batch by hand and the designer kept its
//! question-number → node map in a scratch file, because nothing in reflow2
//! could address a question to someone or record who answered. Answering took
//! two calls: `answer_question` took only an id and the text, and the ANSWERS
//! edge had landed as its own tool
//! (`fact:root-cause-answering-takes-two-calls-and-records-no-answerer-because-the-answers-edge-landed-as-its-own-tool-2026-09-29`).
//! The owner had nothing addressed to it to read
//! (`fact:root-cause-an-owner-outside-the-chat-cannot-read-what-it-is-asked-to-approve-2026-09-29`).
//!
//! Every call here goes through `call_tool` BY NAME, the way a session meets
//! the surface, so the tests compile on a build without the feature and fail
//! there on what the surface refuses or leaves out.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};

#[derive(Clone, Default)]
struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "a-question-is-put-to-a-person".to_string();
        cfg.client_info.version = "test".to_string();
        cfg
    }
}

type Client = rmcp::service::RunningService<rmcp::RoleClient, TestClient>;

async fn connect() -> Client {
    let service = ReflowService::in_memory().expect("in-memory service");
    let (server_rx, client_tx) = tokio::io::duplex(1 << 22);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 22);
    tokio::spawn(async move {
        if let Ok(running) = service.serve((server_rx, server_tx)).await {
            let _ = running.waiting().await;
        }
    });
    TestClient
        .serve((client_rx, client_tx))
        .await
        .expect("the in-process handshake")
}

/// Call a tool by NAME. `Ok(structured reply)` or `Err(refusal text)`.
async fn call(c: &Client, tool: &str, args: Value) -> Result<Value, String> {
    let Value::Object(arguments) = args else {
        panic!("arguments for {tool} must be an object");
    };
    match c
        .call_tool(CallToolRequestParams::new(tool.to_string()).with_arguments(arguments))
        .await
    {
        Ok(r) if r.is_error == Some(true) => Err(format!("{:?}", r.content)),
        Ok(r) => Ok(r.structured_content.unwrap_or(Value::Null)),
        Err(e) => Err(e.to_string()),
    }
}

async fn ok(c: &Client, tool: &str, args: Value) -> Value {
    call(c, tool, args.clone())
        .await
        .unwrap_or_else(|e| panic!("{tool} {args} was refused: {e}"))
}

async fn props(c: &Client, id: &str) -> Value {
    ok(c, "get_node", json!({ "id": id })).await["node"]["properties"].clone()
}

/// A design with three open requirements (three anchored gaps), an owner, a
/// designer agent, a measured finding to cite, and a Decision an answer can
/// become.
async fn seeded() -> Client {
    let c = connect().await;
    ok(&c, "add_project", json!({ "id": "proj:p", "name": "P" })).await;
    for n in 1..=3 {
        ok(
            &c,
            "add_requirement",
            json!({ "id": format!("req:r{n}"), "name": format!("R{n}"),
                    "statement": format!("The system must do thing {n}.") }),
        )
        .await;
    }
    ok(
        &c,
        "add_contributor",
        json!({ "id": "who:owner", "name": "The owner", "kind": "person" }),
    )
    .await;
    ok(
        &c,
        "add_contributor",
        json!({ "id": "who:designer", "name": "The designer", "kind": "automated_agent" }),
    )
    .await;
    ok(
        &c,
        "record_finding",
        json!({ "id": "fact:sized-twice", "subject_id": "req:r1", "fact_type": "finding",
                "name": "The store was sized at twice what it holds",
                "statement": "Measured: the export holds half what the sizing assumed." }),
    )
    .await;
    ok(
        &c,
        "add_decision",
        json!({ "id": "dec:answer", "name": "What the answer became",
                "decision": "Thing 1 is done by the store.", "rationale": "The owner said so." }),
    )
    .await;
    c
}

/// The anchored gaps, in a stable order.
async fn gaps(c: &Client) -> Vec<Value> {
    let v = ok(c, "detect_gaps", json!({})).await;
    let mut out: Vec<Value> = v["items"]
        .as_array()
        .unwrap_or_else(|| panic!("detect_gaps items: {v}"))
        .iter()
        .filter(|g| g["affected_ids"].as_array().is_some_and(|a| !a.is_empty()))
        .cloned()
        .collect();
    out.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    assert!(out.len() >= 3, "three anchored gaps to ask about, got {v}");
    out
}

/// Put `gaps` as ONE batch through `gaps_to_prompts`, both passes, with the
/// batch-level arguments in `extra` and per-gap evidence in `evidence`.
async fn put_batch(
    c: &Client,
    gaps: &[Value],
    evidence: &[Vec<&str>],
    extra: Value,
) -> Result<Value, String> {
    let rows = |answers: Option<&Vec<Value>>| -> Vec<Value> {
        gaps.iter()
            .enumerate()
            .map(|(i, g)| {
                let mut row = json!({ "gap": g, "answers": [] });
                if let Some(a) = answers {
                    row["answers"] = json!([{ "id": a[i], "text": format!("Question {} put plainly?", i + 1) }]);
                }
                if let Some(ev) = evidence.get(i) {
                    row["evidence"] = json!(ev);
                }
                row
            })
            .collect()
    };
    let mut args = json!({ "gaps": rows(None) });
    for (k, v) in extra.as_object().unwrap() {
        args[k] = v.clone();
    }
    let prep = call(c, "gaps_to_prompts", args.clone()).await?;
    let ids: Vec<Value> = prep["gaps"]
        .as_array()
        .unwrap_or_else(|| panic!("prepare pass: {prep}"))
        .iter()
        .map(|g| g["prompts"][0]["id"].clone())
        .collect();
    args["gaps"] = json!(rows(Some(&ids)));
    args["asked_at"] = json!("2026-09-29");
    call(c, "gaps_to_prompts", args).await
}

fn question_ids(served: &Value) -> Vec<String> {
    served["gaps"]
        .as_array()
        .unwrap_or_else(|| panic!("serve pass: {served}"))
        .iter()
        .map(|g| g["question_id"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn a_batch_put_to_a_person_records_whom_each_question_was_put_to_and_its_number() {
    let c = seeded().await;
    let g = gaps(&c).await;
    let served = put_batch(
        &c,
        &g[..2],
        &[vec!["fact:sized-twice"], vec![]],
        json!({ "asked_of": "who:owner", "batch": "round-1" }),
    )
    .await
    .expect("a batch addressed to a Contributor is accepted");
    let qids = question_ids(&served);
    for (i, qid) in qids.iter().enumerate() {
        let p = props(&c, qid).await;
        assert_eq!(p["asked_of"], "who:owner", "whom it was put to: {p}");
        assert_eq!(p["batch"], "round-1", "the batch it travelled in: {p}");
        assert_eq!(
            p["batch_position"],
            json!(i + 1),
            "numbered in the order it was put, so a relay's \"Q{}\" resolves: {p}",
            i + 1
        );
    }
}

#[tokio::test]
async fn an_addressee_who_is_not_a_contributor_is_refused_and_nothing_is_recorded() {
    let c = seeded().await;
    let g = gaps(&c).await;
    let refused = put_batch(&c, &g[..1], &[], json!({ "asked_of": "who:nobody" })).await;
    let e = refused.expect_err("an addressee the design does not hold is refused");
    assert!(e.contains("who:nobody"), "the refusal names the id: {e}");
    let open = ok(&c, "open_questions", json!({})).await;
    assert_eq!(open["count"], 0, "nothing was recorded: {open}");
}

#[tokio::test]
async fn answer_question_records_the_answerer_and_draws_answers_in_the_same_call() {
    let c = seeded().await;
    let g = gaps(&c).await;
    let served = put_batch(&c, &g[..1], &[], json!({ "asked_of": "who:owner", "batch": "round-1" }))
        .await
        .expect("put");
    let qid = question_ids(&served).remove(0);

    let reply = ok(
        &c,
        "answer_question",
        json!({ "question_id": qid, "answer": "The store does thing 1.",
                "answered_by": "who:owner", "answered_at": "2026-09-29",
                "record": "dec:answer", "note": "The owner's answer, as ruled." }),
    )
    .await;
    assert_eq!(reply["answered"], true, "{reply}");

    let p = props(&c, &qid).await;
    assert_eq!(p["status"], "answered");
    assert_eq!(p["answered_by"], "who:owner", "who answered is on the record: {p}");
    assert_eq!(p["answered_at"], "2026-09-29");

    // The ANSWERS edge was drawn by that one call: the loop counts it.
    let ls = ok(&c, "loop_status", json!({})).await;
    assert_eq!(
        ls["answered_naming_their_record"], 1,
        "the record the answer became is linked without a second call: {ls}"
    );
    let open = ok(&c, "open_questions", json!({})).await;
    let row = &open["items"][0];
    assert_eq!(row["answered_by"], "who:owner", "{open}");
    assert_eq!(row["answered_in"], json!(["dec:answer"]), "{open}");
}

#[tokio::test]
async fn an_answerer_or_a_record_that_does_not_exist_is_refused_and_the_question_stays_asked() {
    let c = seeded().await;
    let g = gaps(&c).await;
    let served = put_batch(&c, &g[..1], &[], json!({ "asked_of": "who:owner" }))
        .await
        .expect("put");
    let qid = question_ids(&served).remove(0);

    for (field, bad) in [("answered_by", "who:ghost"), ("record", "dec:ghost")] {
        let mut args = json!({ "question_id": qid, "answer": "Yes.",
                               "answered_by": "who:owner", "record": "dec:answer" });
        args[field] = json!(bad);
        let e = call(&c, "answer_question", args)
            .await
            .expect_err("a name that resolves to nothing is refused");
        assert!(e.contains(bad), "the refusal names {bad}: {e}");
        let p = props(&c, &qid).await;
        assert_eq!(p["status"], "asked", "nothing was written for {field}: {p}");
        assert!(p.get("answered_by").is_none_or(Value::is_null), "{p}");
    }
}

#[tokio::test]
async fn open_questions_for_one_addressee_returns_their_batch_in_order_with_its_evidence() {
    let c = seeded().await;
    let g = gaps(&c).await;
    put_batch(
        &c,
        &g[..2],
        &[vec!["fact:sized-twice"], vec![]],
        json!({ "asked_of": "who:owner", "batch": "round-1" }),
    )
    .await
    .expect("put to the owner");
    // A third question put to nobody by name.
    put_batch(&c, &g[2..3], &[], json!({})).await.expect("put unaddressed");

    let mine = ok(&c, "open_questions", json!({ "asked_of": "who:owner" })).await;
    let items = mine["items"].as_array().unwrap_or_else(|| panic!("{mine}"));
    assert_eq!(items.len(), 2, "only what was put to the owner: {mine}");
    assert_eq!(items[0]["batch_position"], 1, "{mine}");
    assert_eq!(items[1]["batch_position"], 2, "{mine}");
    assert!(items.iter().all(|q| q["asked_of"] == "who:owner"), "{mine}");

    // The evidence travels as LINKS the owner can read, not as a summary: the
    // finding the asker attached, and the nodes the gap was about.
    let ev: Vec<&str> = items[0]["evidence"]
        .as_array()
        .unwrap_or_else(|| panic!("evidence links: {mine}"))
        .iter()
        .map(|e| e["id"].as_str().unwrap())
        .collect();
    assert!(ev.contains(&"fact:sized-twice"), "the attached finding: {mine}");
    assert!(ev.contains(&"proj:p"), "what the gap was about: {mine}");
    let fact = items[0]["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == "fact:sized-twice")
        .unwrap();
    assert_eq!(fact["node_type"], "TemporalFact");
    assert_eq!(fact["name"], "The store was sized at twice what it holds");

    // The read says what it left out rather than letting a filtered answer
    // pass for the whole list.
    assert_eq!(mine["asked_of"], "who:owner", "{mine}");
    assert_eq!(mine["not_put_to_them"], 1, "{mine}");

    // An addressee the design does not hold is refused, never answered empty.
    let e = call(&c, "open_questions", json!({ "asked_of": "who:ghost" }))
        .await
        .expect_err("an unknown addressee is refused");
    assert!(e.contains("who:ghost"), "{e}");
}

#[tokio::test]
async fn an_unaddressed_question_and_an_unattributed_answer_say_so() {
    let c = seeded().await;
    let g = gaps(&c).await;
    let served = put_batch(&c, &g[..1], &[], json!({})).await.expect("put");
    let qid = question_ids(&served).remove(0);

    let open = ok(&c, "open_questions", json!({})).await;
    let row = &open["items"][0];
    assert!(
        row.as_object().unwrap().contains_key("asked_of") && row["asked_of"].is_null(),
        "an unaddressed question carries an explicit null, not a guessed name: {open}"
    );

    let reply = ok(
        &c,
        "answer_question",
        json!({ "question_id": qid, "answer": "Park it." }),
    )
    .await;
    let note = reply["answered_by_note"].as_str().unwrap_or_default();
    assert!(
        note.contains("nobody"),
        "an answer naming no answerer says it carries nobody's name: {reply}"
    );
    let open = ok(&c, "open_questions", json!({})).await;
    let row = &open["items"][0];
    assert!(
        row.as_object().unwrap().contains_key("answered_by") && row["answered_by"].is_null(),
        "{open}"
    );
}

#[tokio::test]
async fn a_new_answer_naming_nobody_does_not_inherit_the_last_answerer() {
    let c = seeded().await;
    let g = gaps(&c).await;
    let served = put_batch(&c, &g[..1], &[], json!({ "asked_of": "who:owner" }))
        .await
        .expect("put");
    let qid = question_ids(&served).remove(0);
    ok(
        &c,
        "answer_question",
        json!({ "question_id": qid, "answer": "First.", "answered_by": "who:owner" }),
    )
    .await;
    ok(
        &c,
        "answer_question",
        json!({ "question_id": qid, "answer": "Second, relayed by someone." }),
    )
    .await;
    let p = props(&c, &qid).await;
    assert_eq!(p["answer"], "Second, relayed by someone.");
    assert!(
        p.get("answered_by").is_none_or(Value::is_null),
        "the owner did not give the second answer, so it must not carry their name: {p}"
    );
}

#[tokio::test]
async fn loop_status_for_the_addressee_lists_the_questions_put_to_them() {
    let c = seeded().await;
    let g = gaps(&c).await;
    put_batch(&c, &g[..2], &[], json!({ "asked_of": "who:owner", "batch": "round-1" }))
        .await
        .expect("put");
    let ls = ok(&c, "loop_status", json!({ "contributor_id": "who:owner" })).await;
    let rows = ls["questions_put_to_them"]
        .as_array()
        .unwrap_or_else(|| panic!("the per-person read lists them: {ls}"));
    assert_eq!(rows.len(), 2, "{ls}");
    assert_eq!(ls["clean"], false, "questions waiting on them are owed by them: {ls}");

    let other = ok(&c, "loop_status", json!({ "contributor_id": "who:designer" })).await;
    assert_eq!(
        other["questions_put_to_them"].as_array().map(Vec::len),
        Some(0),
        "nobody else's questions are attributed to them: {other}"
    );
}
