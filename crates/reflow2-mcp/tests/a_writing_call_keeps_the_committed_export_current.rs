//! A `--call` that writes keeps the project's committed design export current,
//! before the process exits — through the real binary, in real folders.
//!
//! `req:a-writing-call-keeps-the-committed-export-current`, step 2 of
//! `epoch:planned-the-call-door-works-for-an-agent-that-cannot-use-mcp`
//! (`dec:idea-a-writing-call-exports-afterwards`, settled).
//!
//! WHAT FAILED, measured on main c4e1cbd (2026-10-03): in a folder whose
//! `.mcp.json` names `--export-to ./docs/design/proj.json` for its design, a
//! `--call add_requirement` exited 0 with stderr empty and left the export's
//! bytes unchanged; `compare_designs` against it read `identical: false`. The
//! write-through that keeps that file current is a debounced background task of
//! a long-lived server, the door never started one, and the door's exit would
//! have killed it (`fact:root-cause-call-accepts-export-to-and-never-reads-it-2026-10-02`).
//!
//! THE CLASS, asserted rather than the instance: every successful WRITE through
//! the door writes the export the project configures — in every configuration
//! shape the installer writes, or the file `--export-to` names — and by the
//! write-through's own rules (lineage from the committed record, the hand-edit
//! guard); and nothing else does: a read, a refused write and a pointer folder
//! write no export.
//!
//! Each test that depends on the new behaviour was run against main c4e1cbd
//! before the fix and failed there.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

const EXPORT: &str = "docs/design/proj.json";
const PROJECT: &str = r#"{"id":"proj:p","name":"P"}"#;
const REQ: &str = r#"{"id":"req:through-the-door","name":"Through the door","statement":"Written through the --call door, so the export must carry it."}"#;
/// Refused by the handler AFTER the Decision is staged (its relation names a
/// node that does not exist), so the call's write unit has something to
/// discard. A schema-shaped mistake would not do: since #656 it is answered
/// before the handler runs.
const REFUSED: &str = r#"{"id":"dec:probe-link","name":"Probe link","decision":"A decision whose relation names nothing.","related_to":[{"relation":"DEPENDS_ON","other_id":"dec:no-such-decision","evidence":"named on purpose, so the link cannot resolve"}]}"#;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

/// A project folder the binary runs in, and a directory OUTSIDE it for HOME and
/// files the test compares against.
struct Project {
    folder: tempfile::TempDir,
    outside: tempfile::TempDir,
}

impl Project {
    fn bare() -> Project {
        Project {
            folder: tempfile::Builder::new()
                .prefix("reflow2-call-export-folder-")
                .tempdir()
                .unwrap(),
            outside: tempfile::Builder::new()
                .prefix("reflow2-call-export-outside-")
                .tempdir()
                .unwrap(),
        }
    }

    /// A project whose `.mcp.json` names the export its server keeps current,
    /// exactly as `tools/reflow2_init.py` writes it, with the folder the record
    /// lives in (the installer makes it).
    fn configured() -> Project {
        let p = Project::configured_without_the_folder();
        std::fs::create_dir_all(p.path("docs/design")).unwrap();
        p
    }

    fn configured_without_the_folder() -> Project {
        let p = Project::bare();
        p.write(
            ".mcp.json",
            &mcp_json("mcpServers", "./docs/design/proj.json"),
        );
        p
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.folder.path().join(rel)
    }

    fn write(&self, rel: &str, text: &str) {
        let p = self.path(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    fn out(&self, name: &str) -> PathBuf {
        self.outside.path().join(name)
    }

    /// Run the binary in the folder with an isolated environment and a deadline.
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
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("REFLOW2_CONTENT_POLICY")
            .env_remove("REFLOW2_TRUSTED_GATEWAY")
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

    fn call(&self, tool: &str, args: &str) -> Output {
        self.run(&["--call", tool, "--args", args])
    }

    /// Run git in the folder, quietly, as an isolated identity.
    fn git(&self, args: &[&str]) {
        let o = Command::new("git")
            .current_dir(self.folder.path())
            .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("HOME", self.outside.path())
            .output()
            .expect("git runs");
        assert!(
            o.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
}

fn mcp_json(key: &str, export: &str) -> String {
    format!(
        r#"{{"{key}":{{"reflow2":{{"command":"{}","args":["--graph-path","./.reflow2/graph","--export-to","{export}","--shared"]}}}}}}"#,
        bin()
    )
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

fn json_of(o: &Output) -> Value {
    let text = String::from_utf8_lossy(&o.stdout);
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}):\n{text}\nstderr:\n{}", err(o)))
}

fn ok(o: &Output) {
    assert_eq!(o.status.code(), Some(0), "{}", err(o));
}

fn export_at(p: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn holds(doc: &Value, id: &str) -> bool {
    doc["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["node_id"] == id)
}

/// Bytes and modification time, so "untouched" means not rewritten at all.
fn fingerprint(p: &Path) -> (Vec<u8>, std::time::SystemTime) {
    (
        std::fs::read(p).unwrap(),
        std::fs::metadata(p).unwrap().modified().unwrap(),
    )
}

/// `compare_designs` with `base_path` (and `other_path` when given): identical?
fn identical(p: &Project, base: &Path, other: Option<&Path>) -> bool {
    let mut args = serde_json::json!({ "base_path": base.display().to_string() });
    if let Some(o) = other {
        args["other_path"] = Value::String(o.display().to_string());
    }
    let o = p.call("compare_designs", &args.to_string());
    ok(&o);
    let v = json_of(&o);
    v.get("identical")
        .or_else(|| v.pointer("/summary/identical"))
        .and_then(Value::as_bool)
        .unwrap_or_else(|| panic!("compare_designs answered no `identical`: {v}"))
}

// ---- the export is kept current ----------------------------------------------

/// The whole feature: a write through the door in a project that configures
/// its export leaves that export current — the live design and a fresh export
/// both compare `identical` with it — and one line on stderr says where.
#[test]
fn a_writing_call_in_a_configured_project_keeps_its_export_current() {
    let p = Project::configured();
    let export = p.path(EXPORT);

    let o = p.call("add_project", PROJECT);
    ok(&o);
    assert!(
        export.exists(),
        "the first write created no export: {}",
        err(&o)
    );
    assert!(
        err(&o).contains("export written to") && err(&o).contains("proj.json"),
        "{}",
        err(&o)
    );

    let o = p.call("add_requirement", REQ);
    ok(&o);
    let e = err(&o);
    assert_eq!(
        e.lines().count(),
        1,
        "the export is reported in ONE line: {e}"
    );
    assert!(e.contains("export written to"), "{e}");
    assert!(
        e.contains(&export.display().to_string()),
        "names where: {e}"
    );
    assert!(e.contains(".mcp.json"), "names who named the file: {e}");
    // stdout is still exactly the tool's reply.
    assert_eq!(
        json_of(&o)["node_id"],
        "req:through-the-door",
        "{}",
        String::from_utf8_lossy(&o.stdout)
    );

    assert!(holds(&export_at(&export), "req:through-the-door"));
    assert!(
        identical(&p, &export, None),
        "the live design differs from the export it kept"
    );
    let fresh = p.out("fresh.json");
    let o = p.run(&["--export"]);
    ok(&o);
    std::fs::write(&fresh, &o.stdout).unwrap();
    assert!(
        identical(&p, &export, Some(&fresh)),
        "a fresh export differs from the export the door kept"
    );
}

/// A read writes no export, and a write the tool REFUSED writes none either
/// (its write unit discarded what it staged, so there is nothing new to carry),
/// nor does a reply the tool marked an error.
#[test]
fn a_read_or_a_refused_write_leaves_the_export_untouched() {
    let p = Project::configured();
    ok(&p.call("add_project", PROJECT));
    let export = p.path(EXPORT);
    let before = fingerprint(&export);
    // Coarse mtimes would hide a rewrite made in the same tick.
    std::thread::sleep(Duration::from_millis(1100));

    for (tool, args) in [
        ("get_node", r#"{"id":"proj:p"}"#),
        ("loop_status", "{}"),
        ("graph_report", "{}"),
    ] {
        let o = p.call(tool, args);
        ok(&o);
        assert!(
            !err(&o).contains("export"),
            "a read spoke about an export: {}",
            err(&o)
        );
        assert_eq!(fingerprint(&export), before, "`{tool}` rewrote the export");
    }

    let o = p.call("add_decision", REFUSED);
    assert_eq!(o.status.code(), Some(1), "{}", err(&o));
    assert!(!err(&o).contains("export written"), "{}", err(&o));
    assert_eq!(
        fingerprint(&export),
        before,
        "a refused write rewrote the export"
    );
    assert!(!holds(&export_at(&export), "dec:probe-link"));

    // A reply the tool marks an error (exit 2) — here, an argument that does
    // not fit the published schema — writes none either.
    let o = p.call(
        "add_requirement",
        r#"{"id":"req:off-schema","name":"Off schema","statement":"Its priority is not one of the four.","priority":"nope"}"#,
    );
    assert_eq!(o.status.code(), Some(2), "{}", err(&o));
    assert!(!err(&o).contains("export written"), "{}", err(&o));
    assert_eq!(
        fingerprint(&export),
        before,
        "a tool error rewrote the export"
    );
}

/// `--export-to` with `--call` is HONOURED now: the file it names is written,
/// and it wins over what the configuration names, which is left alone.
#[test]
fn export_to_names_the_file_a_writing_call_keeps_and_wins_over_the_configuration() {
    let p = Project::configured();
    ok(&p.call("add_project", PROJECT));
    let configured = p.path(EXPORT);
    let before = fingerprint(&configured);
    let named = p.out("named.json");

    let o = p.run(&[
        "--export-to",
        named.to_str().unwrap(),
        "--call",
        "add_requirement",
        "--args",
        REQ,
    ]);
    ok(&o);
    assert!(err(&o).contains("named by --export-to"), "{}", err(&o));
    assert!(holds(&export_at(&named), "req:through-the-door"));
    assert_eq!(
        fingerprint(&configured),
        before,
        "the configured export was written although --export-to named another file"
    );

    // And a read with --export-to writes nothing at all.
    let unread = p.out("unread.json");
    let o = p.run(&[
        "--export-to",
        unread.to_str().unwrap(),
        "--call",
        "get_node",
        "--args",
        r#"{"id":"proj:p"}"#,
    ]);
    ok(&o);
    assert!(!unread.exists(), "a read wrote the export it was given");
}

/// `--no-export` asks a writing call for no export — for a script making many
/// writes to a large design — and the run says the export is now behind rather
/// than leaving that to be noticed. It cannot be combined with `--export-to`.
#[test]
fn no_export_writes_none_and_says_the_export_is_behind() {
    let p = Project::configured();
    ok(&p.call("add_project", PROJECT));
    let export = p.path(EXPORT);
    let before = fingerprint(&export);
    std::thread::sleep(Duration::from_millis(1100));

    let o = p.run(&["--no-export", "--call", "add_requirement", "--args", REQ]);
    ok(&o);
    let e = err(&o);
    assert!(e.contains("--no-export") && e.contains("behind"), "{e}");
    assert_eq!(
        fingerprint(&export),
        before,
        "--no-export rewrote the export"
    );

    let named = p.out("named.json");
    let o = p.run(&[
        "--no-export",
        "--export-to",
        named.to_str().unwrap(),
        "--call",
        "add_requirement",
        "--args",
        r#"{"id":"req:never","name":"Never","statement":"Never written: the flags contradict."}"#,
    ]);
    assert_eq!(o.status.code(), Some(2), "{}", err(&o));
    assert!(!named.exists());
    let o = p.call("get_node", r#"{"id":"req:never"}"#);
    ok(&o);
    assert!(
        json_of(&o)["node"].is_null(),
        "a contradictory command wrote"
    );
}

/// VS Code's own MCP file is read too, with its `${workspaceFolder}`: an agent
/// whose org blocks MCP still has the file `reflow2 init` wrote for it.
#[test]
fn the_vscode_configuration_names_the_export_too() {
    let p = Project::bare();
    p.write(
        ".vscode/mcp.json",
        &format!(
            r#"{{"servers":{{"reflow2":{{"command":"{}","args":["--graph-path","${{workspaceFolder}}/.reflow2/graph","--export-to","${{workspaceFolder}}/docs/design/proj.json","--shared"]}}}}}}"#,
            bin()
        ),
    );
    std::fs::create_dir_all(p.path("docs/design")).unwrap();
    let o = p.call("add_project", PROJECT);
    ok(&o);
    assert!(err(&o).contains(".vscode/mcp.json"), "{}", err(&o));
    assert!(holds(&export_at(&p.path(EXPORT)), "proj:p"));
}

/// No `--export-to` and no configuration naming one: the write lands, nothing
/// is guessed, and one line says no export was kept current and how to name
/// one — never silence.
#[test]
fn with_no_export_named_anywhere_a_write_says_so_and_writes_none() {
    let p = Project::bare();
    let o = p.call("add_project", PROJECT);
    ok(&o);
    let e = err(&o);
    assert!(e.contains("no export was kept current"), "{e}");
    assert!(e.contains("--export-to"), "{e}");
    assert!(
        !p.path("docs").exists(),
        "an export was written by guessing"
    );
}

/// Two configurations naming DIFFERENT files for this design are refused
/// before anything is opened: no store, no export, both files named.
#[test]
fn configurations_that_disagree_are_refused_before_anything_is_opened() {
    let p = Project::configured();
    p.write(
        ".vscode/mcp.json",
        &mcp_json("servers", "./docs/design/other.json"),
    );
    let o = p.call("add_project", PROJECT);
    let e = err(&o);
    assert_eq!(o.status.code(), Some(1), "{e}");
    assert!(e.contains("proj.json") && e.contains("other.json"), "{e}");
    assert!(
        e.contains("Nothing was opened and nothing was written"),
        "{e}"
    );
    assert!(!p.path(".reflow2").exists(), "a store was created");
    assert!(
        !p.path(EXPORT).exists() && !p.path("docs/design/other.json").exists(),
        "an export was written"
    );
}

// ---- the write-through's rules ----------------------------------------------

/// THE HAND-EDIT GUARD, the write-through's own: a file changed since reflow2
/// last wrote it — here, a merge that left conflict markers in it — is left
/// alone. The run says the write LANDED (so it is not retried) and why the
/// export was not written, and ends 3: a failed export is reported, never
/// swallowed.
///
/// ⚠️ The guard compares the file's own STAMPED `content_hash`, so an edit
/// that leaves the stamp in place is not seen — by the server's write-through
/// either, which runs the same function. Measured while writing this test and
/// recorded on its own; it is not this test's case to paper over.
#[test]
fn a_hand_edited_export_is_left_alone_and_the_run_says_the_write_landed() {
    let p = Project::configured();
    ok(&p.call("add_project", PROJECT));
    let export = p.path(EXPORT);
    let edited = format!(
        "<<<<<<< HEAD\n{}=======\n{{}}\n>>>>>>> theirs\n",
        std::fs::read_to_string(&export).unwrap()
    );
    std::fs::write(&export, &edited).unwrap();

    let o = p.call("add_requirement", REQ);
    let e = err(&o);
    assert_eq!(o.status.code(), Some(3), "{e}");
    assert!(e.contains("LANDED"), "{e}");
    assert!(e.contains("NOT"), "{e}");
    assert!(e.contains("not a readable export"), "{e}");
    assert_eq!(
        std::fs::read_to_string(&export).unwrap(),
        edited,
        "the hand edit was overwritten"
    );
    let o = p.call("get_node", r#"{"id":"req:through-the-door"}"#);
    ok(&o);
    assert!(
        !json_of(&o)["node"].is_null(),
        "exit 3 must mean the write landed"
    );
}

/// An export that cannot be written at all — here, its folder does not exist —
/// is reported the same way: the write landed, the export did not, exit 3.
#[test]
fn an_export_that_cannot_be_written_is_reported_and_the_run_ends_3() {
    let p = Project::configured_without_the_folder();
    let o = p.call("add_project", PROJECT);
    let e = err(&o);
    assert_eq!(o.status.code(), Some(3), "{e}");
    assert!(
        e.contains("LANDED") && e.contains("cannot write export"),
        "{e}"
    );
    let o = p.call("get_node", r#"{"id":"proj:p"}"#);
    ok(&o);
    assert!(
        !json_of(&o)["node"].is_null(),
        "exit 3 must mean the write landed"
    );
}

/// LINEAGE FROM WHAT IS COMMITTED, the write-through's own: in a git repository
/// every export a writing call makes chains from the export as committed, not
/// from the file the last call wrote — so any number of door writes on a
/// branch land one hop on a squash-merge.
#[test]
fn every_export_a_writing_call_makes_chains_from_the_committed_record() {
    let p = Project::configured();
    p.git(&["init", "-q", "-b", "main"]);
    ok(&p.call("add_project", PROJECT));
    let export = p.path(EXPORT);
    p.git(&["add", EXPORT]);
    p.git(&["commit", "-q", "-m", "the committed record"]);
    let committed = export_at(&export)["content_hash"].clone();
    assert!(committed.is_string(), "the export carries no content_hash");

    ok(&p.call("add_requirement", REQ));
    let first = export_at(&export);
    assert_eq!(first["prev_content_hash"], committed, "first door write");
    ok(&p.call(
        "add_requirement",
        r#"{"id":"req:second","name":"Second","statement":"A second door write on the same branch."}"#,
    ));
    let second = export_at(&export);
    assert_ne!(second["content_hash"], first["content_hash"]);
    assert_eq!(
        second["prev_content_hash"], committed,
        "the second door write chained from the first file, not from the committed record"
    );
}

/// Item 1 still holds: in a folder whose `.reflow2.toml` names its design on a
/// server, a writing call refuses whatever export is configured or named, and
/// writes neither a store nor an export.
#[test]
fn in_a_pointer_folder_a_writing_call_still_refuses_and_writes_no_export() {
    let p = Project::configured();
    p.write(
        ".reflow2.toml",
        "[design]\nid = \"abc123def4567890\"\naddress = \"http://127.0.0.1:9/g/abc123def4567890/mcp\"\n",
    );
    let named = p.out("named.json");
    for args in [
        vec!["--call", "add_project", "--args", PROJECT],
        vec![
            "--export-to",
            named.to_str().unwrap(),
            "--call",
            "add_project",
            "--args",
            PROJECT,
        ],
    ] {
        let o = p.run(&args);
        let e = err(&o);
        assert_eq!(o.status.code(), Some(1), "{e}");
        assert!(e.contains("abc123def4567890"), "{e}");
        assert!(!p.path(".reflow2").exists(), "a store was created");
        assert!(
            !p.path(EXPORT).exists(),
            "the configured export was written"
        );
        assert!(!named.exists(), "the named export was written");
    }
}

/// THE PROJECT'S RECORDED EXPORT (field log, 2026-10-06). On the VS Code terminal
/// route no MCP configuration exists, so a writing call found no export and every
/// write said "no export was kept current", while the Stop hook exported to a
/// different file (init's receipt) and init had invented a third
/// (fact:root-cause-the-export-path-has-no-owner-so-writes-the-hook-and-init-disagree-2026-10-06).
/// `.reflow2.toml` `[export] path` is the one record they all read: a project
/// that names its export there gets it kept by every writing call, and a file
/// with no `[design]` table still leaves the folder a LOCAL design.
#[test]
fn a_recorded_export_in_reflow2_toml_is_the_file_a_writing_call_keeps() {
    let p = Project::bare();
    p.write(".reflow2.toml", "[export]\npath = \"reflow2.json\"\n");
    let export = p.path("reflow2.json");

    let o = p.call("add_project", PROJECT);
    ok(&o);
    let e = err(&o);
    assert!(export.exists(), "the recorded export was not written: {e}");
    assert!(e.contains("export written to") && e.contains("reflow2.json"), "{e}");
    assert!(e.contains(".reflow2.toml"), "names who named the file: {e}");

    let o = p.call("add_requirement", REQ);
    ok(&o);
    assert!(holds(&export_at(&export), "req:through-the-door"));
    assert!(!p.path("docs").exists(), "nothing invented a second export");
}
