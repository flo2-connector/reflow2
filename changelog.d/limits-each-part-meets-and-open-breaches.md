### Added

- **A limit can be one each part meets on its own: `composition: each`.** A
  setback, a clearance, a separation distance or a height each part stays
  under is not a budget, and its parts do not add. With `composition: each`,
  `budget_report` judges every stated part alone, reads the worst one (the
  nearest against a minimum, the largest against a maximum), names it in
  `judged_part`, and says `incomplete` while no part has a reading. **What to
  do:** for a clearance or setback recorded as a sum, set `composition: each`
  on it with `add_constraint`. Before this, a barn 12 ft from a well and a
  farm loop 102 ft from it summed to 114 ft and read "within" a 100 ft
  minimum.

### Fixed

- **Anything the design records as broken now shows in `loop_status` and
  `detect_gaps` until something closes it.** One reading, `open_breach`,
  covers an open defect finding (`fact_type: defect`), a limit that is not a
  KPP whose own readings exceed it, a `VIOLATES` edge into a limit, and an
  environment-rule violation ruled `rejected` (must fix) with no
  `resolved_at`. Before, a defect finding was read by nothing in the loop, so
  "the roof is 3 7/16 in over its height limit" left `loop_status` saying
  nothing was open. `loop_status` counts them in a `next` line of their own
  (`breaches_open`) and lists the newest 20 in `breaches`; each is an
  `open_breach` gap saying how its kind closes. **What to do:** close each one
  (`invalidates` from the change or decision that fixed a finding, new
  readings that fit a limit, removing a `VIOLATES` edge, `resolved_at` on a
  must-fix violation), or acknowledge its gap with your reason if it stands on
  purpose. A design with many old defect records will see a large count the
  first time; most are fixes that never drew `invalidates`.
- **A minimum with no reading yet reads `incomplete`, not `exceeded`.**
  Nothing stated is nothing measured, so it is no longer reported as a breach.
