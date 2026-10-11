//! EVERY tool that reads another design takes it BY GRAPH_ID, and a registry
//! server opens no path a call names — the rest of the family after
//! `compare_designs` and `compose_and_analyse`
//! (`tests/a_cross_design_read_takes_the_other_design_by_graph_id.rs`).
//!
//! `req:every-cross-design-operation-takes-its-other-designs-by-graph-id-under-the-hosts-read-filter`,
//! bounded by
//! `dec:a-cross-design-read-is-bounded-by-the-readable-set-the-host-hands-over-not-by-the-root-alone`.
//!
//! WHAT IS ASSERTED, over the real binary serving designs under one root:
//!   · the READS widened — `certify_preservation`, `merge_designs`,
//!     `analyze_alternatives`, `pair_designs`, `seam_report` — answer from the
//!     other design named by graph_id inside the readable set, each with a
//!     field only B's content could have produced
//!   · each of them refuses a design outside the set, naming the id and the
//!     key, and leaking nothing that design holds
//!   · on a registry server EVERY path a call names is refused — the reads
//!     above, `fork_point`'s `export_path` and `describe_designs`' `paths`
//!     (which would otherwise name the design stored at a path on the host),
//!     and the WRITES that take a design from a path: `apply_merge`,
//!     `import_graph`'s `path` and `external_dependency`'s `design_export`
//!   · a graph_id and a path (or an inline export) together are refused,
//!     naming both
//!
//! And in-process, a stdio-shaped server (no registry) refuses `other_graph_id`
//! on each widened read with the sentence that says where it IS served.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const GATEWAY: &str = "flo2.io";
const READABLE: &str = "reflow2/readable_graph_ids";
const NOT_SERVED: &str = "a path is not served here";

fn tmp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "reflow2-every-path-read-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn bin() -> PathBuf {
    let mut p = std::env::current_exe().expect("test binary has a path");
    p.pop();
    if p.ends_with("deps") {
        p.pop();
    }
    p.join("reflow2-mcp")
}

trait WaitKill {
    fn wait_timeout_kill(&mut self, d: Duration) -> Option<std::process::ExitStatus>;
}
impl WaitKill for Child {
    fn wait_timeout_kill(&mut self, d: Duration) -> Option<std::process::ExitStatus> {
        let start = Instant::now();
        loop {
            match self.try_wait() {
                Ok(Some(s)) => return Some(s),
                Ok(None) if start.elapsed() < d => std::thread::sleep(Duration::from_millis(50)),
                _ => {
                    let _ = self.kill();
                    return None;
                }
            }
        }
    }
}

/// Mint a design over stdio: a requirement only it holds (`req:<marker>`,
/// statement `only-in-<marker>`), an Interface only it holds
/// (`if:<marker>-api`), and `who:tester` as a Contributor so a write the test
/// makes as the gateway's caller passes the writes-for check and reaches the
/// tool. Returns its graph_id.
fn mint(dir: &Path, marker: &str) -> String {
    let store = dir.join(".reflow2").join("graph");
    let mut child = Command::new(bin())
        .arg("--graph-path")
        .arg(&store)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn reflow2-mcp to mint a design");
    let calls = [
        json!({"name": "add_requirement", "arguments": {
            "id": format!("req:{marker}"), "name": marker,
            "statement": format!("only-in-{marker}")}}),
        json!({"name": "add_interface", "arguments": {
            "id": format!("if:{marker}-api"), "name": format!("{marker} api")}}),
        json!({"name": "add_contributor", "arguments": {"id": "who:tester", "name": "tester"}}),
    ];
    let last = calls.len() + 1;
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2025-06-18","capabilities":{{}},"clientInfo":{{"name":"mint","version":"1"}}}}}}"#
        )
        .expect("write initialize");
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#
        )
        .expect("write initialized");
        for (i, params) in calls.iter().enumerate() {
            let msg =
                json!({"jsonrpc": "2.0", "id": i + 2, "method": "tools/call", "params": params});
            writeln!(stdin, "{msg}").expect("write a minting call");
        }
    }
    // Read until the last write's reply so the store holds it before stdin closes.
    {
        let stdout = child.stdout.as_mut().expect("stdout");
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let deadline = Instant::now() + Duration::from_secs(60);
        let want = format!(r#""id":{last}"#);
        loop {
            line.clear();
            if reader.read_line(&mut line).unwrap_or(0) == 0 || Instant::now() > deadline {
                break;
            }
            if line.contains(&want) {
                break;
            }
        }
    }
    drop(child.stdin.take());
    let _ = child.wait_timeout_kill(Duration::from_secs(60));
    let id_file = dir.join(".reflow2").join("graph.id.json");
    let raw = std::fs::read_to_string(&id_file)
        .unwrap_or_else(|e| panic!("no identity sidecar at {}: {e}", id_file.display()));
    let v: Value = serde_json::from_str(&raw).expect("identity sidecar is json");
    v["graph_id"]
        .as_str()
        .expect("identity sidecar names a graph_id")
        .to_string()
}

struct Server {
    child: Child,
    port: u16,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A registry over `root` behind a trusted gateway — flo2.io's shape.
fn serve(root: &Path) -> Server {
    let mut child = Command::new(bin())
        .arg("--registry-root")
        .arg(root)
        .arg("--http")
        .arg("127.0.0.1:0")
        .env("REFLOW2_TRUSTED_GATEWAY", GATEWAY)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the registry server");
    let stderr = child.stderr.take().expect("stderr piped");
    let (tx, rx) = std::sync::mpsc::channel::<u16>();
    std::thread::spawn(move || {
        let mut sent = false;
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if !sent && let Some(i) = line.find("http://127.0.0.1:") {
                let tail = &line[i + "http://127.0.0.1:".len()..];
                let digits: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(p) = digits.parse::<u16>() {
                    let _ = tx.send(p);
                    sent = true;
                }
            }
        }
    });
    let port = rx
        .recv_timeout(Duration::from_secs(90))
        .expect("the server prints the address it bound within 90s");
    Server { child, port }
}

/// One HTTP exchange: (headers, body), the body de-chunked.
fn exchange(port: u16, path: &str, body: &str, session: Option<&str>) -> (String, String) {
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
    s.set_read_timeout(Some(Duration::from_secs(300))).ok();
    let mut req = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n\
         Accept: application/json, text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(sid) = session {
        req.push_str(&format!("mcp-session-id: {sid}\r\n"));
    }
    req.push_str("\r\n");
    req.push_str(body);
    s.write_all(req.as_bytes()).expect("write request");
    let mut raw = Vec::new();
    let _ = s.read_to_end(&mut raw);
    let raw = String::from_utf8_lossy(&raw).to_string();
    let (head, rest) = raw.split_once("\r\n\r\n").unwrap_or((raw.as_str(), ""));
    let chunked = head
        .lines()
        .any(|l| l.to_ascii_lowercase().starts_with("transfer-encoding:") && l.contains("chunked"));
    let body = if chunked {
        let mut out = String::new();
        let mut rest = rest;
        while let Some((size, tail)) = rest.split_once("\r\n") {
            let Ok(n) = usize::from_str_radix(size.trim(), 16) else {
                break;
            };
            if n == 0 || tail.len() < n {
                out.push_str(&tail[..n.min(tail.len())]);
                break;
            }
            out.push_str(&tail[..n]);
            rest = tail[n..].strip_prefix("\r\n").unwrap_or(&tail[n..]);
        }
        out
    } else {
        rest.to_string()
    };
    (head.to_string(), body)
}

fn rpc_reply(body: &str, id: u64) -> Value {
    let mut candidates: Vec<Value> = body
        .lines()
        .filter_map(|l| l.strip_prefix("data:"))
        .filter_map(|d| serde_json::from_str(d.trim()).ok())
        .collect();
    if let Ok(v) = serde_json::from_str::<Value>(body.trim()) {
        candidates.push(v);
    }
    candidates
        .into_iter()
        .find(|v| v["id"] == id)
        .unwrap_or_else(|| {
            panic!(
                "no JSON-RPC reply {id} in: {}",
                &body[..body.len().min(2000)]
            )
        })
}

struct Session {
    port: u16,
    path: String,
    sid: String,
    next: u64,
}

/// `Ok(structured)` or `Err(refusal text)`.
type Outcome = Result<Value, String>;

impl Session {
    fn open(port: u16, graph_id: &str) -> Session {
        let path = format!("/g/{graph_id}/mcp");
        let (head, body) = exchange(
            port,
            &path,
            r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"every-path-read-test","version":"1"}}}"#,
            None,
        );
        let _ = rpc_reply(&body, 0);
        let sid = head
            .lines()
            .find_map(|l| {
                let (k, v) = l.split_once(':')?;
                k.trim()
                    .eq_ignore_ascii_case("mcp-session-id")
                    .then(|| v.trim().to_string())
            })
            .unwrap_or_else(|| panic!("the server hands out a session: {head}"));
        let _ = exchange(
            port,
            &path,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            Some(&sid),
        );
        Session {
            port,
            path,
            sid,
            next: 1,
        }
    }

    /// Call `tool` as the gateway would: `_meta` names the signed-in person
    /// and the set of designs they may read.
    fn call(&mut self, tool: &str, args: Value, readable: &[&str]) -> Outcome {
        let id = self.next;
        self.next += 1;
        let params = json!({
            "name": tool,
            "arguments": args,
            "_meta": {"reflow2/writes_for": "who:tester", READABLE: readable},
        });
        let body = json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": params});
        let (_, reply) = exchange(self.port, &self.path, &body.to_string(), Some(&self.sid));
        let r = rpc_reply(&reply, id);
        if let Some(e) = r.get("error") {
            return Err(e.to_string());
        }
        let result = &r["result"];
        if result["isError"] == true {
            return Err(result["content"].to_string());
        }
        Ok(result
            .get("structuredContent")
            .cloned()
            .or_else(|| result.get("content").cloned())
            .unwrap_or(Value::Null))
    }
}

/// Designs under one root, each minted by [`mint`] under its marker.
fn designs(tag: &str, markers: &[&str]) -> (PathBuf, Vec<String>) {
    let root = tmp_root(tag);
    let ids: Vec<String> = markers
        .iter()
        .map(|m| {
            let dir = root.join(m);
            std::fs::create_dir_all(&dir).unwrap();
            mint(&dir, m)
        })
        .collect();
    for (i, a) in ids.iter().enumerate() {
        for b in &ids[i + 1..] {
            assert_ne!(a, b, "two designs must not share a graph_id");
        }
    }
    (root, ids)
}

fn refused(outcome: Outcome, what: &str) -> String {
    match outcome {
        Err(text) => text,
        Ok(v) => panic!("{what} must be refused, but answered: {v}"),
    }
}

/// The widened reads, each called with B as the other design by graph_id.
/// (`merge_designs` also needs a common ancestor, named separately.)
fn by_graph_id(tool: &str, b: &str) -> Value {
    match tool {
        "analyze_alternatives" => json!({"other_graph_ids": [b]}),
        "seam_report" => json!({
            "other_graph_id": b,
            "pairs": [{"ours": "if:alpha-api", "theirs": "if:beta-api"}],
        }),
        _ => json!({"other_graph_id": b}),
    }
}

const WIDENED: [&str; 4] = [
    "certify_preservation",
    "analyze_alternatives",
    "pair_designs",
    "seam_report",
];

#[test]
fn every_widened_read_answers_from_the_other_design_inside_the_readable_set() {
    let (root, ids) = designs("allowed", &["alpha", "beta", "gamma"]);
    let (a, b, c) = (&ids[0], &ids[1], &ids[2]);
    let server = serve(&root);
    let mut on_a = Session::open(server.port, a);
    let all = [a.as_str(), b.as_str(), c.as_str()];

    // certify_preservation: the live graph is the base, B the other side —
    // and B's requirement is function A does not have, so not preserved.
    let cert = on_a
        .call("certify_preservation", json!({"other_graph_id": b}), &all)
        .unwrap_or_else(|e| panic!("certify against B inside the set must answer: {e}"));
    assert_eq!(cert["base"], a.as_str(), "base is this design: {cert}");
    assert_eq!(cert["other"], b.as_str(), "other is B: {cert}");
    assert!(
        cert["function_changes"].to_string().contains("req:beta"),
        "B's requirement is a function change only B could show: {cert}"
    );

    // merge_designs: ours is the live graph, theirs B, the ancestor C.
    let merge = on_a
        .call(
            "merge_designs",
            json!({"other_graph_id": b, "base_graph_id": c}),
            &all,
        )
        .unwrap_or_else(|e| panic!("merge with B over C inside the set must answer: {e}"));
    assert_eq!(merge["ours"], a.as_str(), "ours is this design: {merge}");
    assert_eq!(merge["theirs"], b.as_str(), "theirs is B: {merge}");
    assert_eq!(merge["base"], c.as_str(), "the ancestor is C: {merge}");
    assert!(
        merge.to_string().contains("req:beta"),
        "what theirs added since the ancestor is in the proposal: {merge}"
    );

    // analyze_alternatives: the live graph is the baseline, B a branch.
    let aoa = on_a
        .call(
            "analyze_alternatives",
            by_graph_id("analyze_alternatives", b),
            &all,
        )
        .unwrap_or_else(|e| panic!("alternatives with B inside the set must answer: {e}"));
    assert_eq!(
        aoa["baseline"],
        a.as_str(),
        "baseline is this design: {aoa}"
    );
    let branch = aoa["branches"]
        .as_array()
        .and_then(|bs| bs.iter().find(|x| x["label"] == b.as_str()))
        .unwrap_or_else(|| panic!("B is measured as a branch: {aoa}"));
    assert!(
        branch["divergence_from_baseline"].is_object(),
        "B's divergence from this design is measured: {aoa}"
    );

    // pair_designs: B's boundary, which only B holds, is named among theirs.
    let pairing = on_a
        .call("pair_designs", by_graph_id("pair_designs", b), &all)
        .unwrap_or_else(|e| panic!("pair with B inside the set must answer: {e}"));
    assert!(
        pairing["unclassified_theirs"]
            .to_string()
            .contains("if:beta-api"),
        "B's interface is among theirs: {pairing}"
    );

    // seam_report: the pair resolves on BOTH sides only if B was read.
    let seam = on_a
        .call("seam_report", by_graph_id("seam_report", b), &all)
        .unwrap_or_else(|e| panic!("seam report with B inside the set must answer: {e}"));
    assert_eq!(seam["pairs_checked"], 1, "{seam}");
    assert_eq!(
        seam["unresolved_pairs"],
        json!([]),
        "B's side of the pair was found, so B was read: {seam}"
    );
}

#[test]
fn every_widened_read_refuses_a_design_outside_the_readable_set() {
    let (root, ids) = designs("outside", &["alpha", "beta", "gamma"]);
    let (a, b, c) = (&ids[0], &ids[1], &ids[2]);
    let server = serve(&root);
    let mut on_a = Session::open(server.port, a);

    let mut cases: Vec<(&str, Value)> = WIDENED.iter().map(|t| (*t, by_graph_id(t, b))).collect();
    // The ancestor is readable; theirs is not.
    cases.push((
        "merge_designs",
        json!({"other_graph_id": b, "base_graph_id": c}),
    ));
    let mut problems: Vec<String> = Vec::new();
    for (tool, args) in cases {
        let text = match on_a.call(tool, args, &[a.as_str(), c.as_str()]) {
            Err(text) => text,
            Ok(v) => {
                problems.push(format!("{tool} read a design outside the set: {v}"));
                continue;
            }
        };
        if !(text.contains(b.as_str()) && text.contains(READABLE)) {
            problems.push(format!(
                "{tool}: the refusal does not name the id asked for and the key the host hands \
                 the set in: {text}"
            ));
        }
        for leak in ["only-in-beta", "req:beta", "if:beta-api"] {
            if text.contains(leak) {
                problems.push(format!(
                    "{tool}: the refusal leaks what B holds ({leak}): {text}"
                ));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "{} problem(s):\n{}",
        problems.len(),
        problems.join("\n")
    );

    // The ancestor outside the set is refused the same way, naming it.
    let text = refused(
        on_a.call(
            "merge_designs",
            json!({"other_graph_id": b, "base_graph_id": c}),
            &[a.as_str(), b.as_str()],
        ),
        "merge_designs with its ancestor outside the readable set",
    );
    assert!(text.contains(c.as_str()), "names the ancestor: {text}");
    assert!(!text.contains("only-in-gamma"), "leaks nothing: {text}");
}

#[test]
fn a_registry_server_refuses_every_path_a_call_names() {
    let (root, ids) = designs("path", &["alpha", "beta"]);
    let (a, b) = (&ids[0], &ids[1]);
    let server = serve(&root);
    let mut on_a = Session::open(server.port, a);
    let both = [a.as_str(), b.as_str()];

    // A real, readable export on the host, and B's store — what a path could
    // reach on a host that holds every tenant under one root.
    let export = root.join("some-export.json");
    std::fs::write(
        &export,
        r#"{"graph_id":"leak","nodes":[{"node_type":"Requirement","node_id":"req:leaked","properties":{"name":"leaked","statement":"only-on-the-host"}}],"edges":[]}"#,
    )
    .unwrap();
    let export = export.to_str().unwrap().to_string();
    let b_store = root.join("beta").join(".reflow2").join("graph");
    let b_store = b_store.to_str().unwrap().to_string();

    let cases: Vec<(&str, Value)> = vec![
        // READS
        ("certify_preservation", json!({"base_path": export})),
        (
            "certify_preservation",
            json!({"base_path": export, "other_path": export}),
        ),
        (
            "merge_designs",
            json!({"base_path": export, "ours_path": export, "theirs_path": export}),
        ),
        ("analyze_alternatives", json!({"paths": [export, export]})),
        (
            "fork_point",
            json!({"decision_id": "dec:anything", "export_path": export}),
        ),
        ("describe_designs", json!({"paths": [b_store]})),
        // WRITES
        (
            "apply_merge",
            json!({"base_path": export, "theirs_path": export}),
        ),
        ("import_graph", json!({"path": export})),
        (
            "external_dependency",
            json!({"id": "dep:upstream", "name": "upstream", "source": "somewhere",
                   "version": "1", "design_export": export}),
        ),
    ];
    // Every case is tried and every failure named, so one hole does not hide
    // the next.
    let mut problems: Vec<String> = Vec::new();
    for (tool, args) in cases {
        let text = match on_a.call(tool, args.clone(), &both) {
            Err(text) => text,
            Ok(v) => {
                let v = v.to_string();
                problems.push(format!(
                    "{tool} ({args}) READ THE PATH AND ANSWERED: {}",
                    &v[..v.len().min(300)]
                ));
                continue;
            }
        };
        if !text.contains(NOT_SERVED) || !text.contains("graph_id") {
            problems.push(format!(
                "{tool}: refused, but not because a path is not served here (naming the graph_id \
                 route): {}",
                &text[..text.len().min(300)]
            ));
        }
        for leak in ["only-on-the-host", "req:leaked", b.as_str()] {
            if text.contains(leak) {
                problems.push(format!("{tool}: the refusal leaks {leak}: {text}"));
            }
        }
    }

    // The writes changed nothing: the host's requirement never reached A.
    let read = on_a.call("get_node", json!({"node_id": "req:leaked"}), &both);
    if !(read.is_err() || read.as_ref().is_ok_and(|v| v["node"].is_null())) {
        problems.push(format!(
            "the host's file was imported into this design: {read:?}"
        ));
    }

    // A graph_id together with a path, or with an inline export, is refused
    // naming both arguments.
    let inline = json!({"graph_id": "x", "nodes": [], "edges": []});
    let both_ways: Vec<(&str, Value, &str)> = vec![
        (
            "certify_preservation",
            json!({"other_graph_id": b, "other_path": export}),
            "other_path",
        ),
        (
            "certify_preservation",
            json!({"other_graph_id": b, "base_path": export}),
            "base_path",
        ),
        (
            "merge_designs",
            json!({"other_graph_id": b, "base_graph_id": b, "theirs_path": export}),
            "theirs_path",
        ),
        (
            "analyze_alternatives",
            json!({"other_graph_ids": [b], "paths": [export, export]}),
            "paths",
        ),
        (
            "pair_designs",
            json!({"other_graph_id": b, "design": inline}),
            "design",
        ),
        (
            "seam_report",
            json!({"other_graph_id": b, "design": inline, "pairs": []}),
            "design",
        ),
    ];
    for (tool, args, other) in both_ways {
        match on_a.call(tool, args, &both) {
            Ok(v) => problems.push(format!(
                "{tool} with a graph_id and {other} together answered: {v}"
            )),
            Err(text) if !(text.contains("graph_id") && text.contains(other)) => problems.push(
                format!("{tool}: does not name both arguments it will not combine: {text}"),
            ),
            Err(_) => {}
        }
    }
    assert!(
        problems.is_empty(),
        "{} problem(s):\n{}",
        problems.len(),
        problems.join("\n")
    );
}

/// A server that holds one design and no registry — stdio's shape — says where
/// `other_graph_id` is served instead of guessing, on every widened read.
#[tokio::test]
async fn a_server_without_a_registry_says_where_other_graph_id_is_served() {
    use reflow2_mcp::service::{
        AnalyzeAlternativesReq, CertifyPreservationReq, MergeDesignsReq, PairDesignsReq,
        ReflowService, SeamReportReq,
    };
    use rmcp::handler::server::wrapper::Parameters;

    fn req<T: serde::de::DeserializeOwned>(v: Value) -> Parameters<T> {
        Parameters(
            serde_json::from_value(v.clone())
                .unwrap_or_else(|e| panic!("the request {v} must be accepted: {e}")),
        )
    }

    let s = ReflowService::in_memory_as("lonely").expect("in-memory service");
    let mut refusals = Vec::new();
    refusals.push(
        s.certify_preservation(req::<CertifyPreservationReq>(
            json!({"other_graph_id": "someone-else"}),
        ))
        .await
        .map(|_| ())
        .expect_err("certify by graph_id on a single-design server"),
    );
    refusals.push(
        s.merge_designs(req::<MergeDesignsReq>(
            json!({"other_graph_id": "someone-else", "base_graph_id": "their-ancestor"}),
        ))
        .await
        .map(|_| ())
        .expect_err("merge by graph_id on a single-design server"),
    );
    refusals.push(
        s.analyze_alternatives(req::<AnalyzeAlternativesReq>(
            json!({"other_graph_ids": ["someone-else"]}),
        ))
        .await
        .map(|_| ())
        .expect_err("alternatives by graph_id on a single-design server"),
    );
    refusals.push(
        s.pair_designs(req::<PairDesignsReq>(
            json!({"other_graph_id": "someone-else"}),
        ))
        .await
        .map(|_| ())
        .expect_err("pair by graph_id on a single-design server"),
    );
    refusals.push(
        s.seam_report(req::<SeamReportReq>(
            json!({"other_graph_id": "someone-else", "pairs": []}),
        ))
        .await
        .map(|_| ())
        .expect_err("seam report by graph_id on a single-design server"),
    );
    for err in refusals {
        assert!(
            err.message.contains("served only by a registry server"),
            "says where it is served: {}",
            err.message
        );
    }

    // A call that names no other design at all is refused for that, naming
    // both routes, never answered from nothing.
    let err = s
        .certify_preservation(req::<CertifyPreservationReq>(json!({})))
        .await
        .map(|_| ())
        .expect_err("nothing to certify against is a refusal");
    assert!(
        err.message.contains("base_path") && err.message.contains("other_graph_id"),
        "names both routes: {}",
        err.message
    );
    let err = s
        .merge_designs(req::<MergeDesignsReq>(json!({})))
        .await
        .map(|_| ())
        .expect_err("nothing to merge is a refusal");
    assert!(
        err.message.contains("base_path") && err.message.contains("other_graph_id"),
        "names both routes: {}",
        err.message
    );
    let err = s
        .analyze_alternatives(req::<AnalyzeAlternativesReq>(json!({})))
        .await
        .map(|_| ())
        .expect_err("no alternatives is a refusal");
    assert!(
        err.message.contains("paths") && err.message.contains("other_graph_ids"),
        "names both routes: {}",
        err.message
    );
    for (what, err) in [
        (
            "pair",
            s.pair_designs(req::<PairDesignsReq>(json!({})))
                .await
                .map(|_| ())
                .expect_err("nothing to pair with is a refusal"),
        ),
        (
            "seam",
            s.seam_report(req::<SeamReportReq>(json!({"pairs": []})))
                .await
                .map(|_| ())
                .expect_err("nothing to report a seam against is a refusal"),
        ),
    ] {
        assert!(
            err.message.contains("design") && err.message.contains("other_graph_id"),
            "{what}: names both routes: {}",
            err.message
        );
    }
}
