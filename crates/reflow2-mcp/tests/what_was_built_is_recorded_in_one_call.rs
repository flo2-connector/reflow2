//! What was built is recorded in ONE call, and a fix names the findings it
//! answers in the same call — four decisions Anthony accepted on 2026-10-25,
//! held here over the served path (a real MCP session, so the call's write
//! unit is the one a client gets):
//!
//! - `dec:record-what-was-built-in-one-call`: `record_the_build` records
//!   the passing check, the change (with `answers`), the files' accepted
//!   checksums and the artifacts they register, and the work marked built, in
//!   that order. Each step gets its own receipt; a refused step names itself
//!   and nothing is written. It is the first process tool under
//!   `dec:idea-a-few-core-tools-each-running-a-process-of-subtools` (b + c), and
//!   it serves job 4 of the five: show whether what was built meets what was
//!   asked.
//! - `dec:a-fix-names-the-findings-it-answers-in-the-same-call`:
//!   add_change_event takes `answers` and draws INVALIDATES in the same call;
//!   a defect_fix or test_failure_fix that answers nothing is one loop_status
//!   line until it names what it answered or says it answered none.
//! - `dec:loop-status-carries-one-repair-line`: the standing patches and the
//!   repairs nobody labelled, in one line naming repair_report.
//! - `dec:the-two-renamed-stubs-are-removed-in-the-next-release`.
//!
//! # Why
//!
//! What a tool does in the same call gets done; a separate step mostly does
//! not. CAUSES rides record_finding and is drawn (353 on reflow2's design);
//! INVALIDATES is a second call and is drawn about 4 times in 10 (233)
//! (`fact:root-cause-a-fix-rarely-records-what-it-answered-because-it-is-a-separate-call-2026-10-25`).
//! And agents ran add_change_event then set_artifact_checksums 129 times in a
//! month, the whole record-what-was-built sequence 16 times
//! (`fact:the-processes-people-and-agents-repeat-most-2026-10-25`).

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "record-what-was-built-probe".to_string();
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

    async fn exists(&self, id: &str) -> bool {
        !self.ok("get_node", json!({ "id": id })).await["node"].is_null()
    }

    async fn edges(&self, id: &str, edge_type: &str) -> Vec<Value> {
        self.ok(
            "get_node",
            json!({ "id": id, "include_edges": { "edge_types": [edge_type] } }),
        )
        .await["edges"]["items"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    async fn tool_names(&self) -> Vec<String> {
        self.client
            .list_all_tools()
            .await
            .expect("tools/list")
            .into_iter()
            .map(|t| t.name.to_string())
            .collect()
    }

    /// A capability, a decision governing it, a requirement, and two open
    /// findings a fix can answer.
    async fn seeded() -> Self {
        let s = session().await;
        s.ok(
            "add_requirement",
            json!({ "id": "req:fast", "name": "Fast", "statement": "Replies in under a second." }),
        )
        .await;
        s.ok(
            "add_capability",
            json!({ "id": "cap:reply", "name": "Reply", "description": "Replies to a request." }),
        )
        .await;
        s.ok(
            "add_decision",
            json!({ "id": "dec:cache", "name": "Cache replies", "decision": "Cache every reply.", "kind": "choice" }),
        )
        .await;
        for id in ["fact:slow-reply", "fact:stale-cache"] {
            s.ok(
                "record_finding",
                json!({
                    "id": id,
                    "statement": format!("{id} was observed."),
                    "fact_type": "defect",
                    "subject_id": "cap:reply",
                }),
            )
            .await;
        }
        s
    }
}

fn next_lines(status: &Value) -> Vec<String> {
    status["next"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|l| l.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn the_whole_call() -> Value {
    json!({
        "verification": {
            "id": "ver:reply-is-fast",
            "name": "Reply is fast",
            "method": "test",
            "verifies": ["cap:reply"],
            "last_run_at": "2026-10-25",
            "findings": "12 of 12 replies under a second.",
        },
        "change": {
            "id": "chg:cache-replies",
            "name": "Cache replies",
            "change_type": "defect_fix",
            "subject": "system",
            "summary": "Replies are cached.",
            "rationale": "The slow reply was a missing cache.",
            "detected_at": "2026-10-25",
            "affected": [{ "node_id": "cap:reply" }],
            "answers": [{ "finding_id": "fact:slow-reply", "note": "the cache answers it" }],
        },
        "files": [
            {
                "artifact_id": "art:cache-rs",
                "name": "cache.rs",
                "location": "src/cache.rs",
                "checksum": "sha256:aaaa",
            },
            {
                "artifact_id": "art:cache-test",
                "name": "cache_test.rs",
                "location": "tests/cache_test.rs",
                "checksum": "sha256:bbbb",
                "runs": "ver:reply-is-fast",
            },
        ],
        "built": [
            { "id": "cap:reply" },
            { "id": "dec:cache" },
        ],
    })
}

// ---- add_change_event `answers` ---------------------------------------------------------

/// `answers` draws INVALIDATES from the change to each finding, in the same
/// call, with each finding's note.
#[tokio::test]
async fn answers_draws_invalidates_in_the_same_call() {
    let s = Session::seeded().await;
    s.ok(
        "add_change_event",
        json!({
            "id": "chg:fix",
            "name": "Fix",
            "change_type": "defect_fix",
            "detected_at": "2026-10-25",
            "answers": [
                { "finding_id": "fact:slow-reply", "note": "partly: the cold path is still slow" },
                { "finding_id": "fact:stale-cache" },
            ],
        }),
    )
    .await;
    for finding in ["fact:slow-reply", "fact:stale-cache"] {
        let claims = s.edges(finding, "INVALIDATES").await;
        assert!(
            claims.iter().any(|e| e["from_id"] == "chg:fix"),
            "{finding} is answered by chg:fix: {claims:?}"
        );
    }
    let claims = s.edges("fact:slow-reply", "INVALIDATES").await;
    assert_eq!(
        claims[0]["properties"]["note"], "partly: the cold path is still slow",
        "the note rides the edge: {claims:?}"
    );
}

/// A finding `answers` names that does not exist refuses the call, and the
/// change is not written either.
#[tokio::test]
async fn answers_naming_a_missing_finding_writes_nothing() {
    let s = Session::seeded().await;
    let refused = s
        .call(
            "add_change_event",
            json!({
                "id": "chg:fix",
                "name": "Fix",
                "change_type": "defect_fix",
                "answers": [{ "finding_id": "fact:no-such-finding" }],
            }),
        )
        .await
        .expect_err("a missing finding is refused");
    assert!(refused.contains("fact:no-such-finding"), "{refused}");
    assert!(!s.exists("chg:fix").await, "nothing was written");
}

/// A defect fix that answers nothing is ONE loop line; naming a finding, or
/// saying it answered none (`answers: []`), takes it off.
#[tokio::test]
async fn a_fix_that_answers_nothing_is_one_loop_line_until_it_says() {
    let s = Session::seeded().await;
    let line = |status: &Value| {
        next_lines(status)
            .into_iter()
            .find(|l| l.contains("`answers`"))
    };
    assert!(line(&s.ok("loop_status", json!({})).await).is_none());

    for id in ["chg:silent-1", "chg:silent-2"] {
        s.ok(
            "add_change_event",
            json!({ "id": id, "name": id, "change_type": "test_failure_fix" }),
        )
        .await;
    }
    // A feature is not a fix: never counted.
    s.ok(
        "add_change_event",
        json!({ "id": "chg:feature", "name": "Feature", "change_type": "new_feature" }),
    )
    .await;
    let status = s.ok("loop_status", json!({})).await;
    assert_eq!(status["fixes_answering_nothing"], 2, "{status}");
    let said = line(&status).expect("one line names the fixes that answer nothing");
    assert!(said.starts_with("2 fix"), "{said}");
    assert_eq!(
        next_lines(&status)
            .iter()
            .filter(|l| l.contains("`answers`"))
            .count(),
        1,
        "ONE line, however many fixes"
    );

    // One names what it answered; the other says it answered none.
    s.ok(
        "add_change_event",
        json!({ "id": "chg:silent-1", "answers": [{ "finding_id": "fact:slow-reply" }] }),
    )
    .await;
    s.ok(
        "add_change_event",
        json!({ "id": "chg:silent-2", "answers": [] }),
    )
    .await;
    let status = s.ok("loop_status", json!({})).await;
    assert_eq!(status["fixes_answering_nothing"], 0, "{status}");
    assert!(line(&status).is_none(), "{status}");
}

// ---- loop_status's repair line ------------------------------------------------------------

/// loop_status says, in one line, how many standing patches there are and how
/// many repairs nobody labelled, and names repair_report.
#[tokio::test]
async fn loop_status_carries_one_repair_line() {
    let s = Session::seeded().await;
    s.ok(
        "add_change_event",
        json!({
            "id": "chg:patch",
            "name": "Patch",
            "change_type": "defect_fix",
            "repair": "contained_symptom",
            "stands_in_for": "rewrite the cache",
            "answers": [],
        }),
    )
    .await;
    s.ok(
        "add_change_event",
        json!({ "id": "chg:unlabelled", "name": "Unlabelled", "change_type": "defect_fix", "answers": [] }),
    )
    .await;
    s.ok(
        "add_change_event",
        json!({
            "id": "chg:real-fix",
            "name": "Real fix",
            "change_type": "defect_fix",
            "repair": "corrected_cause",
            "answers": [],
        }),
    )
    .await;
    let status = s.ok("loop_status", json!({})).await;
    let lines: Vec<String> = next_lines(&status)
        .into_iter()
        .filter(|l| l.contains("repair_report"))
        .collect();
    assert_eq!(lines.len(), 1, "one repair line: {lines:?}");
    assert!(
        lines[0].contains("1 standing patch") && lines[0].contains("1 repair(s) nobody labelled"),
        "{}",
        lines[0]
    );
    assert_eq!(status["repairs"]["standing_patches"], 1, "{status}");
    assert_eq!(status["repairs"]["unlabelled"], 1, "{status}");
}

// ---- record_the_build ----------------------------------------------------------------

/// The whole process in one call: every step written, one receipt per step,
/// in the decision's order.
#[tokio::test]
async fn one_call_records_every_step_with_a_receipt_each() {
    let s = Session::seeded().await;
    let reply = s.ok("record_the_build", the_whole_call()).await;
    let steps: Vec<&str> = reply["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .map(|r| r["step"].as_str().expect("each receipt names its step"))
        .collect();
    assert_eq!(
        steps,
        ["verification", "change", "files", "built"],
        "{reply}"
    );
    for r in reply["steps"].as_array().unwrap() {
        assert!(r["receipt"].is_object(), "a receipt per step: {r}");
        assert!(r["tool"].is_string(), "each says the tool it ran: {r}");
    }

    // Step 1: the passing check, run by its test file (IMPLEMENTS).
    let ver = s.ok("get_node", json!({ "id": "ver:reply-is-fast" })).await;
    assert_eq!(ver["node"]["properties"]["status"], "passing", "{ver}");
    let runs = s.edges("ver:reply-is-fast", "IMPLEMENTS").await;
    assert!(
        runs.iter().any(|e| e["from_id"] == "art:cache-test"),
        "the test file runs the check: {runs:?}"
    );
    // Step 2: the change, answering its finding.
    let claims = s.edges("fact:slow-reply", "INVALIDATES").await;
    assert!(
        claims.iter().any(|e| e["from_id"] == "chg:cache-replies"),
        "{claims:?}"
    );
    // Step 3: each file registered, its checksum accepted on the change.
    for art in ["art:cache-rs", "art:cache-test"] {
        let a = s.ok("get_node", json!({ "id": art })).await;
        assert!(!a["node"].is_null(), "{art} registered");
        let changed = s.edges(art, "CHANGED").await;
        assert!(
            changed.iter().any(|e| e["from_id"] == "chg:cache-replies"),
            "{art}'s checksum accepted on the change: {changed:?}"
        );
    }
    // Step 4: the capability built, the decision shown built by it.
    let cap = s.ok("get_node", json!({ "id": "cap:reply" })).await;
    assert_eq!(cap["node"]["properties"]["status"], "realized", "{cap}");
    let governed = s.edges("dec:cache", "GOVERNED_BY").await;
    assert!(
        governed.iter().any(|e| e["from_id"] == "cap:reply"),
        "{governed:?}"
    );
}

/// ALL OR NOTHING: a refusal in the LAST step leaves nothing from the first
/// three behind.
#[tokio::test]
async fn a_refused_step_writes_nothing_at_all() {
    let s = Session::seeded().await;
    let mut args = the_whole_call();
    args["built"] = json!([{ "id": "cap:no-such-capability" }]);
    let refused = s
        .call("record_the_build", args)
        .await
        .expect_err("a missing node to mark built is refused");
    assert!(!refused.is_empty());
    for id in [
        "ver:reply-is-fast",
        "chg:cache-replies",
        "art:cache-rs",
        "art:cache-test",
    ] {
        assert!(!s.exists(id).await, "{id} must not be written: {refused}");
    }
    assert!(
        s.edges("fact:slow-reply", "INVALIDATES").await.is_empty(),
        "no answer drawn"
    );
}

/// A refusal names the step that refused, and the step's tool.
#[tokio::test]
async fn a_refusal_names_its_step() {
    let s = Session::seeded().await;

    let mut args = the_whole_call();
    args["built"] = json!([{ "id": "cap:no-such-capability" }]);
    let refused = s.call("record_the_build", args).await.unwrap_err();
    assert!(
        refused.contains("step 4 (built)") && refused.contains("cap:no-such-capability"),
        "{refused}"
    );
    assert!(refused.contains("Nothing was written"), "{refused}");

    let mut args = the_whole_call();
    args["change"]["answers"] = json!([{ "finding_id": "fact:no-such-finding" }]);
    let refused = s.call("record_the_build", args).await.unwrap_err();
    assert!(
        refused.contains("step 2 (change)") && refused.contains("fact:no-such-finding"),
        "{refused}"
    );

    let mut args = the_whole_call();
    args["files"][1]["runs"] = json!("ver:no-such-check");
    let refused = s.call("record_the_build", args).await.unwrap_err();
    assert!(refused.contains("step 3 (files)"), "{refused}");
}

/// The composed tools stay served, and the process tool is found first for
/// the job in a person's words.
#[tokio::test]
async fn the_process_tool_is_found_first_and_its_parts_stay_served() {
    let s = session().await;
    let names = s.tool_names().await;
    for t in [
        "record_the_build",
        "add_verification",
        "add_change_event",
        "set_artifact_checksums",
        "add_artifact",
        "set_capability_status",
    ] {
        assert!(names.iter().any(|n| n == t), "{t} is served");
    }
    let found = s
        .ok("find_tools", json!({ "query": "record what I built" }))
        .await;
    assert_eq!(found["items"][0]["tool"], "record_the_build", "{found}");
}

// ---- the two renamed stubs ----------------------------------------------------------------

/// The two stubs are gone from the listing, and their old names are refused
/// with their replacements.
#[tokio::test]
async fn the_two_renamed_stubs_are_removed_and_their_names_say_what_to_use() {
    let s = session().await;
    let names = s.tool_names().await;
    for (gone, instead) in [
        ("record_change", "snapshot_before_change"),
        ("manual_work_report", "manual_work_ledger"),
    ] {
        assert!(!names.iter().any(|n| n == gone), "{gone} is not listed");
        let refused = s.call(gone, json!({})).await.unwrap_err();
        assert!(refused.contains(instead), "{gone}: {refused}");
    }
    assert_eq!(names.len(), 199, "200 + 1 process tool - 2 stubs");
}
