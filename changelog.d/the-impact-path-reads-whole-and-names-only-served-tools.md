### Added

- **`propagate_change` takes `budget_chars`, and both blast-radius tools page.**
  A full radius (`full: true`) is held to `budget_chars` (default 30,000) by one
  bounded serializer that `propagate_change` and `propagate_from` share, and
  `offset` reads on: `impacted_page` says the `offset`, how many rows are
  `shown`, the `total` and the `next_offset` to pass back (null on the last
  page). Before, `propagate_change` took no bound, so on a 35-seed change its
  38,395-character radius was withheld whole by a gateway holding replies to
  30,000. The redo list's groups give rows to the budget too, each named by id.
  **What to do:** nothing to keep a call working; pass `offset` with
  `full: true` to read a large radius whole.
- **`set_violation_status` takes `resolved_at`, and `violates_rule` takes
  `detected_at`.** A violation ruled must-fix (`rejected`) leaves the open
  breaches when `set_violation_status` records the date it was fixed; the edge
  is kept for audit. Before, only a hand-written `create_edge` could set it,
  though the open-breach gap told the agent to. `resolved_at` is refused with
  any status but `rejected`.
- **`propagate_change` adds `stale_edge_evidence_silent_on`** (`count`,
  `seeds`, one `coverage_note`): every seed the edge-prose check was silent on
  because no snapshot holds its prior state, named once. Each of those seeds
  keeps its entry in `stale_edge_evidence`, whose `coverage_note` is now a short
  pointer there ("No prior snapshot: see stale_edge_evidence_silent_on.")
  rather than the same paragraph per seed: on a 579-seed change of reflow2's
  own design the copies were 55,599 characters, more than a whole reply's
  budget. Over the budget, `stale_edge_evidence`'s entries give rows like a
  list's, each counted in `budget.lists_shortened`. **What to do:** nothing; the
  explanation is in `stale_edge_evidence_silent_on.coverage_note`.

### Fixed

- **A drawing or a model that DOCUMENTS a part is on that part's redo list.** A
  change's `redo_list` filled `drawings` and `models` only through an Artifact
  that REALIZES a reached part, so a sheet or an IFC model linked to the part
  it shows with `documents` was left off while the note said what a record
  documents counts. And a sheet showing several parts no longer puts the other
  parts on the list as parts to redo. The note now says which link fills each
  group.
- **`loop_status` holds to `budget_chars`, breaches included.** Its bound let
  only `awaiting_word` give rows; with nothing awaiting (every unscoped call) it
  cut every text instead, the to-do lines in `next` with them, and kept every
  breach row: 11,767 characters to a budget of 3,000 on reflow2's own design.
  Every list in the reply now gives rows, from its tail and by what it costs,
  before any text is cut, and `budget.lists_shortened` counts what each one
  left out. `next` is never shortened. A budget smaller than the report's own
  counts is answered at that floor, over the budget, because the counts are the
  answer: the `budget` note says the reply is still that size with every list
  at one row and every long text cut, and why.
- **No served text names `record_change` any more.** It was removed in 0.92.0,
  and five messages still said to call it: the coverage note `propagate_change`
  attaches for a node with no earlier snapshot, the gap about a change that
  does not say its `subject`, the note after a part is moved, the refusal to
  move a plan nobody recorded, and the description of `DesignEpoch.status`.
  Each now names `snapshot_before_change`. A test now reads every served text
  (tool descriptions, the skills and instructions, the schema, and the fixed
  prose of every reply) for a tool that is not served.
- **The open-breach gap names the tool that closes a must-fix violation**
  (`set_violation_status` with `resolved_at`), instead of telling the agent to
  set a property on the edge directly.
