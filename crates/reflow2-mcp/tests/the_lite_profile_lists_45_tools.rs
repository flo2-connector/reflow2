//! The LITE profile lists 45 tools, and every other tool is one call away.
//!
//! Approved by the owner on 2026-11-27 (`crate::profile`): reflow2 serves
//! about two hundred tools, a host that loads every description pays roughly
//! 140,000 tokens a session, and the top 40 tools carried 97.4% of 30,439
//! counted calls. So a lite session is listed `profile::LITE_TOOLS` and
//! `use_another_tool`, which calls any served tool by name; `find_tools`,
//! listed in both, searches every tool and gives each found tool's input
//! schema. FULL is the default and its listing is unchanged (the toolsnaps
//! hold it). The profile is chosen when a session connects, never mid-session.
//!
//! Over the REAL binary, on stdio and over HTTP, because what is pinned is
//! what a host is listed — an in-process listing would agree with itself.
//! Each session is an `--ephemeral` design, so nothing needs a store.

use reflow2_mcp::profile::{LITE_TOOLS, USE_ANOTHER_TOOL};
use reflow2_mcp::service::ReflowService;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

/// One stdio session on the real binary.
struct Session {
    child: Child,
    lines: mpsc::Receiver<String>,
    next_id: u64,
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Session {
    /// Start `--ephemeral` with `args` and `env`, and shake hands.
    fn start(args: &[&str], env: &[(&str, &str)]) -> Session {
        let mut cmd = Command::new(bin());
        cmd.arg("--ephemeral")
            .args(args)
            .env("RUST_LOG", "warn")
            .env_remove(reflow2_mcp::profile::ENV)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        for (k, v) in env {
            cmd.env(k, v);
        }
        let mut child = cmd.spawn().expect("the binary runs");
        let stdout = child.stdout.take().unwrap();
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    return;
                }
            }
        });
        let mut s = Session {
            child,
            lines,
            next_id: 1,
        };
        s.rpc(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "lite-profile-test", "version": "1"}
            }),
        );
        s.send(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        s
    }

    fn send(&mut self, msg: Value) {
        let stdin = self.child.stdin.as_mut().unwrap();
        writeln!(stdin, "{msg}").unwrap();
        stdin.flush().unwrap();
    }

    /// The whole JSON-RPC reply to one request.
    fn rpc(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        loop {
            let line = self
                .lines
                .recv_timeout(Duration::from_secs(60))
                .unwrap_or_else(|_| panic!("no reply to `{method}` within 60 s"));
            let v: Value = serde_json::from_str(&line).unwrap();
            if v.get("id").and_then(Value::as_u64) == Some(id) {
                return v;
            }
        }
    }

    fn listed(&mut self) -> Vec<Value> {
        let r = self.rpc("tools/list", json!({}));
        r["result"]["tools"]
            .as_array()
            .unwrap_or_else(|| panic!("tools/list answered {r}"))
            .clone()
    }

    fn call(&mut self, tool: &str, arguments: Value) -> Value {
        self.rpc("tools/call", json!({"name": tool, "arguments": arguments}))
    }
}

fn names_of(tools: &[Value]) -> BTreeSet<String> {
    tools
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect()
}

/// The 45 names a lite session is listed: the const, and the door.
fn lite_names() -> BTreeSet<String> {
    LITE_TOOLS
        .iter()
        .chain([&USE_ANOTHER_TOOL])
        .map(|s| s.to_string())
        .collect()
}

/// A reply with the fields that measure THIS call rather than say what it
/// found taken out — wall-clock timings — so two calls can be compared.
fn comparable(mut v: Value) -> Value {
    fn strip(v: &mut Value) {
        match v {
            Value::Object(o) => {
                for k in [
                    "elapsed_ms",
                    "took_ms",
                    "duration_ms",
                    "generated_at",
                    "as_of",
                ] {
                    o.remove(k);
                }
                o.values_mut().for_each(strip);
            }
            Value::Array(a) => a.iter_mut().for_each(strip),
            _ => {}
        }
    }
    if let Some(r) = v.get_mut("result") {
        strip(r);
        // The text block of a JSON reply is the payload again, or a sentence
        // pointing at it; the structured payload is what is compared.
        if let Some(o) = r.as_object_mut() {
            o.remove("content");
        }
    }
    if let Some(o) = v.as_object_mut() {
        o.remove("id");
    }
    v
}

#[test]
fn every_name_on_the_lite_list_is_a_served_tool() {
    // A RENAMED TOOL FAILS HERE rather than silently leaving lite.
    let served: BTreeSet<String> = ReflowService::served_tools()
        .iter()
        .map(|t| t.name.to_string())
        .collect();
    let missing: Vec<_> = LITE_TOOLS
        .iter()
        .filter(|n| !served.contains(**n))
        .collect();
    assert!(
        missing.is_empty(),
        "the lite list names tools nothing serves: {missing:?}"
    );
    assert!(served.contains(USE_ANOTHER_TOOL), "the door is served");
    assert_eq!(lite_names().len(), 45);
}

#[test]
fn lite_lists_exactly_45_tools_and_full_lists_every_served_one() {
    let served: BTreeSet<String> = ReflowService::served_tools()
        .iter()
        .map(|t| t.name.to_string())
        .collect();

    // FULL, the default: every served tool, `use_another_tool` included.
    let mut full = Session::start(&[], &[]);
    let full_tools = full.listed();
    assert_eq!(names_of(&full_tools), served);
    let mut asked_full = Session::start(&["--profile", "full"], &[]);
    assert_eq!(
        asked_full.listed(),
        full_tools,
        "--profile full is the default, byte for byte"
    );

    // LITE, by the flag and by the environment.
    let mut lite = Session::start(&["--profile", "lite"], &[]);
    let lite_tools = lite.listed();
    assert_eq!(lite_tools.len(), 45);
    assert_eq!(names_of(&lite_tools), lite_names());
    let mut by_env = Session::start(&[], &[(reflow2_mcp::profile::ENV, "lite")]);
    assert_eq!(names_of(&by_env.listed()), lite_names());

    // FILTERED, NOT REBUILT: each tool lite keeps is listed exactly as full
    // lists it, decorations and all.
    for t in &lite_tools {
        let same = full_tools
            .iter()
            .find(|f| f["name"] == t["name"])
            .expect("a lite tool is a full tool");
        assert_eq!(t, same, "{} is listed differently in lite", t["name"]);
    }
}

#[test]
fn a_profile_that_is_not_one_is_refused_at_startup() {
    for (args, env) in [
        (vec!["--profile", "tiny"], vec![]),
        (vec![], vec![(reflow2_mcp::profile::ENV, "tiny")]),
    ] {
        let mut cmd = Command::new(bin());
        cmd.arg("--ephemeral")
            .args(&args)
            .env_remove(reflow2_mcp::profile::ENV)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        for (k, v) in &env {
            cmd.env(k, v);
        }
        let out = cmd.output().expect("the binary runs");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{args:?} {env:?} was accepted");
        assert!(
            stderr.contains("`tiny` is not a reflow2 profile")
                && stderr.contains("`full`")
                && stderr.contains("`lite`"),
            "the refusal names the value and the two profiles: {stderr}"
        );
    }
}

#[test]
fn use_another_tool_answers_exactly_as_the_tool_called_directly() {
    // Each call in a fresh session on an empty design, so neither reply is
    // shaped by the other (a read's loop hint fires once per change).
    for (tool, args) in [
        ("topic_report", json!({"query": "persistence"})),
        ("what_next", json!({})),
    ] {
        assert!(
            !LITE_TOOLS.contains(&tool),
            "{tool} must be one lite does not list"
        );
        let direct = Session::start(&["--profile", "lite"], &[]).call(tool, args.clone());
        let through = Session::start(&["--profile", "lite"], &[])
            .call(USE_ANOTHER_TOOL, json!({"tool": tool, "arguments": args}));
        assert!(direct.get("result").is_some(), "{tool} answered {direct}");
        assert_ne!(direct["result"]["isError"], json!(true), "{direct}");
        assert_eq!(
            comparable(through),
            comparable(direct),
            "`{tool}` through use_another_tool answered differently"
        );
    }
    // A tool lite does not list is still served when called directly.
    let mut lite = Session::start(&["--profile", "lite"], &[]);
    let r = lite.call("what_next", json!({}));
    assert_ne!(r["result"]["isError"], json!(true), "{r}");
}

#[test]
fn use_another_tool_refuses_itself_and_a_name_nothing_serves() {
    let mut s = Session::start(&["--profile", "lite"], &[]);
    let itself = s.call(USE_ANOTHER_TOOL, json!({"tool": USE_ANOTHER_TOOL}));
    assert_eq!(itself["result"]["isError"], json!(true), "{itself}");
    let text = itself["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("does not call itself"), "{text}");

    // An unknown name is refused exactly as a direct call of it is.
    let direct = s.call("no_such_tool", json!({}));
    let through = s.call(
        USE_ANOTHER_TOOL,
        json!({"tool": "no_such_tool", "arguments": {}}),
    );
    assert!(
        direct["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("tool not found"),
        "{direct}"
    );
    assert_eq!(through["error"], direct["error"]);

    // Arguments that do not fit its schema are refused naming the door.
    let bad = s.call(USE_ANOTHER_TOOL, json!({"arguments": {}}));
    assert_eq!(bad["result"]["isError"], json!(true), "{bad}");
    assert!(
        bad["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains(USE_ANOTHER_TOOL),
        "{bad}"
    );
}

#[test]
fn find_tools_in_lite_searches_every_tool_and_gives_each_its_input_schema() {
    let full_tools = Session::start(&[], &[]).listed();
    let mut lite = Session::start(&["--profile", "lite"], &[]);
    let r = lite.call(
        "find_tools",
        json!({"query": "everything the design says about one subject in one place"}),
    );
    let reply = &r["result"]["structuredContent"];
    assert_eq!(
        reply["searched"].as_u64(),
        Some(full_tools.len() as u64),
        "find_tools searched only what lite lists: {reply}"
    );
    let items = reply["items"].as_array().expect("items");
    let found = items
        .iter()
        .find(|i| i["tool"] == "topic_report")
        .unwrap_or_else(|| panic!("an unlisted tool is found: {reply}"));
    let listed = full_tools
        .iter()
        .find(|t| t["name"] == "topic_report")
        .unwrap();
    assert_eq!(
        found["input_schema"], listed["inputSchema"],
        "find_tools gives the found tool's whole input schema, as tools/list serves it"
    );
    if reply.get("schemas_left_out").is_none() {
        for i in items {
            assert!(i["input_schema"].is_object(), "{i}");
        }
    }
}

// ---- over HTTP: `?profile=lite` on the endpoint URL -------------------------

struct HttpServer {
    child: Child,
    port: u16,
}

impl Drop for HttpServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl HttpServer {
    fn start(args: &[&str]) -> HttpServer {
        let mut child = Command::new(bin())
            .args(args)
            .args(["--http", "127.0.0.1:0"])
            .env("RUST_LOG", "warn")
            .env_remove(reflow2_mcp::profile::ENV)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn reflow2-mcp");
        let stderr = child.stderr.take().unwrap();
        let lines = Arc::new(Mutex::new(Vec::<String>::new()));
        let sink = Arc::clone(&lines);
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                sink.lock().unwrap().push(line);
            }
        });
        let deadline = Instant::now() + Duration::from_secs(90);
        let port = loop {
            if let Some(p) = lines.lock().unwrap().iter().find_map(|l| {
                let i = l.find("http://127.0.0.1:")?;
                l[i + "http://127.0.0.1:".len()..]
                    .chars()
                    .take_while(char::is_ascii_digit)
                    .collect::<String>()
                    .parse::<u16>()
                    .ok()
            }) {
                break p;
            }
            assert!(Instant::now() < deadline, "no address: {lines:?}");
            std::thread::sleep(Duration::from_millis(20));
        };
        HttpServer { child, port }
    }

    /// POST one JSON-RPC message to `path`: (status, session id, messages).
    fn post(
        &self,
        path: &str,
        session: Option<&str>,
        body: &Value,
    ) -> (u16, Option<String>, Vec<Value>) {
        let body = body.to_string();
        let mut s = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(60))).unwrap();
        let sid = session
            .map(|s| format!("Mcp-Session-Id: {s}\r\n"))
            .unwrap_or_default();
        write!(
            s,
            "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\n\
             Accept: application/json, text/event-stream\r\n{sid}Content-Length: {}\r\n\
             Connection: close\r\n\r\n{body}",
            self.port,
            body.len()
        )
        .unwrap();
        let mut raw = Vec::new();
        let _ = s.read_to_end(&mut raw);
        let split = raw
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .expect("an HTTP head");
        let head = String::from_utf8_lossy(&raw[..split]).to_string();
        let mut rest = raw[split + 4..].to_vec();
        let header = |name: &str| {
            head.lines().find_map(|l| {
                let (k, v) = l.split_once(':')?;
                k.eq_ignore_ascii_case(name).then(|| v.trim().to_string())
            })
        };
        if header("transfer-encoding").is_some_and(|v| v.contains("chunked")) {
            rest = dechunk(&rest);
        }
        let status = head
            .split_whitespace()
            .nth(1)
            .and_then(|c| c.parse().ok())
            .unwrap_or(0);
        let text = String::from_utf8_lossy(&rest).to_string();
        let messages = if header("content-type").is_some_and(|v| v.contains("event-stream")) {
            text.lines()
                .filter_map(|l| l.strip_prefix("data:"))
                .filter_map(|d| serde_json::from_str(d.trim()).ok())
                .collect()
        } else {
            serde_json::from_str(&text).into_iter().collect()
        };
        (status, header("mcp-session-id"), messages)
    }

    /// Connect a session at `path` and list its tools — or the refusal.
    fn list_at(&self, path: &str) -> Result<BTreeSet<String>, Value> {
        let (_, sid, hello) = self.post(
            path,
            None,
            &json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                "protocolVersion": "2025-06-18", "capabilities": {},
                "clientInfo": {"name": "lite-http-test", "version": "1"}}}),
        );
        if let Some(err) = hello.iter().find_map(|m| m.get("error")) {
            return Err(err.clone());
        }
        let sid = sid.expect("a session id");
        self.post(
            path,
            Some(&sid),
            &json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        );
        let (_, _, listed) = self.post(
            path,
            Some(&sid),
            &json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
        );
        let tools = listed
            .iter()
            .find_map(|m| m.pointer("/result/tools").and_then(Value::as_array))
            .unwrap_or_else(|| panic!("tools/list answered {listed:?}"));
        Ok(names_of(tools))
    }
}

fn dechunk(mut body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let Some(eol) = body.windows(2).position(|w| w == b"\r\n") else {
            return out;
        };
        let size = usize::from_str_radix(
            String::from_utf8_lossy(&body[..eol])
                .split(';')
                .next()
                .unwrap()
                .trim(),
            16,
        )
        .unwrap_or(0);
        if size == 0 {
            return out;
        }
        let start = eol + 2;
        let end = (start + size).min(body.len());
        out.extend_from_slice(&body[start..end]);
        body = &body[(end + 2).min(body.len())..];
    }
}

#[test]
fn over_http_the_url_chooses_a_sessions_profile_when_it_connects() {
    let server = HttpServer::start(&["--ephemeral"]);
    let served: BTreeSet<String> = ReflowService::served_tools()
        .iter()
        .map(|t| t.name.to_string())
        .collect();
    assert_eq!(server.list_at("/").unwrap(), served, "no parameter is full");
    assert_eq!(server.list_at("/?profile=lite").unwrap(), lite_names());
    assert_eq!(server.list_at("/?profile=full").unwrap(), served);
    // Two sessions on one server, each with its own profile.
    assert_eq!(server.list_at("/").unwrap(), served);
    let refused = server.list_at("/?profile=tiny").unwrap_err();
    assert!(
        refused["message"]
            .as_str()
            .unwrap()
            .contains("`tiny` is not a reflow2 profile"),
        "{refused}"
    );
}

/// A design minted in `dir` by one writing call, and its graph_id.
#[cfg(feature = "rocksdb")]
fn mint(dir: &std::path::Path) -> String {
    let store = dir.join(".reflow2").join("graph");
    let out = Command::new(bin())
        .arg("--graph-path")
        .arg(&store)
        .args(["--no-export", "--call", "add_requirement", "--args"])
        .arg(r#"{"id":"req:lite","name":"lite","statement":"a lite session reaches it"}"#)
        .env_remove(reflow2_mcp::profile::ENV)
        .output()
        .expect("the binary runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let id: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join(".reflow2").join("graph.id.json")).unwrap(),
    )
    .unwrap();
    id["graph_id"].as_str().unwrap().to_string()
}

#[cfg(feature = "rocksdb")]
#[test]
fn through_a_registry_the_url_chooses_a_sessions_profile_too() {
    let root = std::env::temp_dir().join(format!(
        "reflow2-lite-registry-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let dir = root.join("one");
    std::fs::create_dir_all(&dir).unwrap();
    let id = mint(&dir);
    let server = HttpServer::start(&["--registry-root", root.to_str().unwrap()]);
    let served: BTreeSet<String> = ReflowService::served_tools()
        .iter()
        .map(|t| t.name.to_string())
        .collect();
    assert_eq!(server.list_at(&format!("/g/{id}/")).unwrap(), served);
    assert_eq!(
        server.list_at(&format!("/g/{id}/?profile=lite")).unwrap(),
        lite_names()
    );
    drop(server);
    let _ = std::fs::remove_dir_all(&root);
}
