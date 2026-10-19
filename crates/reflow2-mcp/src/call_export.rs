//! A writing `--call` keeps the committed design export current, before the
//! process exits.
//!
//! `req:a-writing-call-keeps-the-committed-export-current`, step 2 of
//! `epoch:planned-the-call-door-works-for-an-agent-that-cannot-use-mcp`, as
//! settled in `dec:idea-a-writing-call-exports-afterwards`.
//!
//! # What failed
//!
//! The write-through that keeps a project's committed export current
//! (`req:the-server-keeps-the-working-tree-export-current`, [`crate::auto_export`])
//! is a debounced BACKGROUND task of a long-lived server. The door never started
//! one, and could not have used one if it had: the door serves one call and
//! tears the server down at once, so a task waiting two seconds for quiet dies
//! with the process. Measured on main c4e1cbd (2026-10-03): in a folder whose
//! `.mcp.json` names `--export-to ./docs/design/proj.json`, a `--call
//! add_requirement` exited 0 with stderr empty, and the export's bytes were
//! unchanged — `compare_designs` against it read `identical: false`
//! (`fact:root-cause-call-accepts-export-to-and-never-reads-it-2026-10-02`).
//! An agent that can reach reflow2 only through the door had to remember
//! `export_graph` with the right path after every write, which is the loss class
//! the write-through was built to end.
//!
//! # The cure: the same write-through, run synchronously at the end of the call
//!
//! Nothing here exports. The door installs the server's own [`AutoExport`]
//! WITHOUT its background task (`ReflowService::keep_export_current_at_exit`)
//! and, after a write has succeeded, runs it once and waits for it
//! (`ReflowService::export_now`). So the export obeys exactly the write-through's
//! rules, because it IS the write-through: the replaced-binary check, the
//! hand-edit guard against what this seat last wrote, the lineage anchored at
//! the committed record (`crate::export_write::chain_and_write`), never
//! `accept_divergence`, overwrite semantics, and the sync record moved at the
//! shared seam. Debouncing is not needed: there is one call.
//!
//! WHEN: after a call that WRITES by its served annotation and SUCCEEDED (exit
//! 0). A read writes no export. A refused call (exit 1) and a reply the tool
//! marked an error (exit 2) write none either — the call's write unit discarded
//! everything they staged, so there is nothing new to carry.
//!
//! # Where the file comes from — the project's configuration, never the agent
//!
//! 1. `--export-to FILE` on the command line: that file.
//! 2. Otherwise the export the project RECORDS in its `.reflow2.toml`
//!    (`[export] path`, read by [`crate::pointer::recorded_export`]): the one
//!    record the VS Code Stop hook, `reflow2 init` and `reflow2_check.py` read
//!    too, so on the terminal route (no MCP configuration at all) a write keeps
//!    the same file they do (field log 2026-10-06,
//!    fact:root-cause-the-export-path-has-no-owner-so-writes-the-hook-and-init-disagree-2026-10-06).
//! 3. Otherwise the file a long-lived server for THIS design would keep
//!    current, read from where that server reads it: the `--export-to` in the
//!    project's MCP configuration ([`MCP_CONFIGS`], the files
//!    `tools/reflow2_init.py` writes), from an entry whose `--graph-path` is
//!    this call's store. The project is the folder the store sits in
//!    (`<project>/.reflow2/graph`), the same root a server measures under.
//! 4. Otherwise nothing is exported, and one line on stderr says so and how to
//!    name one.
//!
//! `--no-export` asks a writing call for no export, for a script making many
//! writes to a large design: measured on reflow2's own design (6,478 nodes, a
//! 28 MB export, release build), a write took 1.6 s alone and 6.0 s with its
//! export. The default stays "keep it current". Not a guess: deriving the path from export history or from a
//!    naming convention was ruled out for the server on 2026-09-12 (one export
//!    to a scratch path would silently re-target every later write), and the
//!    door follows the same ruling.
//!
//! Two configurations that name DIFFERENT files for this design are refused
//! before anything is opened: reflow2 does not pick the committed record by
//! guessing.
//!
//! # What a run says
//!
//! One line on stderr after the reply, so stdout stays exactly the tool's JSON:
//! where the export was written and what the write did (`created`, `changed`,
//! `unchanged`). When the write landed but the export could not be written —
//! the file was hand-edited, it holds work the store does not, it could not be
//! written — the line says why, says the write itself LANDED (so it is not
//! retried), and the exit code is [`EXPORT_NOT_WRITTEN`]: a failed export is
//! reported, never swallowed.
//!
//! # What is deliberately not here
//!
//! `--import` (the mode) loads a record INTO the store; writing the store back
//! over that record would turn a restore into an edit of the committed file
//! with whatever store-only work it held. It is not the operation a session's
//! write is, so it is left alone. `--call import_graph` is a served write and
//! keeps the export current like every other one.

use std::path::{Component, Path, PathBuf};

use serde_json::Value;

/// The exit code of a `--call` whose write LANDED but whose export could not be
/// written. Distinct from 1 (refused: nothing was written) and 2 (the tool
/// marked its reply an error), because the design did change.
pub const EXPORT_NOT_WRITTEN: i32 = 3;

/// The project MCP configurations a long-lived server for this design is
/// started from — the paths `tools/reflow2_init.py` writes (its `MCP_CONFIGS`),
/// relative to the project. A unit test holds the two lists in step.
pub const MCP_CONFIGS: [&str; 4] = [
    ".mcp.json",
    "opencode.json",
    ".vscode/mcp.json",
    ".grok/config.toml",
];

/// The graph path a server entry with no `--graph-path` opens (its default).
const DEFAULT_GRAPH_PATH: &str = "./.reflow2/graph";

/// Who named the export file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamedBy {
    /// `--export-to` on this command line.
    Flag,
    /// The project's MCP configuration, at this path relative to the project.
    Config(String),
    /// The project's `.reflow2.toml` `[export] path`.
    Recorded,
}

/// The file a writing call keeps current, and who named it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// The path handed to the write-through. As given for the flag; resolved
    /// against the project for a configuration (a server resolves it against
    /// its working directory, which its harness sets to the project).
    pub path: String,
    pub named_by: NamedBy,
}

/// What [`find`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    Target(Target),
    /// Nothing names an export for this design. `root` is the project that was
    /// looked in; `notes` names entries that might have and could not be read.
    Unconfigured {
        root: PathBuf,
        notes: Vec<String>,
    },
    /// The caller asked for no export (`--no-export`).
    NotAsked,
}

/// The line a writing call prints under `--no-export`.
pub const NOT_ASKED: &str = "reflow2: export NOT written (--no-export): the committed export is \
     now behind this write. One writing call without --no-export, or `--call export_graph`, \
     brings it current.";

/// A place [`find`] reads the export's name from.
///
/// THE ONE LIST: [`find`] reads [`SOURCES`] in that order, and the line a write
/// prints when none of them names an export ([`unconfigured`]) is generated
/// from the same list, so a source added to the lookup is named in the line
/// with no second list to keep
/// (`req:the-no-export-line-names-every-source-a-writing-call-reads`). Until
/// 2026-10-19 the line was a hand-written sentence that #691 never updated when
/// it added the `.reflow2.toml` record, so the line never named the one source
/// that persists
/// (`fact:root-cause-the-no-export-hint-was-written-before-reflow2-toml-became-a-source-and-nothing-holds-it-to-the-resolver-2026-10-19`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// `--export-to FILE` on this command line: this one call.
    Flag,
    /// The project's `.reflow2.toml` `[export] path`: every later write, the
    /// Stop hook, `reflow2 init` and the gate read it.
    Recorded,
    /// The project's MCP configurations ([`MCP_CONFIGS`]): the file a
    /// long-lived server for this design keeps.
    Configs,
}

/// Every source [`find`] reads, in the order it reads them (the first that
/// names a file wins).
pub const SOURCES: [Source; 3] = [Source::Flag, Source::Recorded, Source::Configs];

impl Source {
    /// Who named a found target, as a source of this list. Exhaustive, so a new
    /// way of naming the export cannot be added without a source here, and
    /// through it a phrase in the line.
    pub fn of(named_by: &NamedBy) -> Source {
        match named_by {
            NamedBy::Flag => Source::Flag,
            NamedBy::Recorded => Source::Recorded,
            NamedBy::Config(_) => Source::Configs,
        }
    }

    /// Whether this source keeps naming the export for every later write.
    /// The line names the persistent record before the per-call flag.
    fn persists(self) -> bool {
        match self {
            Source::Recorded | Source::Configs => true,
            Source::Flag => false,
        }
    }

    /// How the line a write prints names this source, for the project at
    /// `root`: the exact file and the exact form to write.
    pub fn hint(self, root: &Path) -> String {
        match self {
            Source::Recorded => format!(
                "record it in {} as [export] path = \"<FILE>\" (read by every later write, \
                 the Stop hook, reflow2 init and the gate)",
                root.join(crate::pointer::FILE).display()
            ),
            Source::Flag => "pass --export-to <FILE> for one call".to_string(),
            Source::Configs => format!(
                "name it with --export-to in the reflow2 server entry of an MCP configuration \
                 in {} ({})",
                root.display(),
                MCP_CONFIGS.join(", ")
            ),
        }
    }
}

/// The line a writing call prints when nothing names an export for its design.
///
/// Generated from [`SOURCES`]: the record first, because it persists for every
/// later write and is the one a store outside any repository has; then the
/// flag; then the configurations.
pub fn unconfigured(root: &Path, notes: &[String]) -> String {
    let mut order: Vec<Source> = SOURCES.to_vec();
    // The record first, then the per-call flag, then the configurations: a
    // stable sort keeps find()'s order within each rank.
    order.sort_by_key(|s| match s {
        Source::Recorded => 0,
        s if !s.persists() => 1,
        _ => 2,
    });
    let hints: Vec<String> = order.iter().map(|s| s.hint(root)).collect();
    let mut line = format!(
        "reflow2: no export was kept current — nothing names one for this design. The write is \
         in the store. To keep the committed export current: {}.",
        hints.join("; or ")
    );
    if !notes.is_empty() {
        line.push_str(&format!(" Not read: {}.", notes.join("; ")));
    }
    line
}

/// Which file a writing call to the store at `graph_path` keeps current.
///
/// Reads [`SOURCES`] in order; the first that names a file wins.
///
/// `Err` is a refusal to print before anything is opened: the project's
/// configurations disagree about which file it is.
pub fn find(graph_path: &str, export_to: Option<&str>, no_export: bool) -> Result<Found, String> {
    if no_export {
        return Ok(Found::NotAsked);
    }
    let root = crate::wall_check::project_root(Some(graph_path), None);
    for source in SOURCES {
        let found = match source {
            Source::Flag => export_to.map(|path| {
                Found::Target(Target {
                    path: path.to_string(),
                    named_by: NamedBy::Flag,
                })
            }),
            Source::Recorded => recorded(graph_path, &root)?,
            Source::Configs => {
                let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                let store = normalize(&cwd.join(graph_path));
                let home = std::env::var_os("HOME").map(PathBuf::from);
                Some(from_configs(&root, &store, home.as_deref())?)
            }
        };
        if let Some(found) = found {
            return Ok(found);
        }
    }
    // The configurations always answer, so this is reached only if a source is
    // added after them that names nothing: nothing named an export.
    Ok(Found::Unconfigured {
        root,
        notes: Vec::new(),
    })
}

/// [`find`]'s record half: the project's `.reflow2.toml` `[export] path`.
fn recorded(graph_path: &str, root: &Path) -> Result<Option<Found>, String> {
    let Some(rel) = crate::pointer::recorded_export(&crate::pointer::location_for(graph_path))
        .map_err(|why| format!("{why} Nothing was opened and nothing was written."))?
    else {
        return Ok(None);
    };
    // Relative to the folder the record sits in; a trailing `/` keeps the
    // item layout's directory form, which export_graph reads from it.
    let mut path = normalize(&root.join(rel.trim_end_matches('/')))
        .display()
        .to_string();
    if rel.ends_with('/') {
        path.push('/');
    }
    Ok(Some(Found::Target(Target {
        path,
        named_by: NamedBy::Recorded,
    })))
}

/// [`find`]'s configuration half, with the environment passed in.
fn from_configs(root: &Path, store: &Path, home: Option<&Path>) -> Result<Found, String> {
    let mut named: Vec<(String, PathBuf)> = Vec::new();
    let mut notes = Vec::new();
    for rel in MCP_CONFIGS {
        let file = root.join(rel);
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let entries = match server_entries(rel, &text) {
            Ok(e) => e,
            Err(why) => {
                notes.push(format!("{rel} could not be read ({why})"));
                continue;
            }
        };
        for (name, argv) in entries {
            match export_of(&argv, root, store, home) {
                None => {}
                Some(Ok(path)) => named.push((format!("{rel} ({name})"), path)),
                Some(Err(why)) => notes.push(format!("{rel} ({name}): {why}")),
            }
        }
    }
    let mut distinct: Vec<&PathBuf> = named.iter().map(|(_, p)| p).collect();
    distinct.sort();
    distinct.dedup();
    match distinct.as_slice() {
        [] => Ok(Found::Unconfigured {
            root: root.to_path_buf(),
            notes,
        }),
        [one] => {
            let first = named
                .iter()
                .find(|(_, p)| p == *one)
                .map(|(w, _)| w.clone())
                .unwrap_or_default();
            Ok(Found::Target(Target {
                path: one.display().to_string(),
                named_by: NamedBy::Config(first),
            }))
        }
        _ => {
            let each: Vec<String> = named
                .iter()
                .map(|(w, p)| format!("{w} names {}", p.display()))
                .collect();
            Err(format!(
                "the project's MCP configurations name DIFFERENT export files for the design at \
                 {}: {}. A writing call keeps one committed export current, and reflow2 does not \
                 pick it by guessing. Pass --export-to <FILE> to name it, or make the \
                 configurations agree. Nothing was opened and nothing was written.",
                store.display(),
                each.join("; ")
            ))
        }
    }
}

/// Every server entry in one configuration file, as `(name, argv)`: the
/// command and its arguments in one list, which is how every shape the
/// installer writes reduces.
fn server_entries(rel: &str, text: &str) -> Result<Vec<(String, Vec<String>)>, String> {
    if rel.ends_with(".toml") {
        let doc: toml::Table = toml::from_str(text).map_err(|e| e.message().to_string())?;
        let Some(servers) = doc.get("mcp_servers").and_then(|v| v.as_table()) else {
            return Ok(Vec::new());
        };
        return Ok(servers
            .iter()
            .filter_map(|(name, entry)| {
                let entry = entry.as_table()?;
                let mut argv = Vec::new();
                match entry.get("command") {
                    Some(toml::Value::String(s)) => argv.push(s.clone()),
                    Some(toml::Value::Array(a)) => {
                        argv.extend(a.iter().filter_map(|v| v.as_str().map(String::from)))
                    }
                    _ => {}
                }
                if let Some(a) = entry.get("args").and_then(|v| v.as_array()) {
                    argv.extend(a.iter().filter_map(|v| v.as_str().map(String::from)));
                }
                Some((name.clone(), argv))
            })
            .collect());
    }
    let doc: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    // `.mcp.json` keys its servers `mcpServers`, VS Code `servers`, OpenCode `mcp`.
    for key in ["mcpServers", "servers", "mcp"] {
        let Some(servers) = doc.get(key).and_then(Value::as_object) else {
            continue;
        };
        for (name, entry) in servers {
            let mut argv = Vec::new();
            match entry.get("command") {
                Some(Value::String(s)) => argv.push(s.clone()),
                Some(Value::Array(a)) => {
                    argv.extend(a.iter().filter_map(|v| v.as_str().map(String::from)))
                }
                _ => {}
            }
            if let Some(a) = entry.get("args").and_then(Value::as_array) {
                argv.extend(a.iter().filter_map(|v| v.as_str().map(String::from)));
            }
            out.push((name.clone(), argv));
        }
    }
    Ok(out)
}

/// The value of `--flag X` or `--flag=X` in `argv`.
fn flag_value<'a>(argv: &'a [String], flag: &str) -> Option<&'a str> {
    let eq = format!("{flag}=");
    argv.iter().enumerate().find_map(|(i, a)| {
        if a == flag {
            argv.get(i + 1).map(String::as_str)
        } else {
            a.strip_prefix(&eq)
        }
    })
}

/// The export file this server entry keeps current, when it is a server for
/// the design at `store` and keeps one. `Some(Err)` names an entry that might
/// be and cannot be read.
fn export_of(
    argv: &[String],
    root: &Path,
    store: &Path,
    home: Option<&Path>,
) -> Option<Result<PathBuf, String>> {
    let export = flag_value(argv, "--export-to")?;
    // An entry that never writes THIS store through: a client of a server
    // elsewhere, a read-only server (refused a write-through), or a design
    // that is not on disk.
    if ["--remote", "--read-only", "--ephemeral", "--registry-root"]
        .iter()
        .any(|f| {
            argv.iter()
                .any(|a| a == f || a.starts_with(&format!("{f}=")))
        })
    {
        return None;
    }
    let graph = flag_value(argv, "--graph-path").unwrap_or(DEFAULT_GRAPH_PATH);
    let graph = match resolve(graph, root, home) {
        Ok(p) => p,
        Err(why) => return Some(Err(format!("its --graph-path {why}"))),
    };
    if graph != store {
        return None;
    }
    Some(resolve(export, root, home).map_err(|why| format!("its --export-to {why}")))
}

/// A path from a configuration, as its server would open it: relative to the
/// project (the harness starts the server there), with `${workspaceFolder}`
/// (VS Code) and a leading `~/` expanded. Any other variable is not ours to
/// guess.
fn resolve(raw: &str, root: &Path, home: Option<&Path>) -> Result<PathBuf, String> {
    let raw = raw.replace("${workspaceFolder}", &root.display().to_string());
    let path = match raw.strip_prefix("~/") {
        Some(rest) => match home {
            Some(h) => h.join(rest),
            None => {
                return Err(format!(
                    "`{raw}` names a home directory this run has none of"
                ));
            }
        },
        None => PathBuf::from(&raw),
    };
    if raw.contains('$') {
        return Err(format!(
            "`{raw}` holds a variable reflow2 does not expand; pass --export-to <FILE> instead"
        ));
    }
    Ok(normalize(&if path.is_absolute() {
        path
    } else {
        root.join(path)
    }))
}

/// `p` without `.` and `..`, with its longest existing prefix canonicalized —
/// so a store that does not exist yet still compares equal to the same path
/// written another way.
fn normalize(p: &Path) -> PathBuf {
    let mut lexical = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                lexical.pop();
            }
            other => lexical.push(other),
        }
    }
    let mut existing = lexical.clone();
    let mut rest = Vec::new();
    while !existing.exists() {
        match existing.file_name() {
            Some(name) => {
                rest.push(name.to_owned());
                existing.pop();
            }
            None => break,
        }
    }
    let mut out = std::fs::canonicalize(&existing).unwrap_or(existing);
    for name in rest.into_iter().rev() {
        out.push(name);
    }
    out
}

/// Who named the file, for the line a run prints.
fn named_by(t: &Target) -> String {
    match &t.named_by {
        NamedBy::Flag => "named by --export-to".to_string(),
        NamedBy::Config(w) => format!("named by {w}"),
        NamedBy::Recorded => format!("named by {} [export] path", crate::pointer::FILE),
    }
}

/// The one line a writing call prints about its export, and the exit code the
/// run should end with (0, or [`EXPORT_NOT_WRITTEN`]).
pub fn report(tool: &str, target: &Target, flushed: &crate::auto_export::Flushed) -> (String, i32) {
    use crate::auto_export::Flushed;
    match flushed {
        Flushed::Wrote(wrote) => (
            format!(
                "reflow2: export written to {} ({wrote}; {}), so it is current with this write.",
                target.path,
                named_by(target)
            ),
            0,
        ),
        // Not reachable through `export_now`, which always has a write
        // waiting; worded rather than unreachable!() so it can never panic a run.
        Flushed::NothingWaiting => (
            format!(
                "reflow2: export at {} not rewritten: nothing was waiting to be written.",
                target.path
            ),
            0,
        ),
        Flushed::Declined(why) => (
            format!(
                "reflow2: `{tool}` LANDED in the design, but the export at {} ({}) was NOT \
                 written — {why} Do not repeat the write; once the file is resolved, the next \
                 writing call (or `--call export_graph`) writes it.",
                target.path,
                named_by(target)
            ),
            EXPORT_NOT_WRITTEN,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let d = tempfile::Builder::new()
            .prefix("reflow2-call-export-")
            .tempdir()
            .unwrap();
        let root = std::fs::canonicalize(d.path()).unwrap();
        let store = root.join(".reflow2").join("graph");
        (d, root, store)
    }

    fn write(root: &Path, rel: &str, text: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    fn mcp_json(key: &str, graph: &str, export: &str) -> String {
        format!(
            r#"{{"{key}":{{"reflow2":{{"command":"/bin/reflow2-mcp","args":["--graph-path","{graph}","--export-to","{export}","--shared"]}}}}}}"#
        )
    }

    fn target(found: Found) -> Target {
        match found {
            Found::Target(t) => t,
            other => panic!("expected a target, got {other:?}"),
        }
    }

    #[test]
    fn every_shape_the_installer_writes_names_the_export_its_server_keeps() {
        let shapes = [
            (".mcp.json", mcp_json("mcpServers", "./.reflow2/graph", "./docs/design/p.json")),
            (".vscode/mcp.json", mcp_json("servers", "./.reflow2/graph", "./docs/design/p.json")),
            (
                "opencode.json",
                r#"{"mcp":{"reflow2":{"type":"local","command":["/bin/reflow2-mcp","--graph-path",".reflow2/graph","--export-to","docs/design/p.json","--shared"],"enabled":true}}}"#
                    .to_string(),
            ),
            (
                ".grok/config.toml",
                "[mcp_servers.reflow2]\ncommand = \"/bin/reflow2-mcp\"\nargs = [\"--graph-path\", \"./.reflow2/graph\", \"--export-to\", \"./docs/design/p.json\", \"--shared\"]\nenabled = true\n"
                    .to_string(),
            ),
        ];
        for (rel, text) in shapes {
            let (_d, root, store) = project();
            write(&root, rel, &text);
            let t = target(from_configs(&root, &store, None).unwrap());
            assert_eq!(
                PathBuf::from(&t.path),
                root.join("docs/design/p.json"),
                "{rel}"
            );
            assert!(
                matches!(&t.named_by, NamedBy::Config(w) if w.starts_with(rel)),
                "{rel}: {:?}",
                t.named_by
            );
        }
    }

    #[test]
    fn an_entry_for_another_store_or_one_that_never_writes_through_names_nothing() {
        let (_d, root, store) = project();
        write(
            &root,
            ".mcp.json",
            r#"{"mcpServers":{
                "other":{"command":"r","args":["--graph-path","./elsewhere/graph","--export-to","a.json"]},
                "ro":{"command":"r","args":["--graph-path","./.reflow2/graph","--read-only","--export-to","b.json"]},
                "remote":{"command":"r","args":["--remote","https://x/g/1/mcp","--export-to","c.json"]},
                "plain":{"command":"r","args":["--graph-path","./.reflow2/graph","--shared"]}
            }}"#,
        );
        assert!(matches!(
            from_configs(&root, &store, None).unwrap(),
            Found::Unconfigured { .. }
        ));
    }

    #[test]
    fn a_default_graph_path_an_equals_form_and_workspace_folder_all_resolve() {
        let (_d, root, store) = project();
        write(
            &root,
            ".vscode/mcp.json",
            r#"{"servers":{"r":{"command":"r","args":["--export-to=${workspaceFolder}/docs/design/p.json"]}}}"#,
        );
        let t = target(from_configs(&root, &store, None).unwrap());
        assert_eq!(PathBuf::from(&t.path), root.join("docs/design/p.json"));
    }

    #[test]
    fn two_configurations_naming_different_files_are_refused_and_agreeing_ones_are_not() {
        let (_d, root, store) = project();
        write(
            &root,
            ".mcp.json",
            &mcp_json("mcpServers", "./.reflow2/graph", "./docs/design/p.json"),
        );
        write(
            &root,
            ".vscode/mcp.json",
            &mcp_json("servers", ".reflow2/graph", "docs/design/p.json"),
        );
        assert!(matches!(
            from_configs(&root, &store, None).unwrap(),
            Found::Target(_)
        ));
        write(
            &root,
            ".vscode/mcp.json",
            &mcp_json("servers", ".reflow2/graph", "docs/design/q.json"),
        );
        let why = from_configs(&root, &store, None).unwrap_err();
        assert!(why.contains("p.json") && why.contains("q.json"), "{why}");
        assert!(why.contains("--export-to"), "{why}");
    }

    #[test]
    fn an_unexpandable_variable_or_an_unreadable_file_is_named_not_skipped_silently() {
        let (_d, root, store) = project();
        write(
            &root,
            ".mcp.json",
            r#"{"mcpServers":{"r":{"command":"r","args":["--graph-path","./.reflow2/graph","--export-to","$OUT/p.json"]}}}"#,
        );
        write(&root, "opencode.json", "{ not json");
        match from_configs(&root, &store, None).unwrap() {
            Found::Unconfigured { notes, .. } => {
                let all = notes.join(" | ");
                assert!(all.contains("$OUT"), "{all}");
                assert!(all.contains("opencode.json could not be read"), "{all}");
                assert!(unconfigured(&root, &notes).contains("Not read:"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_flag_wins_over_every_configuration() {
        let (_d, root, _store) = project();
        write(
            &root,
            ".mcp.json",
            &mcp_json("mcpServers", "./.reflow2/graph", "./docs/design/p.json"),
        );
        let t = target(
            find(
                &root.join(".reflow2/graph").display().to_string(),
                Some("out.json"),
                false,
            )
            .unwrap(),
        );
        assert_eq!(t.path, "out.json");
        assert_eq!(t.named_by, NamedBy::Flag);
    }

    /// `req:the-no-export-line-names-every-source-a-writing-call-reads`: the
    /// line a write prints when nothing names its export names every place
    /// [`find`] reads — the project's `.reflow2.toml` `[export] path` FIRST (the
    /// record every later write, the Stop hook, `reflow2 init` and the gate
    /// read), then `--export-to` for one call, then each MCP configuration.
    /// Measured 2026-10-19 on 0.87.0 and 0.88.0 (field limitation 22): the line
    /// named the flag and the configurations and never the record, although
    /// find() has read it second since #691.
    #[test]
    fn the_no_export_line_names_the_record_first_then_the_flag_then_the_configurations() {
        let root = Path::new("/somewhere/mystore");
        let line = unconfigured(root, &[]);
        let record = root.join(".reflow2.toml").display().to_string();
        let at = |needle: &str| {
            line.find(needle)
                .unwrap_or_else(|| panic!("the line does not name {needle:?}: {line}"))
        };
        let record_at = at(&record);
        assert!(line.contains("[export] path = \""), "{line}");
        let flag_at = at("--export-to <FILE>");
        assert!(record_at < flag_at, "the record comes first: {line}");
        for rel in MCP_CONFIGS {
            assert!(at(rel) > flag_at, "the configurations come last: {line}");
        }
    }

    /// THE CLASS: every source [`find`] reads is named in the line, by the
    /// phrase generated for it, and every way a target can be named is a
    /// source of that list. A source added to the lookup without a phrase does
    /// not compile ([`Source::hint`] is exhaustive); one left out of [`SOURCES`]
    /// is never read by [`find`], so this fails on the target it would name.
    #[test]
    fn every_source_find_reads_is_named_in_the_no_export_line() {
        let root = Path::new("/somewhere/mystore");
        let line = unconfigured(root, &[]);
        for source in SOURCES {
            assert!(
                line.contains(&source.hint(root)),
                "{source:?} is read by find() and not named: {line}"
            );
        }
        let every_named_by = [
            NamedBy::Flag,
            NamedBy::Recorded,
            NamedBy::Config(".mcp.json".into()),
        ];
        for named_by in &every_named_by {
            // A new variant fails to compile here until it is listed above.
            match named_by {
                NamedBy::Flag | NamedBy::Recorded | NamedBy::Config(_) => {}
            }
            assert!(SOURCES.contains(&Source::of(named_by)), "{named_by:?}");
        }
    }

    /// And the list is the order find() reads: a recorded export is found with
    /// an MCP configuration naming another file, and the flag wins over both.
    #[test]
    fn find_reads_the_sources_in_the_listed_order() {
        let (_d, root, _store) = project();
        let graph = root.join(".reflow2/graph").display().to_string();
        write(
            &root,
            ".mcp.json",
            &mcp_json("mcpServers", "./.reflow2/graph", "./docs/design/p.json"),
        );
        write(&root, ".reflow2.toml", "[export]\npath = \"rec.json\"\n");
        let t = target(find(&graph, None, false).unwrap());
        assert_eq!(Source::of(&t.named_by), SOURCES[1], "{t:?}");
        let t = target(find(&graph, Some("flag.json"), false).unwrap());
        assert_eq!(Source::of(&t.named_by), SOURCES[0], "{t:?}");
        std::fs::remove_file(root.join(".reflow2.toml")).unwrap();
        let t = target(find(&graph, None, false).unwrap());
        assert_eq!(Source::of(&t.named_by), SOURCES[2], "{t:?}");
    }

    /// The configuration files read here are the ones the installer writes.
    /// Kept by hand in two languages, so a check holds them together: a
    /// harness added to `MCP_CONFIGS` in `tools/reflow2_init.py` fails this
    /// until the door reads it too.
    #[test]
    fn the_configurations_read_are_the_ones_the_installer_writes() {
        let init = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/reflow2_init.py"),
        )
        .expect("tools/reflow2_init.py is readable from the crate");
        let start = init
            .find("MCP_CONFIGS = [")
            .expect("reflow2_init.py defines MCP_CONFIGS");
        let block = &init[start..];
        let block = &block[..block.find("\n]\n").expect("MCP_CONFIGS closes")];
        let mut written: Vec<&str> = block
            .lines()
            .filter_map(|l| l.trim().strip_prefix("\"path\": \""))
            .filter_map(|l| l.split('"').next())
            .collect();
        written.sort();
        let mut read = MCP_CONFIGS.to_vec();
        read.sort();
        assert_eq!(written, read);
    }
}
