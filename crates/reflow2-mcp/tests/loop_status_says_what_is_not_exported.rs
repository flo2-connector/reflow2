//! loop_status says when a design has never been exported, and lists what the
//! store holds that its last export does not, grouped by who wrote it.
//!
//! WRITTEN BEFORE THE IMPLEMENTATION, from the VS Code field log of 2026-10-05
//! (`art:vscode-call-field-log-2026-10-05`), on reflow2 0.76.0:
//!
//! - **[gap]** a member design held 195 nodes ONLY in its machine-local store —
//!   no export anywhere, no `--export-to` — "one disk away from loss", and
//!   loop_status answered `clean: true`, `next: []`. A recurrence of
//!   `fact:a-never-exported-local-design-is-still-silent-in-loop-status-through-the-door-on-0-77-0-2026-10-02`.
//! - **[friction]** a store also held ANOTHER session's uncommitted work, so an
//!   export either swept it in or was skipped (twice in one day).
//!
//! THE CAUSE OF THE SILENCE, the same on every door: the only sentence about
//! exports (`sync_debt::unexported_work`) is computed from the records this
//! seat has exported to. A store with none returned `None` — the same answer as
//! a fully exported one — so "nothing tracked" and "nothing owed" could not be
//! told apart. The `--call` door made it the normal case, not a different one.
//!
//! What is pinned here, through the real binary where a door is named:
//!   · a never-exported design raises the item through MCP (stdio), `--call`
//!     and `read`, and in graph_report, naming the one command that fixes it;
//!   · exporting once ends it; acknowledging it (a design kept local-only on
//!     purpose) stops it, through the existing gap acknowledgement;
//!   · a design served without its tree (a host's) is not told;
//!   · an exported store whose later writes come from two sessions lists both
//!     groups, with their writers;
//!   · an exported, up-to-date store says nothing new at all;
//!   · the answers stay bounded on a large store;
//!   · a store ahead of its export says so in `ahead_of_export`, the FIRST line
//!     of the door's printout, even behind a long artifact block — the owner's
//!     0.79.0 upgrade of 20 stores read `--call loop_status | head -40` and
//!     found only the artifact block there.

use std::io::{BufRead, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use reflow2_mcp::service::ReflowService;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

const PROJECT: &str = r#"{"id":"proj:p","name":"P"}"#;
const NEVER: &str = "NEVER BEEN EXPORTED";
const GAP: &str = "gap:the-design-has-never-been-exported";

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

/// A project folder the binary runs in, with an isolated HOME outside it.
struct Folder {
    dir: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl Folder {
    fn new() -> Folder {
        Folder {
            dir: tempfile::Builder::new()
                .prefix("reflow2-not-exported-")
                .tempdir()
                .unwrap(),
            home: tempfile::Builder::new()
                .prefix("reflow2-not-exported-home-")
                .tempdir()
                .unwrap(),
        }
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }

    fn graph(&self) -> String {
        self.path(".reflow2/graph").display().to_string()
    }

    fn command(&self) -> Command {
        let mut c = Command::new(bin());
        c.current_dir(self.dir.path())
            .env("HOME", self.home.path())
            .env("XDG_CONFIG_HOME", self.home.path().join(".config"))
            .env(
                "REFLOW2_CONFIG_DIR",
                self.home.path().join("reflow2-config"),
            )
            .env("RUST_LOG", "error")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("REFLOW2_CONTENT_POLICY")
            .env_remove("REFLOW2_TRUSTED_GATEWAY");
        c
    }

    /// Run the binary to completion with a deadline.
    fn run(&self, args: &[&str]) -> Output {
        let mut child = self
            .command()
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the binary runs");
        let mut out = child.stdout.take().unwrap();
        let mut err = child.stderr.take().unwrap();
        let o = std::thread::spawn(move || {
            let mut b = Vec::new();
            let _ = out.read_to_end(&mut b);
            b
        });
        let e = std::thread::spawn(move || {
            let mut b = Vec::new();
            let _ = err.read_to_end(&mut b);
            b
        });
        let start = Instant::now();
        let status = loop {
            if let Some(s) = child.try_wait().unwrap() {
                break s;
            }
            if start.elapsed() > Duration::from_secs(120) {
                let _ = child.kill();
                panic!("`reflow2-mcp {}` ran past 120 s", args.join(" "));
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        Output {
            status,
            stdout: o.join().unwrap(),
            stderr: e.join().unwrap(),
        }
    }

    /// `--call TOOL --args ARGS`, which must succeed; its JSON reply.
    fn call(&self, tool: &str, args: &str) -> Value {
        let o = self.run(&[
            "--graph-path",
            ".reflow2/graph",
            "--call",
            tool,
            "--args",
            args,
        ]);
        assert!(
            o.status.success(),
            "`--call {tool}` failed: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        serde_json::from_slice(&o.stdout).expect("--call prints JSON")
    }

    /// `read TOOL`, which must succeed; its JSON reply.
    fn read(&self, tool: &str) -> Value {
        let o = self.run(&["read", tool, "--graph-path", ".reflow2/graph"]);
        assert!(
            o.status.success(),
            "`read {tool}` failed: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        serde_json::from_slice(&o.stdout).expect("read prints JSON")
    }

    /// One MCP session over stdio: initialize, then each `(tool, args)` in
    /// order. Returns each call's structured reply.
    fn mcp(&self, calls: &[(&str, Value)]) -> Vec<Value> {
        let mut child = self
            .command()
            .args(["--graph-path", ".reflow2/graph"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the server runs");
        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let mut send = |v: Value| writeln!(stdin, "{v}").expect("write to the server");
        let wait_for = |id: u64| -> Value {
            let deadline = Instant::now() + Duration::from_secs(120);
            loop {
                let left = deadline.saturating_duration_since(Instant::now());
                let line = rx.recv_timeout(left).expect("the server answered");
                let Ok(v) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                if v["id"] == json!(id) {
                    return v;
                }
            }
        };
        send(
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "protocolVersion":"2025-06-18","capabilities":{},
            "clientInfo":{"name":"not-exported-test","version":"0"}}}),
        );
        let hello = wait_for(1);
        assert!(hello.get("result").is_some(), "initialize: {hello}");
        send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        let mut replies = Vec::new();
        for (i, (tool, args)) in calls.iter().enumerate() {
            let id = 2 + i as u64;
            send(json!({"jsonrpc":"2.0","id":id,"method":"tools/call",
                "params":{"name":tool,"arguments":args}}));
            let r = wait_for(id);
            replies.push(r["result"]["structuredContent"].clone());
        }
        drop(send);
        drop(stdin);
        let _ = child.kill();
        let _ = child.wait();
        replies
    }
}

fn next_of(v: &Value) -> Vec<String> {
    v["next"]
        .as_array()
        .unwrap_or_else(|| panic!("loop_status carries `next`: {v}"))
        .iter()
        .filter_map(|x| x.as_str().map(String::from))
        .collect()
}

/// The never-exported item, as every door must carry it: FIRST in `next`,
/// naming the command that fixes it, and in its own block at high severity.
fn assert_never_exported(v: &Value, door: &str, fix: &str) {
    let next = next_of(v);
    let first = next.first().cloned().unwrap_or_default();
    assert!(
        first.contains(NEVER),
        "{door}: the never-exported item must be FIRST in `next`, the list a session acts on: \
         {next:?}"
    );
    assert!(
        first.contains(fix),
        "{door}: it names the one command that fixes it ({fix}): {first}"
    );
    assert!(
        first.contains(GAP),
        "{door}: it says how a design kept local-only on purpose records that choice: {first}"
    );
    let block = &v["export_standing"];
    assert_eq!(block["state"], "never_exported", "{door}: {v}");
    assert_eq!(block["severity"], "high", "{door}: {block}");
    assert_eq!(block["gap_id"], GAP, "{door}: {block}");
}

#[test]
fn a_never_exported_design_is_told_through_mcp() {
    let f = Folder::new();
    let replies = f.mcp(&[
        ("add_project", serde_json::from_str(PROJECT).unwrap()),
        ("loop_status", json!({})),
        ("graph_report", json!({})),
    ]);
    assert_never_exported(&replies[1], "MCP stdio loop_status", "export_graph");
    assert_eq!(
        replies[2]["export_standing"]["state"], "never_exported",
        "graph_report — where-am-i's first read — carries it too: {}",
        replies[2]
    );
}

#[test]
fn a_never_exported_design_is_told_through_the_call_door() {
    let f = Folder::new();
    f.call("add_project", PROJECT);
    let v = f.call("loop_status", "{}");
    assert_never_exported(&v, "--call loop_status", "--call export_graph");
    let printed = f.run(&["--graph-path", ".reflow2/graph", "--call", "loop_status"]);
    let text = String::from_utf8_lossy(&printed.stdout);
    let first = text.lines().nth(1).unwrap_or_default();
    assert!(
        first.contains("\"ahead_of_export\"") && first.contains(NEVER),
        "the headline is the printout's first line: {first}"
    );
    let report = f.call("graph_report", "{}");
    assert_eq!(report["export_standing"]["state"], "never_exported");
}

#[test]
fn a_never_exported_design_is_told_through_read() {
    let f = Folder::new();
    f.call("add_project", PROJECT);
    let v = f.read("loop_status");
    assert_never_exported(&v, "read loop_status", "--call export_graph");
}

#[test]
fn exporting_once_ends_the_item_on_every_door() {
    let f = Folder::new();
    f.call("add_project", PROJECT);
    std::fs::create_dir_all(f.path("docs/design")).unwrap();
    f.call("export_graph", r#"{"path":"docs/design/p.json"}"#);
    for (door, v) in [
        ("--call", f.call("loop_status", "{}")),
        ("read", f.read("loop_status")),
    ] {
        assert!(
            v.get("export_standing").is_none(),
            "{door}: an exported design is not told it was never exported: {v}"
        );
        assert!(
            !next_of(&v).iter().any(|l| l.contains(NEVER)),
            "{door}: {:?}",
            next_of(&v)
        );
    }
}

#[test]
fn a_design_kept_local_on_purpose_is_not_nagged_once_the_choice_is_recorded() {
    let f = Folder::new();
    f.call("add_project", PROJECT);
    f.call(
        "acknowledge_gap",
        &json!({
            "gap_id": GAP,
            "affected_ids": [],
            "reason": "A scratch design, kept on this machine on purpose."
        })
        .to_string(),
    );
    let v = f.call("loop_status", "{}");
    assert!(
        !next_of(&v).iter().any(|l| l.contains(NEVER)),
        "an acknowledged local-only design is not nagged: {:?}",
        next_of(&v)
    );
    assert_eq!(
        v["export_standing"]["acknowledged"]["decision_id"],
        "decision:ack:the-design-has-never-been-exported",
        "the block still says what was chosen, so silence is not mistaken for a copy: {v}"
    );
    // And reviewed_gaps says where this gap is raised, rather than calling it
    // left over from a retired detector.
    let reviewed = f.call("reviewed_gaps", "{}");
    let text = reviewed.to_string();
    assert!(
        text.contains("Raised by loop_status and graph_report"),
        "{reviewed}"
    );
}

#[tokio::test]
async fn a_design_served_without_its_tree_is_not_told() {
    let dir = tempfile::tempdir().unwrap();
    let gp = dir.path().join("graph").display().to_string();
    let s = ReflowService::new(&gp).expect("service").without_tree();
    s.add_project(Parameters(serde_json::from_str(PROJECT).unwrap()))
        .await
        .expect("project");
    let v = s
        .loop_status(Parameters(serde_json::from_value(json!({})).unwrap()))
        .await
        .expect("loop_status")
        .structured_content
        .expect("structured");
    assert!(
        v.get("export_standing").is_none(),
        "a host's design is its server's store, and its backup is the host's: {v}"
    );
}

/// A requirement written the way `call_tool` writes it for a session that
/// declared it writes for `who`.
async fn write_for(s: &ReflowService, who: &str, id: &str) {
    let declared = s.effective_writes_for(Some(who));
    assert!(
        s.writes_for_precheck("add_requirement", declared.as_deref())
            .await
            .is_none()
    );
    s.serving_for(
        declared,
        s.add_requirement(Parameters(
            serde_json::from_value(
                json!({"id": id, "name": id, "statement": "Written since the export."}),
            )
            .unwrap(),
        )),
    )
    .await
    .expect("add_requirement");
}

async fn loop_status(s: &ReflowService, args: Value) -> Value {
    s.loop_status(Parameters(serde_json::from_value(args).unwrap()))
        .await
        .expect("loop_status")
        .structured_content
        .expect("structured")
}

/// An on-disk service with two people, exported to `record`.
async fn exported(dir: &std::path::Path) -> (ReflowService, String) {
    std::fs::create_dir_all(dir).unwrap();
    let gp = dir.join("graph").display().to_string();
    let record = dir.join("record.json").display().to_string();
    let s = ReflowService::new(&gp).expect("service");
    s.add_project(Parameters(serde_json::from_str(PROJECT).unwrap()))
        .await
        .expect("project");
    for (id, name) in [("who:sister", "Sister"), ("who:brother", "Brother")] {
        s.add_contributor(Parameters(
            serde_json::from_value(json!({"id": id, "name": name, "kind": "person"})).unwrap(),
        ))
        .await
        .expect("contributor");
    }
    s.export_graph(Parameters(
        serde_json::from_value(json!({"path": record})).unwrap(),
    ))
    .await
    .expect("export");
    (s, record)
}

#[tokio::test]
async fn two_sessions_unexported_work_is_listed_as_two_groups() {
    let dir = tempfile::tempdir().unwrap();
    let (s, record) = exported(dir.path()).await;
    let other = s.share();

    write_for(&s, "who:sister", "req:sister-one").await;
    write_for(&s, "who:sister", "req:sister-two").await;
    write_for(&other, "who:brother", "req:brother-wip").await;

    let v = loop_status(&s, json!({"since_export": true})).await;
    let u = &v["unexported"];
    assert_eq!(u["base"], record, "{u}");
    let groups = u["groups"].as_array().expect("groups");
    let of = |who: &str| -> &Value {
        groups
            .iter()
            .find(|g| g["written_by"] == who)
            .unwrap_or_else(|| panic!("no group for {who}: {u}"))
    };
    let sister = of("who:sister");
    assert_eq!(sister["counts"]["nodes_added"], 2, "{sister}");
    let brother = of("who:brother");
    assert_eq!(brother["counts"]["nodes_added"], 1, "{brother}");
    assert!(
        brother["ids"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i == "req:brother-wip"),
        "{brother}"
    );
    assert!(
        !sister.to_string().contains("req:brother-wip"),
        "one session's work never lands in the other's group: {sister}"
    );
    assert!(
        u["full_list"]
            .as_str()
            .unwrap_or_default()
            .contains("compare_designs"),
        "{u}"
    );
    let headline = v["ahead_of_export"].as_str().unwrap_or_default();
    assert!(
        headline.contains("3 node(s) added")
            && headline.contains("who:sister")
            && headline.contains("who:brother"),
        "the headline counts the work and names whose it is: {headline}"
    );

    // The default call still says the work is unexported, and now says where
    // the list is.
    let plain = loop_status(&s, json!({})).await;
    let line = next_of(&plain)
        .into_iter()
        .find(|l| l.contains("in no record"))
        .expect("the unexported-work line");
    assert!(line.contains("since_export"), "{line}");
}

#[tokio::test]
async fn an_up_to_date_store_says_nothing_new() {
    let dir = tempfile::tempdir().unwrap();
    let (s, _) = exported(dir.path()).await;

    let v = loop_status(&s, json!({})).await;
    assert!(v.get("export_standing").is_none(), "{v}");
    assert!(v.get("unexported").is_none(), "{v}");
    assert!(v.get("ahead_of_export").is_none(), "{v}");
    let next = next_of(&v);
    assert!(
        !next
            .iter()
            .any(|l| l.contains(NEVER) || l.contains("in no record")),
        "an exported, current design is told nothing about exports: {next:?}"
    );

    // Asked, the list is empty and says so.
    let asked = loop_status(&s, json!({"since_export": true})).await;
    assert_eq!(asked["unexported"]["identical"], true, "{asked}");
    assert!(asked.get("ahead_of_export").is_none(), "{asked}");
    assert_eq!(
        asked["unexported"]["groups"].as_array().map(Vec::len),
        Some(0)
    );
}

#[tokio::test]
async fn both_answers_stay_bounded_on_a_large_store() {
    let dir = tempfile::tempdir().unwrap();

    // Never exported, 300 nodes: the item is one fixed-size sentence.
    let gp = dir.path().join("big").display().to_string();
    let big = ReflowService::new(&gp).expect("service");
    big.add_project(Parameters(serde_json::from_str(PROJECT).unwrap()))
        .await
        .unwrap();
    for i in 0..300 {
        big.add_requirement(Parameters(
            serde_json::from_value(
                json!({"id": format!("req:r{i:03}"), "name": format!("R{i}"),
                "statement": "One of many."}),
            )
            .unwrap(),
        ))
        .await
        .unwrap();
    }
    let v = loop_status(&big, json!({})).await;
    let item = next_of(&v)
        .into_iter()
        .find(|l| l.contains(NEVER))
        .expect("the item");
    assert!(item.len() < 1_500, "{} chars: {item}", item.len());
    assert!(v["export_standing"].to_string().len() < 3_000);

    // Exported, then 25 writers each add 12: the list is bounded and counts
    // everything.
    let (s, _) = exported(&dir.path().join("many")).await;
    for w in 0..25 {
        let who = format!("who:w{w:02}");
        s.add_contributor(Parameters(
            serde_json::from_value(json!({"id": who, "name": who, "kind": "person"})).unwrap(),
        ))
        .await
        .unwrap();
        for i in 0..12 {
            write_for(&s, &who, &format!("req:w{w:02}-{i:02}")).await;
        }
    }
    let v = loop_status(&s, json!({"since_export": true})).await;
    let u = &v["unexported"];
    assert!(u["groups"].as_array().unwrap().len() <= 8, "{u}");
    assert!(u["groups_not_shown"].as_u64().unwrap() > 0, "{u}");
    assert!(u["totals"]["nodes_added"].as_u64().unwrap() >= 300, "{u}");
    let size = u.to_string().len();
    assert!(size < 12_000, "the unexported list is {size} chars");
}

#[test]
fn the_headline_leads_the_printout_even_behind_a_long_artifact_block() {
    let f = Folder::new();
    f.call("add_project", PROJECT);
    // The field shape: a member store whose registered files are remote, so
    // the artifact block lists every one of them as unmeasurable.
    let nodes: Vec<Value> = (0..40)
        .map(|i| {
            json!({"node_type": "Artifact", "id": format!("art:remote-{i:02}"), "props": {
                "name": format!("Remote {i}"),
                "artifact_type": "document",
                "location": format!(
                    "https://example.org/member/{i:02}/a-remote-location-nobody-can-measure.json"
                )
            }})
        })
        .collect();
    f.call("create_nodes", &json!({ "nodes": nodes }).to_string());
    std::fs::create_dir_all(f.path("docs/design")).unwrap();
    f.call("export_graph", r#"{"path":"docs/design/p.json"}"#);
    f.call(
        "add_requirement",
        r#"{"id":"req:late","name":"Late","statement":"Written after the export."}"#,
    );

    let head_of = |args: &[&str]| -> (usize, Vec<String>) {
        let o = f.run(args);
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        let lines: Vec<String> = String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(String::from)
            .collect();
        (lines.len(), lines.into_iter().take(40).collect())
    };
    let headline = |head: &[String]| -> String {
        head.iter()
            .find(|l| l.contains("\"ahead_of_export\""))
            .cloned()
            .unwrap_or_else(|| panic!("no headline in the first 40 lines: {head:#?}"))
    };

    let (total, head) = head_of(&["--graph-path", ".reflow2/graph", "--call", "loop_status"]);
    assert!(
        total > 200,
        "the fixture must reproduce a long artifact block: {total} lines"
    );
    let line = headline(&head);
    assert!(
        line.contains("ahead of")
            && line.contains("docs/design/p.json")
            && line.contains("1 node(s)"),
        "{line}"
    );

    // The same through `read`.
    let (_, head) = head_of(&["read", "loop_status", "--graph-path", ".reflow2/graph"]);
    headline(&head);

    // Asked for the list, the headline counts nodes AND edges and says whose.
    let (_, head) = head_of(&[
        "--graph-path",
        ".reflow2/graph",
        "--call",
        "loop_status",
        "--args",
        r#"{"since_export":true}"#,
    ]);
    let line = headline(&head);
    assert!(
        line.contains("1 node(s) added") && line.contains("no recorded writer"),
        "{line}"
    );

    // sync_status answers it too, in a field of its own; the record's own
    // state keeps its meaning.
    let sync = f.call("sync_status", "{}");
    assert!(
        sync["ahead_of_export"]
            .as_str()
            .unwrap_or_default()
            .contains("ahead of"),
        "{sync}"
    );
    assert_eq!(sync["sync"][0]["state"], "in_step", "{sync}");
}
