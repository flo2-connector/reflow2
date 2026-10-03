//! Every one-shot mode resolves its design before it opens one, and every flag
//! is honoured or refused — through the real binary, in real folders.
//!
//! `req:a-one-shot-call-never-creates-a-design-where-a-folder-names-one-on-a-server`,
//! step 1 of `epoch:planned-the-call-door-works-for-an-agent-that-cannot-use-mcp`.
//!
//! WHAT FAILED, measured on 0.77.0 (2026-10-02), each pinned below:
//! · in a folder whose `.reflow2.toml` names a design on a server, `--call`
//!   (even `find_tools` and `get_skill`, which read no design) and `--export`
//!   minted a fresh local store, exit 0, silent, and `loop_status` then read the
//!   stray design as `clean: true`
//!   (fact:root-cause-one-shot-modes-return-before-the-pointer-check-and-opening-a-store-creates-it-2026-10-02);
//! · `--read-only --call add_requirement` wrote the node, exit 0
//!   (fact:call-ignores-read-only-and-the-write-lands-2026-10-02);
//! · `--export-to FILE --call <writer>` never wrote FILE, exit 0
//!   (fact:root-cause-call-accepts-export-to-and-never-reads-it-2026-10-02) —
//!   refused by name in step 1, and HONOURED since step 2 of the door plan
//!   (`a_writing_call_keeps_the_committed_export_current.rs`);
//! · `--remote URL --call X` dropped the call and ran a proxy on nothing, exit 0.
//!
//! THE CLASS: which rules a run obeyed was decided by where its branch sat in
//! main()'s early returns, and the store opener creates on open. So the tests
//! assert the class, not the instances: every mode that opens the store, in a
//! pointer folder and in an empty one, for reads and writes; and every flag the
//! binary's own `--help` lists, against every one-shot mode.
//!
//! Each test was run against main (9ecb405) before the fix and failed there.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

const ID: &str = "abc123def4567890";
/// Port 9 (discard): nothing listens, and nothing here may try to reach it.
const ADDRESS: &str = "http://127.0.0.1:9/g/abc123def4567890/mcp";
const REQ: &str = r#"{"id":"req:door-probe","name":"Door probe","statement":"written through a door that should have refused"}"#;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

/// A folder the binary runs in, and a second directory OUTSIDE it for HOME,
/// configuration and input files — so "the folder is unchanged" can be read
/// off a listing of the folder alone.
struct Scratch {
    folder: tempfile::TempDir,
    outside: tempfile::TempDir,
}

impl Scratch {
    fn new() -> Scratch {
        Scratch {
            folder: tempfile::Builder::new()
                .prefix("reflow2-door-folder-")
                .tempdir()
                .unwrap(),
            outside: tempfile::Builder::new()
                .prefix("reflow2-door-outside-")
                .tempdir()
                .unwrap(),
        }
    }

    /// A folder whose `.reflow2.toml` names a design on a server.
    fn pointer() -> Scratch {
        let s = Scratch::new();
        s.write_pointer();
        s
    }

    fn write_pointer(&self) {
        std::fs::write(
            self.folder.path().join(".reflow2.toml"),
            format!("[design]\nid = \"{ID}\"\naddress = \"{ADDRESS}\"\n"),
        )
        .unwrap();
    }

    /// A folder that has opted in: `.reflow2/` is there and empty.
    fn opted_in() -> Scratch {
        let s = Scratch::new();
        std::fs::create_dir_all(s.folder.path().join(".reflow2")).unwrap();
        s
    }

    /// A folder holding a design with one project in it.
    fn seeded() -> Scratch {
        let s = Scratch::new();
        let o = s.run(&[
            "--call",
            "add_project",
            "--args",
            r#"{"id":"proj:seed","name":"Seed"}"#,
        ]);
        assert_eq!(o.status.code(), Some(0), "seed: {}", err(&o));
        s
    }

    fn out(&self, name: &str) -> PathBuf {
        self.outside.path().join(name)
    }

    fn out_str(&self, name: &str) -> String {
        self.out(name).to_str().unwrap().to_string()
    }

    /// Run the binary in the folder with an isolated environment and a
    /// deadline. A flag that was silently ignored can leave a SERVER running;
    /// that must fail the test, not hang it.
    fn run(&self, args: &[&str]) -> Output {
        let home = self.outside.path().join("home");
        std::fs::create_dir_all(&home).unwrap();
        let mut child = Command::new(bin())
            .current_dir(self.folder.path())
            .args(args)
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", home.join(".config"))
            .env("REFLOW2_CONFIG_DIR", home.join("reflow2-config"))
            .env("RUST_LOG", "error")
            .env_remove("REFLOW2_CONTENT_POLICY")
            .env_remove("REFLOW2_TRUSTED_GATEWAY")
            .env_remove("REFLOW2_OIDC_ISSUER")
            .env_remove("REFLOW2_PUBLIC_URL")
            .env_remove("REFLOW2_OIDC_AUDIENCE")
            .env_remove("REFLOW2_OIDC_JWKS_URI")
            .env_remove("REFLOW2_OIDC_JWKS_FILE")
            .env_remove("REFLOW2_CONTRIBUTOR_ID")
            .env_remove("REFLOW2_CONTRIBUTOR_MAP")
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
            if start.elapsed() > Duration::from_secs(90) {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "`reflow2-mcp {}` was still running after 90 s: a flag was ignored and \
                     something is serving instead of refusing",
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

    fn store(&self) -> PathBuf {
        self.folder.path().join(".reflow2").join("graph")
    }
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

fn stdout_json(o: &Output) -> serde_json::Value {
    let text = String::from_utf8_lossy(&o.stdout);
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}):\n{text}\nstderr:\n{}", err(o)))
}

/// A real export document, written outside the folder under test.
fn export_doc(dest: &Path) {
    let seed = Scratch::seeded();
    let o = seed.run(&["--export"]);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    std::fs::write(dest, &o.stdout).unwrap();
}

fn node_is_absent(s: &Scratch, id: &str) {
    let o = s.run(&[
        "--call",
        "get_node",
        "--args",
        &format!(r#"{{"id":"{id}"}}"#),
    ]);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    assert!(
        stdout_json(&o)["node"].is_null(),
        "{id} was written although the door refused: {}",
        String::from_utf8_lossy(&o.stdout)
    );
}

// ---- A. "where is this design?" ---------------------------------------------

/// Every mode that opens the store, for a tool that reads, a tool that reads
/// no design at all, and a tool that writes: refused, exit 1, the design's id
/// and address named, and the folder exactly as it was.
#[test]
fn in_a_folder_that_names_its_design_on_a_server_every_mode_that_opens_the_store_refuses_and_creates_nothing()
 {
    let s = Scratch::pointer();
    let doc = s.out_str("doc.json");
    export_doc(&s.out("doc.json"));
    let import_args = format!(r#"{{"path":"{doc}"}}"#);
    let before = s.listing();
    assert_eq!(before, vec![".reflow2.toml".to_string()]);

    let cases: Vec<Vec<&str>> = vec![
        vec!["--export"],
        vec!["--export-snapshot"],
        vec!["--import", &doc],
        vec!["--diff", &doc],
        vec![
            "--call",
            "find_tools",
            "--args",
            r#"{"query":"where am i"}"#,
        ],
        vec!["--call", "get_skill", "--args", r#"{"name":"where-am-i"}"#],
        vec!["--call", "loop_status"],
        vec!["--call", "design_identity"],
        vec!["--call", "add_requirement", "--args", REQ],
        vec!["--call", "import_graph", "--args", &import_args],
        vec!["--only-if-present", "--call", "design_identity"],
        vec!["--graph-path", ".reflow2/graph", "--call", "graph_report"],
        // The lessons a described tool carries are the named design's, which
        // this door cannot reach (step 4 of the door plan).
        vec!["--describe", "get_node"],
        vec!["--list-tools", "--full"],
    ];
    for args in &cases {
        let o = s.run(args);
        let e = err(&o);
        assert_eq!(o.status.code(), Some(1), "{args:?} must refuse: {e}");
        assert!(e.contains(ID), "{args:?} must name the design: {e}");
        assert!(e.contains(ADDRESS), "{args:?} must name its address: {e}");
        assert!(o.stdout.is_empty(), "{args:?} printed a reply");
        assert_eq!(
            s.listing(),
            before,
            "{args:?} changed the folder — a stray design is the defect"
        );
    }

    // The pointer governs the LOCAL STORE. A file-pure mode never opens it,
    // so it still works here.
    let o = s.run(&["--diff", &doc, &doc]);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    assert_eq!(s.listing(), before);
}

/// A store left behind by a move, beside the pointer: neither read nor
/// written, and the refusal says the store is not the design.
#[test]
fn a_store_left_beside_a_pointer_is_neither_opened_nor_written() {
    let s = Scratch::seeded();
    s.write_pointer();
    let before = s.listing();

    let o = s.run(&["--call", "get_node", "--args", r#"{"id":"proj:seed"}"#]);
    assert_eq!(o.status.code(), Some(1), "{}", err(&o));
    let e = err(&o);
    assert!(e.contains(ID) && e.contains("NOT opened"), "{e}");

    let o = s.run(&["--call", "add_requirement", "--args", REQ]);
    assert_eq!(o.status.code(), Some(1), "{}", err(&o));
    assert_eq!(s.listing(), before, "nothing new beside the left store");

    std::fs::remove_file(s.folder.path().join(".reflow2.toml")).unwrap();
    node_is_absent(&s, "req:door-probe");
}

/// Where there is no design, a READ refuses and creates nothing — including
/// the discovery calls an agent is told to run first — and a WRITE creates it.
#[test]
fn where_there_is_no_design_a_read_refuses_and_creates_nothing_and_a_write_creates_it() {
    let s = Scratch::new();
    let doc = s.out_str("doc.json");
    export_doc(&s.out("doc.json"));
    for args in [
        vec!["--export"],
        vec!["--export-snapshot"],
        vec!["--diff", &doc],
        vec![
            "--call",
            "find_tools",
            "--args",
            r#"{"query":"where am i"}"#,
        ],
        vec!["--call", "get_skill", "--args", r#"{"name":"where-am-i"}"#],
        vec!["--call", "loop_status"],
        vec!["--call", "export_graph"],
    ] {
        let o = s.run(&args);
        let e = err(&o);
        assert_eq!(o.status.code(), Some(1), "{args:?} must refuse: {e}");
        assert!(e.contains("no design"), "{args:?}: {e}");
        assert!(
            s.listing().is_empty(),
            "{args:?} created {:?} — a read must never create a design",
            s.listing()
        );
    }

    let o = s.run(&[
        "--call",
        "add_project",
        "--args",
        r#"{"id":"proj:new","name":"New"}"#,
    ]);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    assert!(
        s.store().exists(),
        "a write puts a design where it was asked to"
    );
    let o = s.run(&["--call", "loop_status"]);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));

    let other = Scratch::new();
    let o = other.run(&["--import", &doc]);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    assert!(other.store().exists(), "an import restores a design");
}

/// A folder that opted in (`.reflow2/` there, empty) holds an empty design,
/// and reading it is how the installer mints the design's id.
#[test]
fn an_opted_in_folder_reads_as_the_empty_design_it_is() {
    let s = Scratch::opted_in();
    let o = s.run(&["--export"]);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    let v = stdout_json(&o);
    assert!(v["graph_id"].is_string(), "{v}");
    assert_eq!(v["nodes"].as_array().map(Vec::len), Some(0), "{v}");
}

/// `--only-if-present` means for a one-shot mode what it means for a session:
/// where nobody opted in, nothing is created.
#[test]
fn only_if_present_is_honoured_by_one_shot_modes() {
    let s = Scratch::new();
    let doc = s.out_str("doc.json");
    export_doc(&s.out("doc.json"));
    for args in [
        vec![
            "--only-if-present",
            "--call",
            "add_project",
            "--args",
            r#"{"id":"proj:x","name":"X"}"#,
        ],
        vec!["--only-if-present", "--import", &doc],
        vec!["--only-if-present", "--export"],
    ] {
        let o = s.run(&args);
        let e = err(&o);
        assert_eq!(o.status.code(), Some(1), "{args:?}: {e}");
        assert!(e.contains("--only-if-present"), "{args:?}: {e}");
        assert!(s.listing().is_empty(), "{args:?} created {:?}", s.listing());
    }

    let s = Scratch::opted_in();
    let o = s.run(&[
        "--only-if-present",
        "--call",
        "add_project",
        "--args",
        r#"{"id":"proj:x","name":"X"}"#,
    ]);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
}

// ---- B. every flag honoured or refused ----------------------------------------

/// `--read-only` beside `--call` changes nothing, graph or disk: a writer is
/// refused, and so is a tool marked read-only that would write a FILE.
#[test]
fn read_only_with_call_refuses_a_graph_write_and_a_file_write_and_still_answers_reads() {
    let s = Scratch::seeded();

    let o = s.run(&["--read-only", "--call", "add_requirement", "--args", REQ]);
    assert_eq!(o.status.code(), Some(1), "{}", err(&o));
    assert!(err(&o).contains("--read-only"), "{}", err(&o));
    node_is_absent(&s, "req:door-probe");

    let graph_out = s.out_str("graph.json");
    let surface_out = s.out_str("surface.json");
    for (tool, path) in [
        ("export_graph", &graph_out),
        ("export_surface", &surface_out),
    ] {
        let o = s.run(&[
            "--read-only",
            "--call",
            tool,
            "--args",
            &format!(r#"{{"path":"{path}"}}"#),
        ]);
        assert_eq!(o.status.code(), Some(1), "{tool}: {}", err(&o));
        assert!(
            err(&o).to_lowercase().contains("read-only"),
            "{tool}: {}",
            err(&o)
        );
        assert!(!Path::new(path).exists(), "{tool} wrote {path}");
    }

    // An existing file, with overwrite asked for: still not touched.
    let kept = s.out("kept.json");
    std::fs::write(&kept, "keep me").unwrap();
    let o = s.run(&[
        "--read-only",
        "--call",
        "export_graph",
        "--args",
        &format!(
            r#"{{"path":"{}","overwrite":true}}"#,
            kept.to_str().unwrap()
        ),
    ]);
    assert_eq!(o.status.code(), Some(1), "{}", err(&o));
    assert_eq!(std::fs::read_to_string(&kept).unwrap(), "keep me");

    // Reads still answer — including the export, in the reply.
    let o = s.run(&["--read-only", "--call", "export_graph"]);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    assert!(stdout_json(&o)["nodes"].is_array());
    let o = s.run(&["--read-only", "--call", "graph_report"]);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    let o = s.run(&["--read-only", "--export"]);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));

    // The guard is --read-only's, not everyone's: without it the file lands.
    let o = s.run(&[
        "--call",
        "export_graph",
        "--args",
        &format!(r#"{{"path":"{graph_out}"}}"#),
    ]);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    assert!(Path::new(&graph_out).exists());

    // And --read-only creates nothing, even where a folder opted in.
    for s in [Scratch::new(), Scratch::opted_in()] {
        let before = s.listing();
        let o = s.run(&["--read-only", "--call", "loop_status"]);
        assert_eq!(o.status.code(), Some(1), "{}", err(&o));
        assert_eq!(s.listing(), before);
    }

    // --import writes, so the two contradict each other.
    let o = s.run(&["--read-only", "--import", &graph_out]);
    assert_eq!(o.status.code(), Some(1), "{}", err(&o));
    assert!(
        err(&o).contains("--read-only is not honoured by --import"),
        "{}",
        err(&o)
    );
}

/// `--export-to` beside `--call` was refused by name in step 1, and step 2 of
/// the door plan HONOURS it: a writing call writes the file before it exits —
/// never accepted and quietly not written.
#[test]
fn export_to_with_call_is_honoured_and_a_writing_call_writes_the_file() {
    let s = Scratch::seeded();
    let target = s.out_str("design.json");
    let o = s.run(&[
        "--export-to",
        &target,
        "--call",
        "add_requirement",
        "--args",
        REQ,
    ]);
    let e = err(&o);
    assert_eq!(o.status.code(), Some(0), "{e}");
    assert!(!e.contains("not honoured"), "{e}");
    let written = std::fs::read_to_string(&target).expect("the named export was written");
    assert!(written.contains("req:door-probe"), "{e}");
}

/// `--remote` beside a one-shot mode is refused, never dropped for a proxy
/// that answers nothing with exit 0.
#[test]
fn remote_with_a_one_shot_mode_is_refused_rather_than_dropped() {
    let s = Scratch::new();
    let o = s.run(&[
        "--remote",
        "http://127.0.0.1:9/",
        "--call",
        "get_node",
        "--args",
        r#"{"id":"proj:x"}"#,
    ]);
    let e = err(&o);
    assert_eq!(o.status.code(), Some(1), "{e}");
    assert!(e.contains("--remote is not honoured by --call"), "{e}");
    assert!(
        e.contains("dec:idea-a-one-shot-call-reaches-the-design-where-it-is-served"),
        "{e}"
    );
    assert!(o.stdout.is_empty());

    let o = s.run(&["--remote", "http://127.0.0.1:9/", "--export"]);
    assert_eq!(o.status.code(), Some(1), "{}", err(&o));
    assert!(s.listing().is_empty());
}

/// What each one-shot mode reads, as the spec: kept here as well as in the
/// binary on purpose, so widening a mode's row is a reviewed change in two
/// places rather than a silent one in one.
fn honoured(mode: &str) -> &'static [&'static str] {
    match mode {
        "setup" | "--merge-driver" => &[],
        "--export" | "--export-snapshot" | "--diff BASE" => &[
            "--graph-path",
            "--store-memory",
            "--only-if-present",
            "--read-only",
        ],
        "--import" => &[
            "--graph-path",
            "--store-memory",
            "--only-if-present",
            "--accept-newer",
        ],
        "--call" => &[
            "--graph-path",
            "--store-memory",
            "--only-if-present",
            "--read-only",
            "--tree-root",
            "--args",
            "--export-to",
            "--no-export",
        ],
        "--diff BASE OTHER" | "--merge" => &["--read-only"],
        "--merge-apply" => &["--read-only", "--resolutions"],
        "--stop-shared" => &["--graph-path"],
        // Step 4 of the door plan: they read the design's lessons and change
        // nothing; `--full` asks for the tools/list entries unaltered.
        "--describe" | "--list-tools" => &[
            "--graph-path",
            "--store-memory",
            "--only-if-present",
            "--read-only",
            "--full",
        ],
        other => panic!("no row for {other}"),
    }
}

/// The arguments that select a mode, with throwaway values.
fn mode_argv(mode: &str) -> Vec<&'static str> {
    match mode {
        "setup" => vec!["setup"],
        "--export" => vec!["--export"],
        "--export-snapshot" => vec!["--export-snapshot"],
        "--import" => vec!["--import", "missing.json"],
        "--diff BASE" => vec!["--diff", "missing.json"],
        "--diff BASE OTHER" => vec!["--diff", "a.json", "b.json"],
        "--merge" => vec!["--merge", "a.json", "b.json", "c.json"],
        "--merge-apply" => vec!["--merge-apply", "a.json", "b.json", "c.json"],
        "--merge-driver" => vec!["--merge-driver", "a.json", "b.json", "c.json"],
        "--call" => vec!["--call", "graph_report"],
        "--stop-shared" => vec!["--stop-shared"],
        "--describe" => vec!["--describe", "get_node"],
        "--list-tools" => vec!["--list-tools"],
        other => panic!("no argv for {other}"),
    }
}

const MODES: [&str; 13] = [
    "setup",
    "--export",
    "--export-snapshot",
    "--import",
    "--diff BASE",
    "--diff BASE OTHER",
    "--merge",
    "--merge-apply",
    "--merge-driver",
    "--call",
    "--stop-shared",
    "--describe",
    "--list-tools",
];

/// The mode a flag selects, when it selects one.
fn mode_of_flag(flag: &str) -> Option<&'static str> {
    match flag {
        "--export" => Some("--export"),
        "--export-snapshot" => Some("--export-snapshot"),
        "--import" => Some("--import"),
        // Two paths, so the flag's own values are complete and clap cannot
        // read the next word (`setup`, say) as a second path.
        "--diff" => Some("--diff BASE OTHER"),
        "--merge" => Some("--merge"),
        "--merge-apply" => Some("--merge-apply"),
        "--merge-driver" => Some("--merge-driver"),
        "--call" => Some("--call"),
        "--stop-shared" => Some("--stop-shared"),
        "--describe" => Some("--describe"),
        "--list-tools" => Some("--list-tools"),
        _ => None,
    }
}

/// Every long flag `--help` lists, and the placeholder of its value if it
/// takes one. Read from the BINARY, so a flag added tomorrow is walked
/// tomorrow without anybody editing this test.
fn every_flag() -> Vec<(String, Option<String>)> {
    let o = Command::new(bin()).arg("--help").output().unwrap();
    let help = String::from_utf8_lossy(&o.stdout);
    let mut flags = Vec::new();
    for line in help.lines() {
        let indent = line.len() - line.trim_start().len();
        let t = line.trim_start();
        // Option lines sit at 2 or 6 spaces; their descriptions sit deeper.
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

/// A value clap will accept for a flag, chosen from its placeholder.
fn sample(flag: &str, placeholder: &str) -> &'static str {
    match flag {
        "--content-policy" => "signpost",
        "--http-oidc-required-claim" => "groups=x",
        "--http-contributor-id" => "{sub}",
        "--args" => "{}",
        _ => match placeholder {
            "N" | "MB" | "MINUTES" => "8",
            "DURATION" => "5s",
            "URL" => "http://127.0.0.1:9/",
            "ADDR" => "127.0.0.1:0",
            "JSON" => "{}",
            _ => "x",
        },
    }
}

fn argv_of_flag(flag: &str, placeholder: Option<&str>) -> Vec<String> {
    if let Some(mode) = mode_of_flag(flag) {
        return mode_argv(mode).into_iter().map(str::to_string).collect();
    }
    let mut v = vec![flag.to_string()];
    if let Some(p) = placeholder {
        v.push(sample(flag, p).to_string());
    }
    v
}

/// The flags clap says are missing, as `(flag, placeholder)`.
fn missing_requirements(stderr: &str) -> Vec<(String, Option<String>)> {
    let Some((_, rest)) = stderr.split_once("required arguments were not provided:") else {
        return Vec::new();
    };
    rest.lines()
        .map(str::trim)
        .filter(|l| l.starts_with("--"))
        .map(|l| {
            let mut parts = l.splitn(2, ' ');
            let flag = parts.next().unwrap().to_string();
            let ph = parts
                .next()
                .and_then(|r| r.trim().strip_prefix('<'))
                .and_then(|r| r.split_once('>'))
                .map(|(p, _)| p.to_string());
            (flag, ph)
        })
        .collect()
}

/// THE TABLE, walked: every flag the binary lists, against every one-shot
/// mode. Each combination is either honoured (the mode reads it) or refused
/// by name — never accepted and ignored. A refusal leaves the folder empty and
/// the process gone.
#[test]
fn every_flag_is_honoured_or_refused_by_every_one_shot_mode() {
    let flags = every_flag();
    let names: Vec<&str> = flags.iter().map(|(f, _)| f.as_str()).collect();
    for must in [
        "--read-only",
        "--export-to",
        "--remote",
        "--shared",
        "--only-if-present",
        "--graph-path",
        "--http",
    ] {
        assert!(
            names.contains(&must),
            "--help no longer lists {must}; the walk would be vacuous: {names:?}"
        );
    }
    assert!(
        names.len() >= 40,
        "only {} flags read from --help",
        names.len()
    );

    let mut refused_by_the_table = 0;
    let mut failures = Vec::new();
    for mode in MODES {
        for (flag, placeholder) in &flags {
            if mode_of_flag(flag) == Some(mode) || (mode == "--diff BASE" && flag == "--diff") {
                continue;
            }
            let s = Scratch::new();
            let mut argv = argv_of_flag(flag, placeholder.as_deref());
            argv.extend(mode_argv(mode).into_iter().map(str::to_string));
            let mut o = s.run(&argv.iter().map(String::as_str).collect::<Vec<_>>());
            // A flag that needs another (clap's `requires`, perhaps twice
            // over) is given it, so the combination reaches the table rather
            // than stopping at clap.
            let mut companions: Vec<String> = Vec::new();
            for _ in 0..3 {
                if o.status.code() != Some(2) {
                    break;
                }
                let missing = missing_requirements(&err(&o));
                if missing.is_empty() {
                    break;
                }
                let mut again = Vec::new();
                for (f, ph) in &missing {
                    companions.push(f.clone());
                    again.extend(argv_of_flag(f, ph.as_deref()));
                }
                again.extend(argv.iter().cloned());
                argv = again;
                o = s.run(&argv.iter().map(String::as_str).collect::<Vec<_>>());
            }
            let companion_is_a_mode = companions.iter().any(|c| mode_of_flag(c).is_some());
            let e = err(&o);
            let refused_here = e.contains(&format!("{flag} is not honoured by {mode}:"));
            let two_modes = e.contains("a mode of their own");
            let clap_conflict = o.status.code() == Some(2) && e.contains("cannot be used with");
            let what = format!("`{}` (flag {flag}, mode {mode})", argv.join(" "));

            if !s.listing().is_empty() {
                failures.push(format!("{what} left {:?} in the folder", s.listing()));
            }
            if honoured(mode).contains(&flag.as_str()) {
                if refused_here || two_modes {
                    failures.push(format!("{what} is in the mode's row but was refused: {e}"));
                }
                continue;
            }
            if mode_of_flag(flag).is_some() || companion_is_a_mode {
                if !(o.status.code() == Some(1) && two_modes) {
                    failures.push(format!(
                        "{what} names two modes and was not refused as such (exit {:?}): {e}",
                        o.status.code()
                    ));
                }
                continue;
            }
            if clap_conflict
                && (e.contains(flag.as_str()) || companions.iter().any(|c| e.contains(c.as_str())))
            {
                // The parser itself refused a conflict it declares.
                continue;
            }
            if o.status.code() == Some(1) && refused_here && e.contains(mode) {
                refused_by_the_table += 1;
                continue;
            }
            failures.push(format!(
                "{what} was neither honoured nor refused by name (exit {:?}): {e}",
                o.status.code()
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} combination(s) failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(
        refused_by_the_table > 300,
        "only {refused_by_the_table} combinations reached the table's refusal"
    );
}
