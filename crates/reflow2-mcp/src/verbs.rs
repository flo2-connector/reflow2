//! `reflow2 read <tool>` and `reflow2 write <tool>`: the `--call` door split by
//! what a call does, so a terminal can approve every read with ONE rule and
//! still ask before each write.
//!
//! `req:a-terminal-agent-can-auto-approve-reads-and-confirm-each-write`, step 3
//! of the plan for the `--call` door
//! (`dec:idea-a-shell-driven-agent-approves-reads-once-and-confirms-each-write`).
//!
//! # The class this closes
//!
//! A terminal that asks before running a command (VS Code's agent, through
//! `chat.tools.terminal.autoApprove`) decides on the command's TEXT. A door call
//! names a tool and nothing else about it, and whether that tool reads or
//! writes was known only inside the process — the served `read_only_hint` —
//! and listed nowhere on the command line. So an approval rule had to be a hand
//! copy of 78 tool names, re-copied every release
//! (`fact:root-cause-one-regex-cannot-separate-door-reads-because-the-read-set-is-not-in-the-command-and-not-served-2026-10-02`).
//! The verb moves the classification INTO the command text, and the binary
//! enforces it, so `^reflow2 read ` is a rule that cannot approve a write.
//!
//! # What `read` runs
//!
//! A call that changes nothing, with item 1's meaning of `--read-only`: no
//! node, edge or property of the design, and no file. Decided from two things,
//! neither a list kept by hand:
//!
//! 1. the tool's SERVED `read_only_hint` — the annotation every MCP client is
//!    shown, read by `--call` from the served list before anything is opened;
//! 2. item 1's FILE RULE ([`crate::service::writes_a_file`]): `export_graph` and
//!    `export_surface` are annotated read-only because they do not write the
//!    GRAPH, and with `path` they write a FILE, so `read` runs them only
//!    without one.
//!
//! Anything else is refused BY NAME before anything is opened, exit 1, naming
//! `reflow2 write`. What `read` runs is then run as `--read-only --call` runs
//! it — the same service mode a read-only server uses — so the annotation is
//! not trusted alone: a tool whose annotation said "read" and whose body wrote
//! would be refused at the write guard (graph) or the file guard (disk). And
//! like every read, it never creates a design where there is none.
//!
//! `write` runs any tool, exactly as `--call` does; it is the verb a terminal
//! keeps asking about.
//!
//! # Why the verbs live in the binary and not in the `reflow2` wrapper
//!
//! The classification is the binary's (its served surface), so a shell
//! wrapper could only get it by asking the binary; a wrapper-only verb would
//! also be missing wherever only the binary is installed (a release tarball,
//! a machine where the installer's Python never ran). The wrapper
//! `tools/reflow2_install.py` installs hands every word it does not own to the
//! binary unchanged, so `reflow2 read …` and `reflow2-mcp read …` are one
//! implementation, tested through the real binary.
//!
//! # Why one rule is enough
//!
//! Only three things can follow `read`: the tool, its arguments (a JSON object,
//! positional or `--args`, `-` for stdin), and `--graph-path`. Every other flag
//! is refused by the parser there, and the one-shot table (`crate::one_shot`)
//! refuses any flag typed before the verb that `read` does not honour.
//! `tests/a_terminal_agent_auto_approves_reads_and_confirms_each_write.rs`
//! walks every flag the binary lists, typed after `read`.
//!
//! What it cannot cover is the shell around the command: a redirection
//! (`reflow2 read export_graph > file`) is the SHELL writing, which no program
//! can see. VS Code matches each subcommand of a compound command and has its
//! own, best-effort detection of file writes; its documentation calls
//! auto-approval "a best-effort convenience, not a security boundary".

use rmcp::model::Tool;
use serde_json::{Value, json};

use crate::service::{FILE_WRITING_TOOLS, writes_a_file};

/// The phrase every refusal by the `read` verb carries, so a reader — and a
/// test — can tell it from a tool's own refusal.
pub const READ_REFUSES: &str = "`read` runs only tools that change nothing";

/// Whether a served tool only reads, by its served annotation — the one
/// read/write split `--call` already makes (a missing hint is a write).
pub fn annotated_read(tool: &Tool) -> bool {
    tool.annotations
        .as_ref()
        .and_then(|a| a.read_only_hint)
        .unwrap_or(false)
}

/// Whether `reflow2 read` may run `tool` with `arguments`: `reads` is the
/// tool's served annotation ([`annotated_read`], as `--call` read it), and the
/// file rule decides the rest. `Err` is the refusal to print; nothing has been
/// opened when it is returned.
pub fn read_may_run(tool: &str, reads: bool, arguments: &Value) -> Result<(), String> {
    let write_instead = format!(
        "run it with `reflow2 write {tool} …` (`reflow2-mcp write` without the wrapper), the \
         verb a terminal keeps asking about. `reflow2 read --list` names every tool `read` runs."
    );
    if !reads {
        return Err(format!(
            "`reflow2 read {tool}` was REFUSED: `{tool}` changes the design (its served \
             annotation says it writes), and {READ_REFUSES}, in the design or on disk. Nothing \
             was opened and nothing was written. To run it, {write_instead}"
        ));
    }
    if writes_a_file(tool, Some(arguments)) {
        return Err(format!(
            "`reflow2 read {tool}` was REFUSED: given `path`, `{tool}` writes a file there, and \
             {READ_REFUSES}, in the design or on disk. Nothing was opened and nothing was \
             written. Leave `path` out to get the document in the reply, or, to write the file, \
             {write_instead}"
        ));
    }
    Ok(())
}

/// `reflow2 read --list`: every served tool, under the verb that runs it, and
/// the one argument that moves a read to `write`. Built from the served
/// annotations and the file rule, like the verb itself — never from a list.
pub fn listing(tools: &[Tool]) -> Value {
    let mut read: Vec<String> = Vec::new();
    let mut write: Vec<String> = Vec::new();
    let mut only_without = serde_json::Map::new();
    for t in tools {
        let name = t.name.to_string();
        if annotated_read(t) {
            if FILE_WRITING_TOOLS.contains(&name.as_str()) {
                only_without.insert(name.clone(), json!("path"));
            }
            read.push(name);
        } else {
            write.push(name);
        }
    }
    read.sort();
    write.sort();
    json!({
        "read": {
            "runs": "reflow2 read <tool> [JSON]",
            "count": read.len(),
            "tools": read,
            "only_without": only_without,
        },
        "write": {
            "runs": "reflow2 write <tool> [JSON]",
            "count": write.len(),
            "tools": write,
        },
        "served": tools.len(),
        "decided_by": "Each tool's served read_only_hint, the annotation every MCP client is shown, \
                       plus one rule: a tool under `only_without` writes a file when given that \
                       argument, so `read` runs it only without it. `read` refuses everything \
                       else before anything is opened, and runs what it accepts read-only, so \
                       nothing it runs can change the design or write a file. Auto-approve \
                       `^reflow2 read ` and leave `reflow2 write` asking.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::ReflowService;

    #[test]
    fn a_writer_is_refused_by_name_and_told_to_use_write() {
        let why = read_may_run("add_requirement", false, &json!({})).unwrap_err();
        assert!(why.contains(READ_REFUSES), "{why}");
        assert!(why.contains("reflow2 write add_requirement"), "{why}");
        assert!(why.contains("Nothing was opened"), "{why}");
    }

    #[test]
    fn a_file_writer_reads_without_a_path_and_is_refused_with_one() {
        for tool in FILE_WRITING_TOOLS {
            assert!(read_may_run(tool, true, &json!({})).is_ok());
            assert!(read_may_run(tool, true, &json!({ "path": null })).is_ok());
            let why = read_may_run(tool, true, &json!({ "path": "x.json" })).unwrap_err();
            assert!(why.contains("path"), "{why}");
            assert!(why.contains(&format!("reflow2 write {tool}")), "{why}");
        }
        // The rule is the file writers', not anyone's `path`.
        assert!(read_may_run("compare_designs", true, &json!({ "path": "x" })).is_ok());
    }

    #[test]
    fn the_listing_holds_every_served_tool_once_under_its_annotation() {
        let tools = ReflowService::served_tools();
        let v = listing(&tools);
        let names = |k: &str| -> Vec<String> {
            v[k]["tools"]
                .as_array()
                .unwrap()
                .iter()
                .map(|t| t.as_str().unwrap().to_string())
                .collect()
        };
        let (read, write) = (names("read"), names("write"));
        assert_eq!(read.len() + write.len(), tools.len());
        assert_eq!(v["served"], tools.len());
        for t in &tools {
            let name = t.name.to_string();
            assert_eq!(read.contains(&name), annotated_read(t), "{name}");
            assert_eq!(write.contains(&name), !annotated_read(t), "{name}");
        }
        for tool in FILE_WRITING_TOOLS {
            assert!(
                read.contains(&tool.to_string()),
                "{tool} is served as a read"
            );
            assert_eq!(v["read"]["only_without"][tool], "path");
        }
    }
}
