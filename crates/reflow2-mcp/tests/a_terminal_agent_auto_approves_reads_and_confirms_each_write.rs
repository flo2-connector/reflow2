//! A terminal agent can have every read auto-approved while each write still
//! asks: `reflow2 read <tool>` runs only what changes nothing, and
//! `reflow2 write <tool>` runs anything — through the real binary, in real
//! folders.
//!
//! `req:a-terminal-agent-can-auto-approve-reads-and-confirm-each-write`, step 3
//! of the plan for the `--call` door
//! (`dec:idea-a-shell-driven-agent-approves-reads-once-and-confirms-each-write`).
//!
//! WHAT FAILED, measured on main 293f957 (2026-10-03):
//! · `reflow2-mcp read graph_report` → exit 2, "unrecognized subcommand 'read'".
//!   The command text of a door call carries a tool name and nothing else
//!   about it, so one auto-approve rule cannot tell a read from a write
//!   (fact:root-cause-one-regex-cannot-separate-door-reads-because-the-read-set-is-not-in-the-command-and-not-served-2026-10-02).
//! · Nothing on the command line serves the read set: `find_tools` items
//!   carry `tool`, `summary`, `parameters` and `score`, and no listing says
//!   which of the 195 served tools only read (78).
//! · `--read-only --call` refuses a writer (item 1), but in its own words
//!   ("Drop --read-only to write"), refuses `export_graph`'s file only after
//!   the store is opened, and logs an INFO line on every call and an rmcp WARN
//!   line beside every refusal.
//!
//! THE CLASS: the door's one piece of safety knowledge — read or write — lived
//! inside the process, while the approval is decided outside it, on the
//! command's text. So the tests assert the class: EVERY served tool through
//! `read` (reads run, everything else is refused before anything is opened,
//! and the design and the disk are unchanged), every flag the binary lists
//! typed AFTER `read` (none can make it change anything), the file rule, the
//! listing, and `write`.
//!
//! "Unchanged" is item 1's meaning of `--read-only`: no node, edge or property
//! of the design, and no file, changes. Opening an existing store still moves
//! RocksDB's own files and the records beside it (version stamp, handshake
//! record, usage ledger) — `BOOKKEEPING` below — and nothing else may move:
//! not the folder, not HOME, not TMPDIR.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use reflow2_mcp::service::{FILE_WRITING_TOOLS, ReflowService};

const REQ: &str = r#"{"id":"req:verb-probe","name":"Verb probe","statement":"The probe shall be written only through the write verb."}"#;

/// The phrase `read`'s own refusal carries, so a reader — and these tests —
/// can tell it from a tool's refusal.
const READ_REFUSES: &str = "`read` runs only tools that change nothing";

/// The records beside a store that opening it may move (item 1's stated
/// limit), relative to the folder. Everything else must stay byte-identical.
const BOOKKEEPING: [&str; 3] = [
    ".reflow2/graph.usage.jsonl",
    ".reflow2/graph.client.json",
    ".reflow2/graph.meta.json",
];

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

/// A folder the binary runs in, plus HOME and TMPDIR outside it, so "nothing
/// changed" can be read off three listings.
struct Scratch {
    root: tempfile::TempDir,
}

impl Scratch {
    fn new() -> Scratch {
        let root = tempfile::Builder::new()
            .prefix("reflow2-verbs-")
            .tempdir()
            .unwrap();
        for d in ["folder", "home", "tmp"] {
            std::fs::create_dir_all(root.path().join(d)).unwrap();
        }
        Scratch { root }
    }

    /// A folder holding a design: a project and a requirement.
    fn seeded() -> Scratch {
        let s = Scratch::new();
        for (tool, args) in [
            ("add_project", r#"{"id":"proj:seed","name":"Seed"}"#),
            (
                "add_requirement",
                r#"{"id":"req:seed","name":"Seed","statement":"The seed shall exist."}"#,
            ),
        ] {
            let o = s.run(&["--call", tool, "--args", args], None);
            assert_eq!(o.status.code(), Some(0), "seed {tool}: {}", err(&o));
        }
        s
    }

    fn folder(&self) -> PathBuf {
        self.root.path().join("folder")
    }

    fn outside(&self, name: &str) -> PathBuf {
        self.root.path().join(name)
    }

    /// Run the binary in the folder, with an isolated environment and a
    /// deadline, and NO `RUST_LOG`: what the verbs print by default is part of
    /// what is under test.
    fn run(&self, args: &[&str], stdin: Option<&str>) -> Output {
        let home = self.root.path().join("home");
        let mut child = Command::new(bin())
            .current_dir(self.folder())
            .args(args)
            .env("HOME", &home)
            .env("TMPDIR", self.root.path().join("tmp"))
            .env("XDG_CONFIG_HOME", home.join(".config"))
            .env("REFLOW2_CONFIG_DIR", home.join("reflow2-config"))
            .env_remove("RUST_LOG")
            .env_remove("REFLOW2_CONTENT_POLICY")
            .env_remove("REFLOW2_TRUSTED_GATEWAY")
            .env_remove("REFLOW2_OIDC_ISSUER")
            .env_remove("REFLOW2_PUBLIC_URL")
            .env_remove("REFLOW2_CONTRIBUTOR_ID")
            .env_remove("REFLOW2_CONTRIBUTOR_MAP")
            .stdin(if stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the binary runs");
        if let Some(text) = stdin {
            let mut pipe = child.stdin.take().unwrap();
            pipe.write_all(text.as_bytes()).unwrap();
        }
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
                    "`reflow2-mcp {}` was still running after 120 s: something is serving \
                     instead of answering once",
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

    /// Every file and directory under the scratch root — the folder, HOME and
    /// TMPDIR — with a hash of each file's bytes.
    fn everything(&self) -> BTreeMap<String, u64> {
        fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, u64>) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                let rel = p.strip_prefix(root).unwrap().display().to_string();
                if p.is_dir() {
                    out.insert(format!("{rel}/"), 0);
                    walk(root, &p, out);
                } else {
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    std::fs::read(&p).unwrap_or_default().hash(&mut h);
                    out.insert(rel, h.finish());
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(self.root.path(), self.root.path(), &mut out);
        out
    }

    /// The design's own content hash, read through the plain door.
    fn design(&self) -> String {
        let o = self.run(&["--call", "export_graph"], None);
        assert_eq!(o.status.code(), Some(0), "export: {}", err(&o));
        stdout_json(&o)["content_hash"]
            .as_str()
            .expect("the export carries its content hash")
            .to_string()
    }

    fn node(&self, id: &str) -> serde_json::Value {
        let o = self.run(
            &[
                "--call",
                "get_node",
                "--args",
                &format!(r#"{{"id":"{id}"}}"#),
            ],
            None,
        );
        assert_eq!(o.status.code(), Some(0), "get_node {id}: {}", err(&o));
        stdout_json(&o)["node"].clone()
    }
}

/// What changed between two listings, as `path (how)`.
fn changes(before: &BTreeMap<String, u64>, after: &BTreeMap<String, u64>) -> Vec<String> {
    let mut out = Vec::new();
    for (p, h) in before {
        match after.get(p) {
            None => out.push(format!("{p} (removed)")),
            Some(a) if a != h => out.push(format!("{p} (changed)")),
            _ => {}
        }
    }
    for p in after.keys() {
        if !before.contains_key(p) {
            out.push(format!("{p} (created)"));
        }
    }
    out
}

/// The changes a read that OPENED the store may make: RocksDB's own files and
/// the records beside it. Anything else is a write.
fn beyond_bookkeeping(changed: Vec<String>) -> Vec<String> {
    changed
        .into_iter()
        .filter(|c| {
            let path = c.rsplit_once(" (").map(|(p, _)| p).unwrap_or(c);
            let Some(rel) = path.strip_prefix("folder/") else {
                return true;
            };
            !(rel.starts_with(".reflow2/graph/") || BOOKKEEPING.contains(&rel))
        })
        .collect()
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

fn stdout_json(o: &Output) -> serde_json::Value {
    let text = String::from_utf8_lossy(&o.stdout);
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}):\n{text}\nstderr:\n{}", err(o)))
}

/// The spec for each served tool, taken from what the binary SERVES — its
/// `read_only_hint` — plus item 1's file rule. Kept here as well as in the
/// binary on purpose: the test must not ask the code under test what to expect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Expect {
    /// Annotated read-only, and writes no file.
    Reads,
    /// Annotated read-only, and writes a file at `path` when given one.
    ReadsWithoutAPath,
    /// Everything else.
    Writes,
}

fn expected(t: &rmcp::model::Tool) -> Expect {
    let reads = t.annotations.as_ref().and_then(|a| a.read_only_hint) == Some(true);
    if !reads {
        Expect::Writes
    } else if FILE_WRITING_TOOLS.contains(&t.name.as_ref()) {
        Expect::ReadsWithoutAPath
    } else {
        Expect::Reads
    }
}

/// Arguments for every REQUIRED parameter, so a read's body actually runs
/// rather than stopping at the argument check: a string is the seeded
/// project's id (or an enum's first value), a number 1, a list empty.
fn required_args(t: &rmcp::model::Tool) -> String {
    let schema = &t.input_schema;
    let props = schema.get("properties").and_then(|p| p.as_object());
    let mut args = serde_json::Map::new();
    for name in schema
        .get("required")
        .and_then(|r| r.as_array())
        .into_iter()
        .flatten()
        .filter_map(|n| n.as_str())
    {
        let prop = props.and_then(|p| p.get(name)).cloned().unwrap_or_default();
        let ty = match prop.get("type") {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Array(a)) => a
                .iter()
                .filter_map(|v| v.as_str())
                .find(|s| *s != "null")
                .unwrap_or("null")
                .to_string(),
            _ => String::new(),
        };
        let value = if let Some(first) = prop
            .get("enum")
            .and_then(|e| e.as_array())
            .and_then(|e| e.first())
        {
            first.clone()
        } else {
            match ty.as_str() {
                "string" => serde_json::json!("proj:seed"),
                "integer" | "number" => serde_json::json!(1),
                "boolean" => serde_json::json!(false),
                "array" => serde_json::json!([]),
                "object" => serde_json::json!({}),
                _ => continue,
            }
        };
        args.insert(name.to_string(), value);
    }
    serde_json::Value::Object(args).to_string()
}

/// THE CLASS, walked: every tool this build serves, through `read`, on a
/// design. A read runs and changes nothing but bookkeeping; anything else is
/// refused by name — naming `reflow2 write` — before anything is opened, so
/// the folder is byte-identical, store included. And every read also runs
/// through plain `--call`, unguarded, and changes nothing there either: the
/// annotation that decides what a terminal approves is true, not only
/// enforced.
#[test]
fn every_served_tool_through_read_runs_if_it_changes_nothing_and_is_refused_otherwise() {
    let s = Scratch::seeded();
    let design_before = s.design();
    let tools = ReflowService::served_tools();
    assert!(
        tools.len() >= 190,
        "only {} tools served; the walk would be vacuous",
        tools.len()
    );

    let (mut refused, mut ran, mut answered) = (0, 0, 0);
    let mut failures = Vec::new();
    for t in &tools {
        let name = t.name.to_string();
        let args = required_args(t);
        let before = s.everything();
        let o = s.run(&["read", &name, &args], None);
        let after = s.everything();
        let e = err(&o);
        let what = format!("`read {name} {args}` (exit {:?})", o.status.code());
        match expected(t) {
            Expect::Writes => {
                refused += 1;
                if o.status.code() != Some(1) {
                    failures.push(format!("{what} was not refused with exit 1: {e}"));
                }
                if !(e.contains(READ_REFUSES) && e.contains(&format!("reflow2 write {name}"))) {
                    failures.push(format!(
                        "{what}: the refusal does not name `reflow2 write {name}`: {e}"
                    ));
                }
                if !o.stdout.is_empty() {
                    failures.push(format!("{what} printed a reply"));
                }
                let changed = changes(&before, &after);
                if !changed.is_empty() {
                    failures.push(format!(
                        "{what} was refused but OPENED or changed something: {changed:?}"
                    ));
                }
            }
            Expect::Reads | Expect::ReadsWithoutAPath => {
                ran += 1;
                if e.contains(READ_REFUSES) {
                    failures.push(format!("{what}: a read was refused by the verb: {e}"));
                }
                // A read-annotated tool that reached for the write guard is a
                // tool whose annotation and body disagree.
                if e.contains("READ-ONLY") {
                    failures.push(format!("{what} tried to write: {e}"));
                }
                if o.status.code() == Some(0) {
                    answered += 1;
                }
                let moved = beyond_bookkeeping(changes(&before, &after));
                if !moved.is_empty() {
                    failures.push(format!("{what} CHANGED {moved:?}"));
                }

                // ⭐ THE ANNOTATION ITSELF IS TRUE, not only enforced: `read`
                // runs a tool read-only, which would turn a lying annotation
                // into a refusal and hide it. So the same call goes through
                // plain `--call`, with nothing guarding it, and must change
                // nothing either — the check the requirement asks for, because
                // the annotation now decides what a terminal approves.
                let before = s.everything();
                let o = s.run(&["--call", &name, "--args", &args], None);
                let e = err(&o);
                let what = format!("`--call {name} {args}` (exit {:?})", o.status.code());
                if e.contains("annotation and its body disagree") {
                    failures.push(format!("{what} tried to write the design: {e}"));
                }
                let moved = beyond_bookkeeping(changes(&before, &s.everything()));
                if !moved.is_empty() {
                    failures.push(format!(
                        "{what} is annotated read-only and, run without --read-only, CHANGED \
                         {moved:?}"
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} tools failed:\n{}",
        failures.len(),
        tools.len(),
        failures.join("\n")
    );
    assert_eq!(s.design(), design_before, "the design changed under `read`");
    // Floors, so a walk that ran nothing cannot pass.
    assert!(refused >= 100, "only {refused} tools were refused");
    assert!(ran >= 70, "only {ran} reads ran");
    assert!(
        answered >= 50,
        "only {answered} of {ran} reads answered; the walk is not reaching their bodies"
    );
}

/// Where there is no design, `read` creates none — whether the tool reads or
/// writes, and even where the folder has opted in.
#[test]
fn read_creates_nothing_where_there_is_no_design() {
    for opted_in in [false, true] {
        let s = Scratch::new();
        if opted_in {
            std::fs::create_dir_all(s.folder().join(".reflow2")).unwrap();
        }
        for (tool, args) in [("loop_status", "{}"), ("add_requirement", REQ)] {
            let before = s.everything();
            let o = s.run(&["read", tool, args], None);
            assert_eq!(o.status.code(), Some(1), "{tool}: {}", err(&o));
            assert_eq!(
                changes(&before, &s.everything()),
                Vec::<String>::new(),
                "`read {tool}` created something (opted in: {opted_in})"
            );
        }
    }
}

/// Item 1's file rule, through the verb: `export_graph` and `export_surface`
/// only read without `path`; with one they write a file, so `read` refuses them
/// before anything is opened, and `write` runs them.
#[test]
fn export_with_a_path_through_read_is_refused_and_through_write_is_written() {
    let s = Scratch::seeded();
    let kept = s.outside("kept.json");
    std::fs::write(&kept, "keep me").unwrap();
    for tool in FILE_WRITING_TOOLS {
        for args in [
            serde_json::json!({ "path": s.outside(&format!("{tool}.json")) }),
            serde_json::json!({ "path": kept, "overwrite": true }),
        ] {
            let before = s.everything();
            let o = s.run(&["read", tool, &args.to_string()], None);
            let e = err(&o);
            assert_eq!(o.status.code(), Some(1), "{tool} {args}: {e}");
            assert!(e.contains(READ_REFUSES), "{tool}: {e}");
            assert!(e.contains("path"), "the refusal names the argument: {e}");
            assert!(e.contains(&format!("reflow2 write {tool}")), "{e}");
            assert_eq!(
                changes(&before, &s.everything()),
                Vec::<String>::new(),
                "`read {tool} {args}` opened or wrote something"
            );
        }
        assert!(!s.outside(&format!("{tool}.json")).exists());
        assert_eq!(std::fs::read_to_string(&kept).unwrap(), "keep me");

        // Without a path the document is the reply: a read, and it runs.
        let o = s.run(&["read", tool], None);
        assert_eq!(o.status.code(), Some(0), "{tool}: {}", err(&o));
        // `"path": null` is no path either.
        let o = s.run(&["read", tool, r#"{"path":null}"#], None);
        assert_eq!(o.status.code(), Some(0), "{tool} path null: {}", err(&o));
    }
    let o = s.run(&["read", "export_graph"], None);
    assert!(stdout_json(&o)["nodes"].is_array());

    // `write` runs it, and the file lands.
    let target = s.outside("written.json");
    let o = s.run(
        &[
            "write",
            "export_graph",
            &serde_json::json!({ "path": target }).to_string(),
        ],
        None,
    );
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&target).unwrap()).unwrap();
    assert!(doc["nodes"].is_array());
}

/// `write` runs any tool — a write lands, and a read answers through it too —
/// and takes its arguments as a positional object, as `--args`, or from stdin.
#[test]
fn write_runs_a_write_and_any_other_tool() {
    let s = Scratch::seeded();
    let o = s.run(&["write", "add_requirement", REQ], None);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    assert_eq!(stdout_json(&o)["node_id"], "req:verb-probe");
    assert_eq!(
        s.node("req:verb-probe")["properties"]["name"],
        "Verb probe",
        "the write landed"
    );

    // --args, and `-` from stdin — the quoted-heredoc route.
    let o = s.run(
        &[
            "write",
            "add_requirement",
            "--args",
            r#"{"id":"req:verb-probe-2","name":"Two","statement":"Two shall exist."}"#,
        ],
        None,
    );
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    let o = s.run(
        &["write", "add_requirement", "--args", "-"],
        Some(r#"{"id":"req:verb-probe-3","name":"Three","statement":"Three shall exist."}"#),
    );
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    assert!(!s.node("req:verb-probe-3").is_null());

    // A read through `write` is still answered: `write` asks, it does not narrow.
    let o = s.run(&["write", "get_node", r#"{"id":"req:verb-probe"}"#], None);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));

    // And `read` reads it back, from stdin as well.
    let o = s.run(
        &["read", "get_node", "-"],
        Some(r#"{"id":"req:verb-probe-2"}"#),
    );
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    assert_eq!(stdout_json(&o)["node"]["properties"]["name"], "Two");

    // A tool's own refusal is the door's, unchanged: not exit 0, and the
    // design is not written.
    let o = s.run(&["write", "add_project", r#"{"id":"proj:x"}"#], None);
    assert_ne!(o.status.code(), Some(0), "{}", err(&o));
    assert!(s.node("proj:x").is_null());
}

/// `--read-only` asks for the opposite of `write`, so the two are refused
/// together, by name, with nothing opened.
#[test]
fn write_with_read_only_is_refused() {
    let s = Scratch::seeded();
    let before = s.everything();
    let o = s.run(&["--read-only", "write", "add_requirement", REQ], None);
    let e = err(&o);
    assert_eq!(o.status.code(), Some(1), "{e}");
    assert!(e.contains("--read-only is not honoured by"), "{e}");
    assert_eq!(changes(&before, &s.everything()), Vec::<String>::new());
}

/// The read set is LISTED, from the served annotations plus the file rule —
/// every served tool exactly once — and listing opens nothing.
#[test]
fn the_read_set_is_listed_from_the_served_annotations_and_the_file_rule() {
    let s = Scratch::seeded();
    let before = s.everything();
    let o = s.run(&["read", "--list"], None);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    assert_eq!(
        changes(&before, &s.everything()),
        Vec::<String>::new(),
        "listing opened something"
    );
    let v = stdout_json(&o);
    let names = |key: &str| -> Vec<String> {
        v[key]["tools"]
            .as_array()
            .unwrap_or_else(|| panic!("no `{key}.tools` in {v}"))
            .iter()
            .map(|t| t.as_str().unwrap().to_string())
            .collect()
    };
    let (read, write) = (names("read"), names("write"));
    let tools = ReflowService::served_tools();
    for t in &tools {
        let name = t.name.to_string();
        let in_read = read.contains(&name);
        let in_write = write.contains(&name);
        match expected(t) {
            Expect::Reads | Expect::ReadsWithoutAPath => {
                assert!(in_read && !in_write, "{name} should be listed under read")
            }
            Expect::Writes => {
                assert!(in_write && !in_read, "{name} should be listed under write")
            }
        }
    }
    assert_eq!(read.len() + write.len(), tools.len(), "a tool listed twice");
    assert_eq!(v["read"]["count"], read.len());
    assert_eq!(v["write"]["count"], write.len());
    for tool in FILE_WRITING_TOOLS {
        assert_eq!(
            v["read"]["only_without"][tool], "path",
            "the listing says when {tool} stops being a read: {v}"
        );
    }
    // `write --list` is the same listing.
    let o = s.run(&["write", "--list"], None);
    assert_eq!(stdout_json(&o), v);
}

/// `read` and `write` keep logging quiet: a read that answers prints nothing on
/// stderr, and a refusal prints the refusal — no INFO line on every call, no
/// WARN line repeating the refusal.
#[test]
fn the_verbs_keep_logging_quiet() {
    let s = Scratch::seeded();
    let o = s.run(&["read", "graph_report"], None);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    assert_eq!(err(&o), "", "a read printed on stderr");
    let o = s.run(&["read", "get_node", r#"{"id":"req:seed"}"#], None);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    assert_eq!(err(&o), "", "a read printed on stderr");
    for argv in [
        vec!["read", "add_requirement", REQ],
        vec!["write", "add_project", r#"{"id":"proj:x"}"#],
        vec!["write", "add_requirement", REQ],
    ] {
        let o = s.run(&argv, None);
        let e = err(&o);
        for noise in [" INFO ", " WARN ", " DEBUG "] {
            assert!(
                !e.contains(noise),
                "`{}` logged{noise}: {e}",
                argv.join(" ")
            );
        }
    }
}

/// Every long flag `--help` lists, and the placeholder of its value if it
/// takes one — read from the BINARY, so a flag added tomorrow is walked
/// tomorrow (the same reading item 1's flag walk uses).
fn every_flag() -> Vec<(String, Option<String>)> {
    let o = Command::new(bin()).arg("--help").output().unwrap();
    let help = String::from_utf8_lossy(&o.stdout);
    let mut flags = Vec::new();
    for line in help.lines() {
        let indent = line.len() - line.trim_start().len();
        let t = line.trim_start();
        if indent > 6 || !t.starts_with('-') {
            continue;
        }
        let Some(flag) = t
            .split_whitespace()
            .map(|w| w.trim_end_matches(','))
            .find(|w| w.starts_with("--"))
        else {
            continue;
        };
        if flag == "--help" || flag == "--version" {
            continue;
        }
        let placeholder = t
            .split_once(flag)
            .and_then(|(_, rest)| rest.trim_start().strip_prefix('<'))
            .and_then(|r| r.split_once('>'))
            .map(|(p, _)| p.to_string());
        flags.push((flag.to_string(), placeholder));
    }
    flags
}

/// ⭐ WHY ONE RULE IS ENOUGH: `^reflow2 read ` approves whatever follows, so
/// nothing that can follow may make `read` change anything. Every flag the
/// binary lists, typed after `read` — beside a read and beside a writer — is
/// refused by the parser, refused by the one-shot table, or one `read`
/// honours; and in every case the design and the disk outside bookkeeping are
/// unchanged and the writer never runs.
#[test]
fn nothing_typed_after_read_can_make_it_change_anything() {
    let s = Scratch::seeded();
    let design_before = s.design();
    // Not vacuous: bare, the read runs and the writer is refused by the verb.
    let o = s.run(&["read", "graph_report"], None);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    let o = s.run(&["read", "add_requirement", REQ], None);
    assert_eq!(o.status.code(), Some(1), "{}", err(&o));
    assert!(err(&o).contains(READ_REFUSES), "{}", err(&o));
    let flags = every_flag();
    assert!(
        flags.len() >= 40,
        "only {} flags read from --help",
        flags.len()
    );
    let elsewhere = s.outside("elsewhere");
    let mut failures = Vec::new();
    for (flag, placeholder) in &flags {
        let value = placeholder.as_ref().map(|p| match p.as_str() {
            "N" | "MB" | "MINUTES" => "8".to_string(),
            "DURATION" => "5s".to_string(),
            "URL" => "http://127.0.0.1:9/".to_string(),
            "ADDR" => "127.0.0.1:0".to_string(),
            "JSON" => "{}".to_string(),
            _ => elsewhere.join("x").display().to_string(),
        });
        for head in [
            vec!["read".to_string(), "graph_report".to_string()],
            vec![
                "read".to_string(),
                "add_requirement".to_string(),
                REQ.to_string(),
            ],
        ] {
            let mut argv = head.clone();
            argv.push(flag.clone());
            argv.extend(value.clone());
            let before = s.everything();
            let o = s.run(&argv.iter().map(String::as_str).collect::<Vec<_>>(), None);
            let moved = beyond_bookkeeping(changes(&before, &s.everything()));
            let what = format!("`{}` (exit {:?})", argv.join(" "), o.status.code());
            if !moved.is_empty() {
                failures.push(format!("{what} CHANGED {moved:?}: {}", err(&o)));
            }
            if head[1] == "add_requirement" && o.status.code() == Some(0) {
                failures.push(format!("{what} RAN A WRITER: {}", err(&o)));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} command line(s) failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(s.design(), design_before, "the design changed under `read`");
    assert!(
        s.node("req:verb-probe").is_null(),
        "the writer ran through `read`"
    );
    // `--graph-path` is honoured after `read`; with this walk's value it names
    // a place with no design, which `read` refuses rather than create.
    assert!(!elsewhere.exists(), "a path given after `read` was created");
}

/// `--graph-path` may follow the verb, so a design elsewhere is readable
/// without leaving the rule's prefix.
#[test]
fn graph_path_may_follow_the_verb() {
    let s = Scratch::seeded();
    let other = Scratch::new();
    let graph = s.folder().join(".reflow2").join("graph");
    let o = other.run(
        &[
            "read",
            "get_node",
            r#"{"id":"proj:seed"}"#,
            "--graph-path",
            graph.to_str().unwrap(),
        ],
        None,
    );
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    assert_eq!(stdout_json(&o)["node"]["node_id"], "proj:seed");
    let o = other.run(
        &[
            "write",
            "add_requirement",
            REQ,
            "--graph-path",
            graph.to_str().unwrap(),
        ],
        None,
    );
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    assert!(!s.node("req:verb-probe").is_null());
    assert!(
        !other.folder().join(".reflow2").exists(),
        "the verb opened its own folder instead of --graph-path"
    );
}
