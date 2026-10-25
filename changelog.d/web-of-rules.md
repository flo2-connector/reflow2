### Added

- **A kept computation names its inputs, and a changed quantity marks it owed.**
  `add_artifact` takes `inputs`: for a calculator's record, a drawing or a
  render, each design quantity (a Constraint holding a `limit`, such as
  "household = 4 people" as a setpoint) and each other kept output it took,
  with the value or the version it used. One `DEPENDS_ON` edge is drawn per
  input. Passing `inputs` replaces the whole set; omitted, it stays. reflow2
  never reads the file. When a quantity changes, or an upstream output is
  re-run, `loop_status` lists every output that used it under
  `owed_computations` (transitive, with distances), and every decision, limit
  or check resting on one under `resting_on_stale_numbers`. Both count against
  `clean`. A design that registers no inputs reads exactly as before.
  **What to do:** record a number the math uses as a quantity. Register each
  calc record, drawing or render with its `inputs`. After a re-run, register the
  new version (`set_artifact_checksum`) with what it used (`add_artifact`
  `inputs`). (`req:a-changed-quantity-names-every-kept-computation-that-used-it`.)
- **A change returns its redo list.** `propagate_from` now carries
  `redo_list`, grouped by kind:
  - the computations to re-run;
  - the decisions and limits resting on them;
  - the parts those govern or bound, and the parts depending on a changed part;
  - the quantities describing a changed part;
  - the model files realizing those parts;
  - the drawings made from them.

  It also reaches the decisions a calc record documents, which the radius
  itself does not walk.
  (`req:a-change-to-a-physical-designs-part-returns-its-redo-list-by-trade`;
  grouping by trade is not built yet.)
- `derived_report` declares the two new relations, `computation_owed` and
  `redo_list`.
