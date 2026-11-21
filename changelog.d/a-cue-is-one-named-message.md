### Added

- **Every reply that offers a next step carries one `cues` block.** At the top
  level of its structured content, `cues` is `{"items": [...], "left_out": n}`:
  at most three cues, each `{cue, type, call, args?, size_chars?}`, in rank
  order. `cue` is the id the usage ledger counts it by (`<field>/<call>`, as
  before); `type` is `next_call` (from `hint`, `next_step`, `nudge`,
  `carry_it`), `owed` (from `loop_hint` and `loop_status`'s `next`), `lesson`
  (a lesson this design holds for the step that the reply withheld to fit,
  read with `get_node`) or `related` (the nearest of a write's
  `search_first.near_matches`). **The counts choose what goes in:** a cue
  measured as followed ranks first, the most often followed within five calls
  first (`mint_seat`'s pointer to `claim_region`, 38 of 38; `add_change_event`'s
  step to `set_artifact_checksums`, 67 of 118; `loop_status`, 17 of 228; the
  owner's settling cue, 16 of 320); a cue offered at least 20 times and never
  followed is left out of the block (`loop_status`'s cues to `repair_report`,
  0 of 67, `detect_defects`, 0 of 64, the check-health skill, 0 of 51,
  `answers` and `open_questions`, 0 of 46 each); uncounted cues follow, by
  type. The counts are `usage_report`'s on flo2.io for reflow2's design and
  flo2's, 2026-10-31 to 2026-11-21, summed; `carry_it` was not counted there
  yet, so its figure is the call log's to 2026-10-30. **Every read the block names is asked
  for with a small budget** (`args.budget_chars` 4,000, or `args.limit` 5 for a
  search), or says its size (`size_chars`, for a skill, whose body comes
  whole), or is left out, so following a cue never spends most of a small
  reading budget. The block is held to 600 characters and never pushes a reply
  past the reply's own budget (the `budget_chars` you passed, else 30,000); a
  reply already at its budget, a refusal, and the Markdown report get none.
  **Nothing it is drawn from changes:** `loop_hint`, `hint`, `next`,
  `next_step`, `nudge`, `carry_it`, `lessons` and `search_first` stay exactly
  as they were, so a client reading them reads what it read before.
  **What to do:** read `cues` first when you want to know what to call next.
- **`get_node` takes `budget_chars`.** Omitted, the record comes back whole, as
  before. Passed, the whole reply is held to it, the record's longest prose
  withheld first and every id, number and status kept, and its edges (when
  asked for) get the smaller of their own budget and this one. A single record
  measured 22.6 KB; a reader with a small budget can now ask for less.
- **`design_regions`, `hierarchy_issues` and `open_questions` take
  `budget_chars`** (default 30,000). Measured on a generated design of
  reflow2's size they answered 44,586, 40,327 and 58,916 characters with no
  bound, past the size at which a host such as flo2.io refuses a reply. Prose
  is withheld first, then rows from the end of the list, and `budget` says
  what was withheld; every count and id survives. An empty answer is unchanged.

### Changed

- **The first read a newcomer is cued to is now the search, then the report,
  each with what it costs.** The connect-time instructions, `get_instructions`
  (the loop's step 0a, and the terminal route's first step) and the kit's
  session-start text now say: `search_design` "current state" (or "status as
  of") first, about 2 KB for ten hits, and when several state records come back
  the newest is the one in force; then, if the reading allows, `graph_report`
  with `format` `markdown`, about 6 KB on a design of a few thousand records,
  which takes no budget. Measured: on flo2.io (0.95.0) the report half of the
  cue was followed 0 times in 42 offers and the search 3; in the size sweep the
  12 KB reader that read the report first spent 6,575 of its 12,000 bytes on it
  and scored 5.5/25, and the one that searched first scored 16. The handshake's
  cue ids are offered in the new order, `instructions/search_design` then
  `instructions/graph_report`.
- **What the cue count counts** (`usage_report`'s `tally.cues`). `mint_seat`'s
  `carry_it` is a cue now (`carry_it/claim_region`, the most-followed pointer
  measured: 38 of 38); `describe_schema`'s per-type `hint` is reference text and
  no longer counted (105 of 683 offers in the 0.94.0 sample, followed once); a
  withheld lesson and a near-match are counted (`lessons/get_node`,
  `near_matches/get_node`). **An id whose call moved to another tool is counted
  under the survivor's**, and a call to the old tool acts on it:
  `instructions/graph_report_markdown` is counted as `instructions/graph_report`,
  and the ids folded are listed in `tally.cues.moved`. **What to do:** when you
  compare cue counts across 0.96.0, compare them under these rules; the note in
  `tally.cues` states them.
