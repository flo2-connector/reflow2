### Added

- **`loop_status` says when a design has never been exported, first, and names the command that
  fixes it.** A design whose store holds nodes and has no copy anywhere this machine knows of (no
  record of any export, no `--export-to`, no export named in the project's MCP configuration that
  exists on disk) now gets a high-severity line at the top of `next`, beginning "THIS DESIGN HAS
  NEVER BEEN EXPORTED", and an `export_standing` block. The line names the one command that writes
  a copy: `export_graph` with a path in an MCP session, or the full `reflow2-mcp … --call
  export_graph` command through the `--call` door and `reflow2 read`. `graph_report` carries the
  same block, and `graph_report_markdown` leads with the line. A design whose every earlier export
  has gone gets the same item as `no_export_survives`.
  - Why: a field log on 0.76.0 found a 195-node design held only in its machine-local store, "one
    disk away from loss", while `loop_status` answered `clean: true` and an empty `next`. The only
    export sentence `loop_status` had was computed from the exports a store had already made, so a
    store with none said nothing — on every door. The `--call` door made it the common case.
  - It is additive. `clean` and every existing field mean what they meant. A design that has a
    copy gets exactly the reply it got before. A design served without its project tree (a host's)
    is never told: there the host keeps the backup.
  - **What to do:** if you see it, run the command it names, and commit the file. If a design is
    meant to stay on its machine only, record that once: `acknowledge_gap` with `gap_id`
    `gap:the-design-has-never-been-exported`, `affected_ids: []`, your reason and your
    `approver`. The line then stops; `withdraw_gap_acknowledgement` brings it back.
- **`loop_status` with `since_export: true` lists what the store holds that its export does not,
  grouped by who wrote it.** A new `unexported` block compares the store with the export it is in
  step with and groups the changes by the contributor the store credits (a session that declared
  `writes_for`), the agent it went through (`authored_via`) and the epoch, with counts, up to five
  ids per group and the dates the items carry. At most eight groups are shown, and the rest are
  counted. Changes nobody was credited with, and every removal, are counted under `written_by: null`
  rather than guessed. `compare_designs` with `base_path` still lists every change in full. The
  plain `loop_status` line about unexported work now says where this list is.
  - Why: the same field log found a store holding another session's uncommitted work, so an export
    meant to commit one session's changes either swept the other's in or was skipped (twice in one
    day).
  - It is a list to read before exporting, not a partial export. An export still carries the whole
    store, because exporting one group alone could leave edges pointing at nodes it left out.
  - **What to do:** to see whose work is unexported, call `loop_status` with `since_export: true`.
    For writes to be listed under a name, have each session declare `writes_for` (a write made
    with none in force is listed with no writer).
- **A store ahead of its export says so in the first line of `loop_status`.** A new
  `ahead_of_export` field carries one line, such as "ahead of docs/design/p.json: 3 node(s) not in
  the export". Through `--call` and `reflow2 read` it is the first line printed, ahead of the
  artifact block. By default it counts nodes, because that reading is free. With `since_export:
  true` it counts nodes and edges added, changed and removed, and names who wrote them. A design
  never exported says that instead. `sync_status` carries the same line in its own new
  `ahead_of_export` field, and each record's `state` keeps its meaning. The field is absent when the
  store is not ahead.
  - Why: upgrading 20 stores to 0.79.0, the recipe `--call loop_status | head -40` showed only the
    artifact block (a member store's remote, unmeasurable files). `sync_status` reports being
    ahead of the record by design as no state at all. The check was done by hand, by exporting to
    a scratch file and comparing.
  - **What to do:** read `ahead_of_export` at the top of the reply. Pass `since_export: true` for
    the exact count and whose work it is.
- **The `/debt` command reads `next`, not only `clean`.** It says "nothing owed" only when `next`
  is empty too, so a design that has never been exported is not reported as owing nothing.
  **What to do:** run `python3 tools/reflow2_init.py <your project>` to update the installed
  command.
