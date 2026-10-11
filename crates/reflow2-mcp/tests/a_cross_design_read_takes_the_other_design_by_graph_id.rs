//! A session on design A reads design B BY GRAPH_ID, through the registry that
//! holds both, and only inside the readable set the trusted host handed over.
//!
//! `req:every-cross-design-operation-takes-its-other-designs-by-graph-id-under-the-hosts-read-filter`
//! and the boundary it reopened,
//! `dec:a-cross-design-read-is-bounded-by-the-readable-set-the-host-hands-over-not-by-the-root-alone`:
//! the trusted gateway passes `_meta["reflow2/readable_graph_ids"]` on EVERY
//! call, reflow2 refuses an id outside it naming the id and nothing else, and
//! a server with no trusted host never reaches a second design by graph_id.
//!
//! WHAT IS ASSERTED, over the real binary serving two designs under one root:
//!   · with the set [A, B], `compare_designs` on A with `other_graph_id: B`
//!     answers a diff labelled A (base, the live graph) and B (other), and
//!     `compose_and_analyse` answers a composed report namespaced by B
//!   · with the set [A] alone the same calls are refused, naming B and
//!     revealing nothing B holds — not even whether it exists
//!   · with no `_meta` at all they are refused, naming the key the host must send
//!   · A's own graph_id as `other_graph_id` is refused
//!   · on a registry server a PATH is refused: `base_path`/`other_path` name a
//!     file on the host, which is not a design it serves
//!   · a registry server with NO trusted host refuses every cross-design read
//!   · A→B and B→A, called concurrently and repeatedly, finish — the other
//!     design is exported under its own read guard and that guard is dropped
//!     before this design's is taken, so no call holds two
//!
//! And in-process, a stdio-shaped server (no registry) refuses `other_graph_id`
//! with the sentence that says where it IS served.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const GATEWAY: &str = "flo2.io";
const READABLE: &str = "reflow2/readable_graph_ids";

fn tmp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "reflow2-cross-design-{tag}-{}-{}",
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

/// Mint a design over stdio, write one requirement only it holds, and return
/// its graph_id. The requirement is the content the refusals must never leak.
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
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{"name":"add_requirement","arguments":{{"id":"req:{marker}","name":"{marker}","statement":"only-in-{marker}"}}}}}}"#
        )
        .expect("write the marker requirement");
    }
    // Read until the write's reply so the store holds it before stdin closes.
    {
        let stdout = child.stdout.as_mut().expect("stdout");
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            line.clear();
            if reader.read_line(&mut line).unwrap_or(0) == 0 || Instant::now() > deadline {
                break;
            }
            if line.contains(r#""id":2"#) {
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

/// A registry over `root`, flo2.io's shape; `gateway` declares the trusted
/// host the way a container does, or leaves the server with none.
fn serve(root: &Path, gateway: Option<&str>) -> Server {
    let mut cmd = Command::new(bin());
    cmd.arg("--registry-root")
        .arg(root)
        .arg("--http")
        .arg("127.0.0.1:0")
        .env_remove("REFLOW2_TRUSTED_GATEWAY")
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if let Some(g) = gateway {
        cmd.env("REFLOW2_TRUSTED_GATEWAY", g);
    }
    let mut child = cmd.spawn().expect("spawn the registry server");
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
            r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"cross-design-test","version":"1"}}}"#,
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

    /// Call `tool` as the gateway would: `_meta` carries the signed-in person
    /// and, when `readable` is given, the set of designs they may read.
    fn call(&mut self, tool: &str, args: Value, readable: Option<&[&str]>) -> Outcome {
        let id = self.next;
        self.next += 1;
        let mut params = json!({"name": tool, "arguments": args});
        let mut meta = json!({"reflow2/writes_for": "who:tester"});
        if let Some(ids) = readable {
            meta[READABLE] = json!(ids);
        }
        params["_meta"] = meta;
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
            .unwrap_or(Value::Null))
    }
}

/// Two designs under one root, each holding a requirement only it has.
fn two_designs(tag: &str) -> (PathBuf, String, String) {
    let root = tmp_root(tag);
    let a_dir = root.join("alpha");
    let b_dir = root.join("beta");
    std::fs::create_dir_all(&a_dir).unwrap();
    std::fs::create_dir_all(&b_dir).unwrap();
    let a = mint(&a_dir, "alpha");
    let b = mint(&b_dir, "beta");
    assert_ne!(a, b, "two designs must not share a graph_id");
    (root, a, b)
}

fn refused(outcome: Outcome, what: &str) -> String {
    match outcome {
        Err(text) => text,
        Ok(v) => panic!("{what} must be refused, but answered: {v}"),
    }
}

#[test]
fn a_session_reads_the_other_design_by_graph_id_inside_the_readable_set() {
    let (root, a, b) = two_designs("allowed");
    let server = serve(&root, Some(GATEWAY));
    let mut on_a = Session::open(server.port, &a);

    // compare_designs: the live graph is the base, labelled by this design's
    // graph_id; the other design is labelled by its own.
    let diff = on_a
        .call(
            "compare_designs",
            json!({"other_graph_id": b}),
            Some(&[a.as_str(), b.as_str()]),
        )
        .unwrap_or_else(|e| panic!("compare against B inside the readable set must answer: {e}"));
    assert_eq!(
        diff["base"],
        a.as_str(),
        "base is this design, by graph_id: {diff}"
    );
    assert_eq!(diff["other"], b.as_str(), "other is B, by graph_id: {diff}");
    assert_eq!(
        diff["summary"]["identical"], false,
        "A holds req:alpha and B holds req:beta, so they differ: {diff}"
    );
    let added = diff["design"]["added"].to_string();
    assert!(
        added.contains("req:beta"),
        "B's requirement is what B has that A does not: {diff}"
    );

    // compose_and_analyse: the other design arrives namespaced by its graph_id
    // when no namespace is given.
    let composed = on_a
        .call(
            "compose_and_analyse",
            json!({"other_graph_id": b}),
            Some(&[a.as_str(), b.as_str()]),
        )
        .unwrap_or_else(|e| panic!("compose with B inside the readable set must answer: {e}"));
    assert_eq!(
        composed["imported_graph_id"],
        b.as_str(),
        "the composed report names B: {composed}"
    );
    assert_eq!(
        composed["namespace"],
        b.as_str(),
        "namespace defaults to the other graph_id: {composed}"
    );
    assert!(
        composed["imported_nodes"].as_u64().unwrap_or(0) >= 1,
        "B's requirement was composed in: {composed}"
    );

    // An explicit namespace is still honoured.
    let composed = on_a
        .call(
            "compose_and_analyse",
            json!({"other_graph_id": b, "namespace": "partner"}),
            Some(&[a.as_str(), b.as_str()]),
        )
        .expect("compose with an explicit namespace answers");
    assert_eq!(composed["namespace"], "partner");
}

#[test]
fn a_graph_id_outside_the_readable_set_is_refused_naming_it_and_nothing_else() {
    let (root, a, b) = two_designs("outside");
    let server = serve(&root, Some(GATEWAY));
    let mut on_a = Session::open(server.port, &a);

    for tool in ["compare_designs", "compose_and_analyse"] {
        // B is not in the set the host handed over.
        let text = refused(
            on_a.call(tool, json!({"other_graph_id": b}), Some(&[a.as_str()])),
            &format!("{tool} against a design outside the readable set"),
        );
        assert!(
            text.contains(&b),
            "{tool}: the refusal names the id asked for: {text}"
        );
        assert!(
            text.contains(READABLE),
            "{tool}: and the key the host hands the set in: {text}"
        );
        assert!(
            !text.contains("only-in-beta") && !text.contains("req:beta"),
            "{tool}: NOTHING B HOLDS may leak through a refusal: {text}"
        );
        assert!(
            !text.to_ascii_lowercase().contains("no design named")
                && !text.to_ascii_lowercase().contains("is registered here"),
            "{tool}: the refusal must not say whether a design by that id exists: {text}"
        );

        // No `_meta` set at all: the host handed nothing over.
        let text = refused(
            on_a.call(tool, json!({"other_graph_id": b}), None),
            &format!("{tool} with no readable set on the call"),
        );
        assert!(text.contains(&b), "{tool}: names the id: {text}");
        assert!(
            text.contains(READABLE),
            "{tool}: names the key the host must send on every call: {text}"
        );
        assert!(
            !text.contains("only-in-beta"),
            "{tool}: nothing B holds may leak: {text}"
        );

        // This design's own graph_id is not another design.
        let text = refused(
            on_a.call(
                tool,
                json!({"other_graph_id": a}),
                Some(&[a.as_str(), b.as_str()]),
            ),
            &format!("{tool} naming this design as the other one"),
        );
        assert!(text.contains(&a), "{tool}: names the id: {text}");
        assert!(
            text.to_ascii_lowercase().contains("itself")
                || text.to_ascii_lowercase().contains("this design"),
            "{tool}: says it is this design: {text}"
        );
    }
}

#[test]
fn a_registry_server_refuses_a_path_and_names_the_graph_id_route() {
    let (root, a, b) = two_designs("path");
    let server = serve(&root, Some(GATEWAY));
    let mut on_a = Session::open(server.port, &a);

    // A real, readable file on the host — exactly the hole: a bare registry
    // read any path it was given.
    let export = root.join("some-export.json");
    std::fs::write(&export, r#"{"graph_id":"leak","nodes":[],"edges":[]}"#).unwrap();
    let export = export.to_str().unwrap().to_string();

    let text = refused(
        on_a.call(
            "compare_designs",
            json!({"base_path": export}),
            Some(&[a.as_str(), b.as_str()]),
        ),
        "a base_path on a registry server",
    );
    assert!(
        text.contains("a path is not served here"),
        "the refusal says a path is not served here: {text}"
    );
    assert!(
        text.contains("graph_id"),
        "and names the route that works, the design by graph_id: {text}"
    );

    let text = refused(
        on_a.call(
            "compare_designs",
            json!({"other_graph_id": b, "other_path": export}),
            Some(&[a.as_str(), b.as_str()]),
        ),
        "other_graph_id together with other_path",
    );
    assert!(
        text.contains("other_graph_id") && text.contains("other_path"),
        "names both arguments it will not combine: {text}"
    );
}

#[test]
fn a_registry_server_with_no_trusted_host_reaches_no_other_design() {
    let (root, a, b) = two_designs("undeclared");
    let server = serve(&root, None);
    let mut on_a = Session::open(server.port, &a);

    for tool in ["compare_designs", "compose_and_analyse"] {
        // Even a call that CLAIMS a readable set: nothing trusted handed it over.
        let text = refused(
            on_a.call(
                tool,
                json!({"other_graph_id": b}),
                Some(&[a.as_str(), b.as_str()]),
            ),
            &format!("{tool} on a registry with no trusted host"),
        );
        assert!(
            text.contains("trusted gateway") || text.contains("--http-trusted-gateway"),
            "{tool}: the refusal says what would establish the readable set: {text}"
        );
        assert!(
            !text.contains("only-in-beta"),
            "{tool}: nothing B holds may leak: {text}"
        );
    }
}

#[test]
fn a_to_b_and_b_to_a_called_together_do_not_deadlock() {
    let (root, a, b) = two_designs("concurrent");
    let server = serve(&root, Some(GATEWAY));
    let port = server.port;
    const ROUNDS: usize = 6;

    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
    for (own, other) in [(a.clone(), b.clone()), (b.clone(), a.clone())] {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let mut session = Session::open(port, &own);
            let readable = [own.as_str(), other.as_str()];
            for round in 0..ROUNDS {
                let tool = if round % 2 == 0 {
                    "compare_designs"
                } else {
                    "compose_and_analyse"
                };
                if let Err(e) =
                    session.call(tool, json!({"other_graph_id": other}), Some(&readable))
                {
                    let _ = tx.send(Err(format!("{own} -> {other} round {round}: {e}")));
                    return;
                }
            }
            let _ = tx.send(Ok(()));
        });
    }
    drop(tx);
    let deadline = Instant::now() + Duration::from_secs(240);
    let mut finished = 0;
    while finished < 2 {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left) {
            Ok(Ok(())) => finished += 1,
            Ok(Err(e)) => panic!("a cross-design read failed: {e}"),
            Err(_) => panic!(
                "DEADLOCK OR HANG: {finished} of 2 sessions finished {ROUNDS} rounds of A->B and \
                 B->A within the deadline"
            ),
        }
    }
}

/// A server that holds one design and no registry — stdio's shape — says where
/// `other_graph_id` is served instead of guessing.
#[tokio::test]
async fn a_server_without_a_registry_says_where_other_graph_id_is_served() {
    use reflow2_mcp::service::{CompareDesignsReq, ComposeReq, ReflowService};
    use rmcp::handler::server::wrapper::Parameters;

    let s = ReflowService::in_memory_as("lonely").expect("in-memory service");
    let err = s
        .compare_designs(Parameters(CompareDesignsReq {
            budget_chars: None,
            base_path: None,
            other_path: None,
            other_graph_id: Some("someone-else".into()),
        }))
        .await
        .expect_err("compare by graph_id on a server without a registry is refused");
    assert!(
        err.message.contains("served only by a registry server"),
        "says where it is served: {}",
        err.message
    );
    assert!(
        err.message.contains("path"),
        "and what to pass here instead: {}",
        err.message
    );

    let err = s
        .compose_and_analyse(Parameters(ComposeReq {
            design: None,
            namespace: None,
            other_graph_id: Some("someone-else".into()),
        }))
        .await
        .expect_err("compose by graph_id on a server without a registry is refused");
    assert!(
        err.message.contains("served only by a registry server"),
        "says where it is served: {}",
        err.message
    );

    // And a call that names neither source is refused for that, not answered
    // from nothing.
    let err = s
        .compare_designs(Parameters(CompareDesignsReq {
            budget_chars: None,
            base_path: None,
            other_path: None,
            other_graph_id: None,
        }))
        .await
        .expect_err("nothing to compare is a refusal");
    assert!(
        err.message.contains("base_path") && err.message.contains("other_graph_id"),
        "names both routes: {}",
        err.message
    );
    let err = s
        .compose_and_analyse(Parameters(ComposeReq {
            design: None,
            namespace: None,
            other_graph_id: None,
        }))
        .await
        .expect_err("nothing to compose is a refusal");
    assert!(
        err.message.contains("design") && err.message.contains("other_graph_id"),
        "names both routes: {}",
        err.message
    );
}
