//! ON AN ENGINE SERVED FOR OTHERS, A SIGNATURE IS THE CALLER'S OWN — THROUGH
//! EVERY WRITER THE SERVER SERVES.
//!
//! #616 fix 4, under the settled ruling
//! `dec:idea-authentication-is-somebody-elses-layer-and-the-line-is-the-contributor-id`
//! (option (e)), widened on 2026-09-30 to every write path
//! (fact:root-cause-the-settle-rule-guards-the-typed-doors-and-the-generic-writers-go-around-it-2026-09-29,
//! "SCOPE WIDENED"): on an exposed or registry-served engine, "every
//! AUTHORED_BY a call writes (author or approver) is for that contributor, and
//! a call naming anyone else is refused". Measured on 0.74.0: create_edge
//! AUTHORED_BY {roles:[approver]} naming another contributor succeeded, and so
//! did acknowledge_gaps with a per-item approver; a hosting gateway had to
//! contain it by scanning argument names, and its own check read passing while
//! the rule was broken because it was derived from its own list.
//!
//! So this test NEVER says which arguments carry a signature. It runs a REAL
//! reflow2 serving a registry behind a declared trusted gateway, reads the
//! write tools off the SERVED tool list, and requires one call per write tool
//! in `fixtures/every_writer_signs_as_the_caller.json` — an unknown writer
//! fails, so a new door cannot open unnoticed. Each call names the CALLER
//! (who:alice) wherever it names a person, at any depth: nested items, bulk
//! forms, a whole document, a nested helper call inside `draw_edges`. It runs
//! as the caller and must succeed. Then it runs again with EVERY occurrence of
//! the caller's id replaced by another contributor's, and the store is read
//! back: no AUTHORED_BY naming the other contributor may be added, changed or
//! removed, and a refused call must leave the design exactly as it was.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Map, Value, json};

const FIXTURE: &str = include_str!("fixtures/every_writer_signs_as_the_caller.json");
const GATEWAY: &str = "test-gateway";

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

fn tmp_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "reflow2-signature-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn wait_or_kill(child: &mut Child, d: Duration) {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) if start.elapsed() < d => std::thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
        }
    }
}

// ---- the design, seeded on a LOCAL engine ----------------------------------

struct LocalClient;

impl rmcp::ClientHandler for LocalClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "signature-fixture-seed".to_string();
        cfg.client_info.version = "test".to_string();
        cfg
    }
}

type Local = rmcp::service::RunningService<rmcp::RoleClient, LocalClient>;

async fn local() -> Local {
    let service = ReflowService::in_memory().expect("in-memory service");
    let (server_rx, client_tx) = tokio::io::duplex(1 << 22);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 22);
    tokio::spawn(async move {
        if let Ok(running) = service.serve((server_rx, server_tx)).await {
            let _ = running.waiting().await;
        }
    });
    LocalClient
        .serve((client_rx, client_tx))
        .await
        .expect("the in-process handshake")
}

async fn local_call(c: &Local, tool: &str, args: Value) -> Value {
    let Value::Object(arguments) = args else {
        panic!("arguments for {tool} must be an object");
    };
    let r = c
        .call_tool(CallToolRequestParams::new(tool.to_string()).with_arguments(arguments))
        .await
        .unwrap_or_else(|e| panic!("seed {tool}: {e}"));
    assert!(r.is_error != Some(true), "seed {tool}: {:?}", r.content);
    r.structured_content.unwrap_or(Value::Null)
}

/// Seed the fixture design on a LOCAL engine, where nothing is held to a
/// caller, and return it as a document plus the values the writer table
/// refers to by `$NAME`.
async fn seeded(fixture: &Value) -> (Value, Map<String, Value>) {
    let c = local().await;
    for step in fixture["seed"].as_array().expect("seed") {
        local_call(&c, step[0].as_str().unwrap(), step[1].clone()).await;
    }
    let mut ctx = Map::new();
    // Real gap candidates, as `detect_gaps` hands them out.
    let gaps = local_call(&c, "detect_gaps", json!({})).await;
    let list: Vec<Value> = gaps["items"].as_array().cloned().unwrap_or_default();
    assert!(
        list.len() >= 4,
        "the fixture design must raise at least four gaps to drive the gap tools: {gaps}"
    );
    // The rewrite a question is recorded with: one answer per prompt the
    // server hands out for the gap. Asking without answers writes nothing.
    let answers_for = async |gap: &Value| -> Value {
        let prompts = local_call(
            &c,
            "gap_to_prompt",
            json!({"gap": gap, "asked_of": "who:alice"}),
        )
        .await;
        Value::Array(
            prompts["prompts"]
                .as_array()
                .expect("prompts")
                .iter()
                .map(|p| json!({"id": p["id"], "text": "Is this the right reading?"}))
                .collect(),
        )
    };
    // Two are put to the caller now, so answer_question, answers and
    // withdraw_question have questions to work on.
    for (i, key) in [(0usize, "ASKED_GAP"), (3, "ASKED_GAP2")] {
        let answers = answers_for(&list[i]).await;
        let asked = local_call(
            &c,
            "gap_to_prompt",
            json!({"gap": list[i], "asked_of": "who:alice", "answers": answers}),
        )
        .await;
        let question = asked["question_id"]
            .as_str()
            .unwrap_or_else(|| panic!("gap_to_prompt names the question it recorded: {asked}"));
        if i == 0 {
            ctx.insert("QUESTION".into(), json!(question));
        }
        ctx.insert(key.into(), list[i]["id"].clone());
    }
    ctx.insert("GAP".into(), list[1].clone());
    ctx.insert("GAP_ANSWERS".into(), answers_for(&list[1]).await);
    ctx.insert("GAP2".into(), list[2].clone());
    ctx.insert("GAP2_ANSWERS".into(), answers_for(&list[2]).await);
    ctx.insert(
        "PROPOSAL".into(),
        local_call(&c, "propose_heal", json!({})).await,
    );
    let export = local_call(&c, "export_graph", json!({})).await;

    // Another design's published surface, for mirror_surface.
    let other = local().await;
    local_call(
        &other,
        "add_project",
        json!({"id": "prj:other", "name": "Other design"}),
    )
    .await;
    local_call(
        &other,
        "add_interface",
        json!({"id": "ifc:other", "name": "Other API"}),
    )
    .await;
    local_call(
        &other,
        "set_interface_designation",
        json!({"interface_id": "ifc:other", "designation": "published"}),
    )
    .await;
    // Every in-memory design has the default id, and a design will not mirror
    // itself: the other design's surface is renamed as the other design.
    let mut surface = local_call(&other, "export_surface", json!({})).await["document"].clone();
    surface["graph_id"] = json!("other-design");
    ctx.insert("SURFACE".into(), surface);

    (export, ctx)
}

/// Replace every `"$NAME"` string, at any depth, with its value, and every
/// `{"$file": ...}` with the path of a document written for it — so a tool
/// that takes a PATH is driven with a document that names whoever the call
/// names.
fn resolve(v: &Value, ctx: &Map<String, Value>, dir: &Path) -> Value {
    match v {
        Value::Object(o) if o.contains_key("$file") => {
            let doc = match o["$file"].as_str() {
                Some("empty") => json!({"nodes": [], "edges": []}),
                Some("signed") => {
                    let who = o["signed_by"].as_str().expect("signed_by");
                    json!({
                        "nodes": [
                            {"node_type": "Decision", "node_id": "dec:merged", "properties": {"name": "Merged choice", "decision": "Take it.", "kind": "choice", "status": "accepted"}}
                        ],
                        "edges": [{"edge_type": "AUTHORED_BY", "from_id": "dec:merged", "to_id": who, "properties": {"roles": ["approver"]}}]
                    })
                }
                other => panic!("the fixture names an unknown $file kind {other:?}"),
            };
            let path = dir.join(format!(
                "{}.json",
                v.to_string()
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                    .collect::<String>()
            ));
            std::fs::write(&path, doc.to_string()).unwrap();
            Value::String(path.to_string_lossy().to_string())
        }
        Value::String(s) if s.starts_with('$') => ctx.get(&s[1..]).cloned().unwrap_or_else(|| {
            panic!(
                "the fixture names ${} and the test has no value for it",
                &s[1..]
            )
        }),
        Value::Array(a) => Value::Array(a.iter().map(|x| resolve(x, ctx, dir)).collect()),
        Value::Object(o) => Value::Object(
            o.iter()
                .map(|(k, x)| (k.clone(), resolve(x, ctx, dir)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Replace every string equal to `from`, at any depth, with `to` — the ONE
/// thing the test knows about a call is where it named the caller.
fn renamed(v: &Value, from: &str, to: &str) -> Value {
    match v {
        Value::String(s) if s == from => Value::String(to.to_string()),
        Value::Array(a) => Value::Array(a.iter().map(|x| renamed(x, from, to)).collect()),
        Value::Object(o) => Value::Object(
            o.iter()
                .map(|(k, x)| (k.clone(), renamed(x, from, to)))
                .collect(),
        ),
        other => other.clone(),
    }
}

// ---- a real server, and one JSON-RPC session over its HTTP transport -------

/// Mint a real design at the registry shape; its graph_id.
fn mint(dir: &Path) -> String {
    let store = dir.join(".reflow2").join("graph");
    let mut child = Command::new(bin())
        .arg("--graph-path")
        .arg(&store)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn reflow2-mcp to mint a design");
    writeln!(
        child.stdin.as_mut().expect("stdin"),
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2025-06-18","capabilities":{{}},"clientInfo":{{"name":"mint","version":"1"}}}}}}"#
    )
    .expect("write initialize");
    drop(child.stdin.take());
    wait_or_kill(&mut child, Duration::from_secs(60));
    graph_id(dir)
}

/// The id the registry serves a design under, read from its identity
/// sidecar — an import ADOPTS the document's id, so read it after one.
fn graph_id(dir: &Path) -> String {
    let raw = std::fs::read_to_string(dir.join(".reflow2").join("graph.id.json"))
        .expect("identity sidecar");
    let v: Value = serde_json::from_str(&raw).expect("identity sidecar is json");
    v["graph_id"].as_str().expect("graph_id").to_string()
}

/// Load a document into a store with the CLI, before any server holds it.
fn import(dir: &Path, doc: &Value) {
    let file = dir.join("seed.json");
    std::fs::write(&file, doc.to_string()).unwrap();
    let out = Command::new(bin())
        .arg("--graph-path")
        .arg(dir.join(".reflow2").join("graph"))
        .arg("--import")
        .arg(&file)
        .output()
        .expect("run --import");
    assert!(
        out.status.success(),
        "--import: {}",
        String::from_utf8_lossy(&out.stderr)
    );
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

/// A real reflow2 serving every design under `root` — flo2.io's shape — with
/// `env` added (the trusted-gateway declaration travels as an environment
/// variable, the way a container is configured).
fn serve(root: &Path, env: &[(&str, &str)]) -> Server {
    let mut cmd = Command::new(bin());
    cmd.arg("--registry-root")
        .arg(root)
        .arg("--http")
        .arg("127.0.0.1:0")
        .env_remove("REFLOW2_TRUSTED_GATEWAY")
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    for (k, v) in env {
        cmd.env(k, v);
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

/// The JSON-RPC message carrying `id` in a reply body (JSON or SSE).
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
    /// What the handshake told this session.
    instructions: String,
}

/// A tool call's outcome: `Ok(structured)` or `Err(refusal text)`.
type Outcome = Result<Value, String>;

impl Session {
    fn open(port: u16, graph_id: &str) -> Session {
        let path = format!("/g/{graph_id}/mcp");
        let (head, body) = exchange(
            port,
            &path,
            r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"signature-class-test","version":"1"}}}"#,
            None,
        );
        let instructions = rpc_reply(&body, 0)["result"]["instructions"]
            .as_str()
            .unwrap_or_default()
            .to_string();
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
            instructions,
        }
    }

    fn rpc(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        let body = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        let (_, reply) = exchange(self.port, &self.path, &body.to_string(), Some(&self.sid));
        rpc_reply(&reply, id)
    }

    /// Call `tool` as `caller` would through the gateway: the person on
    /// `_meta["reflow2/writes_for"]`, or nobody.
    fn call(&mut self, tool: &str, args: &Value, caller: Option<&str>) -> Outcome {
        let mut params = json!({"name": tool, "arguments": args});
        if let Some(who) = caller {
            params["_meta"] = json!({"reflow2/writes_for": who});
        }
        let r = self.rpc("tools/call", params);
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

    fn write_tools(&mut self) -> Vec<String> {
        let mut out = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let params = match &cursor {
                Some(c) => json!({"cursor": c}),
                None => json!({}),
            };
            let r = self.rpc("tools/list", params);
            for t in r["result"]["tools"].as_array().expect("tools") {
                if t["annotations"]["readOnlyHint"] != true {
                    out.push(t["name"].as_str().unwrap().to_string());
                }
            }
            cursor = r["result"]["nextCursor"].as_str().map(String::from);
            if cursor.is_none() {
                break;
            }
        }
        out.sort();
        out
    }

    /// The design's nodes and edges, canonical, for before/after comparison.
    fn design(&mut self) -> (Vec<String>, Vec<Value>) {
        let doc = self
            .call("export_graph", &json!({}), None)
            .expect("export_graph");
        let mut nodes: Vec<String> = doc["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .map(|n| n.to_string())
            .collect();
        nodes.sort();
        let mut edges: Vec<Value> = doc["edges"].as_array().expect("edges").clone();
        edges.sort_by_key(|e| e.to_string());
        (nodes, edges)
    }
}

/// The AUTHORED_BY edges naming `who`, keyed (from, to) -> properties.
fn signatures_of(edges: &[Value], who: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = edges
        .iter()
        .filter(|e| e["edge_type"] == "AUTHORED_BY" && e["to_id"] == who)
        .map(|e| {
            (
                e["from_id"].as_str().unwrap().to_string(),
                e["properties"].to_string(),
            )
        })
        .collect();
    out.sort();
    out
}

/// ⭐ THE CLASS TEST. Every served write tool, driven as the caller and then
/// in another contributor's name, through a real registry behind a declared
/// trusted gateway.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_served_writer_writes_only_the_callers_own_signature() {
    let fixture: Value = serde_json::from_str(FIXTURE).expect("fixture is JSON");
    let me = fixture["caller"].as_str().unwrap();
    let other = fixture["other"].as_str().unwrap();
    let root = tmp_root("class");
    let dir = root.join("design");
    std::fs::create_dir_all(&dir).unwrap();
    let files = root.join("files");
    std::fs::create_dir_all(&files).unwrap();
    let (doc, ctx) = seeded(&fixture).await;
    mint(&dir);
    import(&dir, &doc);
    let id = graph_id(&dir);
    let server = serve(&root, &[("REFLOW2_TRUSTED_GATEWAY", GATEWAY)]);

    let mut probe = Session::open(server.port, &id);
    assert!(
        probe.instructions.contains("TRUSTED GATEWAY") && probe.instructions.contains(GATEWAY),
        "the handshake says a signature here is the caller's own: {}",
        probe.instructions
    );
    let writers = probe.write_tools();
    let table = fixture["writers"].as_object().expect("writers");
    let mut problems: Vec<String> = Vec::new();
    for w in &writers {
        if !table.contains_key(w) {
            problems.push(format!(
                "`{w}` is a served write tool with no call in the fixture: add one naming the \
                 caller ({me}) wherever it names a person, so the class test drives it"
            ));
        }
    }
    for k in table.keys() {
        if !writers.contains(k) {
            problems.push(format!(
                "the fixture drives `{k}`, which is not a served write tool"
            ));
        }
    }

    let (mut driven, mut named, mut refused) = (0usize, 0usize, 0usize);
    let mut other_refusals: Vec<String> = Vec::new();
    for tool in &writers {
        let Some(call) = table.get(tool) else {
            continue;
        };
        // The caller's id is replaced BEFORE the values are filled in, so a
        // document passed by path names whoever the call names.
        let forged = resolve(&renamed(call, me, other), &ctx, &files);
        let names_a_person = renamed(call, me, other) != *call;
        let call = resolve(call, &ctx, &files);
        let mut s = Session::open(server.port, &id);

        // As the caller: it must go through, and sign only as the caller.
        let before = s.design();
        match s.call(tool, &call, Some(me)) {
            Err(e) => {
                problems.push(format!(
                    "`{tool}` REFUSED ITS CALLER'S OWN CALL, so it is not really driven: {}",
                    &e[..e.len().min(600)]
                ));
                continue;
            }
            Ok(_) => driven += 1,
        }
        let after = s.design();
        for e in after.1.iter().filter(|e| !before.1.contains(e)) {
            if e["edge_type"] == "AUTHORED_BY" && e["to_id"] != me {
                problems.push(format!(
                    "`{tool}`, called by {me}, recorded a signature for {}: {e}",
                    e["to_id"]
                ));
            }
        }

        // In another contributor's name: every occurrence of the caller's id,
        // at any depth, becomes theirs.
        if !names_a_person {
            continue;
        }
        named += 1;
        let sigs_before = signatures_of(&after.1, other);
        let outcome = s.call(tool, &forged, Some(me));
        let now = s.design();
        let sigs_now = signatures_of(&now.1, other);
        if sigs_now != sigs_before {
            problems.push(format!(
                "`{tool}` WROTE A SIGNATURE IN {other}'S NAME for caller {me}: before {sigs_before:?}, after {sigs_now:?}"
            ));
        }
        if let Err(text) = outcome {
            if now != after {
                problems.push(format!(
                    "`{tool}` was refused and still WROTE to the design (a half-write): {}",
                    &text[..text.len().min(400)]
                ));
            }
            if text.contains("signature") || text.contains("AUTHORED_BY") {
                refused += 1;
                if !text.contains(me) {
                    problems.push(format!(
                        "`{tool}`'s refusal does not say who the caller is ({me}): {}",
                        &text[..text.len().min(600)]
                    ));
                }
            } else {
                // Refused for a reason of its own (the name is no signature):
                // still held to "nothing written" and "no signature moved"
                // above, and said here so a reader sees it.
                other_refusals.push(format!("{tool}: {}", &text[..text.len().min(200)]));
            }
        }
    }
    eprintln!(
        "{} served write tools; {driven} driven as the caller; {named} name a person and were \
         driven again in {other}'s name; {refused} of those refused as a signature in someone \
         else's name; refused for another reason: {other_refusals:#?}",
        writers.len()
    );
    assert!(
        problems.is_empty(),
        "{} problem(s):\n{}",
        problems.len(),
        problems.join("\n")
    );
    assert!(
        driven > 100 && refused > 10,
        "the class test measured too little to mean anything: {driven} driven, {refused} refused"
    );
}

/// With NOTHING declared, a registry — served for others — still reads and
/// takes proposals, and refuses every signature and every settle, naming the
/// flag that would establish the caller. Behind a declared gateway, a call the
/// gateway names nobody on is held the same way.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_engine_that_does_not_know_who_is_calling_signs_and_settles_nothing() {
    let fixture: Value = serde_json::from_str(FIXTURE).expect("fixture is JSON");
    for (label, env, caller) in [
        ("undeclared", vec![], Some("who:alice")),
        (
            "gateway-named-nobody",
            vec![("REFLOW2_TRUSTED_GATEWAY", GATEWAY)],
            None,
        ),
    ] {
        let root = tmp_root(label);
        let dir = root.join("design");
        std::fs::create_dir_all(&dir).unwrap();
        let (doc, _) = seeded(&fixture).await;
        mint(&dir);
        import(&dir, &doc);
        let id = graph_id(&dir);
        let server = serve(&root, &env);
        let mut s = Session::open(server.port, &id);
        assert!(
            s.instructions.contains("SERVED FOR OTHERS"),
            "[{label}] the handshake says what signing means here: {}",
            s.instructions
        );

        let settling = [
            (
                "set_decision_status",
                json!({"decision_id": "dec:d2", "status": "accepted", "approver": "who:alice"}),
            ),
            (
                "create_edge",
                json!({"edge_type": "AUTHORED_BY", "from_id": "dec:d", "to_id": "who:alice", "props": {"roles": ["approver"]}}),
            ),
            (
                "create_node",
                json!({"node_type": "Requirement", "id": "req:settled-here", "props": {"name": "N", "statement": "S", "status": "accepted"}, "approver": "who:alice"}),
            ),
            (
                "import_graph",
                json!({"document": {"nodes": [{"node_type": "Decision", "node_id": "dec:imported-unsigned", "properties": {"name": "I", "decision": "D", "kind": "choice", "status": "accepted"}}], "edges": []}}),
            ),
            (
                "delete_edge",
                json!({"edge_type": "AUTHORED_BY", "from_id": "dec:signed", "to_id": "who:mallory"}),
            ),
        ];
        let mut problems = Vec::new();
        for (tool, args) in settling {
            let before = s.design();
            match s.call(tool, &args, caller) {
                Ok(r) => problems.push(format!("[{label}] `{tool}` signed or settled: {r}")),
                Err(text) => {
                    if s.design() != before {
                        problems.push(format!("[{label}] `{tool}` was refused and still wrote"));
                    }
                    if !text.contains("--http-trusted-gateway") && !text.contains(GATEWAY) {
                        problems.push(format!(
                            "[{label}] `{tool}`'s refusal does not say how a caller is established: {text}"
                        ));
                    }
                }
            }
        }
        // A proposal still lands, and a read still answers.
        if let Err(e) = s.call(
            "add_requirement",
            &json!({"id": "req:proposed-here", "name": "A proposal", "statement": "Maybe."}),
            caller,
        ) {
            problems.push(format!("[{label}] a proposal was refused: {e}"));
        }
        if let Err(e) = s.call("get_node", &json!({"id": "req:r"}), caller) {
            problems.push(format!("[{label}] a read was refused: {e}"));
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }
}
