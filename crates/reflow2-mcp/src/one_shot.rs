//! A one-shot mode resolves its design before it opens one, and honours or
//! refuses every flag it is given.
//!
//! `req:a-one-shot-call-never-creates-a-design-where-a-folder-names-one-on-a-server`,
//! step 1 of `epoch:planned-the-call-door-works-for-an-agent-that-cannot-use-mcp`.
//!
//! # The class this closes
//!
//! `main()` is a chain of early returns, and which rules a run obeyed used to be
//! decided by WHERE its branch sat in that chain. The pointer check
//! (`.reflow2.toml`), the `--only-if-present` check and every serve-time flag
//! (`--read-only`, `--export-to`, `--remote`, `--shared`) were read below the
//! one-shot branches, and the store opener creates a missing store. So, measured
//! on 0.77.0 (2026-10-02):
//!
//! · `--call` and `--export` in a folder whose `.reflow2.toml` names a design on
//!   a server minted a fresh empty local design, exit 0, silent — and the next
//!   `--call loop_status` there said `clean: true`
//!   (`fact:root-cause-one-shot-modes-return-before-the-pointer-check-and-opening-a-store-creates-it-2026-10-02`).
//!   Even `find_tools` and `get_skill`, which read no design, left a store behind.
//! · `--read-only --call add_requirement` wrote the node and exited 0
//!   (`fact:call-ignores-read-only-and-the-write-lands-2026-10-02`).
//! · `--export-to FILE --call <writer>` never wrote FILE
//!   (`fact:root-cause-call-accepts-export-to-and-never-reads-it-2026-10-02`).
//! · `--remote URL --call X` ran a stdio proxy on nothing and exited 0
//!   (`fact:root-cause-the-door-opens-the-store-itself-and-never-reads-the-shared-servers-rendezvous-2026-10-02`).
//!
//! A guard added for one entry point protects only the branches below it. So the
//! two rules here are not placed among the branches; `main` runs both FIRST, once,
//! for every mode, before anything can open a store:
//!
//! 1. [`gate`] — THE TABLE. Each one-shot mode names the command-line flags it
//!    reads ([`Mode::honours`]). Any other flag given on the command line is
//!    refused by name, with the mode, exit 1. A NEW flag is therefore refused by
//!    every one-shot mode until somebody decides it is read there; it cannot
//!    arrive silently ignored. Values that come from the environment or from a
//!    default are not the caller's words for this run and are left alone.
//! 2. [`resolve_design`] — "where is this design?", for every mode that opens the
//!    store at `--graph-path`: a folder that names its design on a server is
//!    refused with that design's id and address; `--only-if-present` and
//!    `--read-only` are honoured; and a READ never creates a design.
//!
//! # Why a read refuses where there is no design, and a write may create one
//!
//! The split is the one the door already makes from each tool's served
//! `read_only_hint` (`--call`), or that the mode is (`--export` reads, `--import`
//! writes) — no second list. A design that comes into being because somebody
//! asked what was in it answers as a healthy empty design, and nothing says it
//! was made by asking. A write is the caller putting a design at that path.
//! The one exception for a read is a folder that has OPTED IN (`.reflow2/` is
//! there and empty, `DesignPathState::OptedIn`): the design there is genuinely
//! empty, a session started there would open it the same way, and the installer
//! exports it on purpose to mint the design's id (`tools/reflow2_init.py`).

use std::path::Path;

use clap::parser::ValueSource;
use clap::{Arg, ArgMatches, Command};

/// A way of running `reflow2-mcp` that does one thing and exits instead of
/// serving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// `reflow2-mcp setup …`
    Setup,
    /// `--export`
    Export,
    /// `--import FILE`
    Import,
    /// `--diff BASE`: one file against the design at `--graph-path`.
    DiffStore,
    /// `--diff BASE OTHER`: two files, no store.
    DiffFiles,
    /// `--merge BASE OURS THEIRS`
    Merge,
    /// `--merge-apply BASE OURS THEIRS`
    MergeApply,
    /// `--merge-driver ANCESTOR OURS THEIRS`
    MergeDriver,
    /// `--export-snapshot`
    ExportSnapshot,
    /// `--call TOOL`
    Call,
    /// `--stop-shared`
    StopShared,
    /// `reflow2-mcp read TOOL [JSON]`: `--call` for a tool that changes
    /// nothing, refusing every other by name (`crate::verbs`).
    Read,
    /// `reflow2-mcp write TOOL [JSON]`: `--call` for any tool, the verb a
    /// terminal keeps asking about (`crate::verbs`).
    Write,
}

/// What a mode does to the design at `--graph-path`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// It only reads the design.
    Reads,
    /// It changes the design, and may therefore create it.
    Writes,
}

impl Mode {
    /// Every one-shot mode, in the order `main` dispatches them.
    pub const ALL: [Mode; 13] = [
        Mode::Setup,
        Mode::DiffStore,
        Mode::DiffFiles,
        Mode::Merge,
        Mode::MergeApply,
        Mode::MergeDriver,
        Mode::ExportSnapshot,
        Mode::Export,
        Mode::Call,
        Mode::Import,
        Mode::StopShared,
        Mode::Read,
        Mode::Write,
    ];

    /// The argument id that selects this mode — `setup`, `read` and `write`
    /// are subcommands.
    pub fn id(self) -> &'static str {
        match self {
            Mode::Setup => "setup",
            Mode::Export => "export",
            Mode::Import => "import",
            Mode::DiffStore | Mode::DiffFiles => "diff",
            Mode::Merge => "merge",
            Mode::MergeApply => "merge_apply",
            Mode::MergeDriver => "merge_driver",
            Mode::ExportSnapshot => "export_snapshot",
            Mode::Call => "call",
            Mode::StopShared => "stop_shared",
            Mode::Read => "read",
            Mode::Write => "write",
        }
    }

    /// Whether the mode is selected by a subcommand rather than by a flag.
    pub fn is_subcommand(self) -> bool {
        matches!(self, Mode::Setup | Mode::Read | Mode::Write)
    }

    /// How the mode is written on a command line, for the refusals.
    pub fn named(self) -> &'static str {
        match self {
            Mode::Setup => "setup",
            Mode::Export => "--export",
            Mode::Import => "--import",
            Mode::DiffStore => "--diff BASE",
            Mode::DiffFiles => "--diff BASE OTHER",
            Mode::Merge => "--merge",
            Mode::MergeApply => "--merge-apply",
            Mode::MergeDriver => "--merge-driver",
            Mode::ExportSnapshot => "--export-snapshot",
            Mode::Call => "--call",
            Mode::StopShared => "--stop-shared",
            Mode::Read => "the `read` verb",
            Mode::Write => "the `write` verb",
        }
    }

    /// ⭐ THE TABLE: the OTHER command-line arguments this mode reads, by clap
    /// id. Everything not listed is refused when given with this mode.
    ///
    /// Adding a flag to a row is a claim that the mode ACTS on it. `--read-only`
    /// is listed for the modes that change nothing by construction (so its
    /// promise holds) and for `--call`, which enforces it; it is refused by the
    /// modes that write. `main`'s tests hold every id here to an argument that
    /// exists, so a renamed flag cannot leave a dead entry behind.
    pub fn honours(self) -> &'static [&'static str] {
        // The store-opening modes all read these: the path, its memory budget,
        // and the opt-in check.
        match self {
            Mode::Setup => &[],
            Mode::Export | Mode::ExportSnapshot | Mode::DiffStore => {
                &["graph_path", "store_memory", "only_if_present", "read_only"]
            }
            Mode::Import => &[
                "graph_path",
                "store_memory",
                "only_if_present",
                "accept_newer",
            ],
            Mode::Call => &[
                "graph_path",
                "store_memory",
                "only_if_present",
                "read_only",
                "tree_root",
                "call_args",
            ],
            // File-pure: no store is opened and nothing is written but stdout.
            Mode::DiffFiles | Mode::Merge => &["read_only"],
            Mode::MergeApply => &["read_only", "resolutions"],
            // Writes git's %A file: a write, so --read-only is refused here.
            Mode::MergeDriver => &[],
            // Reads the server record beside the store at --graph-path.
            Mode::StopShared => &["graph_path"],
            // THE VERBS are `--call` with the read/write split in the command
            // text (`crate::verbs`), so they read what `--call` reads, less
            // what contradicts them: `write` refuses `--read-only`. Their own
            // words — the tool, its JSON, `--args`, `--list` — belong to the
            // subcommand and are not here; `--graph-path` is the one flag that
            // may also follow the verb, so reading a design elsewhere stays
            // inside a `^reflow2 read ` approval rule.
            Mode::Read => &[
                "graph_path",
                "store_memory",
                "only_if_present",
                "read_only",
                "tree_root",
            ],
            Mode::Write => &["graph_path", "store_memory", "only_if_present", "tree_root"],
        }
    }

    /// Whether this mode opens the store at `--graph-path` — the modes the
    /// "where is this design?" step governs. `--call`'s access is its tool's,
    /// which only the served tool list knows, so `main` supplies it; so is a
    /// verb's.
    pub fn opens_the_store(self) -> bool {
        matches!(
            self,
            Mode::Export
                | Mode::ExportSnapshot
                | Mode::DiffStore
                | Mode::Import
                | Mode::Call
                | Mode::Read
                | Mode::Write
        )
    }
}

/// The marker every per-flag refusal carries, so a reader (and the flag × mode
/// test) can tell this refusal from any other.
pub const NOT_HONOURED: &str = "is not honoured by";

/// The flag as a person types it.
fn long(arg: &Arg) -> String {
    match arg.get_long() {
        Some(l) => format!("--{l}"),
        None => arg.get_id().to_string(),
    }
}

/// Why `flag` (a clap id) means nothing to `mode`, in a sentence that says what
/// would work instead.
fn why_not(id: &str, mode: Mode) -> String {
    match (id, mode) {
        ("export_to", Mode::Call) => "a one-shot call does not keep the committed export current \
             yet. That is step 2 of the plan for the --call door (\"a writing call keeps the \
             committed export current\", req:a-writing-call-keeps-the-committed-export-current), \
             which will honour this flag. Until then, follow a writing call with `--call \
             export_graph --args '{\"path\":\"<FILE>\",\"overwrite\":true}'`."
            .to_string(),
        ("export_to", _) => "--export-to is the write-through of a server that keeps running; \
             this mode does not serve, so it would write nothing."
            .to_string(),
        ("remote", Mode::Call) => "a one-shot call opens the design on THIS machine and cannot \
             reach one on a server yet (whether it should is the open question \
             dec:idea-a-one-shot-call-reaches-the-design-where-it-is-served). Reach that design \
             through an MCP client at the URL — `reflow2-mcp --remote <URL>` is the MCP server \
             entry that does that."
            .to_string(),
        ("remote", _) => "--remote forwards an MCP session to a server; this mode works on files \
             or on the store on this machine and would never contact it."
            .to_string(),
        ("shared", Mode::Call) => "a one-shot call opens the store itself and does not join a \
             shared server. While one holds the design, a read answers from a best-effort \
             snapshot and a write needs that server stopped first (`--stop-shared`). Joining it \
             is the open question dec:idea-a-one-shot-call-reaches-the-design-where-it-is-served."
            .to_string(),
        ("read_only", Mode::Import) => "--import writes the design, and --read-only refuses \
             every write; the two ask for opposite things."
            .to_string(),
        ("read_only", Mode::Write) => "the `write` verb is the one that may change the design, \
             and --read-only refuses every write; the two ask for opposite things. To run a tool \
             that only reads, use `reflow2 read <tool>`."
            .to_string(),
        ("call_args", Mode::Read | Mode::Write) => "a verb takes the tool's arguments AFTER the \
             tool — `reflow2 read <tool> '<json>'`, or `--args <json>` after the tool — not \
             before the verb."
            .to_string(),
        ("read_only", _) => "this mode writes (a file or a process's state), and --read-only \
             refuses every write."
            .to_string(),
        ("only_if_present" | "graph_path" | "store_memory", _) => {
            "this mode never opens a design store, so there is nothing for it to govern."
                .to_string()
        }
        _ => "this mode does not read it, so it would change nothing.".to_string(),
    }
}

/// THE TABLE, applied: which one-shot mode this command line asks for (`None`
/// when it serves), or why it is refused. Runs before anything is opened.
///
/// `cmd` must be the command `matches` were parsed with, so every argument is
/// asked about by an id it knows.
pub fn gate(cmd: &Command, matches: &ArgMatches) -> Result<Option<Mode>, String> {
    let given: Vec<&Arg> = cmd
        .get_arguments()
        .filter(|a| {
            matches!(
                matches.value_source(a.get_id().as_str()),
                Some(ValueSource::CommandLine)
            )
        })
        .collect();
    let is_given = |id: &str| given.iter().any(|a| a.get_id() == id);

    let mut modes: Vec<Mode> = Vec::new();
    if let Some(sub) = matches.subcommand_name()
        && let Some(mode) = Mode::ALL
            .into_iter()
            .find(|m| m.is_subcommand() && m.id() == sub)
    {
        modes.push(mode);
    }
    if is_given("diff") {
        let paths = matches
            .get_many::<String>("diff")
            .map(|v| v.len())
            .unwrap_or(0);
        modes.push(if paths >= 2 {
            Mode::DiffFiles
        } else {
            Mode::DiffStore
        });
    }
    for mode in Mode::ALL {
        if !mode.is_subcommand()
            && !matches!(mode, Mode::DiffStore | Mode::DiffFiles)
            && is_given(mode.id())
        {
            modes.push(mode);
        }
    }

    let mode = match modes.as_slice() {
        [] => return Ok(None),
        [one] => *one,
        many => {
            let names: Vec<&str> = many.iter().map(|m| m.named()).collect();
            return Err(format!(
                "{} are each a mode of their own, and one run does one thing: pass one of them. \
                 Nothing was opened and nothing was written.",
                names.join(" and ")
            ));
        }
    };

    let unhonoured: Vec<&&Arg> = given
        .iter()
        .filter(|a| {
            let id = a.get_id().as_str();
            id != mode.id() && !mode.honours().contains(&id)
        })
        .collect();
    if unhonoured.is_empty() {
        return Ok(Some(mode));
    }
    let names: Vec<String> = unhonoured.iter().map(|a| long(a)).collect();
    let mut why = format!(
        "{} does not read {}, so this command was REFUSED rather than run with {} silently \
         ignored. Nothing was opened and nothing was written.",
        mode.named(),
        names.join(", "),
        if names.len() == 1 { "it" } else { "them" }
    );
    for a in &unhonoured {
        why.push_str(&format!(
            "\n  {} {NOT_HONOURED} {}: {}",
            long(a),
            mode.named(),
            why_not(a.get_id().as_str(), mode)
        ));
    }
    Err(why)
}

/// What one run asks of the design at `--graph-path`.
#[derive(Debug, Clone)]
pub struct Asked<'a> {
    /// How the run is named in a refusal, e.g. "`--export`" or "`--call loop_status`".
    pub what: &'a str,
    pub graph_path: &'a str,
    pub access: Access,
    pub only_if_present: bool,
    /// What asked for "change nothing", as a person typed it — `--read-only`,
    /// or the `read` verb — so a refusal names the thing that was typed;
    /// `None` when nothing did.
    pub read_only: Option<&'a str>,
}

/// ⭐ "WHERE IS THIS DESIGN?" — answered once, before any one-shot mode opens
/// the store at `--graph-path`, and never by opening it.
///
/// `Ok` means the mode may open (and, for a write, create) the store. `Err` is
/// the refusal to print, and nothing has been opened or created.
pub fn resolve_design(asked: &Asked<'_>) -> Result<(), String> {
    let what = asked.what;
    let graph_path = asked.graph_path;

    // 1. A FOLDER THAT NAMES ITS DESIGN ON A SERVER. Checked before anything
    // else, and even when a local store also sits here: that store is not the
    // design the folder names
    // (fact:an-agent-opened-in-a-moved-designs-folder-is-served-the-frozen-store-2026-09-27).
    match crate::pointer::read(&crate::pointer::location_for(graph_path)) {
        Ok(None) => {}
        Ok(Some(pointer)) => {
            let leftover = if Path::new(graph_path).exists() {
                format!(
                    " A design store also sits at {graph_path}. It was NOT opened: it is not the \
                     design this folder names, so do not read it, copy it or write to it \
                     expecting it to be current."
                )
            } else {
                String::new()
            };
            return Err(format!(
                "{what} opened nothing and created nothing: this folder names its design on a \
                 server. {} says this folder implements design {} at {}. A one-shot command \
                 works on a design on THIS machine and cannot reach one on a server yet \
                 (whether it should is the open question \
                 dec:idea-a-one-shot-call-reaches-the-design-where-it-is-served). Reach design \
                 {} through an MCP client at {}: an MCP session started in this folder attaches \
                 to it by itself, and `reflow2-mcp --remote {}` is the MCP server entry that \
                 reaches it from anywhere.{leftover}",
                pointer.path.display(),
                pointer.design.id,
                pointer.design.address,
                pointer.design.id,
                pointer.design.address,
                pointer.design.address,
            ));
        }
        Err(why) => {
            return Err(format!(
                "{what} opened nothing and created nothing: {why} A folder that names its design \
                 never falls back to a local store, so correct or remove that file first."
            ));
        }
    }

    // 2. --read-only, HONOURED: a write is refused before anything is opened.
    if let Some(read_only) = asked.read_only
        && asked.access == Access::Writes
    {
        return Err(format!(
            "{what} writes the design, and {read_only} refuses every write: nothing was opened \
             and nothing was written. Drop {read_only} to write, or call a tool that only reads."
        ));
    }

    // 3. --only-if-present, HONOURED with the meaning it has for a session:
    // a directory whose graph, or the directory that would hold it, does not
    // exist has not opted into a design, and nothing is created for it.
    if asked.only_if_present && !crate::latent::design_present(graph_path) {
        return Err(format!(
            "{what} opened nothing and created nothing: no design has been started here \
             ({graph_path} does not exist, nor the directory that would hold it), and \
             --only-if-present asks for exactly that. To start a design here, run the same \
             command without --only-if-present."
        ));
    }

    // 4. NO STORE AT THE PATH. Asked without opening anything: `describe_at`
    // reads only the sidecars, so looking cannot mint an identity.
    if !Path::new(graph_path).exists() {
        let found = reflow2_core::describe_at(graph_path);
        if let Some(read_only) = asked.read_only {
            return Err(format!(
                "{what} opened nothing and created nothing: there is no design store at \
                 {graph_path} ({}), and {read_only} creates nothing.",
                found.reading
            ));
        }
        if asked.access == Access::Reads && found.state != reflow2_core::DesignPathState::OptedIn {
            return Err(format!(
                "{what} reads a design, and there is no design at {graph_path} ({}). Nothing was \
                 opened and nothing was created: a read never creates a design, because an empty \
                 one would answer as a healthy design with nothing saying it was made by asking. \
                 Point --graph-path at the design's store, or start a design here with a call \
                 that writes (`--call genesis`) or with `--import <export>`.",
                found.reading
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> tempfile::TempDir {
        tempfile::Builder::new()
            .prefix(&format!("reflow2-one-shot-{tag}-"))
            .tempdir()
            .unwrap()
    }

    fn asked(graph_path: &str, access: Access) -> Asked<'_> {
        Asked {
            what: "`--test`",
            graph_path,
            access,
            only_if_present: false,
            read_only: None,
        }
    }

    #[test]
    fn a_read_where_there_is_no_design_is_refused_and_a_write_may_create_it() {
        let d = scratch("absent");
        let g = d.path().join(".reflow2").join("graph");
        let g = g.to_str().unwrap();
        let why = resolve_design(&asked(g, Access::Reads)).unwrap_err();
        assert!(why.contains("no design at"), "{why}");
        assert!(resolve_design(&asked(g, Access::Writes)).is_ok());
        assert!(
            !d.path().join(".reflow2").exists(),
            "resolving opens nothing"
        );
    }

    #[test]
    fn an_opted_in_folder_is_read_as_the_empty_design_it_is() {
        let d = scratch("opted");
        std::fs::create_dir_all(d.path().join(".reflow2")).unwrap();
        let g = d.path().join(".reflow2").join("graph");
        assert!(resolve_design(&asked(g.to_str().unwrap(), Access::Reads)).is_ok());
    }

    #[test]
    fn a_pointer_refuses_reads_and_writes_and_names_the_design_and_its_server() {
        let d = scratch("pointer");
        std::fs::write(
            d.path().join(".reflow2.toml"),
            "[design]\nid = \"abc123def4567890\"\naddress = \"http://127.0.0.1:9/g/abc123def4567890/mcp\"\n",
        )
        .unwrap();
        let g = d.path().join(".reflow2").join("graph");
        for access in [Access::Reads, Access::Writes] {
            let why = resolve_design(&asked(g.to_str().unwrap(), access)).unwrap_err();
            assert!(why.contains("abc123def4567890"), "{why}");
            assert!(
                why.contains("http://127.0.0.1:9/g/abc123def4567890/mcp"),
                "{why}"
            );
        }
    }

    #[test]
    fn read_only_refuses_a_write_and_creates_nothing_even_where_a_folder_opted_in() {
        let d = scratch("ro");
        std::fs::create_dir_all(d.path().join(".reflow2")).unwrap();
        let g = d.path().join(".reflow2").join("graph");
        let mut a = asked(g.to_str().unwrap(), Access::Writes);
        a.read_only = Some("--read-only");
        assert!(resolve_design(&a).unwrap_err().contains("--read-only"));
        a.access = Access::Reads;
        assert!(resolve_design(&a).unwrap_err().contains("--read-only"));
    }

    #[test]
    fn only_if_present_refuses_where_nobody_opted_in() {
        let d = scratch("oip");
        let g = d.path().join(".reflow2").join("graph");
        let mut a = asked(g.to_str().unwrap(), Access::Writes);
        a.only_if_present = true;
        assert!(
            resolve_design(&a)
                .unwrap_err()
                .contains("--only-if-present")
        );
        std::fs::create_dir_all(d.path().join(".reflow2")).unwrap();
        assert!(resolve_design(&a).is_ok());
    }
}
