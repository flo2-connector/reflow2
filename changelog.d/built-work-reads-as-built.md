### Added

- **A Component says what its evidence shows built: `built_reading` on `get_node` and
  `scan_nodes`.** A component's `status` is kept by hand and starts at `planned`, and nothing
  moves it when the work ships, so on reflow2's own design 107 of 109 components read `planned`
  while most had built work behind them. Every Component now also carries `built_reading`:
  `reads_as` is `built`, `partly_built` or `not_built`, with the evidence counted
  (`capabilities_allocated`, `capabilities_built`, `files_built`, `checks_passing`). It is
  computed on each read from what the design already records: the capabilities allocated to the
  part and their status, a realized file, a passing check. The stored `status` is never touched;
  moving it stays the owner's call. `understated_status` now reads the same computation, so the
  finding and the reading always agree, and its message says how many components read as built
  by their evidence. `derived_report` lists it as the `built` relation.
- **`loop_status` with a `contributor_id` lists what awaits an owner's word: `awaiting_word`.**
  Two kinds of built work whose record is still open: a `proposed` requirement its thread already
  delivers, and an open idea whose promotion has landed (the ones `decision_overtaken_by_promotion`
  reports). Each item names what delivers it (`evidence_ids`, `evidence`) and the call that
  closes it (`closed_by`). The list is design-wide, so it is not counted in that person's `clean`,
  and the scoped `next` says so. It moves no status. Unscoped, `awaiting_word` is an empty list,
  as `gaps_on_owned_ground` and `questions_put_to_them` are, so the call every session makes costs
  what it did.
- **`loop_status` and `scan_nodes` take `budget_chars` and fit a 30,000-character reply by
  default.** A host such as flo2.io holds every reply to 30,000 characters and refuses a larger
  one; it asks a tool for less only when the tool takes `budget_chars`, and neither did. The
  scoped `loop_status` would have answered 38,939 characters on reflow2's own design once
  `awaiting_word` was added, and every `scan_nodes` of a large type already answered 37,000 to
  40,000. Over its budget, `loop_status` cuts `awaiting_word`'s long names and evidence sentences
  first, then withholds its last rows, and only then shortens prose elsewhere; every count, id and
  closing call stays, and the reply's `budget` block says what was withheld and how to see the
  rest. `scan_nodes` now counts its whole reply against the budget, and pages sooner: by default
  about 30,000 characters per page instead of 40,000 characters of nodes. As before, `capped_by`
  and `next_offset` say where to resume.

### Changed

- **The ship step now closes the idea it answers.** The link-artifacts and capture-session skills,
  and step 3 of the served instructions, now say: when the work you ship answers a recorded idea,
  draw `EVOLVES_INTO` from that idea to the requirement or capability it became, and leave the
  idea's status for its owner. Without that edge nothing can tell the idea was answered, and
  `detect_gaps` cannot put it to the owner. On reflow2's own design, 47 of 59 ideas found already
  built had no such edge.

  **What to do:** nothing. Replies gained fields and no field changed or went away. A
  `scan_nodes` caller that read a whole large type in one call now pages one call sooner: follow
  `next_offset`, or pass a larger `budget_chars`.
