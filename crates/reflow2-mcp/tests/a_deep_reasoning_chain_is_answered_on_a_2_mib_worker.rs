//! A DEEP REASONING CHAIN IS ANSWERED ON A 2 MiB WORKER, through the server's
//! real request path
//! (`dec:deep-reasoning-walks-keep-their-own-stack-and-the-runtime-gets-headroom`,
//! Anthony, 2026-10-26: "Go with 1 + 3").
//!
//! Measured on a release build of main 0c567349c
//! (`fact:a-release-server-overflows-its-2-mib-worker-stack-on-a-reasoning-chain-about-2000-steps-deep-and-the-whole-process-aborts-2026-10-25`):
//! get_node on the last claim of a chain about 3,900 steps deep, and
//! loop_status on a chain about 2,000 steps deep whose steps hold, overflowed
//! the 2 MiB stack of the tokio worker running the call, and the whole process
//! aborted (exit 134). Two walks recursed once per step with no bound:
//! `foundations_walk` (get_node) and `proof_chain_walk` (loop_status).
//!
//! The server now gives its runtime threads a larger stack
//! (`reflow2_mcp::runtime::SERVER_THREAD_STACK`). These tests deliberately do
//! NOT use it: the server runs on a runtime whose workers have the 2 MiB the
//! server's workers had before the fix, so a pass here says the walks were
//! fixed, not that the stack grew.
//!
//! A STACK OVERFLOW ABORTS THE WHOLE TEST PROGRAM, so each case runs in a
//! CHILD PROCESS: this test binary again, filtered to the one test, with
//! [`CHILD`] set. The parent asserts the child exited cleanly and ran.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};
use std::process::Command;

/// The stack each runtime worker gets here: 2 MiB, the size every tokio
/// worker had before the fix (std's default for a spawned thread).
const WORKER_STACK: usize = 2 << 20;

/// Set in the child process: the test does the work instead of spawning.
const CHILD: &str = "REFLOW2_DEEP_CHAIN_CHILD";

/// How deep the chain is: deep enough that the walks, recursing, overflowed a
/// 2 MiB worker in THIS build, and no deeper, because loop_status reads every
/// claim's proof status, a walk to the bottom each, so its time grows with
/// the square of the depth (a separate and known problem). A debug build's
/// frames are several times a release build's, so the depths differ.
/// Measured on main 0c567349c with the design [`chain`] builds, on a 2 MiB
/// worker: in debug, loop_status answered at 300 steps and overflowed at 600
/// and 1,000, and get_node overflowed at 1,000; in release, `proof_chain_walk`
/// overflowed at about 2,000 steps and `foundations_walk` at about 3,900. On
/// this branch loop_status takes about 35 s at 1,000 steps in debug and about
/// 70 s at 3,000 in release.
const STEPS: usize = if cfg!(debug_assertions) { 1_000 } else { 3_000 };

/// A design whose reasoning is ONE chain `steps` deep, as an export:
/// `con:ground` → `inf:s0` → `req:c1` → `inf:s1` → … → `req:c{steps}`, every
/// step also resting on `con:limit`, both constraints, which hold.
///
/// EVERY STEP READS THE CLAIM IT RESTS ON AS ITS OWNER'S INTENT
/// (`reads_as: intent`), and every claim is accepted, so every step holds
/// without the truth of the chain beneath it being worked out. That keeps the
/// truth evaluation, which is bounded at 64 steps and whose frames a DEBUG
/// build makes far larger (it needs between 4 and 6 MiB there for a chain
/// past 64 steps, measured on this branch), out of the way: what runs deep is
/// exactly the two walks. get_node then walks the whole chain twice, once for
/// its foundations and once for its proof status, because its top step holds.
fn chain(steps: usize) -> (Value, String) {
    let w = steps.to_string().len();
    let claim = |k: usize| format!("req:c{k:0w$}");
    let mut nodes = vec![
        json!({"node_type": "Constraint", "node_id": "con:ground",
            "properties": {"name": "the ground", "statement": "A limit the design keeps."}}),
        json!({"node_type": "Constraint", "node_id": "con:limit",
            "properties": {"name": "a limit", "statement": "A limit the design keeps."}}),
    ];
    let mut edges = Vec::new();
    for k in 1..=steps {
        nodes.push(json!({"node_type": "Requirement", "node_id": claim(k),
            "properties": {"name": format!("claim {k}"), "statement": "The owner's need.",
                           "status": "accepted"}}));
    }
    for k in 0..steps {
        let s = format!("inf:s{k:0w$}");
        nodes.push(json!({"node_type": "Inference", "node_id": s,
            "properties": {"name": s, "rule": "all"}}));
        let (premise, reads) = if k == 0 {
            ("con:ground".to_string(), json!({}))
        } else {
            (claim(k), json!({"reads_as": "intent"}))
        };
        for (t, f, to, props) in [
            ("PREMISE_OF", premise, s.clone(), reads),
            ("PREMISE_OF", "con:limit".to_string(), s.clone(), json!({})),
            ("CONCLUDES", s.clone(), claim(k + 1), json!({})),
        ] {
            edges.push(json!({"edge_type": t, "from_id": f, "to_id": to, "properties": props}));
        }
    }
    let doc = json!({"graph_id": format!("deep-{steps}"), "nodes": nodes, "edges": edges});
    (doc, claim(steps))
}

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "deep-chain-probe".to_string();
        cfg
    }
}

/// Serve an in-memory design over a pipe, import `doc`, make one call, and
/// return its reply — every step on a runtime whose workers have exactly
/// [`WORKER_STACK`]. The server is spawned onto the runtime, and rmcp runs
/// each request as a task there, so the call runs on a worker, as in a
/// served process.
fn call_on_a_2_mib_worker(doc: Value, tool: &str, args: Value) -> Value {
    // The server's own runtime builder, with only the stack changed.
    let rt = reflow2_mcp::runtime::server_runtime(WORKER_STACK).expect("a runtime");
    rt.block_on(async move {
        let svc = ReflowService::in_memory().expect("an in-memory service");
        let (server_rx, client_tx) = tokio::io::duplex(1 << 22);
        let (client_rx, server_tx) = tokio::io::duplex(1 << 22);
        let server = tokio::spawn(async move {
            if let Ok(running) = svc.serve((server_rx, server_tx)).await {
                let _ = running.waiting().await;
            }
        });
        let client = Probe
            .serve((client_rx, client_tx))
            .await
            .expect("the in-process handshake");
        let call = |tool: &str, args: Value| {
            let params = rmcp::model::CallToolRequestParams::new(tool.to_string())
                .with_arguments(args.as_object().cloned().unwrap_or_default());
            let client = &client;
            async move {
                let r = client.call_tool(params).await.expect("an answer");
                assert!(!r.is_error.unwrap_or(false), "refused: {:?}", r.content);
                r.structured_content.unwrap_or(Value::Null)
            }
        };
        call("import_graph", json!({"document": doc})).await;
        let reply = call(tool, args).await;
        let _ = client.cancel().await;
        let _ = server.await;
        reply
    })
}

/// In the parent: run THIS test again in a child process and assert it exited
/// cleanly and ran (a filter that matched nothing also exits 0). True in the
/// child, where the caller does the work.
fn in_a_child(test: &str, what: &str) -> bool {
    if std::env::var_os(CHILD).is_some() {
        return true;
    }
    let path = format!(
        "{}::{test}",
        module_path!()
            .split_once("::")
            .map_or(module_path!(), |(_, rest)| rest)
    );
    let out = Command::new(std::env::current_exe().expect("this test binary"))
        .args([path.as_str(), "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD, "1")
        .output()
        .expect("the child test process runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "{what} on a {} MiB worker did not answer: the child process {path} exited {}.\n\
         --- stdout\n{stdout}\n--- stderr\n{stderr}",
        WORKER_STACK >> 20,
        out.status
    );
    assert!(
        stdout.contains("1 passed"),
        "{path}: the child ran no test.\n--- stdout\n{stdout}\n--- stderr\n{stderr}"
    );
    false
}

/// get_node on the last claim of a chain [`STEPS`] deep answers: its
/// `foundations` name the two constraints the whole chain rests on, and its
/// `proof_status` counts every step beneath it — both walks went to the
/// bottom.
#[test]
fn get_node_answers_on_the_last_claim_of_a_deep_chain() {
    if !in_a_child(
        "get_node_answers_on_the_last_claim_of_a_deep_chain",
        "get_node on a deep chain",
    ) {
        return;
    }
    let (doc, top) = chain(STEPS);
    let reply = call_on_a_2_mib_worker(doc, "get_node", json!({"id": top}));
    let node = &reply["node"];
    assert_eq!(node["node_id"], top, "{reply}");
    let found: Vec<&str> = node["foundations"]["foundations"]
        .as_array()
        .unwrap_or_else(|| panic!("no foundations on {top}: {node}"))
        .iter()
        .filter_map(|f| f["id"].as_str())
        .collect();
    assert_eq!(found, ["con:ground", "con:limit"], "{node}");
    assert_eq!(
        node["proof_status"]["total_steps"], STEPS,
        "every step is under the claim: {}",
        node["proof_status"]
    );
}

/// loop_status on a design whose one chain is [`STEPS`] deep and holds
/// answers: it reads every claim's proof status, and the top claim's walks
/// the whole chain.
#[test]
fn loop_status_answers_on_a_deep_holding_chain() {
    if !in_a_child(
        "loop_status_answers_on_a_deep_holding_chain",
        "loop_status on a deep holding chain",
    ) {
        return;
    }
    let (doc, _) = chain(STEPS);
    let reply = call_on_a_2_mib_worker(doc, "loop_status", json!({}));
    assert!(reply.is_object(), "loop_status answered no object: {reply}");
}
