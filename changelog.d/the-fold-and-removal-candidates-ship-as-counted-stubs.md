### Deprecated

- **Fifteen tools are retired as counted refusing stubs, on the owner's word
  (Anthony, 2026-10-30, "Yes, go with B").** Each name stays listed and
  callable, changes nothing when called, and answers with the exact call that
  now does its job — or, for a retired report, that it was retired and to say
  so if it was needed. Every call to one is counted in the usage ledger under
  its own name, and `usage_report` lists each stub with its count, zeros
  included, under `retired_stubs`. **The re-count is about 2026-12-23** (the
  90-day re-run of the flo2.io usage baseline): a stub nobody called is deleted,
  and one somebody called is restored as the tool it was. Why: the 2026-10-30
  counts found six tools whose job a twin already does and nine reports nobody
  reached, and an absent tool is silent (an agent works around it and says
  nothing) while a listed stub turns "it was missed" into a count. The computing
  code stays in place until the re-count, so a restore re-points the handler.
  find_tools does not offer a stub, and the served skills, instructions and
  replies name the survivor instead, so the count measures need rather than the
  text pointing at it. **MCP users (Alex among them) hear with this release**: a client of 0.95.0
  that calls one of these names gets the refusal below, with what to do.
- **`set_artifact_checksum` folds into `set_artifact_checksums`.**
  **What to do:** call `set_artifact_checksums` with a one-item `accepts` list; each item
  takes the same fields (`artifact_id`, `checksum`, `disposition`,
  `change_type`, `design_change_event_id`, `note`, `at`). The stub hands back
  that call, built from the arguments it was given.
- **`acknowledge_gap` folds into `acknowledge_gaps`.** **What to do:** call
  `acknowledge_gaps` with a one-item `gaps` list (`gap_id`, `reason`,
  `affected_ids`, `approver`, `acted_at`).
- **`dimension_drift` folds into `dimension_drifts`.** **What to do:** call
  `dimension_drifts` with `target_id` and `dimension`, which now reads one
  node's drift on one dimension exactly as `dimension_drift` did (either alone
  narrows the sweep).
- **`propagate_change` folds into `propagate_from`.** **What to do:** call
  `propagate_from` with the ChangeEvent's id as its one seed (`seed_ids:
  ["chg:…"]`), which now reads that change's blast radius from the nodes it
  CHANGED, with the stale edge prose beside it, exactly as `propagate_change`
  did (`max_depth`, `full`, `offset` and `budget_chars` unchanged). Its reply
  says `seeded_by_change`.
- **`verifies` folds into `add_verification`.** **What to do:** call
  `add_verification` with the check's `id` and `verifies: [{"target_id": …}]`;
  on an existing check every field you leave out keeps its stored value. In
  `draw_edges`, an item naming `verifies` (or `gate_on`) is refused with the
  stub's answer.
- **`graph_report_markdown` folds into `graph_report`.** **What to do:** call
  `graph_report` with `format: "markdown"`, which returns the same document.
  Without `format` (or with `json`) `graph_report` answers exactly as before.
- **Nine reports are retired: `evidence_report`, `forecast_readiness`,
  `frontier`, `ility_report`, `granularity_report`, `consumption_report`,
  `surprising_connections`, `readiness_report` and `gate_on`.** Each answers
  that it was retired on 2026-10-30 at the owner's word and that nothing was
  computed or written. **What to do:** nothing replaces them; if you needed
  one, say so to the person you work for (report-friction records it) — a
  retired tool somebody called is restored at the re-count. What other tools
  computed from them is unchanged: `graph_report` still carries its
  granularity and surprising-connection sections, and the derived relations
  still read evidence and readiness.

### Changed

- **The first-read cue names `graph_report` with `format` `markdown`.** The
  connect-time instructions, `get_instructions` step 0a, the VS Code terminal
  route and both SessionStart texts read "read `graph_report` with `format`
  `markdown` first". **The cue's id changes** from
  `instructions/graph_report_markdown` to `instructions/graph_report` (a cue id
  is `<field>/<named call>` and never its arguments), so a count compared across
  this release reads the two ids as one cue.
- **Three pairs that read alike say which is which** in their first words:
  `report_manual_work` WRITES one record and `manual_work_ledger` READS them
  all; `add_epoch` is the past and `plan_epoch` the future; `scan_nodes` lists
  every node of one TYPE and `search_design` finds nodes by what they SAY. No
  name changes.
- **The remote client no longer measures for the retired one-artifact
  accept.** A `set_artifact_checksum` call is forwarded as it came, so the
  server answers it and counts it; `link_artifact`, `set_artifact_checksums`
  and `reconcile_artifacts` are measured as before.
- **In your setup.** The `--call` setup (a one-shot call on a design on your
  own machine, as in VS Code where MCP is blocked) gets every change with the
  binary: `reflow2 read graph_report '{"format": "markdown"}'` prints the
  document, a call to a stub exits 1 with its answer and is written to the
  ledger under the stub's name, and `--call usage_report` shows
  `retired_stubs`. Checked on this branch's build; re-check on the release
  build at the cut.
