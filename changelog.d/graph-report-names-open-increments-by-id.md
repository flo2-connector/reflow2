### Changed

- **`graph_report` names the open increments by id** (its reply changes shape). Left
  without `include_open_increments`, its `open_increments` keeps every count and lists the
  oldest ten as `{"epoch_id": ...}` only. `full_list` (under 80 characters, so no budget cuts
  it) names `include_open_increments: true` as the call that lists every one in full;
  `loop_status` still gives the oldest ten in full. `reads` is no longer in this view, since it
  explains fields the view does not carry. Unchanged: `include_open_increments: true`,
  `loop_status`, and `format: "markdown"`'s **Open increments** lines. Why: measured
  2026-11-18 on flo2's own design under 0.100.0, the report was 28,728 characters, 2,647 of
  them the seven full rows, and it no longer fit the 28,000 that flo2's chat door reads it in,
  so flo2's test that its design reads back whole failed (Anthony, 2026-11-18, option A: keep
  the guard and fix it here). **What to do:** a client that read each row's name, delivery or
  signals from the default `graph_report` reads them from `loop_status`, or passes
  `include_open_increments: true`.
