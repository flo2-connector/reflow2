# Using reflow2 in VS Code when MCP is blocked — the `--call` door

Written 2026-10-01 on the work machine (`lps-172683`), reflow2 `0.76.0` installed at
`~/.local/bin/reflow2-mcp`.

## The problem

The work GitHub Copilot subscription blocks third-party MCP servers, so VS Code's Copilot agent
cannot register reflow2 through `.vscode/mcp.json`, which is the path `reflow2 init` installs.
reflow2's tools never appear in the agent's tool list. Claude Code on the same machine is not
affected and still runs reflow2 over MCP. A `--serve-shared` daemon was running for
`hxm_program` during this session.

## What we did

Nothing in reflow2 changed. The binary already has a one-shot CLI door, `--call`, added for build
scripts (bhome, 2026-09-18). It runs any served tool once through the same server code path a
session uses (an in-process client over an in-memory pipe), so refusals and replies are worded
the same:

```bash
RUST_LOG=error reflow2-mcp --graph-path .reflow2/graph --call <tool> --args '<one JSON object>'
# reflow2 --graph-path ... --call ...   also works: the wrapper passes unknown flags through
```

Exit 0 means the reply JSON is on stdout. Exit 1 means a refusal on stderr. Exit 2 means the tool
marked its own reply an error. `--args -` reads the object from stdin (heredoc).

We then taught Copilot to use the door with a **user-scope VS Code instructions file**, which
applies to every workspace on the machine:

`~/.config/Code/User/prompts/reflow2-cli.instructions.md` (`applyTo: '**'`)

That file tells the agent to:

- act only when the workspace has `.reflow2/` or `REFLOW2.md`;
- treat every "call `X`" in `REFLOW2.md`, a skill or a tool reply as `--call X`, and never
  reimplement a tool or hand-edit `.reflow2/` or an export;
- discover tools with `--call find_tools --args '{"query":"…"}'`;
- fetch skills from the binary with `--call list_skills` and
  `--call get_skill --args '{"name":"…"}'`;
- start a session with `--call loop_status` and then the `where-am-i` skill;
- handle the two behavioural differences from MCP: the single writer and the missing
  automatic export (both covered below);
- still wait for the user's explicit word before accepting a Requirement or a Decision.

### What was verified, not assumed

| Check | Result |
|---|---|
| `--call graph_report` on a scratch graph | Works. About **0.2 s** wall time per call, including opening the store. |
| `--call find_tools --args '{"query":"record a new requirement"}'` | Returns ranked tools with **parameter names** and summaries. |
| `--call add_requirement --args '{}'` | Exit 2. The message names the missing `id` and what it should look like. |
| `--call get_skill --args '{}'` | Exit 2. Names the missing `name` argument. |
| `--call export_graph --args '{"path":…}'` twice | First call writes the file and returns `content_hash`, `chained_from`. Second call is refused: *"already exists — … Pass overwrite=true"*. |
| `--call loop_status` on `hxm_program` while Claude Code's daemon held it | Answered from a **best-effort snapshot**, with a loud stderr warning. |
| `--call add_requirement` on that held graph | **Refused**: *"writes, so it needs the graph itself and not a snapshot copy … another process already has the design graph open"*. Nothing was written. |
| `RUST_LOG=warn` | Still prints an `rmcp` WARN line on every refusal, so the instructions use `RUST_LOG=error`. |

## Limitations and issues

1. **reflow2 is not a tool in Copilot's tool list.** The agent reaches it only through the
   terminal, and only because an instructions file says so. Discovery depends on that file being
   loaded and followed. Nothing in VS Code shows reflow2 as available.
2. **VS Code asks for approval on every call.** Each `--call` is a terminal command, so VS Code
   prompts each time unless `chat.tools.terminal.autoApprove` matches it. One regex cannot tell
   a read from a write, because both look like `reflow2-mcp … --call <tool>`. So it is either
   approve everything or approve each call.
3. **One writer at a time, and `--call` cannot join a shared daemon.** If a Claude Code session
   (or any `--serve-shared` daemon) holds the graph, VS Code can read a snapshot but cannot
   write. The only remedies are closing that session or `--stop-shared`, which cuts the other
   session off. `--call` opens the store directly and never goes through the `--shared` proxy
   (`call_one_tool` in `crates/reflow2-mcp/src/main.rs`).
4. **No automatic export.** `--export-to` write-through starts only in a long-lived server. The
   `--call` branch exits before `start_auto_export` is reached. After writes, the agent has to
   remember to run `export_graph` with `overwrite: true` and the right path. Forgetting is the
   exact loss class the write-through was built to end.
5. **No up-front tool schemas.** `find_tools` gives parameter names but not types, required-ness
   or descriptions. The agent learns the shape from exit-2 refusals: correct, but one round trip
   per mistake.
6. **Lessons attached to tools are invisible.** Since 2026-09-12 a `DesignRule` or `TemporalFact`
   with `steps` is appended to the named tool's description in `tools/list`. Under `--call` the
   agent never reads `tools/list`, so per-tool lessons never arrive. Per-skill lessons still
   arrive through `get_skill`.
7. **Skills are not native in a consumer project.** VS Code auto-discovers `SKILL.md` files (it
   lists this repo's `.claude/skills/*`), but a consumer project installed thin has none on disk.
   Skills arrive only if the agent calls `get_skill`. Slash commands such as `/genesis`, `/jot`
   and `/feedback` do not exist in VS Code chat.
8. **Each call is a fresh session.** Anything scoped to a session (seat identity, claim liveness,
   per-session nudges) does not carry between calls. *Not measured this session.* Claims made
   through `--call` may read as `gone` immediately; check before relying on `parallel-work` from
   VS Code.
9. **No loop nudges.** Claude Code has hooks and OpenCode has the loop-nudge plugin. VS Code has
   nothing that prompts `loop_status` at a boundary, so the loop runs only as far as the agent
   remembers it.
10. **Usage telemetry cannot tell VS Code from a Makefile.** Every `--call` identifies as
    `reflow2-mcp --call`, so `usage_report` and `/feedback` cannot attribute calls to the VS Code
    harness.
11. **The setup is one person's local file.** The instructions live in one user's VS Code profile.
    `reflow2 init` / `reflow2 update` do not install or refresh them, and they will drift from
    the binary as the tool surface changes.
12. **Shell quoting.** JSON in single quotes breaks on apostrophes in statements. Agents need the
    `--args -` heredoc form for any real prose, and the instructions say so.

## Ideas for improvement

Ordered roughly by value for the least work.

1. **A served CLI-door instructions file from `reflow2 init`.** Add a harness (e.g.
   `--harness vscode-cli`, or auto-offer it alongside `vscode`) that writes
   `.github/instructions/reflow2.instructions.md` with the content above, served from the binary
   the way the working instructions already are (`req:thin-install`), so `reflow2 update` keeps
   it current. This fixes limitation 11 and makes the setup committable for a team.
2. **`--call` joins a running shared daemon.** When the graph is held by a `--serve-shared`
   daemon, route the call through it (the same proxy `--shared` uses) instead of refusing writes.
   VS Code and Claude Code could then work the same design at once. This fixes limitation 3 and
   is the biggest functional gap.
3. **`--call … --export-to FILE`.** After a successful *writing* call, export once to the committed
   path with the same chaining and tamper guard as the write-through. Better still, read the path
   from `REFLOW2.md` / project config so the agent does not have to know it. This fixes
   limitation 4.
4. **A short, approvable verb split by read vs write.** For example `reflow2 read <tool> [json]`
   would refuse any tool not annotated `read_only_hint`, and `reflow2 write <tool> [json]` would
   handle the rest. Both would default `--graph-path ./.reflow2/graph` and quiet logging. Then
   `chat.tools.terminal.autoApprove` can safely auto-approve `^reflow2 read ` and leave writes
   prompted. This fixes limitation 2 and shortens every command line.
5. **Describe a tool on the CLI.** `--describe <tool>` (or `--call describe_tool`) would print the
   full input schema *and* the lessons `tools/list` would have appended. This fixes limitations 5
   and 6. A `--list-tools` that dumps the whole served list as JSON would let an instructions
   generator pre-render a schema cheat-sheet.
6. **Batch calls in one process.** `--call-batch` would read JSONL of `{tool, args}`, run them in
   order against one open store, and stop at the first refusal. That means one approval, one
   store open and one export for a capture burst (genesis seeds dozens of nodes).
7. **Skill stubs VS Code can discover.** Have init write `.github/skills/<name>/SKILL.md` stubs:
   the frontmatter (name, description) plus one line, *"run `reflow2 --call get_skill --args
   '{"name":"<name>"}'` and follow it"*. VS Code would then route to the right skill by
   description, the body would stay served (thin install preserved), and prompt files could map
   `/genesis`-style commands. This fixes limitation 7.
8. **A harness name for the door.** Accept `REFLOW2_HARNESS=vscode` (or `--harness`) under `--call`
   so `usage_report` attributes calls correctly. This fixes limitation 10.
9. **A VS Code extension that registers Language Model Tools** (`vscode.lm.registerTool`). Each
   served tool would become a first-class Copilot tool with its schema, backed by `--call` or a
   long-lived local process. This gives the closest thing to MCP without MCP, and could keep one
   server open, which would also fix limitations 3, 4 and 8. **Check with IT first:** if the
   policy's intent is "no unvetted agent tools" rather than "no MCP transport", an extension that
   reintroduces the tools is the wrong answer.
10. **Ask the admin about an MCP registry allowlist.** If the org policy is registry-based rather
    than off entirely, adding reflow2 to the allowed registry restores the normal setup, and
    every limitation above goes away.
11. **A CI probe for the door, the way a harness is tested.** For example
    `tools/test_call_door.py`, alongside `test_opencode_plugin.py`, would drive the binary the
    way the instructions tell an agent to:
    - discovery via `find_tools`;
    - an exit-2 refusal that names the missing argument;
    - a write on a free graph;
    - a refused write on a held graph;
    - a snapshot read on a held graph;
    - an export.

    Then the VS Code path cannot regress silently.
