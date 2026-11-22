//! Recording a change accepts the checksums of the files it touched, in the
//! same call.
//!
//! Measured on flo2.io, 10-30 to 11-20: in 137 of 163 runs holding both,
//! add_change_event was followed by set_artifact_checksums a median 15.6 s
//! later, with no call between in 115, while record_the_build, which does
//! both, was called 8 times
//! (reflow2:fact:root-cause-record-the-build-is-skipped-because-nothing-an-agent-reads-names-it-and-the-change-reply-names-the-step-2026-11-21).
//! So add_change_event takes `checksums`, and record_the_build is announced for
//! removal
//! (dec:recording-a-change-accepts-its-files-checksums-and-record-the-build-is-announced-for-removal,
//! Anthony 2026-11-21).

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};

struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "a-change-accepts-its-files-checksums".to_string();
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
    call(c, tool, args)
        .await
        .unwrap_or_else(|e| panic!("{tool} must answer: {e}"))
}

fn sum(d: char) -> String {
    format!("sha256:{}", d.to_string().repeat(64))
}

async fn fixtures(c: &Client) {
    for (id, location) in [
        ("art:pump-controller", "src/pump.rs"),
        ("art:tank-level", "src/tank.rs"),
    ] {
        ok(
            c,
            "add_artifact",
            json!({
                "id": id,
                "name": location,
                "location": location,
                "artifact_type": "code",
                "checksum": sum('0'),
            }),
        )
        .await;
    }
}

async fn checksum_of(c: &Client, id: &str) -> Value {
    ok(c, "get_node", json!({ "id": id })).await["node"]["properties"]["checksum"].clone()
}

async fn change_exists(c: &Client, id: &str) -> bool {
    !ok(c, "get_node", json!({ "id": id })).await["node"].is_null()
}

#[tokio::test]
async fn a_change_accepts_its_files_checksums_and_owes_no_next_step() {
    let c = connect().await;
    fixtures(&c).await;
    let got = ok(
        &c,
        "add_change_event",
        json!({
            "id": "chg:the-pump-stops-at-a-full-tank",
            "name": "The pump stops at a full tank",
            "change_type": "new_feature",
            "summary": "The controller reads the tank level and stops the pump when it is full.",
            "detected_at": "2026-11-21",
            "checksums": [
                {"artifact_id": "art:pump-controller", "checksum": sum('a')},
                {"artifact_id": "art:tank-level", "checksum": sum('b')},
            ],
        }),
    )
    .await;
    assert_eq!(
        checksum_of(&c, "art:pump-controller").await,
        json!(sum('a'))
    );
    assert_eq!(checksum_of(&c, "art:tank-level").await, json!(sum('b')));
    assert_eq!(
        got["checksums"]["applied"],
        json!(true),
        "the acceptances are in the reply: {got}"
    );
    assert!(
        got.get("next_step").is_none(),
        "every touched file was accepted, so no step is owed: {got}"
    );
    let changed = got["changed"].to_string();
    assert!(
        changed.contains("art:pump-controller") && changed.contains("art:tank-level"),
        "each file is drawn CHANGED without being listed in `affected`: {got}"
    );
}

#[tokio::test]
async fn a_file_not_accepted_is_still_named_and_the_cue_names_checksums() {
    let c = connect().await;
    fixtures(&c).await;
    let got = ok(
        &c,
        "add_change_event",
        json!({
            "id": "chg:the-tank-level-is-read-twice-a-second",
            "name": "The tank level is read twice a second",
            "change_type": "performance_optimization",
            "summary": "The level is sampled at 2 Hz instead of 1 Hz.",
            "affected": [{"node_id": "art:tank-level"}],
            "checksums": [{"artifact_id": "art:pump-controller", "checksum": sum('c')}],
        }),
    )
    .await;
    let next = got["next_step"].as_str().unwrap_or_default();
    assert!(
        next.contains("1 registered file(s)") && next.contains("`checksums`"),
        "the one file in `affected` and not in `checksums` is still owed, and the cue names \
         `checksums`: {got}"
    );
}

#[tokio::test]
async fn an_unknown_file_is_refused_before_anything_is_written() {
    let c = connect().await;
    fixtures(&c).await;
    let refused = call(
        &c,
        "add_change_event",
        json!({
            "id": "chg:a-change-naming-a-file-nobody-registered",
            "name": "A change naming a file nobody registered",
            "change_type": "new_feature",
            "checksums": [{"artifact_id": "art:no-such-file", "checksum": sum('d')}],
        }),
    )
    .await
    .expect_err("an unregistered file is refused");
    assert!(refused.contains("art:no-such-file"), "{refused}");
    assert!(
        !change_exists(&c, "chg:a-change-naming-a-file-nobody-registered").await,
        "nothing was written"
    );
}

#[tokio::test]
async fn an_acceptance_refused_after_the_change_takes_the_change_back() {
    let c = connect().await;
    fixtures(&c).await;
    // This server holds no project tree, so a checksum left out cannot be
    // measured, and the acceptance is refused after the change is written.
    let refused = call(
        &c,
        "add_change_event",
        json!({
            "id": "chg:a-change-whose-file-cannot-be-measured",
            "name": "A change whose file cannot be measured",
            "change_type": "new_feature",
            "checksums": [
                {"artifact_id": "art:pump-controller", "checksum": sum('e')},
                {"artifact_id": "art:tank-level"},
            ],
        }),
    )
    .await
    .expect_err("an acceptance that cannot measure is refused");
    assert!(refused.contains("NOTHING WAS WRITTEN"), "{refused}");
    assert!(
        !change_exists(&c, "chg:a-change-whose-file-cannot-be-measured").await,
        "the change was taken back with the refused acceptance"
    );
    assert_eq!(
        checksum_of(&c, "art:pump-controller").await,
        json!(sum('0')),
        "the other file's baseline did not move"
    );
}

#[tokio::test]
async fn record_the_build_is_removed_and_its_old_name_says_what_to_do() {
    // DEPRECATED in 0.102.0, REMOVED in the release after: it is no longer
    // listed, and a call to the old name is refused saying what to call
    // instead, for the one release that removes it.
    let c = connect().await;
    let tools = c.list_all_tools().await.expect("tools/list");
    assert!(
        tools.iter().all(|t| t.name != "record_the_build"),
        "record_the_build is no longer listed"
    );
    let refused = call(
        &c,
        "record_the_build",
        json!({"change": {"id": "chg:x", "name": "x", "change_type": "refactor"}}),
    )
    .await
    .expect_err("the removed tool's old name is refused");
    assert!(
        refused.contains("removed") && refused.contains("`add_change_event` with `checksums`"),
        "the refusal says it was removed and what to do instead: {refused}"
    );
}
