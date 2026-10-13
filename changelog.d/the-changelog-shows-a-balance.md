### Added

- **The changelog now shows what was taken away beside what was added.** `changelog_view`'s draft
  carries a `balance` between its two ends: requirements added against those dropped or deferred,
  capabilities added against those removed or withdrawn, decisions accepted against those reversed,
  checks that went from passing to failing, and the window's `test_failure_fix` changes as the
  failures that came back. Each count says how it was counted. Read release after release, it shows
  whether a design only grows. It never says a design does too much: scope stays your call.
- **A cut keeps the release's status snapshot.** `release_includes_all` with `apply: true` now also
  records the status of every requirement, capability, decision and check, and whether an accepted
  decision has withdrawn it. It is stored on the release as a delta from the previous release's
  snapshot, the first one whole. Its reply's new `status_snapshot` says it was written and how many
  entries it holds. A re-run never rewrites one, and a release whose manifest was frozen before the
  call gets none, because today's statuses are not what it saw.
- **What the balance cannot see, it says.** An end that keeps no snapshot is named in
  `balance.notes` and its counts are absent, never zero. The counts are the net move between the
  two ends, so something added and dropped inside one window is in neither column. `[Unreleased]`
  reads the last deployed release against the live design.

  **What to do:** nothing to switch on. Your next cut keeps the first snapshot, so the balance
  counts from the release after it. Releases cut before this keep none, and the balance names them.
  Filling past releases in from the design copy committed at each tag is planned as a follow-up.
  Both replies only gained a field; nothing an existing caller reads has moved.
