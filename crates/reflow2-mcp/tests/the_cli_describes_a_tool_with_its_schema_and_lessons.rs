//! The CLI describes a served tool in full — its input schema and the lessons
//! the OPENED design holds for it — through the real binary, and the served
//! `describe_schema` with `tool` gives the same answer through any door.
//!
//! `req:the-cli-describes-any-tool-with-its-full-schema-and-lessons`, step 4 of
//! `epoch:planned-the-call-door-works-for-an-agent-that-cannot-use-mcp`
//! (`dec:idea-the-cli-describes-a-tool-with-its-schema-and-lessons`).
//!
//! WHAT FAILED, measured on 0.77.0 (2026-10-02) and again on main 293f957
//! before this change:
//! · through the `--call` door an agent learned argument shapes one refusal at
//!   a time: the door held every tool's input schema and printed none of it,
//!   `find_tools` returned top-level parameter NAMES, `--describe` did not
//!   exist (clap: "unexpected argument"), and `describe_schema` refused `tool`
//!   (fact:root-cause-argument-shapes-are-learned-by-refusal-because-the-door-holds-the-full-schema-and-prints-none-of-it-2026-10-02);
//! · the lessons a design hangs on a tool rode only `tools/list`, which the
//!   door never reads — on reflow2's own design 269 lessons on 91 tools at
//!   293f957, and 0 in any reply the door could reach
//!   (fact:root-cause-tool-lessons-reach-only-tools-list-and-no-door-reply-carries-one-2026-10-02);
//! · the one list the door built was built on an empty in-memory design, so a
//!   describe that printed it would pass a SHAPE test and drop every lesson
//!   (fact:the-call-doors-tool-list-is-built-on-an-empty-design-so-it-holds-no-lessons-2026-10-02).
//!   So the lesson tests below hang a lesson on a scratch design and assert its
//!   ID arrives — the test that finding names.
//!
//! Each real-binary test here was run against main (293f957) before the fix
//! and failed there.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Output, Stdio};
use std::time::{Duration, Instant};

use reflow2_mcp::service::ReflowService;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

const BUDGET: usize = 30_000;
const POINTER_ID: &str = "abc123def4567890";
/// Port 9 (discard): nothing listens, and nothing here may try to reach it.
const POINTER_ADDRESS: &str = "http://127.0.0.1:9/g/abc123def4567890/mcp";

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

/// A folder the binary runs in, with HOME and configuration OUTSIDE it, so
/// "nothing was created here" can be read off a listing of the folder alone.
struct Scratch {
    folder: tempfile::TempDir,
    outside: tempfile::TempDir,
}

impl Scratch {
    fn new() -> Scratch {
        Scratch {
            folder: tempfile::Builder::new()
                .prefix("reflow2-describe-folder-")
                .tempdir()
                .unwrap(),
            outside: tempfile::Builder::new()
                .prefix("reflow2-describe-outside-")
                .tempdir()
                .unwrap(),
        }
    }

    /// A folder holding a design with one project in it.
    fn seeded() -> Scratch {
        let s = Scratch::new();
        s.call_ok(
            "add_project",
            json!({"id": "proj:p", "name": "P", "description": "A design for describing tools."}),
        );
        s
    }

    fn command(&self, args: &[&str]) -> Command {
        let home = self.outside.path().join("home");
        std::fs::create_dir_all(&home).unwrap();
        let mut c = Command::new(bin());
        c.current_dir(self.folder.path())
            .args(args)
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", home.join(".config"))
            .env("REFLOW2_CONFIG_DIR", home.join("reflow2-config"))
            .env("RUST_LOG", "error")
            .env_remove("REFLOW2_CONTENT_POLICY")
            .env_remove("REFLOW2_TRUSTED_GATEWAY")
            .env_remove("REFLOW2_OIDC_ISSUER")
            .env_remove("REFLOW2_CONTRIBUTOR_ID");
        c
    }

    /// Run the binary with a deadline: a describe that started serving instead
    /// must fail the test, not hang it.
    fn run(&self, args: &[&str]) -> Output {
        let mut child = self
            .command(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the binary runs");
        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let out_t = std::thread::spawn(move || {
            let mut b = Vec::new();
            let _ = stdout.read_to_end(&mut b);
            b
        });
        let err_t = std::thread::spawn(move || {
            let mut b = Vec::new();
            let _ = stderr.read_to_end(&mut b);
            b
        });
        let start = Instant::now();
        let status = loop {
            if let Some(s) = child.try_wait().unwrap() {
                break s;
            }
            if start.elapsed() > Duration::from_secs(120) {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "`reflow2-mcp {}` was still running after 120 s",
                    args.join(" ")
                );
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        Output {
            status,
            stdout: out_t.join().unwrap(),
            stderr: err_t.join().unwrap(),
        }
    }

    fn call_ok(&self, tool: &str, args: Value) -> Value {
        let a = args.to_string();
        let o = self.run(&["--call", tool, "--args", &a]);
        assert_eq!(o.status.code(), Some(0), "--call {tool}: {}", err(&o));
        stdout_json(&o)
    }

    /// Every path under the folder, relative, sorted.
    fn listing(&self) -> Vec<String> {
        fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                out.push(p.strip_prefix(root).unwrap().display().to_string());
                if p.is_dir() {
                    walk(root, &p, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(self.folder.path(), self.folder.path(), &mut out);
        out.sort();
        out
    }

    /// Hang a lesson on `steps`: a dated finding, or a rule.
    fn lesson(&self, id: &str, steps: &[&str]) {
        if id.starts_with("rule:") {
            self.call_ok(
                "add_design_rule",
                json!({
                    "id": id,
                    "name": format!("Rule {id}"),
                    "statement": "Say which relation, and why, on every link.",
                    "steps": steps,
                }),
            );
        } else {
            self.call_ok(
                "record_finding",
                json!({
                    "id": id,
                    "subject_id": "proj:p",
                    "name": format!("Lesson {id}"),
                    "statement": "Read the node's connections, not only the node.",
                    "valid_from": "2026-10-03",
                    "steps": steps,
                }),
            );
        }
    }
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

fn stdout_json(o: &Output) -> Value {
    let text = String::from_utf8_lossy(&o.stdout);
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}):\n{text}\nstderr:\n{}", err(o)))
}

fn describe(s: &Scratch, args: &[&str]) -> Value {
    let o = s.run(args);
    assert_eq!(
        o.status.code(),
        Some(0),
        "`reflow2-mcp {}` failed: {}",
        args.join(" "),
        err(&o)
    );
    stdout_json(&o)
}

fn lesson_ids(v: &Value) -> Vec<String> {
    v["lessons"]["items"]
        .as_array()
        .unwrap_or_else(|| panic!("no lessons.items in {v}"))
        .iter()
        .filter_map(|l| l["id"].as_str().map(String::from))
        .collect()
}

// ---- the shape ------------------------------------------------------------------

/// The report's case: a Decision with one link took four calls because the
/// link item's shape and the relation names were learned by refusal. The
/// default (brief) description shows both, under the reply budget.
#[test]
fn describe_add_decision_shows_the_link_item_shape_and_the_relation_enum() {
    let s = Scratch::seeded();
    let v = describe(&s, &["--describe", "add_decision"]);
    assert_eq!(v["tool"], "add_decision");
    assert_eq!(v["form"], "brief");
    let schema = &v["input_schema"];
    assert_eq!(
        schema["properties"]["related_to"]["items"]["$ref"], "#/$defs/RelationLinkReq",
        "the link item is a $ref into $defs: {schema}"
    );
    let link = &schema["$defs"]["RelationLinkReq"];
    assert_eq!(
        link["required"],
        json!(["relation", "other_id", "evidence"]),
        "{link}"
    );
    let relations: Vec<&str> = link["properties"]["relation"]["enum"]
        .as_array()
        .unwrap_or_else(|| panic!("no relation enum: {link}"))
        .iter()
        .filter_map(Value::as_str)
        .collect();
    for r in [
        "CONTRADICTS",
        "DEPENDS_ON",
        "CAUSES",
        "OBSOLETES",
        "VIOLATES",
    ] {
        assert!(relations.contains(&r), "{r} missing from {relations:?}");
    }
    assert!(
        link["properties"]["evidence"]["description"]
            .as_str()
            .is_some_and(|d| d.starts_with("WHY this relation is true")),
        "a field's first sentence is kept: {link}"
    );
    assert!(
        v["full_form"]
            .as_str()
            .unwrap_or_default()
            .contains("--full"),
        "the brief form says how to get the whole entry: {v}"
    );
    assert!(
        v.to_string().len() <= BUDGET,
        "the brief form is {} characters, over the reply budget",
        v.to_string().len()
    );
}

// ---- the lessons ----------------------------------------------------------------

/// THE TEST THE EMPTY-PROBE FINDING NAMES: a lesson hung on a tool by THIS
/// design arrives with the tool's description; on a design that holds none,
/// none arrive and the reply says so. Through `--describe`, `--list-tools`, and
/// the served route through the `--call` door.
#[test]
fn a_lesson_the_design_holds_for_a_tool_arrives_and_a_design_holding_none_says_so() {
    let s = Scratch::seeded();

    // A design holding no lesson for the tool.
    let v = describe(&s, &["--describe", "get_node"]);
    assert_eq!(v["lessons"]["count"], 0, "{v}");
    assert_eq!(v["lessons"]["items"], json!([]));
    let why = v["lessons"]["none_because"].as_str().unwrap_or_default();
    assert!(why.contains("holds no lesson for `get_node`"), "{why}");
    let v = describe(&s, &["--list-tools"]);
    assert_eq!(v["lessons"]["count"], 0, "{}", v["lessons"]);
    assert!(v["lessons"]["none_because"].is_string(), "{}", v["lessons"]);

    // Two lessons for get_node: a dated finding and a rule (which names a
    // second tool too).
    s.lesson("fact:read-the-edges-too", &["get_node"]);
    s.lesson("rule:say-which-relation", &["get_node", "add_decision"]);

    let v = describe(&s, &["--describe", "get_node"]);
    let ids = lesson_ids(&v);
    assert_eq!(v["lessons"]["count"], 2, "{v}");
    assert!(
        ids.contains(&"fact:read-the-edges-too".to_string())
            && ids.contains(&"rule:say-which-relation".to_string()),
        "{ids:?}"
    );
    // The brief description is the tool's own; the lessons are carried once,
    // as records.
    assert!(
        !v["description"]
            .as_str()
            .unwrap_or_default()
            .contains("LESSONS THIS DESIGN HOLDS"),
        "{}",
        v["description"]
    );

    let full = describe(&s, &["--describe", "get_node", "--full"]);
    let d = full["served"]["description"].as_str().unwrap_or_default();
    assert!(
        d.contains("LESSONS THIS DESIGN HOLDS FOR `get_node` (2)")
            && d.contains("fact:read-the-edges-too")
            && d.contains("rule:say-which-relation"),
        "the full entry carries the lessons where tools/list puts them: {d}"
    );
    assert_eq!(full["lessons"]["count"], 2);

    let all = describe(&s, &["--list-tools"]);
    let tools = all["tools"].as_array().expect("tools");
    let lessons_on = |name: &str| {
        tools
            .iter()
            .find(|t| t["tool"] == name)
            .map(|t| t["lessons"].clone())
            .unwrap_or_else(|| panic!("{name} is not listed"))
    };
    assert_eq!(lessons_on("get_node"), 2);
    assert_eq!(lessons_on("add_decision"), 1);
    assert_eq!(lessons_on("loop_status"), 0);
    assert_eq!(all["lessons"]["count"], 3, "{}", all["lessons"]);
    assert_eq!(all["lessons"]["tools_with_lessons"], 2);
    assert!(
        all.to_string().len() <= BUDGET,
        "the index is {} characters, over the reply budget",
        all.to_string().len()
    );

    // The SERVED route, through the --call door: same lessons, same shape.
    let served = s.call_ok("describe_schema", json!({"tool": "get_node"}));
    assert_eq!(lesson_ids(&served), ids, "{served}");
    assert_eq!(served["input_schema"], v["input_schema"]);
    assert!(
        served["full_form"]
            .as_str()
            .unwrap_or_default()
            .contains("describe_schema"),
        "{served}"
    );

    // And find_tools names the route, so a door agent finds it.
    let found = s.call_ok(
        "find_tools",
        json!({"query": "write down a decision we made and the reasoning"}),
    );
    let route = found["describe"].as_str().unwrap_or_default();
    assert!(
        route.contains("describe_schema") && route.contains("--describe"),
        "find_tools does not say how to read a tool's whole schema: {found}"
    );
}

// ---- the same entry tools/list serves ------------------------------------------

/// An MCP stdio session on a design: the tools/list a client of that design is
/// given. Killed on drop, so a failing assertion never leaves it holding the
/// store.
struct Session {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    id: u64,
}

impl Session {
    fn start(s: &Scratch) -> Session {
        let mut child = s
            .command(&["--graph-path", ".reflow2/graph"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the server starts");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut session = Session {
            child,
            stdin,
            stdout,
            id: 0,
        };
        session.rpc(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "describe-probe", "version": "0"}
            }),
        );
        writeln!(
            session.stdin,
            "{}",
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"})
        )
        .unwrap();
        session
    }

    fn rpc(&mut self, method: &str, params: Value) -> Value {
        self.id += 1;
        writeln!(
            self.stdin,
            "{}",
            json!({"jsonrpc": "2.0", "id": self.id, "method": method, "params": params})
        )
        .unwrap();
        self.stdin.flush().unwrap();
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("{method}: {e}: {line}"))
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// `--full` IS the tools/list entry a session on that design is given — not a
/// copy that could drift: compared with a live session's tools/list on the
/// same design, entry by entry and as a whole. And while that session HOLDS
/// the design, the describe still reads it (from a snapshot copy), lessons
/// included.
#[test]
fn full_is_exactly_what_tools_list_serves_on_that_design_even_while_it_is_held() {
    let s = Scratch::seeded();
    s.lesson("fact:read-the-edges-too", &["get_node"]);
    s.lesson("rule:say-which-relation", &["add_decision"]);

    let mut session = Session::start(&s);
    let listed = session.rpc("tools/list", json!({}))["result"]["tools"].clone();
    let listed = listed.as_array().expect("tools/list answered");
    assert!(listed.len() >= 180, "{} tools listed", listed.len());

    // The session holds the store now.
    let o = s.run(&["--describe", "add_decision", "--full"]);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    assert!(
        err(&o).contains("BEST-EFFORT SNAPSHOT"),
        "a held design is read from a copy, and stderr says so: {}",
        err(&o)
    );
    let v = stdout_json(&o);
    let entry = listed
        .iter()
        .find(|t| t["name"] == "add_decision")
        .expect("add_decision is listed");
    assert_eq!(
        &v["served"], entry,
        "--describe --full differs from tools/list"
    );
    assert!(
        entry["description"]
            .as_str()
            .unwrap_or_default()
            .contains("rule:say-which-relation"),
        "the session's own listing carries the lesson: {entry}"
    );

    let o = s.run(&["--list-tools", "--full"]);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    let all = stdout_json(&o);
    assert_eq!(
        all["tools"].as_array().map(Vec::len),
        Some(listed.len()),
        "--list-tools --full lists a different number of tools"
    );
    assert_eq!(
        &all["tools"],
        &Value::Array(listed.clone()),
        "--list-tools --full differs from tools/list"
    );
    assert_eq!(all["lessons"]["count"], 2);
    drop(session);
}

// ---- where is the design? --------------------------------------------------------

/// No design at the path: the surface is described, every lesson block says
/// there was no design, and nothing is opened or created. With
/// `--only-if-present` it refuses, as every one-shot mode does.
#[test]
fn where_there_is_no_design_it_describes_the_surface_says_so_and_creates_nothing() {
    let s = Scratch::new();
    let v = describe(&s, &["--describe", "get_node"]);
    assert_eq!(v["lessons"]["count"], 0);
    let why = v["lessons"]["none_because"].as_str().unwrap_or_default();
    assert!(why.contains("no design at"), "{why}");
    assert!(v["input_schema"]["properties"]["id"].is_object(), "{v}");
    let v = describe(&s, &["--list-tools"]);
    assert!(
        v["lessons"]["none_because"]
            .as_str()
            .unwrap_or_default()
            .contains("no design at"),
        "{}",
        v["lessons"]
    );
    assert!(s.listing().is_empty(), "left {:?}", s.listing());

    let o = s.run(&["--only-if-present", "--describe", "get_node"]);
    assert_eq!(o.status.code(), Some(1), "{}", err(&o));
    assert!(err(&o).contains("--only-if-present"), "{}", err(&o));
    assert!(s.listing().is_empty(), "left {:?}", s.listing());
}

/// A folder that names its design on a server: the lessons are that design's,
/// which this door cannot reach, so it refuses and names the design.
#[test]
fn in_a_folder_that_names_its_design_on_a_server_it_refuses_and_names_the_design() {
    let s = Scratch::new();
    std::fs::write(
        s.folder.path().join(".reflow2.toml"),
        format!("[design]\nid = \"{POINTER_ID}\"\naddress = \"{POINTER_ADDRESS}\"\n"),
    )
    .unwrap();
    for args in [
        vec!["--describe", "get_node"],
        vec!["--list-tools", "--full"],
    ] {
        let o = s.run(&args);
        assert_eq!(o.status.code(), Some(1), "{args:?}: {}", err(&o));
        assert!(
            err(&o).contains(POINTER_ID) && err(&o).contains(POINTER_ADDRESS),
            "{}",
            err(&o)
        );
        assert!(o.stdout.is_empty());
        assert_eq!(s.listing(), vec![".reflow2.toml".to_string()]);
    }
}

/// A name nothing serves: exit 1, the nearest served names, and find_tools.
#[test]
fn an_unknown_tool_is_refused_with_the_nearest_names() {
    let s = Scratch::seeded();
    let o = s.run(&["--describe", "get_nod"]);
    assert_eq!(o.status.code(), Some(1), "{}", err(&o));
    let e = err(&o);
    assert!(
        e.contains("get_node") && e.contains("find_tools"),
        "the refusal names the nearest tool and how to find one: {e}"
    );
    assert!(o.stdout.is_empty());
}

/// `--full` alone, on a serving command line, asks for nothing a server does:
/// refused rather than ignored.
#[test]
fn full_without_a_describe_mode_is_refused() {
    let s = Scratch::new();
    let o = s.run(&["--full"]);
    assert_eq!(o.status.code(), Some(1), "{}", err(&o));
    assert!(err(&o).contains("--describe"), "{}", err(&o));
    assert!(s.listing().is_empty(), "left {:?}", s.listing());
}

// ---- in process: every tool, and the budget ---------------------------------------

fn strip_prose(v: &Value) -> Value {
    match v {
        Value::Object(m) => Value::Object(
            m.iter()
                .filter(|(k, v)| !(k.as_str() == "description" && v.is_string()))
                .map(|(k, v)| {
                    let kept = if matches!(k.as_str(), "default" | "enum" | "const") {
                        v.clone()
                    } else {
                        strip_prose(v)
                    };
                    (k.clone(), kept)
                })
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.iter().map(strip_prose).collect()),
        o => o.clone(),
    }
}

async fn describe_schema(s: &ReflowService, args: Value) -> Value {
    s.describe_schema(Parameters(serde_json::from_value(args).unwrap()))
        .await
        .expect("describe_schema answers")
        .structured_content
        .expect("structured")
}

/// EVERY served tool: the brief form drops no structural fact (with every
/// description removed, its schema IS the served one), fits the reply budget,
/// and the full form IS the served entry.
#[tokio::test]
async fn every_tools_brief_form_keeps_every_structural_fact_and_fits_the_budget() {
    let s = ReflowService::in_memory().unwrap();
    let served = s.tools_with_lessons_for_test().await;
    assert!(served.len() >= 180, "{} tools", served.len());
    let mut failures = Vec::new();
    for tool in &served {
        let name = tool.name.to_string();
        let brief = describe_schema(&s, json!({"tool": name})).await;
        let schema = Value::Object((*tool.input_schema).clone());
        if strip_prose(&brief["input_schema"]) != strip_prose(&schema) {
            failures.push(format!("{name}: the brief schema lost structure"));
        }
        if brief.to_string().len() > BUDGET {
            failures.push(format!(
                "{name}: brief is {} chars",
                brief.to_string().len()
            ));
        }
        let full = describe_schema(&s, json!({"tool": name, "full": true})).await;
        if full["served"] != serde_json::to_value(tool).unwrap() {
            failures.push(format!("{name}: the full form is not the served entry"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A tool a design has hung MANY long lessons on is still described within
/// the reply budget, and no lesson id is budgeted away.
#[tokio::test]
async fn a_tool_with_many_long_lessons_is_described_within_the_budget_and_keeps_every_id() {
    let s = ReflowService::in_memory().unwrap();
    s.add_project(Parameters(
        serde_json::from_value(json!({"id": "proj:p", "name": "P"})).unwrap(),
    ))
    .await
    .unwrap();
    let mut ids = Vec::new();
    for i in 0..40 {
        let id = format!("fact:lesson-number-{i:02}-about-loop-status");
        s.record_finding(Parameters(
            serde_json::from_value(json!({
                "id": id,
                "subject_id": "proj:p",
                "name": format!("Lesson {i}: {}", "a long name for a lesson. ".repeat(12)),
                "statement": "what bit us and what to do instead. ".repeat(40),
                "valid_from": "2026-10-03",
                "steps": ["loop_status"],
            }))
            .unwrap(),
        ))
        .await
        .unwrap();
        ids.push(id);
    }
    let v = describe_schema(&s, json!({"tool": "loop_status"})).await;
    let chars = v.to_string().len();
    assert!(chars <= BUDGET, "{chars} characters");
    assert_eq!(v["lessons"]["count"], 40);
    let got = lesson_ids(&v);
    for id in &ids {
        assert!(got.contains(id), "{id} was budgeted away");
    }
}

/// A half-given or mixed request is a mistake, not a request for everything.
#[tokio::test]
async fn describe_schema_refuses_a_tool_mixed_with_the_vocabulary_and_full_alone() {
    let s = ReflowService::in_memory().unwrap();
    for args in [
        json!({"tool": "get_node", "node_type": "Requirement"}),
        json!({"full": true}),
        json!({"tool": "no_such_tool"}),
    ] {
        let r = s
            .describe_schema(Parameters(serde_json::from_value(args.clone()).unwrap()))
            .await;
        assert!(r.is_err(), "{args} was answered: {r:?}");
    }
}
