### Added

- **The design can be saved one file per node and one per edge, so git's ordinary merge merges
  it — on GitHub too, with no merge driver.** Give `export_graph` (or `--export-to`) a directory
  path ending in `/`, such as `docs/design/<project>/`, and it writes `nodes/<Type>/<id>.json`,
  `edges/<xx>/<hash>.json` and a small `design.json`, rewriting only the files whose item changed.
  Two branches that changed different parts of the design touch different files and merge with
  no conflict, in either order; a real conflict is one item both sides changed, shown as that
  item's file (take one side of that file, import, write what the item should say, export). Each
  changed item records the hash it had where the branch left the default branch
  (`prev_item_hash`), so you can export as often as you like — before, during or after merging
  main — and a squash-merge still lands each item one step on; an export during a merge anchors
  where the merge will land, and every export re-checks the lineage of the items your branch
  changed, so re-exporting repairs one a merge left stale. The whole-design hash is computed
  when the design is read and equals what the single file stated for the same design, so watch
  baselines and release pins keep working. `taken_at` moves to a git-ignored `taken_at.json`
  beside the items. **What to do:** nothing yet if you keep a single `.json` file — every reader
  (`--import`, `import_graph`, `--diff`, `compare_designs`, `--merge`, the upstream watch,
  `fork_point`, `reflow2_check.py`) now takes either form, and a `.json` path is written exactly
  as before. To move, export once to the directory beside your file, delete the file, and point
  `--export-to` and your CI gate at the directory. reflow2's own design moves in its own pull
  request; `reflow2_init` does not convert a project for you yet.
- **An accepted checksum now rides the change that accepted it.** `set_artifact_checksum(s)`
  writes the checksum onto the accepting change's `CHANGED` edge (`checksum_after`, its
  `checksum_basis`, and an `accepted_seq` saying which acceptance is current). The Artifact keeps
  its `checksum` in the store, but the saved design leaves it off while it equals the current
  acceptance and the import puts it back — so two pull requests that edit one file each write
  their own edge instead of both rewriting one value. You call the tools exactly as before.
- **The coherence gate checks the item layout per item.** `reflow2_check.py` fails on an item file
  that does not match its own `content_hash` or sits at the wrong path (`INTEGRITY`), and on a
  changed item whose `prev_item_hash` is not its hash at the merge-base (`LINEAGE`). Its
  design-vs-build check becomes git-aware for the layout: every registered file a pull request
  changed must have its new checksum accepted in that pull request, matching the file at the PR
  head; on the default branch, every registered file a commit changed must be covered by an
  acceptance in that commit. A pull request need not be up to date with main to pass. The kit now
  ships `tools/design_io.py` beside the gate, which reads the design in either form — keep the two
  files together.
- **CHANGELOG entries are fragments now: one file per pull request in `changelog.d/`.** Write your
  entry as `changelog.d/<a-few-words>.md` under a Keep a Changelog heading (`### Fixed` and so on)
  instead of editing `CHANGELOG.md`; `python3 tools/changelog_fragments.py --check` (in CI) fails a
  malformed fragment or an entry written straight into `[Unreleased]`. At the cut,
  `tools/changelog_fragments.py --cut <version>` assembles them into `CHANGELOG.md` and removes
  them. For contributors to reflow2; nothing changes for users.

### Changed

- **reflow2's own `.gitattributes` no longer routes the design export through `--merge-driver`.**
  The driver still exists, and stays useful (and optional) for a project whose design is a single
  file. reflow2's contributor instructions (AGENTS.md, COORD.md) and the served `parallel-work` and
  `ci-gate` skills now describe both forms: the item layout needs no driver and no "export once,
  last" rule; a single file still does.
