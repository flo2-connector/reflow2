### Added

- **`loop_status` and `graph_report` now list every increment that was started and never
  finished.** An increment is a planned epoch; it is open until it arrives. Both replies carry a
  new `open_increments` section, and `graph_report` with `format` `markdown` shows it under
  **Open increments**. It gives the whole counts, then at most the oldest ten increments, each
  with how much of what was scheduled for it is delivered and which of three signs it shows:
  `done_but_not_closed` (everything scheduled is delivered and it still reads planned),
  `overtaken` (an epoch it was drawn before, with `precedes`, has arrived) and `empty` (nothing is
  scheduled for it). Each row also says how many of the design's recorded changes have passed
  since anything scheduled for it moved, or `"never"`. Nothing is called stalled: the count is a
  number of changes and no threshold is applied to it. Oldest first is the order the store
  recorded each epoch in, never its `sequence` and never a date somebody typed. The list is not
  counted against `clean`. **What to do:** nothing; to see every open increment rather than the
  oldest ten, call `graph_report` with the new option `include_open_increments` set to true. A
  list is never cut there, so on a design with many open increments the whole list can take the
  report past its budget, and the reply's `budget` block says so (31 of them did, by 1,651
  characters, on reflow2's own design on 2026-11-14). An increment leaves the list when
  `set_epoch_status` marks it `arrived`.
- **The where-am-i skill reads the open increments back**, at the start or the end of a session or
  when asked, and `derived_report` counts a new declared relation, `open_increments`.
- **`derived_report` takes `budget_chars`** (default 30,000), like the other long reads. The
  thirtieth declared relation took its whole reply past 30,000 characters on a design of
  realistic size, the size at which a client refuses a reply. Over the budget, the declarations'
  text is cut and the reply says so at `budget`; every relation stays listed and every count and
  example id is untouched. **What to do:** nothing; `only` reads named relations whole.
