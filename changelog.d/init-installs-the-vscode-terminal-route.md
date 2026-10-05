### Added

- **`reflow2 init --harness vscode-cli` sets a project up for VS Code where MCP is blocked.** The
  agent reaches reflow2 through the terminal (`reflow2 read` / `reflow2 write`), and init now
  writes everything it needs under `.github/`, for the team to commit. Until now the route rested
  on one person's hand-written VS Code user file, which init and update could neither install nor
  refresh (the VS Code `--call` field report of 2026-10-02, limitations 7, 9 and 11).
  - `.github/instructions/reflow2.instructions.md`: how to use the two verbs, the exit codes,
    prose on stdin, and what to do after a refusal. It is the text the binary serves as
    `get_instructions` section `vscode-terminal-route`, so `reflow2 update` keeps it in step with
    the binary.
  - `.github/hooks/reflow2.json`: two VS Code hooks running `reflow2 hook vscode`. SessionStart
    runs `loop_status` and puts its reading in the agent's context. Stop writes the committed
    design record when the turn wrote to the design and no MCP configuration keeps it current. It
    never writes over a record that is not a reflow2 export. It also nudges once when a write had
    no loop check after it. Writes are counted from reflow2's own usage ledger. No reflow2 hook
    reads the text of a command, so design prose that mentions `ssh` cannot trip one.
  - `.github/skills/<skill>/SKILL.md`: one stub per served skill, carrying the skill's own
    description so VS Code picks the one that fits, and routing to `reflow2 read get_skill`.
  - `.github/prompts/<command>.prompt.md`: the slash commands, such as `/gaps`, `/req` and
    `/where`.
  - Each Markdown file carries a one-line reflow2 mark with a hash of the rest of it.
    `reflow2 update` refreshes a file whose mark is intact. It leaves alone any file somebody
    edited and any file of their own, even on a fresh clone with no install receipt.
  - Nothing changes for a project on MCP. `all`, and an install nobody answered, still mean every
    MCP harness, and the terminal route is chosen only by name. An init or update for an MCP
    harness writes exactly what it wrote before, byte for byte; `tools/test_init.py` pins that
    against a golden taken from the previous installer. No tool's arguments or reply shape
    changed.
  - `tools/test_call_door.py` drives the door the way a VS Code agent does, and runs in CI.
  - **What to do, for a VS Code user whose organisation blocks MCP:**
    1. Upgrade reflow2 with `tools/install.sh`, which also refreshes the `reflow2` command.
    2. In the project, run `reflow2 init . --harness vscode-cli`. Use `vscode,vscode-cli` if some
       teammates do have MCP.
    3. Commit what it wrote under `.github/`.
    4. Add `"chat.tools.terminal.autoApprove": {"/^reflow2 read /": true}` to your own VS Code
       settings, so every read is approved once and each write still asks. reflow2 never writes
       your settings.
    5. Delete the hand-written user instructions file you used before. The project's files
       replace it.

### Changed

- **The loop nudge's report no longer says "none is possible" for VS Code.** VS Code runs agent
  hooks, and reflow2 installs them with the terminal route. `loop_status` now reads a project's
  `.github/hooks/` file as an installed nudge. Init no longer tells an OpenCode project that its
  nudge is missing beside the plugin it just installed.
