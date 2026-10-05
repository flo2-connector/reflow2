# Using reflow2 in VS Code when MCP is blocked — the `--call` door

> Part of the **Reflow 2.0** docs — see **[overview.md](overview.md)** for the map.

A guide, kept current, for running reflow2 from VS Code's Copilot agent where an organisation's Copilot policy
blocks third-party MCP servers. **Checked on 2026-10-05 against main after v0.78.0 (e7e304d).** Every command
and behaviour in "How to run" and "Fixed since the report" was run on a binary built from that commit. The
limitations that still hold cite their 2026-10-02 findings; limitations 3, 7, 9, 10, 11, 13 and 15 were
checked again on main.

The guide grew out of a running field log, which is kept locally in `docs/feedback/` (git-ignored,
`dec:field-reports-are-untracked-because-the-repository-is-public`). The version the 2026-10-02 triage read
is frozen there and registered in reflow2's design as `art:vscode-call-door-field-report-2026-10-02`. This
file has moved on from it. Limitations and ideas keep that report's numbers, so each one can be traced to its
finding in the design.

## Where reflow2 stands

**This route is supported on purpose** (`dec:idea-the-call-door-becomes-a-supported-harness-for-an-agent-that-cannot-use-mcp`,
2026-10-01). One reflow2 serves both routes: native MCP for an agent that can call an MCP server, and the
`--call` door for one that can only run a terminal command (`dec:one-reflow2-serves-both-routes-mcp-and-the-call-door`).
MCP stays the main route. Every tool, check and refusal is written once and reached both ways, so there is no
separate build or branch for the door.

The door plan (`epoch:planned-the-call-door-works-for-an-agent-that-cannot-use-mcp`):

| Step | What | Where it is |
|---|---|---|
| 1 | A one-shot call never creates a design in a folder that names one on a server (limitation 16) | **Shipped in v0.78.0** |
| 2 | A writing call keeps the committed export current (limitation 4, idea 3) | **Shipped in v0.78.0** |
| 3 | `reflow2 read` / `reflow2 write`, so reads can be approved once (limitation 2, idea 4) | **Shipped in v0.78.0** |
| 4 | A full tool description on the command line: `--describe`, `--list-tools` (limitations 5 and 6, idea 5) | **Shipped in v0.78.0** |
| 5 | `reflow2 init` installs this route for VS Code, `reflow2 update` refreshes it, and CI drives it the way an agent does (limitations 1, 7, 9, 10 and 11) | **Built, in the release after v0.79.0** (ideas 1, 7, 11 and 12, approved on 2026-10-05 as one increment, `dec:the-vs-code-setup-is-built-as-one-increment-2026-10-05`): `reflow2 init --harness vscode-cli`, below. A harness name for door calls (idea 8) is in this step's requirement and was not in that increment, so it is still open. |
| 6 | A batch of calls under one approval and one export (idea 6) | Planned (`req:a-batch-of-calls-runs-under-one-approval-and-one-export`) |

v0.78.0 also shipped the fixes the report's triage asked for outside the plan. A refused write stores nothing
(limitation 17). Every argument refusal names the tool and the field path. `get_node` reads a node's edges
(limitation 19). `detect_defects` names stored data the current schema refuses (limitation 18). `--call`
honours `--read-only`. A read of a design another session holds is as true as a read of the design itself.
`external_dependency` replies with a receipt. Each is under [Fixed since the report](#fixed-since-the-report).

Still open: a call joining a running shared server (idea 2), a hub address book (idea 13), and a warning for
a design that was never exported (idea 14). [The last section](#every-idea-and-where-it-is-recorded) lists every
idea with the record that holds it.

## The problem

With third-party MCP servers blocked, VS Code cannot register reflow2 through `.vscode/mcp.json` (the path
`reflow2 init` installs), so reflow2's tools never appear in the agent's tool list.

## How to run reflow2 from the terminal today

The binary's one-shot door runs any served tool once through the same server path a session uses (an
in-process client over an in-memory pipe), so refusals and replies read the same as they do over MCP. It has
two spellings: one that splits reads from writes in the command text, and the plain `--call`.

```bash
# Runs a tool only if it changes nothing (in the design or on disk). Any other tool is refused
# by name before anything is opened.
reflow2 read loop_status
reflow2 read get_node '{"id":"req:x","include_edges":true}'

# Runs any tool, exactly as --call does, so a write keeps the committed export current.
reflow2 write add_requirement --args - <<'EOF'
{"id": "req:x", "name": "…", "statement": "…"}
EOF

# The same door, unsplit:
RUST_LOG=error reflow2-mcp --graph-path .reflow2/graph --call <tool> --args '<one JSON object>'
```

`reflow2 read` and `reflow2 write` default to `--graph-path .reflow2/graph` and print no routine log lines;
stderr carries a refusal, a warning or the export line only. `reflow2` is the command the installer puts on
your PATH; with only the binary, the verbs are `reflow2-mcp read …` and `reflow2-mcp write …`. Flags such as
`--export-to FILE` and `--no-export` go before the verb (`reflow2 --no-export write …`). Only the tool, its
arguments and `--graph-path` may follow it.

| Exit | Meaning |
|---|---|
| 0 | The reply, on stdout. |
| 1 | A refusal, on stderr: a tool's own rule; `reflow2 read` given a tool that writes; a read where there is no design; a write while another session holds the design; a flag the one-shot mode does not read (`--remote`, `--shared`); or a folder whose `.reflow2.toml` names a design on a server. |
| 2 | A reply the tool marked as an error. Arguments that do not fit the tool's published schema land here, and the message names the tool and the field path (`related_to[0].evidence`). A command line that cannot be parsed (an unknown flag after the verb, or two flags that exclude each other) also exits 2. |
| 3 | The write landed, and the export could not be written (stderr says why). Fix the file; do not repeat the write. |

**Read how to call a tool before calling it.** `reflow2-mcp --graph-path .reflow2/graph --describe add_decision`
prints the tool's input schema, with every nested shape, allowed value and required field, and the lessons this
design holds for it. It is brief by default; `--full` prints the tools/list entry unchanged. `--list-tools`
lists every tool with whether it only reads, its required arguments and its lesson count. `reflow2 read --list`
prints which tools `read` runs and which need `write`. The same description is the tool `describe_schema` with
`tool`, through any door.

**Approve the reads once, and leave each write asking.** In VS Code's settings:

```json
"chat.tools.terminal.autoApprove": {
  "/^reflow2 read /": true
}
```

With only the binary on your PATH, the rule is `"/^reflow2-mcp read /": true`.
[getting-started/SETUP.md](../getting-started/SETUP.md) says what this rule does and does not cover: a shell
redirection is the shell writing, which reflow2 never sees, and VS Code calls auto-approval a convenience, not a
security boundary.

**Set the project up for the door with `reflow2 init . --harness vscode-cli`** (from the release after
v0.79.0; add `,vscode` for teammates who do have MCP). It writes no MCP config. Under `.github/`, for the team
to commit, it writes the instructions file below, VS Code hooks, a skill stub per served skill and the slash
commands. `reflow2 update` refreshes them and keeps any file somebody edited
([getting-started/SETUP.md](../getting-started/SETUP.md), "VS Code where MCP is blocked"). The instructions
file is the text the binary serves as `get_instructions` section `vscode-terminal-route`.

On an older release, **teach the agent the door with a user-scope instructions file**:
`~/.config/Code/User/prompts/<name>.instructions.md`, with `applyTo: '**'`. It should say:

- act only when the workspace has `.reflow2/` or `REFLOW2.md`. In a folder whose `.reflow2.toml` names a design
  on a server, the door refuses and names that design's address: nothing is created there, and nothing is
  reached either;
- every "call `X`" in `REFLOW2.md`, a skill or a tool reply means `reflow2 read X` when `X` changes nothing, and
  `reflow2 write X` otherwise. Never reimplement a tool, and never hand-edit `.reflow2/` or an export;
- before calling a tool for the first time, run `--describe X`;
- discover tools with `reflow2 read find_tools '{"query":"…"}'`, and skills with `reflow2 read list_skills` and
  `reflow2 read get_skill '{"name":"…"}'`;
- start with `reflow2 read loop_status`, then the `where-am-i` skill;
- pass prose through `--args -` with a QUOTED heredoc delimiter (`<<'EOF'`), never inline (limitation 12);
- on exit 3, fix the export file and do not repeat the write;
- a Requirement or Decision status change still needs the person's explicit word.

Two lines from the 2026-10-02 version are gone, because v0.78.0 made them unnecessary: "read the node back
after a refused write" (a refused write now stores nothing) and "handle the missing automatic export" (a
writing call now exports).

### Measured on main (e7e304d), 2026-10-05

| Check | Result |
|---|---|
| `reflow2 read graph_report` on a small scratch design | about **0.14 s**, including opening the store |
| `reflow2 read get_node` on reflow2's own design (6,660 nodes, 43,748 edges) | about **1.5 s**; `loop_status` and `graph_report` there take about 40 s on a busy 4-core machine |
| `reflow2 read` of a tool that writes | exit 1, refused by name before anything is opened, naming `reflow2 write` |
| `reflow2 read` where there is no design | exit 1; nothing is created |
| a write with `--export-to docs/design/x.json` (or a directory ending in `/`) | the export is written before the command exits, and one stderr line says where |
| a write in a project whose `.vscode/mcp.json` names `--export-to` | the file it names is written |
| a write where no export is named anywhere | the write lands; stderr says no export was kept current, and how to name one |
| a write after the export file was hand-edited | exit 3: the write landed, the file was left alone, and stderr says not to repeat the write |
| a write with `--no-export` | stderr says the export is now behind |
| an argument missing inside a list item (`related_to[0].evidence`) | exit 2; the message names the tool, the path and the field's description; nothing is written |
| a value outside its allowed set (`"kind": "maybe"`) | exit 2; the allowed values are listed; nothing is written |
| a write refused by the tool's own rule (`add_decision` with `status: accepted` and no `approver`) | exit 1; `get_node` then finds no node |
| `--read-only --call add_requirement` | exit 1; nothing opened, nothing written |
| `--remote URL --call …` | exit 1, refused by name: a one-shot call works on a design on this machine |
| `get_node` with `"include_edges": true` | every edge with its type, direction, the node at the other end and the edge's properties |
| `find_tools` for "read one node by id with its properties and edges" | `get_node` first |
| `--describe add_decision` | about 7.6 KB, with the nested `$defs` shapes and the `relation` values; `--full` about 12 KB |
| a read while another session's `--serve-shared` server holds the design | answered from a copy, with a loud stderr warning; `search_design` finds what the design holds |
| a write while the design is held | exit 1, nothing written; `--stop-shared` releases it |
| any call in a folder whose `.reflow2.toml` names a design on a server | exit 1; names the design's id and address; nothing is created |
| `external_dependency` | a receipt of the declared Resource |
| `export_graph` with no `path` | the whole design on stdout, exit 0 |

### Working patterns that pay off

- **Bulk work goes through one document.** Generate the nodes and edges with a small script kept outside the
  repository (hash each file it registers), `reflow2 write import_graph` it once, then `compare_designs` with
  `base_path` set to the export: it should read `identical`. Typed constructors cost one approval per node and
  cannot reach every property.
- **Learn shapes before writing:** `--describe <tool>` for a tool's arguments, `describe_schema` with
  `node_type` for a node's properties and their values, and `check_only` for an edge batch.
- **A node's edges:** `get_node` with `"include_edges": true`, not the export.
- **Many writes to a large design:** each write costs an export (seconds on a very large design). Pass
  `--no-export` before the verb on each, and finish with one write without it, or with
  `reflow2 write export_graph` and the path.
- **Large replies:** pipe through a short `python -c` that prints only the fields you need. Replies over about
  10 KB are cut off by the agent's terminal tool and saved to a file.

### VS Code agent hooks (VS Code 1.138)

VS Code runs agent hooks: `chat.useHooks` is on by default (an organisation policy can turn it off with preview
features), reading `.github/hooks/*.json` (workspace) and `~/.copilot/hooks/*.json` (personal).
`chat.useClaudeHooks` (off by default) would also read `.claude/settings*.json`. A hook gets
`{tool_name, tool_input, tool_use_id}` on stdin, and a `PreToolUse` reply of
`hookSpecificOutput.permissionDecision: "deny"` blocks the call. This was verified live. It matters for reflow2
because hooks can supply what the door lacks. The accepted design is a `SessionStart` hook that runs
`loop_status` and a `Stop` hook that exports and nudges
(`dec:idea-how-reflow2-triggers-the-loop-for-a-call-door-agent-in-vs-code`), and step 5 installs them as
`.github/hooks/reflow2.json`. Both run `reflow2 hook vscode`. The Stop hook counts the session's writes from
reflow2's own usage ledger, so no reflow2 hook reads a command's text.

## Limitations that still hold

Each was root-caused on 2026-10-02, and its finding is in reflow2's design under the id given.

1. **Not a tool in Copilot's tool list.** Reachable only because an instructions file says so. The cause is
   the organisation's policy, and nothing in reflow2 causes it: MCP is reflow2's one way to register as an agent
   tool. (`fact:root-cause-reflow2-is-not-a-copilot-tool-because-the-org-blocks-third-party-mcp-and-nothing-in-reflow2-causes-it-2026-10-02`)
3. **One writer, and a call cannot join a shared server.** While another session's `--serve-shared` server
   holds the design, a read through the door answers from a copy. Since v0.78.0 that copy answers as the design
   itself would, search included. A write is refused and nothing is written; `--stop-shared` releases the
   design. Joining the running server is idea 2, still open.
   (`fact:root-cause-the-door-opens-the-store-itself-and-never-reads-the-shared-servers-rendezvous-2026-10-02`)
7. **Skills are not native in a thin-installed project.** VS Code does read project skills (`.github/skills`,
   `.claude/skills`) and lists them as `/` commands, chosen by their description, so the report's "slash
   commands don't exist in VS Code chat" is overturned by VS Code's documentation. What is missing is reflow2's
   part: init writes nothing VS Code reads as a skill. **Fixed by step 5** (the release after v0.79.0):
   `--harness vscode-cli` writes a skill stub per served skill and a prompt file per slash command.
   (`fact:root-cause-vs-code-reads-project-skills-and-slash-prompts-and-init-installs-neither-for-it-2026-10-02`)
8. **Each call is a fresh session.** A claim made through the door is `gone` by the next call, so two door
   agents claiming the same region see no collision, and an author named for a session lasts one call.
   (`fact:root-cause-a-door-call-is-a-session-that-ends-at-exit-and-the-sessionless-backstop-reads-only-the-protocol-revision-2026-10-02`)
9. **No loop nudges.** Nothing prompts `loop_status` at a boundary in VS Code. init installs no hook for it, the
   nudge script counts writes by MCP tool name, and `loop_status` tells a VS Code project no nudge is possible.
   **Fixed by step 5's hooks** (the release after v0.79.0), and `loop_status` no longer says no nudge is
   possible for VS Code. (`fact:root-cause-no-loop-nudge-reaches-a-call-door-agent-in-vs-code-2026-10-02`)
10. **Usage records can't name the harness.** Every door call is recorded as the client `reflow2-mcp --call`.
    (`fact:root-cause-door-calls-carry-a-fixed-client-name-and-no-attribution-route-reaches-the-door-2026-10-02`)
11. **The setup is one person's local file.** `reflow2 init --harness vscode` knows VS Code only as an MCP
    client, so `reflow2 update` has nothing of the door's to refresh. **Fixed by step 5** (the release after
    v0.79.0): the setup is committed files that `reflow2 update` refreshes.
    (`fact:root-cause-init-knows-vs-code-only-as-an-mcp-config-so-the-door-setup-is-one-persons-file-2026-10-02`)
12. **Shell quoting.** Prose crosses two quoting layers, the shell's and JSON's. `--args -` with a quoted heredoc
    delimiter (`<<'EOF'`) stores it byte for byte. An unquoted one expands `$HOME` into the stored text, and a
    raw newline inside a JSON string is refused (write `\n`).
    (`fact:root-cause-door-prose-passes-two-quoting-layers-and-args-stdin-removes-the-shells-only-with-a-quoted-delimiter-2026-10-02`)
13. **A hub can't say where a member's store is.** A local hub's member list is its declared dependencies, and
    the door reads it (`upstream_status`); since v0.78.0 the served hub skill says so. What no record holds is
    where each member's store is on this machine (idea 13, open).
    (`fact:root-cause-a-local-hubs-list-is-its-watch-manifest-but-the-hub-skill-names-the-mcp-config-and-no-pin-says-where-a-store-is-2026-10-02`)
15. **A design can exist only in its store.** `loop_status` on a design that was never exported still says
    `clean: true`, with nothing about exports. With `.reflow2/` git-ignored, that design lives on one disk (idea
    14, open). (`fact:a-never-exported-local-design-is-still-silent-in-loop-status-through-the-door-on-0-77-0-2026-10-02`)
20. **Command-line safety hooks misfire on reflow2 calls.** A `PreToolUse` hook that blocks remote-shell words
    anywhere in the command text also blocks writes whose design text *mentions* those words. Match the
    command's executable position (after env assignments and wrappers like `sudo -u`/`timeout`), strip heredoc
    bodies and quoted strings first, and test the hook with reflow2 payloads. reflow2 ships no such hook.
    (`fact:root-cause-a-text-matching-guard-blocks-door-writes-that-mention-remote-shell-words-and-reflow2-ships-no-such-hook-2026-10-02`)

## Fixed since the report

Numbered as in the report. Each change shipped in v0.78.0, and each row was checked on main.

| Limitation | What changed | PR |
|---|---|---|
| 2. Approval on every call | `reflow2 read <tool>` runs only a tool that changes nothing, and refuses any other by name before opening anything. One VS Code rule, `^reflow2 read `, approves every read and no write. | #666 |
| 3, the read half. A read of a held design answered from an incomplete copy | The copy now answers as the design itself would. Before, `search_design` said nothing matched for words the design holds. The write half still holds (above). | #657 |
| 4. No automatic export | A writing call updates the committed export before it exits: the file `--export-to` names, else the one the project's MCP configuration names. Exit 3 means the write landed and the export did not. | #661 |
| 5. No up-front schemas for arguments | `--describe <tool>` prints the input schema with every nested shape, allowed value and required field. `--list-tools` covers every tool, and `describe_schema` takes `tool`. | #667 |
| 6. Lessons attached to tools are invisible | `--describe` carries the lessons this design holds for the tool, the ones `tools/list` appends. | #667 |
| 14. A hub can't see a member's unexported changes | **Narrowed, not closed.** A door write now updates the export a hub's export watch reads. The watch is still only as current as that file: a write made with `--no-export`, or in a project that names no export file, stays invisible to it. An address watch sees unexported work. (`fact:an-export-watch-reads-unchanged-over-unexported-work-and-an-address-watch-sees-it-2026-10-02`) | #661 |
| 16. A call in a folder whose `.reflow2.toml` names a design on a server created a stray local design (this item's text is cut short in the report) | The door refuses there with exit 1, names the design's id and address, and creates nothing. A read where there is no design refuses too. | #654 |
| 17. A refused write is not always atomic | A refused write stores nothing: each write call is one unit. A node such a refusal left behind before v0.78.0 is still in the design. | #655 |
| 18. A narrowed schema leaves stored data unchecked | `detect_defects` names every stored item the current schema refuses (category `refused_by_schema`), and the edge the import names as its replacement where it knows one. | #660 |
| 19. No edge reader | `get_node` with `"include_edges": true` lists every edge in and out, with its evidence. `find_tools` ranks `get_node` first for "a node with its edges". | #659 |
| Found in the triage: `--read-only --call` wrote | `--call` honours `--read-only`: a tool that writes is refused by name, and nothing is written. | #654, #658 |

### Tool friction found along the way, and where each stands

- **A string where a list is expected came back as a bare deserialize error.** Fixed (#656): an argument
  refusal names the tool, the field path and what the schema expects there, and exits 2.
- **`external_dependency` replied with the whole dependency manifest.** Fixed (#662): it replies with a
  receipt. Read the manifest with `reconcile_dependencies`.
- **`find_tools` did not return `get_node` for "read one node by id with its properties and edges".** Fixed
  (#659): it ranks first.
- **`add_epoch` refuses a missing `sequence` on every first epoch.** Still true on main. The refusal names
  `sequence` and `epoch_type` in one round trip, and the published schema still requires only `id`.
  (`fact:add-epoch-create-requirements-are-published-nowhere-and-describe-schema-says-only-name-is-required-2026-10-02`)
- **`add_decision` link items need `evidence`, and the refusal said the schema "publishes no description of
  it".** Fixed (#656): the refusal quotes the field's description. `relation` takes the review relations only
  (`DEPENDS_ON`, `ANTICIPATES`, …), by design; `--describe add_decision` lists them.
- **`set_artifact_checksums` `disposition` values appeared only after a wrong one.** `--describe
  set_artifact_checksums` lists them before the first call (#667).
- **`TemporalFact.basis` is `measured | forecast`, and the provenance words were refused.** Unchanged, and not
  the door's: those words belong to a different property, `provenance`
  (`fact:root-cause-basis-names-four-value-sets-around-a-temporal-fact-and-provenance-a-fifth-2026-10-02`).
- **`coverage_report` wants `{path}` records, and a design that registers commit-pinned URLs claims 0 files.**
  Half fixed. The item shape is published and a string entry is refused naming the tool and the field (#656).
  A location that is a URL is still compared with the swept paths as a literal string
  (`fact:root-cause-coverage-report-reads-an-artifact-location-as-a-literal-path-a-meaning-the-shared-location-rule-never-reached-2026-10-02`).
- **A Verification recording a test collection error read as "did not work as designed".** Fixed (#662):
  record the outcome `blocked`, which raises `blocked_verification` and says nothing is known about the part.
- **`export_graph` with no `path` prints the whole design to stdout.** Still true, and documented: without a
  `path` it answers in the reply. Pass a `path` (through `reflow2 write`) to write a file and get a receipt.
  (`fact:root-cause-export-graph-with-no-path-is-a-documented-default-exempt-from-the-reply-bound-and-the-door-prints-27-mb-2026-10-02`)

## Every idea, and where it is recorded

Each idea is an exploratory Decision in reflow2's design, or is linked to the finding it answers. Ideas 1 to
11 were first recorded from this guide's 2026-10-01 version, a day before the report was frozen. The
2026-10-02 triage cited those records rather than recording the ideas again, and the report was linked to
them on 2026-10-05
(`fact:the-door-reports-carried-over-ideas-were-recorded-from-its-first-version-and-linked-only-in-prose-2026-10-05`).

| # | Idea, in short | Record in reflow2's design | Where it is |
|---|---|---|---|
| 1 | A served CLI-door harness in `reflow2 init`, with VS Code hook files | `dec:idea-the-call-door-becomes-a-supported-harness-for-an-agent-that-cannot-use-mcp`; `req:init-installs-the-terminal-route-for-vs-code-and-update-keeps-it-current` | **Built** (#680; `dec:the-vs-code-setup-is-built-as-one-increment-2026-10-05`) |
| 2 | `--call` joins a running shared server instead of refusing writes | `dec:idea-a-one-shot-call-reaches-the-design-where-it-is-served` | Open |
| 3 | A writing call exports | `dec:idea-a-writing-call-exports-afterwards` | Shipped in v0.78.0 |
| 4 | A read/write verb split | `dec:idea-a-shell-driven-agent-approves-reads-once-and-confirms-each-write` | Shipped in v0.78.0 |
| 5 | `--describe <tool>` / `--list-tools` | `dec:idea-the-cli-describes-a-tool-with-its-schema-and-lessons` | Shipped in v0.78.0 |
| 6 | `--call-batch`: calls in one process, one approval, stop at the first refusal | `dec:idea-a-shell-driven-agent-approves-reads-once-and-confirms-each-write`; `req:a-batch-of-calls-runs-under-one-approval-and-one-export` | Planned (step 6) |
| 7 | Skill stubs in `.github/skills/` that route to `get_skill` | as idea 1 | **Built** (#680) |
| 8 | `REFLOW2_HARNESS=vscode` under `--call` | as idea 1 (the requirement's attribution clause) | Planned; not in the increment being built |
| 9 | A VS Code extension registering Language Model Tools | `dec:idea-where-an-org-blocks-third-party-mcp-ask-for-an-allowlist-before-building-around-it` | Not pursued (2026-10-01) |
| 10 | An MCP registry allowlist | as idea 9 | Asked for, and not available in the organisation this report comes from. It stays the clean route wherever an admin allows it. |
| 11 | A CI probe for the door (`tools/test_call_door.py`) | as idea 1 | **Built** (#680) |
| 12 | Ship VS Code hooks (`SessionStart` → `loop_status`, `Stop` → export) | `dec:idea-how-reflow2-triggers-the-loop-for-a-call-door-agent-in-vs-code` | **Built** (#680) |
| 13 | A hub address book | `dec:idea-a-hub-on-one-machine-can-say-where-each-tracked-design-is-reached-from-the-door` | Open |
| 14 | Warn on a never-exported design | `dec:idea-loop-status-says-whether-an-export-is-owed-on-a-hosted-design` (linked as a duplicate) | Open |
| 15 | Refusals name the tool and the field | `dec:idea-every-argument-refusal-names-the-tool-and-the-field-path` | Shipped in v0.78.0 |
| 16 | Atomic typed writes | `dec:idea-a-refused-typed-write-stores-nothing` | Shipped in v0.78.0 |
| 17 | Re-check stored data on a schema change | `dec:idea-stored-data-is-rechecked-against-the-current-schema` | Shipped in v0.78.0 |
| 18 | An edge reader | `dec:idea-an-edge-reader-returns-one-nodes-edges-and-find-tools-finds-it` | Shipped in v0.78.0 |
| 19 | Receipts everywhere | answered as a finding on `cap:a-write-replies-with-a-receipt` (`fact:root-cause-external-dependency-replies-with-the-whole-manifest-because-the-receipt-shapes-only-node-and-edge-records-2026-10-02`) | Shipped in v0.78.0 for `external_dependency` |
