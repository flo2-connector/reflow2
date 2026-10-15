//! Every test file runs in exactly one test program.
//!
//! This crate's test files are compiled into a FEW test programs rather than
//! one each (2026-10-15): `autotests = false` in Cargo.toml, and each
//! `[[test]]` program's root declares the files it runs as modules. One
//! program per file had made ~225 programs of ~22 MB each in a debug build
//! (no features), and cargo keeps every old copy.
//!
//! THE SHAPE HAS ONE NEW WAY TO LOSE A TEST, AND THIS IS ITS GUARD. With
//! automatic discovery off, a file added to `tests/` and declared in no
//! program compiles nowhere and runs nowhere, and nothing fails — the
//! quietest possible loss. So this reads Cargo.toml and every program's root
//! and fails on: a file in no program, a file in two (its tests would run
//! twice, in two processes), a declared module with no file, a program whose
//! root is missing, and discovery switched back on (every file would be built
//! as its own program again, beside the programs that already run it).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn manifest() -> toml::Value {
    let text = std::fs::read_to_string(manifest_dir().join("Cargo.toml")).expect("Cargo.toml");
    toml::from_str(&text).expect("Cargo.toml parses")
}

/// Each `[[test]]` program: its name and its root file, relative to the crate.
fn programs() -> Vec<(String, PathBuf)> {
    manifest()
        .get("test")
        .and_then(toml::Value::as_array)
        .expect("Cargo.toml declares its test programs as [[test]] targets")
        .iter()
        .map(|t| {
            let name = t["name"]
                .as_str()
                .expect("a [[test]] has a name")
                .to_string();
            let path = t
                .get("path")
                .and_then(toml::Value::as_str)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(format!("tests/{name}.rs")));
            (name, path)
        })
        .collect()
}

/// The files a program's root declares as modules, resolved the way rustc
/// resolves them from a crate root: `mod x;` beside the root, or where a
/// `#[path = "..."]` on it says.
fn declared(root: &Path) -> Vec<(String, PathBuf)> {
    let text = std::fs::read_to_string(manifest_dir().join(root))
        .unwrap_or_else(|e| panic!("the program root {} cannot be read: {e}", root.display()));
    let dir = root.parent().expect("a root has a directory");
    let mut out = Vec::new();
    let mut path_attr: Option<String> = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("#[path") {
            let quoted = rest.split('"').nth(1).expect("#[path = \"...\"]");
            path_attr = Some(quoted.to_string());
            continue;
        }
        if let Some(name) = line
            .strip_prefix("mod ")
            .and_then(|rest| rest.strip_suffix(';'))
        {
            let name = name.trim().to_string();
            let file = match path_attr.take() {
                Some(p) => dir.join(p),
                None => dir.join(format!("{name}.rs")),
            };
            out.push((name, file));
            continue;
        }
        if !line.is_empty() && !line.starts_with("//") && !line.starts_with("#[") {
            path_attr = None;
        }
    }
    out
}

/// Every test file directly under `tests/`, relative to the crate.
fn test_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(manifest_dir().join("tests"))
        .expect("tests/")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "rs"))
        .map(|p| {
            p.strip_prefix(manifest_dir())
                .expect("under the crate")
                .to_path_buf()
        })
        .collect();
    files.sort();
    files
}

#[test]
fn automatic_test_discovery_stays_off() {
    let autotests = manifest()["package"]
        .get("autotests")
        .and_then(toml::Value::as_bool);
    assert_eq!(
        autotests,
        Some(false),
        "with `autotests` on, cargo builds every tests/*.rs as its own program AS WELL AS the \
         programs that already run it: every test twice, and ~225 programs back on the disk"
    );
}

#[test]
fn every_test_file_is_in_exactly_one_program() {
    let programs = programs();
    assert!(!programs.is_empty(), "no [[test]] programs were read");
    let mut runs_in: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();
    for (name, root) in &programs {
        assert!(
            manifest_dir().join(root).is_file(),
            "the program `{name}` names a root that does not exist: {}",
            root.display()
        );
        runs_in.entry(root.clone()).or_default().push(name.clone());
        for (module, file) in declared(root) {
            assert!(
                manifest_dir().join(&file).is_file(),
                "the program `{name}` declares `mod {module};`, and {} does not exist",
                file.display()
            );
            runs_in.entry(file).or_default().push(name.clone());
        }
    }

    let mut nowhere = Vec::new();
    let mut twice = Vec::new();
    for file in test_files() {
        match runs_in.get(&file).map(Vec::len) {
            None => nowhere.push(file.display().to_string()),
            Some(1) => {}
            Some(_) => twice.push(format!("{} in {:?}", file.display(), runs_in[&file])),
        }
    }
    assert!(
        nowhere.is_empty(),
        "these test files are in NO test program, so they compile nowhere and none of their \
         tests runs. Declare each as `mod <file>;` in tests/in_memory.rs, or in the \
         feature-gated program if the whole file needs that feature:\n  {}",
        nowhere.join("\n  ")
    );
    assert!(
        twice.is_empty(),
        "these test files are in more than one program, so their tests run twice:\n  {}",
        twice.join("\n  ")
    );
}

/// The guard's own premise, so it cannot pass by reading nothing: the
/// programs together declare most of the crate's test files.
#[test]
fn the_guard_reads_the_real_programs() {
    let declared_files: usize = programs().iter().map(|(_, r)| declared(r).len()).sum();
    assert!(
        declared_files > 200,
        "only {declared_files} files were read as declared in a program — the parse broke"
    );
}
