### Fixed

- **reasoning_report, loop_status and detect_gaps no longer grow explosively
  with a design's reasoning.** On flo2.io a design of about 600 nodes and 56
  reasoning steps took reasoning_report 13.3 s and loop_status past the
  gateway's 30 s, and the engine stalled. Two things caused it: every claim a
  report read got its own evaluation, so the reasoning beneath was worked out
  again for each one; and a truth read beneath a circle (objections answering
  each other, a claim that comes back to itself) was never kept, so everything
  above the circle was worked out again every time it was asked. Now one
  evaluation is shared across everything a report, loop_status, detect_gaps or
  a what-if reads, and a reading under a circle is kept with the state it
  depends on and handed back only when that state is the same, so every
  reading is exactly what it was. Measured on that design's export (release
  build): reasoning_report 2.6 s → 0.25 s, loop_status 4.5 s → 0.30 s,
  detect_gaps 2.9 s → 0.22 s; on a generated design of the same shape with
  objection circles, from minutes to under a quarter of a second each.
  **Nothing to do.**
- **reasoning_report keeps to a small `budget_chars`.** Asked with
  `budget_chars: 1000` it could answer 34,450 characters: the claims were
  whole and small, and the summary lists (what the reasoning rests on, the
  uncited premises, the unsupported claims, the assumptions) were trimmed of
  prose but never shortened. Over the budget they are now shortened by the
  characters each costs, each named in `budget.lists_shortened` with what it
  shows of its total. The claims on a page stay whole, so one whole claim, one
  entry of each list and the note saying so are the least a reply can be.
  **What to do:** read a shortened list whole with a larger `budget_chars`.
