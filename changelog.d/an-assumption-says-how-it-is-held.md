### Added

- **An assumed premise says how it is held: `add_inference` takes `held_as`.**
  `supposition` (a proof's hypothesis, which a later step must discharge),
  `principle` (an argument's chosen starting point) or `model` (a model's
  idealization, held on purpose), written on each assumed premise's link
  (PREMISE_OF `held_as`). Only a supposition left undischarged is a
  `reasoning_not_as_stated` gap. A principle or a model is counted in
  `loop_status`'s `reasoning.assumptions_held_on_purpose` (with its ids), and
  what rests on it still reads conditional on it; reasoning_report's
  `assumptions` carry `held_as` and read "held on purpose". Before, a physical
  model's nine stated idealizations read "THE REASONING IS NOT AS STATED: 9
  assumption(s) never discharged", and so did an argument's governing
  principles. `held_as` is refused with nothing in `assumed`, and as
  `principle` or `model` for a statement of kind `hypothesis`. **What to do:**
  nothing for an existing design: an assumption whose step says nothing reads
  as a supposition, exactly as before. Mark a model's or an argument's
  assumptions with `held_as` to stop them reading as not as stated.
- **`create_nodes` and `create_node` say no near-match check ran.** Their
  replies carry `near_match_check`, which says the generic path runs none (by
  design, so a bulk seed is never blocked) and names the reads that check:
  `search_design` per item before writing, a typed tool, or
  `relation_candidates` after. Before, the reply was silent, and silence read as
  "nothing close". **What to do:** nothing.

### Fixed

- **`reasoning_report` keeps to its budget, and its summary keeps its names.**
  The summary's room is reserved before the claims page is filled, and its
  names are cut to no fewer than 80 characters, never to "…". When one claim
  alone will not fit beside the summary, the page returns no claim, the
  summary, `next_offset` at that claim, and `page.claim_needs_budget_chars`,
  the budget that claim needs; `page.reads` says so. Before, at budget_chars
  6,000 the reply kept a 7,300-character claim whole, blanked all 20 summary
  fields, and answered 9,529 characters. A budget below the summary's own floor
  (every count, one entry per list, names at 80 characters) is answered at that
  floor, and `page.reads` says so. **What to do:** when `claims` is empty and
  `next_offset` is set, ask again with the budget the page names, or read the
  claim alone by its id.
