# Using reflow2 in VS Code when MCP is blocked — the `--call` door

A guide, kept current, for running reflow2 from VS Code's Copilot agent where an organisation's Copilot policy
blocks third-party MCP servers. **The running field log behind it is kept locally** in `docs/feedback/` (git-ignored,
`dec:field-reports-are-untracked-because-the-repository-is-public`); generic lessons from it are folded in here.

## Where reflow2 stands

**This route is supported on purpose (2026-10-01).** One reflow2 serves both routes: native MCP for an agent that
can call an MCP server, and the `--call` door for one that can only run a terminal command. MCP stays the main
route. Every tool, check and refusal is written once and reached both ways, so there is no separate build or
branch for the door.

What is planned for the door, in order:

1. a one-shot call never creates a design in a folder that names one on a server (limitation 16);
2. a writing call keeps the committed export current (limitation 4, idea 3);
3. `reflow2 read` / `reflow2 write`, so reads can be auto-approved (limitation 2, idea 4);
4. a full tool description on the CLI (limitations 5 and 6, idea 5);
5. `reflow2 init` installs this route for VS Code, `reflow2 update` refreshes it, and CI drives it the way an agent
   does (limitations 1, 10 and 11, ideas 1, 7, 8 and 11);
6. a batch of calls under one approval and one export (idea 6).

Not planned yet: joining a running shared server (idea 2), and ideas 12–19.

## The problem

With third-party MCP servers blocked, VS Code cannot register reflow2 through `.vscode/mcp.json` (the path
`reflow2 init` installs), so reflow2's tools never appear in the agent's tool list.

## What works today, with no change to reflow2

The binary's one-shot door, `--call`, runs any served tool once through the same server path a session uses (an
in-process client over an in-memory pipe), so refusals and replies read the same:

```bash
RUST_LOG=error reflow2-mcp --graph-path .reflow2/graph --call <tool> --args '<one JSON object>'
```

Exit 0 is the reply on stdout; 1 is a refusal on stderr; 2 is a reply the tool marked as an error. `--args -` reads
the object from stdin. `RUST_LOG=error` hides the per-call INFO line and the WARN line a refusal adds.

Teach the agent to use it with a **user-scope VS Code instructions file**
(`~/.config/Code/User/prompts/<name>.instructions.md`, `applyTo: '**'`) that says:

- act only when the workspace has `.reflow2/` or `REFLOW2.md`, and **not** where a `.reflow2.toml` names a design
  on a server (limitation 16);
- every "call `X`" in `REFLOW2.md`, a skill or a tool reply means `--call X`; never reimplement a tool or hand-edit
  `.reflow2/` or an export;
- discover tools with `--call find_tools --args '{"query":"…"}'`, skills with `--call list_skills` and
  `--call get_skill --args '{"name":"…"}'`;
- start with `--call loop_status`, then the `where-am-i` skill;
- handle the single writer and the missing automatic export (below);
- after a refused write, read the node back before retrying (limitation 17);
- a Requirement or Decision status change still needs the person's explicit word.

### Measured

| Check | Result |
|---|---|
| `--call graph_report` on a scratch graph | about **0.2 s** per call, including opening the store |
| `--call find_tools` | ranked tools with **parameter names** and summaries |
| a write with a missing argument | exit 2; the message names the missing argument |
| `--call export_graph` to an existing path | refused until `"overwrite": true` |
| a read on a graph held by another session's `--serve-shared` server | answered from a **best-effort snapshot**, loud stderr warning |
| a write on that held graph | **refused**, nothing written; `--stop-shared` releases it |
| `--call upstream_status` from a hub | watches the hub's pinned designs, as under MCP |
| `--call describe_schema` with `node_type` and a large `budget_chars` | full property list with **enum values**; the default budget withholds the prose |
| `--call import_graph` of a hand-built document (about 110 nodes, 140 edges) | written in one call, nothing skipped; a refused import writes nothing |
| `--call create_edges` with `"check_only": true` | validates the batch and writes nothing |

### Working patterns that pay off

- **Bulk work goes through one document.** Generate the nodes and edges with a small script kept outside the
  repository (hash each file it registers), `import_graph` it once, `export_graph`, then `compare_designs` with
  `base_path` set to the export: it should read `identical`. Typed constructors cost one approval per node and
  cannot reach every property.
- **Copy shapes from an existing export.** The fastest way to learn what a node or edge must carry is to read one
  of the same type from a design that already imported cleanly.
- **Learn enums before writing:** `describe_schema` for property values; `check_only` for an edge batch.
- **Large replies:** pipe through a short `python -c` that prints only the fields you need. Replies over about
  10 KB are cut off by the agent's terminal tool and saved to a file.
- **Check a write that returns no receipt** with `get_node`, and check a node's edges by reading the export
  (limitation 19).

### VS Code agent hooks (VS Code 1.138)

VS Code runs agent hooks: `chat.useHooks` is on by default (an organisation policy can turn it off with preview
features), reading `.github/hooks/*.json` (workspace) and `~/.copilot/hooks/*.json` (personal).
`chat.useClaudeHooks` (off by default) would also read `.claude/settings*.json`. A hook gets
`{tool_name, tool_input, tool_use_id}` on stdin, and a `PreToolUse` reply of
`hookSpecificOutput.permissionDecision: "deny"` blocks the call — verified live. This matters for reflow2 because
hooks can supply what the `--call` door lacks (ideas 1 and 12 below).

## Limitations

1. **Not a tool in Copilot's tool list.** Reachable only because an instructions file says so.
2. **Approval on every call.** One `chat.tools.terminal.autoApprove` regex cannot tell a read from a write.
3. **One writer, and `--call` cannot join a shared server.** A design held by another session is read-only from
   VS Code; `call_one_tool` opens the store directly.
4. **No automatic export.** `--export-to` write-through only starts in a long-lived server; `--call` exits first.
5. **No up-front schemas for arguments.** `find_tools` gives parameter names only. `describe_schema` now serves
   node *property* enums, but nested argument shapes (link items, verification targets, affected-node lists) and
   the allowed relation names are still learned one refusal at a time. Measured: five writes in one session each
   needed at least one refused attempt; later, one Decision with one link took four calls.
6. **Lessons attached to tools are invisible.** `tools/list` appends `steps` lessons to tool descriptions; a
   `--call` agent never reads `tools/list`. Skill lessons still arrive through `get_skill`.
7. **Skills are not native in a thin-installed project**, and slash commands don't exist in VS Code chat.
8. **Each call is a fresh session.** Session-scoped state (seats, claim liveness) does not carry between calls.
   *Not yet measured.*
9. **No loop nudges.** Nothing prompts `loop_status` at a boundary in VS Code.
10. **Telemetry can't name the harness.** Every call identifies as `reflow2-mcp --call`.
11. **The setup is one person's local file**; `reflow2 init` / `update` don't install or refresh it.
12. **Shell quoting.** Prose needs the `--args -` heredoc or a file.
13. **A hub has no member list.** The served `hub` skill says a local hub's list *is* the session's MCP config;
    under `--call` there is none, so the list must live elsewhere (a project file of store paths, for now).
14. **A hub can't see a member's unexported changes.** `upstream_status` compares the committed export, not the
    live store; without write-through the export routinely lags.
15. **A design can exist only in its store.** Nothing warns when a design has never been exported anywhere.
    don't run the door in such a folder.
17. **A refused write is not always atomic.** Measured twice on typed writers: a call refused on a later check
    (a link's arguments, a property enum) had already stored the node body; the corrected call then reported
    "already held" or replaced a prior value. Import is atomic; typed writers are not. After a refusal, `get_node`
    before retrying, and expect the retry to be an update.
18. **A narrowed schema leaves stored data unchecked.** An edge written under an older version stays in the store
    and in every export, and only `--import` applies the current rule, refusing the whole file. With `.reflow2/`
    git-ignored, `--import` is how a second contributor gets a store, so an export can be unloadable by the binary
    that wrote it. Containment: re-import committed exports into a scratch store in CI.
19. **No edge reader.** `get_node` takes no edges option, and `find_tools` does not surface a neighbours tool.
20. **Command-line safety hooks misfire on reflow2 calls.** A `PreToolUse` hook that blocks remote-shell words
    anywhere in the command text also blocks `--call` writes whose design text *mentions* those words. Match the
    command's executable position (after env assignments and wrappers like `sudo -u`/`timeout`), strip heredoc
    bodies and quoted strings first, and test the hook with reflow2 payloads.

### Tool friction found along the way

- `external_dependency` with a string where a list is expected → `failed to deserialize parameters: invalid type:
  string "…", expected a sequence`: names neither the tool nor the field.
- `external_dependency` replies with a dependency-declaration rendering (about 11 KB) rather than a node receipt;
  only a `get_node` shows the pin was written.
- `find_tools` for "read one node by id with its properties and edges" does not return `get_node`.
- `add_epoch` refuses a missing `sequence` on every first epoch (clear message, one extra round trip).
- `add_decision` link items are `{other_type, other_id, relation, evidence}`. `evidence` is required, and the
  refusal says the schema "publishes no description of it"; `relation` accepts only review relations
  (`DEPENDS_ON`, `ANTICIPATES`, …), not a generic `RELATES_TO`.
- `set_artifact_checksums` `disposition` values appear only after passing a wrong one.
- `TemporalFact.basis` is `measured | forecast`; the provenance words (`inferred`, …) are refused.
- `coverage_report` wants `{path}` records, not strings; kind-2 designs that register commit-pinned URLs need the
  same URL form in the sweep or report 0 files claimed.
- A Verification recording a test *collection* error (no test body ran) produces `failing_verification` prose
  saying the part "did not work as designed", which overstates the run.
- `export_graph --args '{}'` exits 0 and prints the whole design to stdout, unlike tools that name a missing
  argument.

## Ideas, in rough order of value for effort

1. **A served CLI-door harness in `reflow2 init`** (`--harness vscode-cli`): writes the instructions file into
   `.github/instructions/`, served from the binary so `reflow2 update` refreshes it, **plus VS Code hook files**
   (idea 12). Fixes 1, 9, 11.
2. **`--call` joins a running shared server** instead of refusing writes. Fixes 3.
3. **`--call … --export-to FILE`** (or the path from project config) after a successful write. Fixes 4 and 14.
4. **A read/write verb split** (`reflow2 read` refuses any tool not `read_only_hint`), so reads can be auto-approved
   safely. Fixes 2. Cheaper than it looks: `--call` already sorts every tool into read or write from that annotation,
   to decide whether a held design may be read from a snapshot.
5. **`--describe <tool>` / `--list-tools`** with full schemas (nested argument shapes and relation enums
   included) and the lessons `tools/list` would append. Fixes 5, 6.
6. **`--call-batch`**: JSONL of calls, one store open, one approval, stop at the first refusal.
7. **Skill stubs in `.github/skills/`** that route to `get_skill`, so VS Code picks skills by description. Fixes 7.
8. **`REFLOW2_HARNESS=vscode`** under `--call`. Fixes 10.
9. **A VS Code extension registering Language Model Tools** backed by `--call` or a local process. Check with the
   organisation first: if the policy means "no unvetted agent tools", this is the wrong answer.
10. **An MCP registry allowlist**, if the organisation's policy is registry-based.
11. **A CI probe for the door** (`tools/test_call_door.py`, beside `test_opencode_plugin.py`).
12. **Ship VS Code hooks**: a `Stop` hook running `--call export_graph` and a `SessionStart` hook running
    `--call loop_status`. Fixes 4 and 9 without touching the binary. Any shipped command-line guard must parse
    the executable position, not words in the text (limitation 20).
13. **A hub address book**: hub pins that carry a store location, and a tool that resolves "where is member X's
    store on this machine". Fixes 13.
14. **Warn on a never-exported design** in `loop_status` / `where-am-i`. Fixes 15.
15. **Refusal shape**: the `refusal_speaks` gate should cover bare deserialize errors (the `external_dependency`
    case above).
16. **Atomic typed writes**: validate links and properties before storing anything, or say in the refusal what was
    stored. Fixes 17.
17. **Re-check stored data on a schema change**: `detect_defects` (or a warning on `export_graph`) validates stored
    edges against the current rules and suggests the replacement edge type. Fixes 18.
18. **An edge reader**: `get_node` with `include_edges`, or a `neighbours` tool that `find_tools` returns. Fixes 19.
19. **Receipts everywhere**: `external_dependency` returns the same receipt shape as other writers.
