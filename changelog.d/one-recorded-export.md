### Fixed

- **A project records where its export lives in one place, `.reflow2.toml`, and everything reads
  it.** On the VS Code terminal route three parts each found the export their own way and
  disagreed:
  - every `reflow2 write` said "no export was kept current", because it looked only at
    `--export-to` and MCP configurations, and that route has none;
  - the end-of-turn hook exported to the path in `init`'s git-ignored receipt;
  - `reflow2 init` invented `docs/design/<project>.json`, with `.gitattributes` and `.gitignore`
    lines for it, beside an export the project already kept.

  Now `.reflow2.toml` holds an `[export] path` that writing calls, the hook, `reflow2 init` and
  `reflow2_check.py` all read. `init --harness vscode-cli` records the export the design already
  has: one an MCP configuration names, or a committed export of this design. It invents
  `docs/design/<project>.json` only when there is none. A `.reflow2.toml` with only an
  `[export]` table leaves the folder a local design. Projects set up for MCP are unchanged.
  **What to do:** on the terminal route, re-run `reflow2 init . --harness vscode-cli` (it
  records your existing export), or add one yourself:
  `[export]` / `path = "reflow2.json"` in `.reflow2.toml`. Then delete any
  `docs/design/<project>.json` that `init` created by mistake, with its `.gitattributes` line.
- **`reflow2 init --check` names everything the run will do.** It used to leave out the design
  record, the `.gitattributes` line and the merge driver, which the run then wrote. A test now
  fails if the run touches a file the preview did not name.

### Added

- **`search_design` hits carry the node's `status` and `kind`** (when it has them), so you can
  tell settled intent from an open idea without reading each hit.
