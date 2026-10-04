### Changed

- **reflow2's own committed design moved from `docs/design/reflow2.json` to the item layout,
  `docs/design/reflow2/`.** For contributors to reflow2; nothing changes for users. Two pull
  requests that change different parts of the design no longer conflict on it, so they merge in
  either order without rebuilding and replaying a record. The 579 registered artifacts' accepted
  checksums moved onto one baseline change, so from here every acceptance lives on the change that
  made it. Reassembled, the layout's whole-design hash equals the single file's last one, so a
  watch or pin taken on the old file reads the move as one change, not as a different design.
  **What to do if you work on reflow2:** `git pull`; point a local server's `--export-to` at
  `./docs/design/reflow2/` (a trailing `/` names the layout); export as often as you like; bring
  main in with `git merge origin/main`; write your CHANGELOG entry as a `changelog.d/` fragment. A
  branch opened before this landed re-records once: merge main, import `docs/design/reflow2/`
  into its throwaway store, replay its writes, and export to the directory.
