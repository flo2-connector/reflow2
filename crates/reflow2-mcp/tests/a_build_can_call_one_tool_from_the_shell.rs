//! A build can call one tool from the shell.
//!
//! bhome S3 (2026-09-18): the plan-sheet generator read geometry from the IFC
//! and the budget from `design-facts.json`, a hand transcription of
//! `budget_report`, "because reflow2 is an MCP server and not a library the
//! script can import" — so the half of the sheet the design owns was the half
//! with no mechanical guarantee. Measured: the CLI had --export, --import,
//! --diff and --merge-driver, and no one-shot door to a report.
//!
//! `reflow2-mcp --call <tool> --args '<json>'` is that door. It runs the tool
//! through the same server path a session uses, so a refusal is worded the
//! same and usage is recorded the same; it prints the reply's JSON on stdout;
//! and a read-only tool still answers while a server holds the graph, from the
//! best-effort snapshot `--export-snapshot` already uses.
// Some tests here open a design store in a folder: each is marked
// `#[cfg(feature = "rocksdb")]`, and a build without on-disk storage compiles
// them out — and with them helpers only they use, which is all this allows.
#![cfg_attr(not(feature = "rocksdb"), allow(dead_code, unused_imports))]

use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

fn call(graph: &std::path::Path, tool: &str, args: &str) -> Output {
    Command::new(bin())
        .args([
            "--graph-path",
            graph.to_str().unwrap(),
            "--call",
            tool,
            "--args",
            args,
        ])
        .output()
        .expect("the binary runs")
}

fn stdout_json(o: &Output) -> serde_json::Value {
    let text = String::from_utf8_lossy(&o.stdout);
    serde_json::from_str(&text).unwrap_or_else(|e| {
        panic!(
            "stdout is not one JSON document ({e}):\n{text}\nstderr:\n{}",
            String::from_utf8_lossy(&o.stderr)
        )
    })
}

#[cfg(feature = "rocksdb")]
#[test]
fn a_write_then_a_report_from_the_shell_and_the_reply_is_json_on_stdout() {
    let dir = tempfile::tempdir().expect("tempdir");
    let graph = dir.path().join("graph");
    let o = call(
        &graph,
        "add_project",
        r#"{"id":"proj:sheet","name":"Plan sheet"}"#,
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let v = stdout_json(&o);
    assert_eq!(v["node_id"], "proj:sheet");

    let o = call(&graph, "graph_report", "{}");
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let v = stdout_json(&o);
    assert!(v.is_object(), "{v}");
}

#[cfg(feature = "rocksdb")]
#[test]
fn a_refusal_goes_to_stderr_with_exit_one_and_names_what_the_tool_wants() {
    let dir = tempfile::tempdir().expect("tempdir");
    let graph = dir.path().join("graph");
    // add_project without a name: the same missing-field refusal a session
    // gets, on the wire path, not a CLI paraphrase.
    let o = call(&graph, "add_project", r#"{"id":"proj:x"}"#);
    assert_eq!(
        o.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("add_project"), "{err}");
    assert!(err.contains("name"), "{err}");
    assert!(
        o.stdout.is_empty(),
        "a refusal prints nothing a script would parse"
    );
}

#[test]
fn an_unknown_tool_and_non_object_args_are_refused_before_the_graph_is_touched() {
    let dir = tempfile::tempdir().expect("tempdir");
    let graph = dir.path().join("graph");
    let o = call(&graph, "no_such_tool", "{}");
    assert_eq!(o.status.code(), Some(1));
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("no_such_tool"), "{err}");
    assert!(err.contains("find_tools"), "{err}");
    assert!(
        !graph.exists(),
        "an unknown tool must not mint a graph directory"
    );

    let o = call(&graph, "graph_report", "[1,2]");
    assert_eq!(o.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&o.stderr).contains("JSON object"),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
}

#[cfg(feature = "rocksdb")]
#[test]
fn args_can_come_from_stdin() {
    use std::io::Write;
    let dir = tempfile::tempdir().expect("tempdir");
    let graph = dir.path().join("graph");
    let mut child = Command::new(bin())
        .args([
            "--graph-path",
            graph.to_str().unwrap(),
            "--call",
            "add_project",
            "--args",
            "-",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(br#"{"id":"proj:stdin","name":"From stdin"}"#)
        .unwrap();
    let o = child.wait_with_output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(stdout_json(&o)["node_id"], "proj:stdin");
}

/// A read-only tool answers while another process holds the graph; a writer
/// refuses. Measured against a REAL held lock: a server on stdio whose stdin we
/// keep open.
#[cfg(feature = "rocksdb")]
#[test]
fn while_a_server_holds_the_graph_a_read_answers_from_a_snapshot_and_a_write_refuses() {
    use std::io::{BufRead, Write};
    let dir = tempfile::tempdir().expect("tempdir");
    let graph = dir.path().join("graph");
    let o = call(&graph, "add_project", r#"{"id":"proj:held","name":"Held"}"#);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));

    let mut holder = Command::new(bin())
        .args(["--graph-path", graph.to_str().unwrap()])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn holder");
    // WAIT FOR THE HOLDER'S OWN WORD, AND OPEN NOTHING WHILE IT STARTS. A stdio
    // server opens the store before it reads its first message, so its answer
    // to `initialize` comes once it holds the store, and a holder that found
    // the store taken says so in that answer ("UNAVAILABLE").
    //
    // Until 2026-10-03 this waited by probing with a WRITE through `--call`,
    // which takes the store when it is free. A probe that reached the store
    // before the holder did wrote into the design and left the holder degraded,
    // until one of its retries fell between two probes: a wait that raced the
    // thing it was waiting for. The same wait in
    // a_held_designs_read_is_as_true_as_the_live_one.rs made its holder exit
    // and failed CI twice
    // (fact:a-held-design-test-flakes-waiting-30s-for-the-holder-to-take-the-store-2026-10-03).
    let mut stdin = holder.stdin.take().expect("the holder's stdin");
    writeln!(
        stdin,
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2025-06-18","capabilities":{{}},"clientInfo":{{"name":"held-test","version":"0"}}}}}}"#
    )
    .expect("send initialize to the holder");
    let stdout = holder.stdout.take().expect("the holder's stdout");
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let _ = std::io::BufReader::new(stdout).read_line(&mut line);
        let _ = tx.send(line);
    });
    let hello = rx
        .recv_timeout(std::time::Duration::from_secs(120))
        .expect("the holder answered initialize");
    assert!(
        hello.contains("\"result\"") && !hello.contains("UNAVAILABLE"),
        "the holder did not take the store: {hello}"
    );

    // Held: a write from the shell refuses.
    let o = call(
        &graph,
        "add_project",
        r#"{"id":"proj:second","name":"Second"}"#,
    );
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(!o.status.success(), "a write while held must refuse: {err}");
    assert!(
        err.contains("writes") || err.contains("holds") || err.contains("lock"),
        "{err}"
    );

    let o = call(&graph, "graph_report", "{}");
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(o.status.success(), "{err}");
    assert!(
        err.contains("SNAPSHOT"),
        "a read while held must say it is best-effort: {err}"
    );
    assert!(stdout_json(&o).is_object());

    drop(stdin);
    let _ = holder.kill();
    let _ = holder.wait();
}

/// A call whose request and reply are each longer than 16 MiB is answered.
///
/// rmcp 3.6.0 (#1049) bounds an incoming line at 16 MiB and DISCARDS a longer
/// one with only a log line. The door runs its call over an in-process pipe of
/// that transport, so with rmcp's default the side waiting for the long line
/// waited forever: measured 2026-10-22, this exact call answered in 1.5 s on
/// rmcp 3.5.0 and was still waiting when killed at 120 s on 3.6.0, and
/// `--call export_graph` on reflow2's own design (~37 MB) hung the same way.
/// `get_node` echoes the id it could not find, so ONE call carries a long line
/// each way: the request into the server and the reply back to the door.
#[cfg(feature = "rocksdb")]
#[test]
fn a_call_longer_than_rmcps_line_bound_is_answered_both_ways() {
    use std::io::Write;
    use std::time::{Duration, Instant};
    let dir = tempfile::tempdir().expect("tempdir");
    let graph = dir.path().join("graph");
    let o = call(&graph, "add_project", r#"{"id":"proj:big","name":"Big"}"#);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));

    let long_id = format!("req:{}", "x".repeat(17 * 1024 * 1024));
    let args = serde_json::json!({ "id": long_id }).to_string();
    let mut child = Command::new(bin())
        .args([
            "--graph-path",
            graph.to_str().unwrap(),
            "--call",
            "get_node",
            "--args",
            "-",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stdin = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || stdin.write_all(args.as_bytes()));
    let stdout = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut out = Vec::new();
        std::io::Read::read_to_end(&mut { stdout }, &mut out).map(|_| out)
    });
    // Bounded, so a regression fails here instead of hanging the suite.
    let deadline = Instant::now() + Duration::from_secs(120);
    let status = loop {
        if let Some(s) = child.try_wait().expect("wait") {
            break s;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("a call longer than 16 MiB got no answer within 120 s: the door hung");
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    writer.join().unwrap().expect("the arguments were written");
    let out = reader.join().unwrap().expect("stdout read");
    let mut err = String::new();
    std::io::Read::read_to_string(&mut child.stderr.take().unwrap(), &mut err).unwrap();
    assert!(status.success(), "{err}");
    assert!(
        !err.contains("exceeded the maximum line length"),
        "a line was discarded: {err}"
    );
    assert!(
        out.len() > 16 * 1024 * 1024,
        "the reply carries the long id back, so it passes the bound too: {} bytes",
        out.len()
    );
    let v: serde_json::Value = serde_json::from_slice(&out).expect("stdout is one JSON document");
    assert!(v["node"].is_null(), "no such node, and it says so");
}

/// WHAT IS PRINTED IS WHAT THE BUDGET MEASURED (PR B "replies fit", item 5).
/// The door printed the reply pretty-printed, so a reply held to 30,000
/// characters printed 35,087 (the heat rung's read-back, measured 2026-10-23
/// on 0.90.0), and one held to 60,000 printed 73,497. It prints compact JSON
/// now: one line, the size `budget_chars` held.
#[cfg(feature = "rocksdb")]
#[test]
fn a_reply_held_to_n_prints_at_most_n_characters() {
    let dir = tempfile::tempdir().expect("tempdir");
    let graph = dir.path().join("graph");
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../reflow2-core/tests/fixtures/proof_trial_heat_equation.json"
    );
    let o = Command::new(bin())
        .args(["--graph-path", graph.to_str().unwrap(), "--import", fixture])
        .output()
        .expect("the binary runs");
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    for budget in [30_000usize, 60_000] {
        let o = call(
            &graph,
            "reasoning_report",
            &format!(r#"{{"claim":"req:thm-uniqueness","budget_chars":{budget}}}"#),
        );
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        let printed = String::from_utf8_lossy(&o.stdout);
        let printed = printed.trim_end();
        assert!(
            printed.len() <= budget,
            "held to {budget}, printed {}",
            printed.len()
        );
        assert!(!printed.contains('\n'), "one line of compact JSON");
        assert_eq!(stdout_json(&o)["page"]["budget_chars"], budget);
    }
}
