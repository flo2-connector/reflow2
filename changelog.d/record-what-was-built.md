### Added

- **`record_the_build`: record a finished piece of work in one call.** It
  runs, in order, the steps an agent used to make one call at a time: the check
  that passed (`add_verification`, recorded `passing`), the change
  (`add_change_event`, with `answers`), the changed files (`add_artifact` for a
  file not yet registered, `set_artifact_checksums` accepting each on the
  change, and IMPLEMENTS from a test file to the check it `runs`), and the work
  marked built (`set_capability_status` for a capability; a decision is shown
  built by the capabilities in the same list, and `replace` revises its words
  through `replace_text`). Each step returns its own receipt. **All or nothing:**
  a refused step names itself (its number, its name and the tool that refused)
  and nothing is written. It serves job 4 of reflow2's five, "show whether what
  was built meets what was asked". The tools it runs stay served for the
  unusual case. Measured before it was built: agents called `add_change_event`
  then `set_artifact_checksums` 129 times in a month, and the whole sequence 16
  times.
- **`add_change_event` takes `answers`: the findings a change answered, in the
  same call.** Each is a Verification or TemporalFact; an INVALIDATES edge is
  drawn from the change to it, carrying its `note` and the change's
  `detected_at`. `answers: []` says the change answered none. Every finding
  must exist, or nothing is written. `invalidates` still answers a finding
  after the fact. Why: closing a finding was a separate, optional call, and on
  reflow2's own design 316 of 502 defect findings still read open.
- **`loop_status` carries two new lines.** One counts the fixes
  (`defect_fix`, `test_failure_fix`) that name no finding they answered, until
  each names one or says it answered none (`fixes_answering_nothing`). The
  other gives `repair_report`'s two numbers: the standing patches and the
  repairs nobody labelled (`repairs`). Neither is counted against `clean`.

### Changed

- **`find_tools` returns relations only when one scores close to its top
  tool.** A plain tool search no longer carries relations every time: over the
  tool corpus, the share of replies carrying relations went from every one
  (198 of 198) to fewer than half (95 of 199, the corpus now holding the new
  tool's query). Every relation question in the relation corpus still finds
  its relation; the corpus now holds 26 questions, all six of Alex's among
  them. When no relation is close enough, the reply leaves `relations` out.

### Deprecated

- **`record_change` is removed in this release; its name answers with what to
  use for this one release more.** It was renamed `snapshot_before_change` on
  2026-09-18 and has refused every call since. **What to do:** call
  `snapshot_before_change`, which takes the same arguments.
- **`manual_work_report` is removed in this release; its name answers with what
  to use for this one release more.** It was renamed `manual_work_ledger` on
  2026-09-18 and has refused every call since. **What to do:** call
  `manual_work_ledger`, which takes the same arguments.

### Removed

- **The two renamed stubs, `record_change` and `manual_work_report`.** Use
  `snapshot_before_change` in place of `record_change`, and
  `manual_work_ledger` in place of `manual_work_report`. Neither stub was
  called through flo2.io from 2026-09-24 to 2026-10-25. The served tool count
  goes from 200 to 199: one tool added (`record_the_build`) and two
  removed.
