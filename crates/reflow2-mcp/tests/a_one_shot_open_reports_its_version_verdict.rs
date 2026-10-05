//! Every one-shot open says what its version check found, and a downgrade
//! keeps the newer stamp.
//!
//! WRITTEN BEFORE THE IMPLEMENTATION, from
//! `fact:root-cause-every-one-shot-open-drops-the-version-verdict-and-the-repair-report-2026-10-05`:
//! a store written by 0.74.0 opened with 0.79.0 through `reflow2 read`,
//! `--call` and `--export` printed no "written by" line, and a store stamped
//! 0.79.0 opened with 0.78.0 through `--call` printed nothing and had its
//! stamp rewritten DOWN to 0.78.0. The serving modes said both.
//!
//! Driven through the real binary, the way a terminal agent runs it. The stamp
//! beside the store is edited to stand in for an older or a newer reflow2: the
//! vocabulary is this binary's own, so only the version differs.

use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::Value;

const PROJECT: &str = r#"{"id":"proj:p","name":"P"}"#;
const NOW: &str = env!("CARGO_PKG_VERSION");

struct Folder {
    dir: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl Folder {
    fn new() -> Folder {
        let f = Folder {
            dir: tempfile::Builder::new()
                .prefix("reflow2-verdict-")
                .tempdir()
                .unwrap(),
            home: tempfile::Builder::new()
                .prefix("reflow2-verdict-home-")
                .tempdir()
                .unwrap(),
        };
        let o = f.run(&[
            "--graph-path",
            ".reflow2/graph",
            "--call",
            "add_project",
            "--args",
            PROJECT,
        ]);
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        f
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_reflow2-mcp"))
            .current_dir(self.dir.path())
            .env("HOME", self.home.path())
            .env("XDG_CONFIG_HOME", self.home.path().join(".config"))
            .env(
                "REFLOW2_CONFIG_DIR",
                self.home.path().join("reflow2-config"),
            )
            .env("RUST_LOG", "error")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("REFLOW2_CONTENT_POLICY")
            .env_remove("REFLOW2_TRUSTED_GATEWAY")
            .args(args)
            .output()
            .unwrap()
    }

    /// stderr of a one-shot run that must succeed.
    fn stderr_of(&self, args: &[&str]) -> String {
        let o = self.run(args);
        let err = String::from_utf8_lossy(&o.stderr).to_string();
        assert!(o.status.success(), "{args:?} failed:\n{err}");
        err
    }

    fn meta_path(&self) -> PathBuf {
        self.dir.path().join(".reflow2/graph.meta.json")
    }

    fn meta(&self) -> Value {
        serde_json::from_str(&std::fs::read_to_string(self.meta_path()).unwrap()).unwrap()
    }

    /// Stand in for another reflow2: its version, this binary's vocabulary.
    fn stamp_as(&self, version: &str) {
        let mut m = self.meta();
        m["reflow2_version"] = Value::String(version.into());
        std::fs::write(self.meta_path(), serde_json::to_string_pretty(&m).unwrap()).unwrap();
    }
}

/// Every one-shot door the root cause names, minus `--import` and `--diff`,
/// which need a document and open through the same `one_shot::open_store`.
const DOORS: &[&[&str]] = &[
    &["read", "graph_report", "--graph-path", ".reflow2/graph"],
    &["--graph-path", ".reflow2/graph", "--call", "loop_status"],
    &["--graph-path", ".reflow2/graph", "--export"],
];

#[test]
fn a_one_shot_open_of_an_older_store_says_so_and_records_where_it_came_from() {
    let f = Folder::new();
    for door in DOORS {
        f.stamp_as("0.1.0");
        let err = f.stderr_of(door);
        assert!(
            err.contains("this graph was written by reflow2 0.1.0") && err.contains(NOW),
            "{door:?} said nothing about the upgrade:\n{err}"
        );
        let m = f.meta();
        assert_eq!(m["reflow2_version"], NOW, "an upgrade restamps, as before");
        assert_eq!(
            m["previous_reflow2_version"], "0.1.0",
            "{door:?}: the stamp must keep the version it came from, so the upgrade \
             can be confirmed after this process has gone"
        );
        // The second open has nothing to say, and the record stays.
        let again = f.stderr_of(door);
        assert!(
            !again.contains("written by"),
            "{door:?} repeated itself:\n{again}"
        );
        assert_eq!(f.meta()["previous_reflow2_version"], "0.1.0");
    }
}

#[test]
fn a_downgrade_warns_and_keeps_the_newer_stamp() {
    let f = Folder::new();
    f.stamp_as("99.0.0");
    for door in DOORS {
        let err = f.stderr_of(door);
        assert!(
            err.contains("WARNING") && err.contains("BEHIND") && err.contains("99.0.0"),
            "{door:?} opened a store a newer reflow2 wrote without a warning:\n{err}"
        );
        assert_eq!(
            f.meta()["reflow2_version"],
            "99.0.0",
            "{door:?} rewrote the newer stamp down to {NOW}"
        );
    }
}

#[test]
fn a_reply_is_unchanged_by_the_verdict() {
    // stdout is the reply; the verdict goes to stderr only.
    let f = Folder::new();
    let plain = f.run(&[
        "read",
        "get_node",
        r#"{"id":"proj:p"}"#,
        "--graph-path",
        ".reflow2/graph",
    ]);
    f.stamp_as("0.1.0");
    let after = f.run(&[
        "read",
        "get_node",
        r#"{"id":"proj:p"}"#,
        "--graph-path",
        ".reflow2/graph",
    ]);
    assert_eq!(plain.stdout, after.stdout);
}
