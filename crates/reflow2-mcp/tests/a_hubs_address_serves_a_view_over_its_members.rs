//! A hub's address serves a VIEW over its members, when the host names them.
//!
//! Layer 2 of the cross-design work, on top of the readable set
//! (`tests/a_cross_design_read_takes_the_other_design_by_graph_id.rs`): the
//! trusted host passes `_meta["reflow2/view_graph_ids"]` — the hub's members
//! the caller may read — on every call to a hub's own design, and reflow2
//! serves that address as a view over the hub and those members at the tool
//! layer. No tool is added; the one argument a hub's listing declares,
//! `member`, is held by `through_a_hub_every_tool_runs_on_one_named_member.rs`.
//!
//! WHAT IS ASSERTED, over the real binary serving H, A and B under one root
//! behind a trusted gateway (readable [H, A, B], view [A, B] unless a test
//! says otherwise):
//!   · ROUTED: `get_node "A:<id>"` through H answers A's node, its ids
//!     prefixed "A:"; a write naming "A:<id>" lands in A (read back on A
//!     directly), credited to the caller; a call naming ids in A AND B is
//!     refused before anything runs, and nothing is written in either
//!   · FAN-OUT: `loop_status` and `detect_gaps` through H answer `by_design`
//!     keyed H, A, B, with a text block saying the reply is a view
//!   · MERGED SEARCH: `search_design` through H finds a word only B holds,
//!     its hit prefixed "B:" and ranked with every design's hits; the
//!     caller's one `limit` bounds the merged list, and `found_by_design`
//!     says how many hits each design returned
//!   · THE READABLE SET BINDS: with B outside it, B is absent from
//!     `by_design`, named in `unreachable`, and nothing B holds is revealed;
//!     a routed call to "B:<id>" is refused naming B and nothing else
//!   · NO VIEW `_meta`: every reply is byte-identical to the same call on an
//!     untouched copy of the root served by a registry that never admits a
//!     view (no trusted host); and a WRITE naming no member, made with the
//!     view on, is byte-identical to the same write made with no view at all;
//!     and a server that does not admit a view (no trusted host) answers a
//!     call naming one for the hub alone, saying so in `view_note`
//!   · NOT VIEWED: `claim_report` through H acts on H only, and says so in a
//!     `view_note`
//!   · NO DEADLOCK: two hubs H1 (view A, B) and H2 (view B, A), called
//!     together many times with reads, routed reads and routed writes,
//!     all finish within a deadline

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

pub(super) const GATEWAY: &str = "flo2.io";
pub(super) const READABLE: &str = "reflow2/readable_graph_ids";
pub(super) const VIEW: &str = "reflow2/view_graph_ids";
pub(super) const CALLER: &str = "who:tester";

pub(super) fn tmp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "reflow2-hub-view-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

pub(super) fn bin() -> PathBuf {
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

/// Mint a design over stdio holding the caller as a Contributor and one
/// requirement only it has — `req:<marker>`, whose statement carries `word` —
/// and return its graph_id.
pub(super) fn mint(dir: &Path, marker: &str, word: &str) -> String {
    mint_holding(
        dir,
        &[(format!("req:{marker}"), format!("only-in-{marker} {word}"))],
    )
}

/// Mint a design over stdio holding the caller as a Contributor and the
/// requirements given as (id, statement), and return its graph_id.
pub(super) fn mint_holding(dir: &Path, requirements: &[(String, String)]) -> String {
    let store = dir.join(".reflow2").join("graph");
    let mut child = Command::new(bin())
        .arg("--graph-path")
        .arg(&store)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn reflow2-mcp to mint a design");
    let mut lines = vec![
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"mint","version":"1"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"add_contributor","arguments":{"id": CALLER, "name": "Tester", "kind": "person"}}}),
    ];
    for (i, (id, statement)) in requirements.iter().enumerate() {
        let name = id.strip_prefix("req:").unwrap_or(id);
        lines.push(json!({"jsonrpc":"2.0","id": 3 + i as u64,"method":"tools/call","params":{"name":"add_requirement","arguments":{"id": id, "name": name, "statement": statement}}}));
    }
    let last_id = format!(r#""id":{}"#, 2 + requirements.len());
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for l in &lines {
            writeln!(stdin, "{l}").expect("write a line to the minting server");
        }
    }
    // Read until the last write's reply so the store holds it before stdin closes.
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
                assert!(
                    !line.contains(r#""isError":true"#),
                    "minting the contributor failed: {line}"
                );
            } else if line.contains(r#""id":"#) {
                assert!(
                    !line.contains(r#""isError":true"#),
                    "minting a requirement failed: {line}"
                );
            }
            if line.contains(&last_id) {
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

pub(super) struct Server {
    child: Child,
    pub(super) port: u16,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A registry over `root`, flo2.io's shape; `gateway` declares the trusted
/// host the way a container does, or leaves the server with none.
pub(super) fn serve(root: &Path, gateway: Option<&str>) -> Server {
    let mut cmd = Command::new(bin());
    cmd.arg("--registry-root")
        .arg(root)
        .arg("--http")
        .arg("127.0.0.1:0")
        .env_remove("REFLOW2_TRUSTED_GATEWAY")
        .env_remove("REFLOW2_CONTENT_POLICY")
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
pub(super) fn exchange(
    port: u16,
    path: &str,
    body: &str,
    session: Option<&str>,
) -> (String, String) {
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

/// The JSON-RPC message answering `id`, as the exact text the server sent.
pub(super) fn raw_reply(body: &str, id: u64) -> String {
    let mut candidates: Vec<String> = body
        .lines()
        .filter_map(|l| l.strip_prefix("data:"))
        .map(|d| d.trim().to_string())
        .collect();
    candidates.push(body.trim().to_string());
    candidates
        .into_iter()
        .find(|text| {
            serde_json::from_str::<Value>(text)
                .map(|v| v["id"] == id)
                .unwrap_or(false)
        })
        .unwrap_or_else(|| {
            panic!(
                "no JSON-RPC reply {id} in: {}",
                &body[..body.len().min(2000)]
            )
        })
}

pub(super) struct Session {
    pub(super) port: u16,
    pub(super) path: String,
    pub(super) sid: String,
    pub(super) next: u64,
}

/// What a call came back with: the reply as sent, and its parsed form.
pub(super) struct Reply {
    pub(super) raw: String,
    pub(super) value: Value,
}

impl Reply {
    pub(super) fn result(&self) -> &Value {
        &self.value["result"]
    }
    /// `Ok(structuredContent)` or `Err(the refusal's text)`.
    pub(super) fn outcome(&self) -> Result<Value, String> {
        if let Some(e) = self.value.get("error") {
            return Err(e.to_string());
        }
        let r = self.result();
        if r["isError"] == true {
            return Err(r["content"].to_string());
        }
        Ok(r.get("structuredContent").cloned().unwrap_or(Value::Null))
    }
    pub(super) fn text(&self) -> String {
        self.result()["content"]
            .as_array()
            .map(|blocks| {
                blocks
                    .iter()
                    .filter_map(|b| b["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default()
    }
}

impl Session {
    pub(super) fn open(port: u16, graph_id: &str) -> Session {
        let path = format!("/g/{graph_id}/mcp");
        let (head, body) = exchange(
            port,
            &path,
            r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"hub-view-test","version":"1"}}}"#,
            None,
        );
        let _ = raw_reply(&body, 0);
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

    /// Call `tool` as the gateway would: `_meta` names the signed-in person,
    /// the designs they may read when `readable` is given, and the hub's
    /// members when `view` is given.
    pub(super) fn call(
        &mut self,
        tool: &str,
        args: Value,
        readable: Option<&[&str]>,
        view: Option<&[&str]>,
    ) -> Reply {
        let id = self.next;
        self.next += 1;
        let mut params = json!({"name": tool, "arguments": args});
        let mut meta = json!({"reflow2/writes_for": CALLER});
        if let Some(ids) = readable {
            meta[READABLE] = json!(ids);
        }
        if let Some(ids) = view {
            meta[VIEW] = json!(ids);
        }
        params["_meta"] = meta;
        let body = json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": params});
        let (_, reply) = exchange(self.port, &self.path, &body.to_string(), Some(&self.sid));
        let raw = raw_reply(&reply, id);
        let value = serde_json::from_str(&raw).expect("a JSON-RPC reply is JSON");
        Reply { raw, value }
    }
}

/// H, A and B under one root, each holding the caller and one requirement:
/// `req:hub` (word `harbour`), `req:alpha` (`aardvark`), `req:beta`
/// (`zanzibar`). Returns (root, H, A, B).
pub(super) fn hub_and_members(tag: &str) -> (PathBuf, String, String, String) {
    let root = tmp_root(tag);
    let mut ids = Vec::new();
    for (dir, marker, word) in [
        ("hub", "hub", "harbour"),
        ("alpha", "alpha", "aardvark"),
        ("beta", "beta", "zanzibar"),
    ] {
        let d = root.join(dir);
        std::fs::create_dir_all(&d).unwrap();
        ids.push(mint(&d, marker, word));
    }
    let (h, a, b) = (ids[0].clone(), ids[1].clone(), ids[2].clone());
    assert!(h != a && a != b && h != b, "three designs, three graph_ids");
    (root, h, a, b)
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

pub(super) fn ok(reply: &Reply, what: &str) -> Value {
    reply
        .outcome()
        .unwrap_or_else(|e| panic!("{what} must answer, but was refused: {e}"))
}

pub(super) fn refused(reply: &Reply, what: &str) -> String {
    match reply.outcome() {
        Err(text) => text,
        Ok(v) => panic!("{what} must be refused, but answered: {v}"),
    }
}

#[test]
fn a_routed_read_answers_from_the_member_with_its_ids_prefixed() {
    let (root, h, a, b) = hub_and_members("routed-read");
    let server = serve(&root, Some(GATEWAY));
    let mut on_h = Session::open(server.port, &h);
    let readable = [h.as_str(), a.as_str(), b.as_str()];
    let view = [a.as_str(), b.as_str()];

    let reply = on_h.call(
        "get_node",
        json!({"id": format!("{a}:req:alpha")}),
        Some(&readable),
        Some(&view),
    );
    let got = ok(&reply, "get_node A:req:alpha through H");
    assert_eq!(
        got["node"]["node_id"],
        format!("{a}:req:alpha"),
        "A's node, its id prefixed by A's graph_id: {got}"
    );
    assert!(
        got.to_string().contains("only-in-alpha"),
        "and it is A's node, with A's content: {got}"
    );
    assert!(
        got.get("by_design").is_none(),
        "a routed call answers from ONE design, not as a fan-out: {got}"
    );
}

#[test]
fn a_routed_write_lands_in_the_member_credited_to_the_caller() {
    let (root, h, a, b) = hub_and_members("routed-write");
    let server = serve(&root, Some(GATEWAY));
    let mut on_h = Session::open(server.port, &h);
    let readable = [h.as_str(), a.as_str(), b.as_str()];
    let view = [a.as_str(), b.as_str()];

    let reply = on_h.call(
        "add_requirement",
        json!({"id": format!("{a}:req:routed"), "name": "routed", "statement": "written through the hub"}),
        Some(&readable),
        Some(&view),
    );
    let receipt = ok(&reply, "a write naming A:req:routed through H");
    assert!(
        receipt.to_string().contains(&format!("{a}:req:routed")),
        "the receipt names the node by its prefixed id: {receipt}"
    );

    // Read it back on A DIRECTLY — no hub, no view.
    let mut on_a = Session::open(server.port, &a);
    let back = on_a.call(
        "get_node",
        json!({"id": "req:routed", "include_edges": true}),
        Some(&readable),
        None,
    );
    let back = ok(&back, "reading the routed write back on A");
    assert_eq!(
        back["node"]["node_id"], "req:routed",
        "the write landed in A under its own id: {back}"
    );
    let edges = back["edges"].to_string();
    assert!(
        edges.contains("AUTHORED_BY") && edges.contains(CALLER),
        "credited to the caller the gateway named: {back}"
    );

    // And NOT in H.
    let in_h = on_h.call(
        "get_node",
        json!({"id": "req:routed"}),
        Some(&readable),
        None,
    );
    let in_h = ok(&in_h, "looking for the routed write in H");
    assert!(
        in_h["node"].is_null(),
        "the write did not land in the hub: {in_h}"
    );
}

#[test]
fn a_call_naming_two_members_is_refused_and_writes_nothing() {
    let (root, h, a, b) = hub_and_members("two-members");
    let server = serve(&root, Some(GATEWAY));
    let mut on_h = Session::open(server.port, &h);
    let readable = [h.as_str(), a.as_str(), b.as_str()];
    let view = [a.as_str(), b.as_str()];

    let reply = on_h.call(
        "create_edge",
        json!({
            "edge_type": "DEPENDS_ON",
            "from_id": format!("{a}:req:alpha"),
            "to_id": format!("{b}:req:beta"),
        }),
        Some(&readable),
        Some(&view),
    );
    let text = refused(&reply, "an edge across two members");
    assert!(
        text.contains("a write lands in one design")
            && text.contains("review_relations")
            && text.contains("other_design")
            && text.contains("other_id"),
        "the refusal says where a relation across designs is recorded: {text}"
    );

    // Nothing was written in either member.
    for (design, id) in [(&a, "req:alpha"), (&b, "req:beta")] {
        let mut s = Session::open(server.port, design);
        let got = s.call(
            "get_node",
            json!({"id": id, "include_edges": true}),
            Some(&readable),
            None,
        );
        let got = ok(&got, "reading a member after the refusal");
        assert!(
            !got["edges"].to_string().contains("DEPENDS_ON"),
            "{id}: no edge was drawn: {got}"
        );
    }
}

#[test]
fn a_read_with_no_member_id_fans_out_over_the_hub_and_its_members() {
    let (root, h, a, b) = hub_and_members("fan-out");
    let server = serve(&root, Some(GATEWAY));
    let mut on_h = Session::open(server.port, &h);
    let readable = [h.as_str(), a.as_str(), b.as_str()];
    let view = [a.as_str(), b.as_str()];

    for tool in ["loop_status", "detect_gaps"] {
        let reply = on_h.call(tool, json!({}), Some(&readable), Some(&view));
        let got = ok(&reply, &format!("{tool} through H"));
        let by = got["by_design"]
            .as_object()
            .unwrap_or_else(|| panic!("{tool}: a fan-out answers by_design: {got}"));
        for id in [&h, &a, &b] {
            assert!(
                by.contains_key(id.as_str()),
                "{tool}: by_design names {id}: {got}"
            );
            assert!(
                by[id.as_str()].get("error").is_none(),
                "{tool}: {id} answered rather than failed: {got}"
            );
        }
        assert_eq!(by.len(), 3, "{tool}: H, A and B and nothing else: {got}");
        assert_eq!(got["view"], json!(view), "{tool}: names the view: {got}");
        assert_eq!(
            got["unreachable"],
            json!([]),
            "{tool}: every member was readable: {got}"
        );
        assert!(
            reply.text().to_ascii_lowercase().contains("view"),
            "{tool}: a text block says the reply is a view: {}",
            reply.text()
        );
    }
}

#[test]
fn search_through_the_hub_ranks_every_designs_hits_together() {
    let (root, h, a, b) = hub_and_members("search");
    let server = serve(&root, Some(GATEWAY));
    let mut on_h = Session::open(server.port, &h);
    let readable = [h.as_str(), a.as_str(), b.as_str()];
    let view = [a.as_str(), b.as_str()];

    let reply = on_h.call(
        "search_design",
        json!({"query": "zanzibar"}),
        Some(&readable),
        Some(&view),
    );
    let got = ok(&reply, "search_design through H");
    let hits = got["hits"]
        .as_array()
        .unwrap_or_else(|| panic!("ONE merged list of hits, not by_design: {got}"));
    assert!(
        hits.iter()
            .any(|hit| hit["node_id"] == format!("{b}:req:beta")),
        "the word only B holds is found, its hit prefixed by B: {got}"
    );
    assert_eq!(
        got["designs_searched"],
        json!([h, a, b]),
        "names every design searched, the hub first: {got}"
    );
    assert_eq!(
        got["limit"],
        json!(10),
        "the one limit applied, search_design's default: {got}"
    );
    assert_eq!(
        got["found_by_design"][b.as_str()],
        json!(1),
        "how many hits each design returned: {got}"
    );
    assert_eq!(got["hits_withheld"], json!(0), "{got}");
    let scores: Vec<f64> = hits
        .iter()
        .map(|hit| hit["score"].as_f64().expect("every hit keeps its score"))
        .collect();
    assert!(
        scores.windows(2).all(|w| w[0] >= w[1]),
        "ranked together by score: {scores:?}"
    );

    // The hub's own hits stay unprefixed.
    let reply = on_h.call(
        "search_design",
        json!({"query": "harbour"}),
        Some(&readable),
        Some(&view),
    );
    let got = ok(&reply, "search_design for the hub's own word");
    assert!(
        got["hits"]
            .as_array()
            .is_some_and(|hits| hits.iter().any(|hit| hit["node_id"] == "req:hub")),
        "the hub's own hit is unprefixed: {got}"
    );
}

/// Measured 2026-10-11 on flo2.io: `search_design {limit: 10}` through a hub
/// of seven designs answered 65 hits, because the view asked each design for
/// 10 and returned every design's hits merged. The hub's address is ONE design
/// to the caller, so its `limit` bounds the one merged list; each design is
/// still asked for the same limit (per-design top-k, then merged top-k, is
/// exact), and `found_by_design` says how many each design returned, so a
/// design that matched beyond what is shown is still named.
#[test]
fn search_through_the_hub_applies_the_limit_to_the_merged_list() {
    let root = tmp_root("merged-limit");
    let mut ids = Vec::new();
    for (dir, marker) in [("hub", "hub"), ("alpha", "alpha"), ("beta", "beta")] {
        let d = root.join(dir);
        std::fs::create_dir_all(&d).unwrap();
        let reqs: Vec<(String, String)> = (0..3)
            .map(|i| {
                (
                    format!("req:{marker}-quokka-{i}"),
                    format!("{marker} holds a quokka, number {i}"),
                )
            })
            .collect();
        ids.push(mint_holding(&d, &reqs));
    }
    let (h, a, b) = (ids[0].clone(), ids[1].clone(), ids[2].clone());
    let server = serve(&root, Some(GATEWAY));
    let mut on_h = Session::open(server.port, &h);
    let readable = [h.as_str(), a.as_str(), b.as_str()];
    let view = [a.as_str(), b.as_str()];

    let reply = on_h.call(
        "search_design",
        json!({"query": "quokka", "limit": 2}),
        Some(&readable),
        Some(&view),
    );
    let got = ok(&reply, "search_design with limit 2 through H");
    let hits = got["hits"].as_array().expect("one merged list of hits");
    assert_eq!(
        hits.len(),
        2,
        "the caller's limit bounds the MERGED list, not each design's: {got}"
    );
    assert_eq!(
        got["limit"],
        json!(2),
        "the one limit applied is echoed: {got}"
    );
    for id in [&h, &a, &b] {
        assert_eq!(
            got["found_by_design"][id.as_str()],
            json!(2),
            "every design's own count is named, so a design that matched beyond what is \
             shown is not lost: {got}"
        );
    }
    assert_eq!(
        got["hits_withheld"],
        json!(4),
        "six ranked hits, two shown, four withheld: {got}"
    );
    assert!(
        got.get("limit_per_design").is_none(),
        "there is one limit, not one per design: {got}"
    );
    let note = got["view_note"].as_str().unwrap_or_default();
    assert!(
        !note.contains("applies to each design"),
        "the note no longer says the limit is per design: {note}"
    );
    let scores: Vec<f64> = hits
        .iter()
        .map(|hit| hit["score"].as_f64().expect("every hit keeps its score"))
        .collect();
    assert!(
        scores.windows(2).all(|w| w[0] >= w[1]),
        "the two shown are the top of the merged ranking: {scores:?}"
    );

    // The default limit (10) bounds the merged list the same way.
    let reply = on_h.call(
        "search_design",
        json!({"query": "quokka"}),
        Some(&readable),
        Some(&view),
    );
    let got = ok(&reply, "search_design with the default limit through H");
    assert_eq!(got["limit"], json!(10), "{got}");
    assert_eq!(
        got["hits"].as_array().map(Vec::len),
        Some(9),
        "nine hits exist across the three designs, all under the default limit: {got}"
    );
    assert_eq!(got["hits_withheld"], json!(0), "{got}");
}

#[test]
fn a_member_outside_the_readable_set_is_unreachable_and_reveals_nothing() {
    let (root, h, a, b) = hub_and_members("unreadable");
    let server = serve(&root, Some(GATEWAY));
    let mut on_h = Session::open(server.port, &h);
    // The host names B a member, but the caller may not read it.
    let readable = [h.as_str(), a.as_str()];
    let view = [a.as_str(), b.as_str()];

    let reply = on_h.call("loop_status", json!({}), Some(&readable), Some(&view));
    let got = ok(&reply, "loop_status with B unreadable");
    let by = got["by_design"].as_object().expect("by_design");
    assert!(by.contains_key(h.as_str()) && by.contains_key(a.as_str()));
    assert!(
        !by.contains_key(b.as_str()),
        "B is absent from by_design: {got}"
    );
    assert_eq!(
        got["unreachable"],
        json!([b]),
        "B is named unreachable: {got}"
    );
    assert!(
        !reply.raw.contains("only-in-beta") && !reply.raw.contains("zanzibar"),
        "nothing B holds is revealed: {}",
        reply.raw
    );

    let reply = on_h.call(
        "search_design",
        json!({"query": "zanzibar"}),
        Some(&readable),
        Some(&view),
    );
    let got = ok(&reply, "search with B unreadable");
    assert!(
        !reply.raw.contains("req:beta") && !reply.raw.contains("only-in-beta"),
        "a search reveals nothing B holds: {}",
        reply.raw
    );
    assert_eq!(
        got["unreachable"],
        json!([b]),
        "and names B unreachable: {got}"
    );

    let reply = on_h.call(
        "get_node",
        json!({"id": format!("{b}:req:beta")}),
        Some(&readable),
        Some(&view),
    );
    let text = refused(&reply, "a routed call to an unreadable member");
    assert!(text.contains(&b), "names the id: {text}");
    assert!(
        text.contains(READABLE),
        "and the key the set came in: {text}"
    );
    assert!(
        !text.contains("only-in-beta") && !text.contains("zanzibar"),
        "nothing B holds: {text}"
    );
}

#[test]
fn with_no_view_meta_every_reply_is_what_it_was() {
    let (root, h, a, b) = hub_and_members("no-view");
    // Untouched copies of the same root, taken before any server opened it.
    let plain_root = root.with_extension("plain");
    copy_tree(&root, &plain_root);
    let write_root = root.with_extension("write");
    copy_tree(&root, &write_root);

    let readable = [h.as_str(), a.as_str(), b.as_str()];
    let view = [a.as_str(), b.as_str()];
    let probes: Vec<(&str, Value)> = vec![
        ("get_node", json!({"id": "req:hub"})),
        ("search_design", json!({"query": "harbour"})),
        ("loop_status", json!({})),
        ("detect_gaps", json!({})),
        ("claim_report", json!({})),
        ("graph_report", json!({"format": "markdown"})),
    ];

    // The gateway's server, no view on any call.
    let gated = serve(&root, Some(GATEWAY));
    let mut on_gated = Session::open(gated.port, &h);
    let gated_replies: Vec<String> = probes
        .iter()
        .map(|(tool, args)| on_gated.call(tool, args.clone(), Some(&readable), None).raw)
        .collect();
    // A registry that never admits a view — no trusted host — on a copy.
    let plain = serve(&plain_root, None);
    let mut on_plain = Session::open(plain.port, &h);
    let plain_replies: Vec<String> = probes
        .iter()
        .map(|(tool, args)| on_plain.call(tool, args.clone(), Some(&readable), None).raw)
        .collect();
    // The ONE thing two sessions may not share: the seat handle each session is
    // minted (`host:pid:nonce`), which `loop_status` reports. Masked, by value,
    // and nothing else is.
    let gated_seat = seat_in(&gated_replies);
    let plain_seat = seat_in(&plain_replies);
    for (((tool, _), gated_raw), plain_raw) in probes.iter().zip(&gated_replies).zip(&plain_replies)
    {
        assert_eq!(
            gated_raw.replace(&gated_seat, "<seat>"),
            plain_raw.replace(&plain_seat, "<seat>"),
            "{tool}: with no view _meta the reply is byte-identical to a server that never \
             serves a view"
        );
        for key in ["by_design", "view_note", "unreachable", "designs_searched"] {
            assert!(
                !gated_raw.contains(&format!("\"{key}\"")),
                "{tool}: no view decoration without the view _meta: {gated_raw}"
            );
        }
    }

    // A server that does not admit a view — no trusted host — answers a call
    // that names one for the hub alone, and SAYS so rather than ignoring it.
    let reply = on_plain.call("loop_status", json!({}), Some(&readable), Some(&view));
    let got = ok(&reply, "a view named on a registry with no trusted host");
    assert!(
        got.get("by_design").is_none(),
        "no view is served without a trusted host: {got}"
    );
    let note = got["view_note"]
        .as_str()
        .unwrap_or_else(|| panic!("the reply says the view was not served: {got}"));
    assert!(
        note.contains("does not serve one") && note.contains(&h),
        "names why, and the design that answered: {note}"
    );
    assert!(
        !reply.raw.contains("only-in-alpha") && !reply.raw.contains("only-in-beta"),
        "and no member was reached: {}",
        reply.raw
    );

    // A WRITE naming no member writes the hub's own design exactly as today:
    // made WITH the view on the gateway's server, and with NO view on a copy,
    // the two replies are the same bytes.
    let write = json!({"id": "req:written", "name": "written", "statement": "no member named"});
    let with_view = on_gated
        .call(
            "add_requirement",
            write.clone(),
            Some(&readable),
            Some(&view),
        )
        .raw;
    let other = serve(&write_root, Some(GATEWAY));
    let mut on_other = Session::open(other.port, &h);
    // Bring the copy's session to the same state — the same calls in the same
    // order, so the JSON-RPC ids and the session's own read count match.
    let other_replies: Vec<String> = probes
        .iter()
        .map(|(tool, args)| on_other.call(tool, args.clone(), Some(&readable), None).raw)
        .collect();
    let other_seat = seat_in(&other_replies);
    let without = on_other
        .call("add_requirement", write, Some(&readable), None)
        .raw;
    assert_eq!(
        with_view.replace(&gated_seat, "<seat>"),
        without.replace(&other_seat, "<seat>"),
        "a write naming no member is byte-identical with and without the view"
    );
}

/// The seat handle a session was minted, read from its `loop_status` reply.
fn seat_in(raws: &[String]) -> String {
    raws.iter()
        .filter_map(|raw| serde_json::from_str::<Value>(raw).ok())
        .find_map(|v| {
            v["result"]["structuredContent"]["session"]["seat"]
                .as_str()
                .map(String::from)
        })
        .expect("loop_status names the session's seat")
}

#[test]
fn a_tool_that_is_not_viewed_acts_on_the_hub_alone_and_says_so() {
    let (root, h, a, b) = hub_and_members("not-viewed");
    let server = serve(&root, Some(GATEWAY));
    let mut on_h = Session::open(server.port, &h);
    let readable = [h.as_str(), a.as_str(), b.as_str()];
    let view = [a.as_str(), b.as_str()];

    let reply = on_h.call("claim_report", json!({}), Some(&readable), Some(&view));
    let got = ok(&reply, "claim_report through H");
    assert!(
        got.get("by_design").is_none(),
        "claims are not viewed — H's own report, not a fan-out: {got}"
    );
    let note = got["view_note"]
        .as_str()
        .unwrap_or_else(|| panic!("a view_note says it acted on the hub alone: {got}"));
    assert!(
        note.contains("claim_report") && note.contains(&h),
        "the note names the tool and the hub it acted on: {note}"
    );
}

#[test]
fn two_hubs_viewing_the_same_members_in_opposite_orders_do_not_deadlock() {
    let (root, h, a, b) = hub_and_members("deadlock");
    let h2_dir = root.join("hub2");
    std::fs::create_dir_all(&h2_dir).unwrap();
    let h2 = mint(&h2_dir, "hubtwo", "hydrant");
    let server = serve(&root, Some(GATEWAY));
    let port = server.port;
    const ROUNDS: usize = 8;

    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
    for (hub, view, first_node, writes_into) in [
        (
            h.clone(),
            [a.clone(), b.clone()],
            format!("{a}:req:alpha"),
            a.clone(),
        ),
        (
            h2.clone(),
            [b.clone(), a.clone()],
            format!("{b}:req:beta"),
            b.clone(),
        ),
    ] {
        let tx = tx.clone();
        let readable = [hub.clone(), a.clone(), b.clone()];
        std::thread::spawn(move || {
            let mut session = Session::open(port, &hub);
            let readable: Vec<&str> = readable.iter().map(String::as_str).collect();
            let view: Vec<&str> = view.iter().map(String::as_str).collect();
            for round in 0..ROUNDS {
                let (tool, args) = match round % 4 {
                    0 => ("loop_status", json!({})),
                    1 => ("search_design", json!({"query": "only"})),
                    2 => ("get_node", json!({"id": first_node})),
                    _ => (
                        "add_requirement",
                        json!({
                            "id": format!("{writes_into}:req:from-{hub}-{round}"),
                            "name": format!("from {hub} round {round}"),
                            "statement": "a routed write under contention",
                        }),
                    ),
                };
                let reply = session.call(tool, args, Some(&readable), Some(&view));
                if let Err(e) = reply.outcome() {
                    let _ = tx.send(Err(format!("{hub} round {round} {tool}: {e}")));
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
            Ok(Err(e)) => panic!("a view call failed under contention: {e}"),
            Err(_) => panic!(
                "DEADLOCK OR HANG: {finished} of 2 hubs finished {ROUNDS} rounds within the \
                 deadline"
            ),
        }
    }
}
