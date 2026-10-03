//! A WRITE replies with a RECEIPT, not the whole stored node — asked of every
//! write tool the server serves, and of the writes a design session makes most.
//!
//! # The failure this pins
//!
//! `fact:root-cause-a-write-replies-with-the-whole-stored-node-and-replace-text-adds-each-prior-field-2026-09-29`
//! (I4 of the dev_reflow2 two-agent exercise, reflow2 0.74.0): every write
//! answered with the node as stored, whatever it changed, and `replace_text`
//! added each replaced field's prior value in full.
//! - A 12-character append to a 1-character rationale returned 7,501 characters,
//!   because the node's untouched 6 KB decision came back with it.
//! - A designer agent echoed about 290 KB for about 40 KB of edits over three
//!   rounds.
//!
//! No reply contract for a write had ever been written; each handler returned
//! the stored node by convention.
//!
//! # The contract (`req:a-write-replies-with-a-receipt-not-the-whole-node`)
//!
//! ONE code path — the `call_tool` choke point — shapes every write's reply,
//! so a new write tool joins by being served, not by remembering:
//! - a value of at most [`SHORT`] characters is echoed as stored; a longer one
//!   is given by size under `elided`;
//! - a revise names each replaced field's size before and after, and WHERE its
//!   prior value is kept (`prior_in`). A prior value nothing else holds
//!   (`fields_at_risk`) is still echoed in full, because then the reply is its
//!   only copy;
//! - `echo: "node"` returns the whole stored node and every prior value.
//!
//! # Why a size bound is the assertion
//!
//! The defect was a reply that GREW WITH THE STORED CONTENT. So the cases give
//! every node a 6,000-character field and assert the default reply stays under
//! [`BOUND`] — a number that does not move with the content — and never carries
//! that text back.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};

/// Values at most this long are echoed; longer ones are given by size. Mirrors
/// `reflow2_mcp::receipt::SHORT`, restated so a change to it is a visible edit
/// to the contract this file pins.
const SHORT: usize = 200;

/// A default write reply stays under this many characters whatever the node
/// holds. Generous enough for the server's own notes (hints, near-match
/// warnings, drawn edges), far below the 6,000-character field every case
/// stores.
const BOUND: usize = 3_000;

/// The long stored text every case carries.
fn long(tag: &str) -> String {
    let mut s = format!("{tag}: ");
    while s.len() < 6_000 {
        s.push_str("a paragraph that a receipt must not carry back to its writer. ");
    }
    s
}

struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "a-write-replies-with-a-receipt".to_string();
        cfg.client_info.version = "test".to_string();
        cfg
    }
}

type Client = rmcp::service::RunningService<rmcp::RoleClient, TestClient>;

/// One in-memory server, reached the way a session reaches it — through
/// `call_tool`, where the receipt is made.
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
    call(c, tool, args)
        .await
        .unwrap_or_else(|e| panic!("{tool} refused: {e}"))
}

fn chars(v: &Value) -> usize {
    v.to_string().chars().count()
}

/// Does any string anywhere in `v` contain `needle`?
fn carries(v: &Value, needle: &str) -> bool {
    match v {
        Value::String(s) => s.contains(needle),
        Value::Array(a) => a.iter().any(|x| carries(x, needle)),
        Value::Object(o) => o.values().any(|x| carries(x, needle)),
        _ => false,
    }
}

/// The served write tools, with their input schemas. "Write" is read off the
/// served annotations, the same predicate `--call` and a read-only surface use.
async fn write_tools(c: &Client) -> Vec<(String, Value)> {
    let tools = c.list_all_tools().await.expect("tools/list");
    let mut out: Vec<(String, Value)> = tools
        .into_iter()
        .filter(|t| {
            !t.annotations
                .as_ref()
                .and_then(|a| a.read_only_hint)
                .unwrap_or(false)
        })
        .map(|t| {
            (
                t.name.to_string(),
                serde_json::to_value(&t.input_schema).expect("schema"),
            )
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Every served write tool declares `echo`, and a bad value is refused BEFORE
/// its handler runs, naming the two values. The refusal needs no valid
/// arguments at all — that is the point: it proves the choke point sees every
/// write tool, so the receipt is not something a handler can forget.
#[tokio::test]
async fn every_write_tool_declares_echo_and_the_choke_point_sees_it() {
    let c = connect().await;
    let writes = write_tools(&c).await;
    assert!(
        writes.len() > 50,
        "expected the served surface's write tools, found {}",
        writes.len()
    );
    let mut undeclared = Vec::new();
    let mut unseen = Vec::new();
    for (name, schema) in &writes {
        let echo = &schema["properties"]["echo"];
        let values: Vec<&str> = echo["enum"]
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        if values != ["receipt", "node"] {
            undeclared.push(name.clone());
        }
        match call(&c, name, json!({"echo": "everything"})).await {
            Err(e) if e.contains("echo") && e.contains("receipt") && e.contains("node") => {}
            other => unseen.push(format!("{name}: {other:?}")),
        }
    }
    assert!(
        undeclared.is_empty(),
        "{} write tool(s) do not declare `echo` with [\"receipt\", \"node\"]: {undeclared:?}",
        undeclared.len()
    );
    assert!(
        unseen.is_empty(),
        "{} write tool(s) did not refuse a bad `echo` before running (the receipt layer \
         cannot see them): {unseen:#?}",
        unseen.len()
    );
    // A read gains nothing: it answers what it was asked for, and `echo`
    // stays an unknown argument there.
    let reply = call(&c, "loop_status", json!({"echo": "node"})).await;
    assert!(
        reply.is_err(),
        "a read-only tool must not take `echo`: {reply:?}"
    );
}

/// The designer's own write mix, measured before and after on the SAME call:
/// each write is made once with `echo: "node"` (the reply exactly as the
/// handler builds it, which is what every write sent before receipts) and its
/// receipt is cut from that reply by the same function the choke point uses.
/// Every receipt stays under [`BOUND`] and never carries the stored 6,000
/// characters back; a few writes are also made with the default, to prove the
/// choke point sends that receipt.
#[tokio::test]
async fn a_writes_default_reply_does_not_grow_with_what_the_node_holds() {
    let c = connect().await;
    let mut measured: Vec<(&str, usize, usize)> = Vec::new();
    let mut receipts: Vec<(&str, Value)> = Vec::new();
    let mut step = |what: &'static str, full: Value| {
        let r = reflow2_mcp::receipt::receipt(full.clone());
        measured.push((what, chars(&full), chars(&r)));
        receipts.push((what, r));
        full
    };

    let full = step(
        "add_decision (6,000-char body)",
        ok(
            &c,
            "add_decision",
            json!({"id":"dec:long","name":"A decision with a long body","decision":long("decision"),"rationale":"r","kind":"choice","echo":"node"}),
        )
        .await,
    );
    assert!(
        full["properties"]["decision"]
            .as_str()
            .is_some_and(|d| d.starts_with("decision: ")),
        "echo: \"node\" returns the whole stored node"
    );
    step(
        "replace_text, 12 chars onto a 1-char rationale",
        ok(
            &c,
            "replace_text",
            json!({"node_id":"dec:long","field":"rationale","new":"twelve chars","echo":"node"}),
        )
        .await,
    );
    let appended = step(
        "replace_text, 170 chars onto the 6,000-char body",
        ok(
            &c,
            "replace_text",
            json!({"node_id":"dec:long","field":"decision","new":"x".repeat(170),"echo":"node"}),
        )
        .await,
    );
    assert!(
        appended["revision"]["replaced"][0]["prior"].is_string(),
        "echo: \"node\" returns every prior value"
    );
    step(
        "set_decision_status",
        ok(
            &c,
            "set_decision_status",
            json!({"decision_id":"dec:long","status":"rejected","echo":"node"}),
        )
        .await,
    );
    ok(
        &c,
        "add_component",
        json!({"id":"cmp:long","name":"A part","description":long("component")}),
    )
    .await;
    step(
        "add_component revise (name only)",
        ok(
            &c,
            "add_component",
            json!({"id":"cmp:long","name":"A renamed part","echo":"node"}),
        )
        .await,
    );
    step(
        "add_requirement (6,000-char statement)",
        ok(
            &c,
            "add_requirement",
            json!({"id":"req:long","name":"A long requirement","statement":long("requirement"),"echo":"node"}),
        )
        .await,
    );
    step(
        "record_finding (6,000-char statement)",
        ok(
            &c,
            "record_finding",
            json!({"id":"fact:long","subject_id":"req:long","name":"A long finding","statement":long("finding"),"fact_type":"finding","echo":"node"}),
        )
        .await,
    );
    ok(
        &c,
        "add_decision",
        json!({"id":"dec:other","name":"Another decision","decision":"short","kind":"choice"}),
    )
    .await;
    step(
        "create_edge (6,000-char evidence)",
        ok(
            &c,
            "create_edge",
            json!({"edge_type":"CONTRADICTS","from_id":"dec:long","to_id":"dec:other","props":{"evidence":long("evidence")},"echo":"node"}),
        )
        .await,
    );

    eprintln!("chars per reply — as built (echo: node) → receipt (the default):");
    for (what, before, after) in &measured {
        eprintln!("  {what}: {before} → {after}");
    }
    for (what, r) in &receipts {
        assert!(
            chars(r) < BOUND,
            "{what}: the receipt is {} characters — a write's reply must not grow with what \
             the node holds (bound {BOUND})",
            chars(r)
        );
        assert!(
            !carries(r, "a paragraph that a receipt must not carry back"),
            "{what}: the receipt carried the stored text back"
        );
    }

    // The choke point sends exactly that receipt by default.
    let created = ok(
        &c,
        "add_decision",
        json!({"id":"dec:plain","name":"A decision sent by default","decision":long("plain"),"rationale":"r","kind":"choice","distinct_from":["dec:long"]}),
    )
    .await;
    assert_eq!(created["node_id"], "dec:plain");
    assert_eq!(created["properties"]["rationale"], "r", "{created}");
    assert_eq!(
        created["elided"]["decision"].as_u64(),
        Some(long("plain").chars().count() as u64),
        "{created}"
    );
    assert!(chars(&created) < BOUND, "{created}");
    let settled = ok(
        &c,
        "set_decision_status",
        json!({"decision_id":"dec:plain","status":"rejected"}),
    )
    .await;
    assert_eq!(settled["properties"]["status"], "rejected", "{settled}");
    assert!(chars(&settled) < BOUND, "{}", chars(&settled));
    let revised = ok(
        &c,
        "replace_text",
        json!({"node_id":"dec:plain","field":"decision","new":"z".repeat(170)}),
    )
    .await;
    let entry = &revised["revision"]["replaced"][0];
    let at_risk = revised["revision"]["fields_at_risk"]
        .as_array()
        .is_some_and(|a| a.iter().any(|f| f == "decision"));
    if at_risk {
        assert!(
            entry["prior"].is_string(),
            "a prior value nothing else holds is echoed in full — it is the only copy: {entry}"
        );
    } else {
        assert!(
            entry.get("prior").is_none()
                && entry["prior_chars"].as_u64().is_some()
                && entry["prior_in"].as_str().is_some_and(|s| !s.is_empty()),
            "a preserved prior value is given by size and where it is kept: {entry}"
        );
    }
    assert!(chars(&revised) < BOUND, "{}", chars(&revised));
}

/// Echoed values are exactly the short ones: nothing over [`SHORT`] characters
/// rides in `properties`, and every elided field names its size.
#[tokio::test]
async fn a_receipt_echoes_short_values_and_sizes_long_ones() {
    let c = connect().await;
    let name = "n".repeat(SHORT + 1);
    let r = ok(
        &c,
        "add_requirement",
        json!({"id":"req:edge","name":name,"statement":"x".repeat(SHORT)}),
    )
    .await;
    assert_eq!(
        r["properties"]["statement"].as_str().map(str::len),
        Some(SHORT),
        "a value of exactly {SHORT} characters is echoed: {r}"
    );
    assert_eq!(
        r["elided"]["name"].as_u64(),
        Some((SHORT + 1) as u64),
        "a value one character longer is given by size: {r}"
    );
    assert!(
        r["properties"].get("name").is_none(),
        "and is not also echoed: {r}"
    );
}

/// EVERY served write is classified by what its default reply carries, and a
/// write that joins the surface unclassified fails here.
///
/// `fact:root-cause-external-dependency-replies-with-the-whole-manifest-because-the-receipt-shapes-only-node-and-edge-records-2026-10-02`:
/// the receipt contract was enforced by STRUCTURE — "a new write tool joins by
/// being served" — and that holds only for a write whose reply is a node or
/// edge record. `external_dependency` replied with every declaration in the
/// design as TOML (about 150 bytes more per declared dependency), declared
/// `echo`, passed every test above, and was silently exempt. So the membership
/// a receipt cannot see is now written down and reviewed, like
/// `replies_are_bounded.py`'s `DOCUMENT_SHAPED`: a new write must be put in
/// one of the three lists below by whoever adds it, with a reason when it is a
/// report.
///
/// What this pins is the REVIEW: membership can no longer be silent. The
/// lists are claims a person checks when adding a write; the measured
/// behaviour of the one write that was wrong is pinned in
/// `a_dependency_declaration_replies_with_a_receipt.rs`, which failed on main
/// at 293f957.
#[tokio::test]
async fn every_served_write_is_classified_by_what_its_reply_carries() {
    // The reply carries the node or edge this call wrote, as a record (at the
    // top or nested): the choke point shapes it into a receipt.
    const RECORD: &[&str] = &[
        "add_actor",
        "add_artifact",
        "add_capability",
        "add_change_event",
        "add_component",
        "add_constraint",
        "add_contributor",
        "add_decision",
        "add_design_rule",
        "add_environment",
        "add_environment_rule",
        "add_epoch",
        "add_flow",
        "add_interface",
        "add_project",
        "add_readiness",
        "add_release",
        "add_requirement",
        "add_resource",
        "add_verification",
        "allocate",
        "answers",
        "authored_by",
        "calibrated_against",
        "constrains",
        "consumes",
        "contains",
        "create_edge",
        "create_node",
        "create_nodes",
        "decomposes",
        "deploy_to",
        "depends_on",
        "documents",
        "external_dependency",
        "forecast_readiness",
        "gate_on",
        "governed_by",
        "invalidates",
        "link_artifact",
        "owned_by",
        "part_of_flow",
        "performed_in",
        "plan_epoch",
        "provides",
        "realizes",
        "record_alias",
        "record_finding",
        "release_includes",
        "replace_text",
        "require_resource",
        "satisfies",
        "set_artifact_checksum",
        "set_artifact_checksums",
        "set_artifact_intent",
        "set_capability_delivery",
        "set_capability_signature",
        "set_capability_status",
        "set_closure_criterion",
        "set_decision_status",
        "set_epoch_status",
        "set_evidence_scope",
        "set_interface_designation",
        "set_interface_spec",
        "set_project_mode",
        "set_provenance",
        "set_quality_target",
        "set_requirement_designation",
        "set_requirement_lineage",
        "set_requirement_status",
        "set_verification_kind",
        "set_verification_status",
        "snapshot_before_change",
        "verifies",
    ];
    // A fixed-size acknowledgement: the ids this call acted on, and a flag or a
    // status. Nothing stored comes back, so nothing can grow.
    const ACKNOWLEDGEMENT: &[&str] = &[
        "acknowledge_defect",
        "acknowledge_gap",
        "answer_question",
        "complies_with",
        "delete_edge",
        "delete_node",
        "imposes",
        "operates_in",
        "pin_at_epoch",
        "precedes",
        "release_claim",
        "report_manual_work",
        "schedule_for",
        "set_violation_status",
        "violates_rule",
        "withdraw_defect_acknowledgement",
        "withdraw_gap_acknowledgement",
        "withdraw_question",
    ];
    // A REPORT about this call's own work. Each grows only with what the call
    // was given or asked to do, never with the rest of the design — or is cut
    // by `budget_chars`. The reason is the review.
    const REPORT: &[(&str, &str)] = &[
        ("acknowledge_gaps", "one line per gap the call acknowledged"),
        (
            "apply_heal",
            "the operations of the proposal it was handed: applied, skipped, verified",
        ),
        ("apply_merge", "what the merge it was handed changed"),
        (
            "claim_region",
            "the claim: its seed, depth and the region it covers",
        ),
        (
            "collapse_decision",
            "the alternatives of the one decision it collapsed",
        ),
        (
            "contain_component",
            "the one move it made: from, to, and what that changed",
        ),
        (
            "design_identity",
            "this design's own identity, a fixed set of fields",
        ),
        (
            "draw_edges",
            "one line per edge item it was handed; bounded by `budget_chars`",
        ),
        (
            "create_edges",
            "one line per edge it was handed; bounded by `budget_chars`",
        ),
        (
            "gap_to_prompt",
            "the prompt for the one gap it was asked about",
        ),
        (
            "gaps_to_prompts",
            "the prompts for the gaps it was asked about",
        ),
        (
            "genesis",
            "what it created from the paragraph it was handed",
        ),
        (
            "import_graph",
            "counts and ids of what the import it was handed wrote",
        ),
        (
            "ingest_corpus_step",
            "this step over the documents it was handed",
        ),
        ("ingest_step", "this step over the input it was handed"),
        (
            "mirror_surface",
            "what it mirrored from the surface it was pointed at",
        ),
        (
            "move_component",
            "the one move it made: from, to, and what that changed",
        ),
        (
            "reconcile_artifacts",
            "one finding per artifact that diverged; bounded by `budget_chars`",
        ),
        (
            "reconcile_deployment",
            "one finding per environment it was handed",
        ),
        (
            "reconcile_verification",
            "one finding per observed check it was handed",
        ),
        (
            "register_alternative",
            "a reference to the one alternative it registered",
        ),
        (
            "release_includes_all",
            "the manifest of the one release it was asked about",
        ),
        (
            "reopen_choice",
            "the one decision it reopened and the one it created",
        ),
        ("review_relations", "one outcome per relation it was handed"),
        (
            "usage_report",
            "the usage window it closes, which is the report it exists to give",
        ),
    ];
    let c = connect().await;
    let served: std::collections::BTreeSet<String> =
        write_tools(&c).await.into_iter().map(|(n, _)| n).collect();
    let mut listed: std::collections::BTreeMap<&str, &str> = std::collections::BTreeMap::new();
    for (name, list) in RECORD
        .iter()
        .map(|n| (*n, "RECORD"))
        .chain(ACKNOWLEDGEMENT.iter().map(|n| (*n, "ACKNOWLEDGEMENT")))
        .chain(REPORT.iter().map(|(n, _)| (*n, "REPORT")))
    {
        if let Some(other) = listed.insert(name, list) {
            panic!("`{name}` is in both {other} and {list}: a reply has one shape");
        }
    }
    let unclassified: Vec<&String> = served
        .iter()
        .filter(|n| !listed.contains_key(n.as_str()))
        .collect();
    assert!(
        unclassified.is_empty(),
        "{} served write tool(s) are not classified by what their reply carries: \
         {unclassified:?}. Put each in RECORD (it replies with the node or edge it wrote — the \
         receipt layer then shapes it), ACKNOWLEDGEMENT (a fixed-size reply naming ids), or \
         REPORT with the reason its reply does not grow with the rest of the design. A reply \
         that is none of these is the defect this test exists for.",
        unclassified.len()
    );
    let stale: Vec<&&str> = listed.keys().filter(|n| !served.contains(**n)).collect();
    assert!(
        stale.is_empty(),
        "classified but not a served write tool (renamed, retired, or now read-only): {stale:?}"
    );
}
