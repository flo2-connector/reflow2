### Added

- **`add_decision` names its authors and the documents behind it, in the same call.** It takes
  `authored_by` (each a contributor, with an optional `role` and `acted_at`) and `documented_by`
  (each an artifact, with an optional `doc_kind`). It draws AUTHORED_BY and DOCUMENTS exactly as
  `authored_by` and `documents` do. On flo2.io, 45 of 51 `authored_by` runs came with an
  `add_decision`. Where reflow2 is served for others, a signature is still the caller's own.
- **`add_constraint` names what it constrains, in the same call.** It takes `constrains` (each a
  target, with `constrains`' own fields: contribution, unit, source, basis, measured_at, note).
  On flo2.io, 13 of 17 `constrains` runs came right after an `add_constraint`.
- In both, an entry that cannot be drawn refuses the whole call, and nothing is written. The edge
  tools stay served for every other use.
