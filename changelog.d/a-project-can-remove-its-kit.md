### Added

- **`reflow2 init --remove` takes reflow2's kit back out of one project, and keeps its
  design.** It deletes the kit files nobody edited and names the edited ones it keeps,
  strips the lines the kit added to `.gitignore`, `.gitattributes`, `.reflow2.toml` and your
  instruction files (`AGENTS.md`, `CLAUDE.md`, `.github/copilot-instructions.md`, …), and
  marks `.reflow2/kit-version.json` removed. The design store and its export are not
  touched, and the `.reflow2/` line stays in `.gitignore` so the store is never committed
  by accident. `--remove --check` previews it. It also says what it leaves on purpose (this
  machine's MCP config entries, the loop-nudge hooks, the git merge driver) and which
  machine files git no longer ignores. **What to do:** to take the kit out of a project, run
  `reflow2 init --remove` in it; `reflow2 init .` installs it again. The `reflow2` command
  passes `--remove` through unchanged, so the installed wrapper needs no update; re-run the
  installer only for the new help line.

### Fixed

- **`reflow2 update` no longer brings back a kit somebody removed.** It used to treat the
  install receipt's presence as "a kit is installed here", so in a project whose kit was
  deleted by hand it wrote the whole kit back (measured in the field: ~72 files in each of
  11 repos, and lines appended to four tracked files), and restoring the receipt did not
  stop the next run. It now refuses, in the words it uses for a project with no receipt,
  when the receipt says removed or when none of the files it records is left. Deleting ONE
  kit file still means "give me reflow2's copy". **What to do:** in a project whose kit you
  removed by hand, run `reflow2 init --remove` once to record it (and strip any lines left
  behind).
- **An update no longer installs every MCP harness when the receipt does not say which.**
  A receipt with no `harnesses` list made a non-interactive `reflow2 update` write
  `opencode.json`, `.vscode/mcp.json`, `.grok/config.toml` and the OpenCode plugin into a
  project that had chosen one harness. It now refuses and names the choice. **What to do:**
  `reflow2 update --harness <names>` once; the receipt remembers the answer. A first
  install with no terminal and no `--harness` still installs every MCP harness, as before.
