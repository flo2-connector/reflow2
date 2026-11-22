### Fixed

- **A read through a hub's address that fans out over its designs now answers
  within a time budget, and names the designs it did not reach.** A search,
  `loop_status` or any other read over a whole hub ran on the hub and then on
  each member in turn, and a member closed after a quiet spell was reopened
  inside its turn. On flo2.io a cold search over 29 designs took a median of
  13.5 s, and four took past the 30 s the gateway waits, losing the whole answer
  although most designs had been reached. The view now spends one budget across
  its designs, 25 s by default. When it runs out, the reply holds what the
  designs it reached returned, and names each one it did not reach under
  `not_reached` (with `why: "budget"`, whether its turn had begun, and the call
  that reaches it alone: the same tool with `member`, or the design's own
  address). A `time_budget` block, in `loop_status`'s words, says the answer is
  partial. A merged search's `designs_searched`, `searched` and `found_in` cover
  only the designs searched. An open the budget stopped waiting for still
  finishes, so the same call made again reaches that design. A whole answer is
  unchanged and carries neither field. **What to do:** nothing. An operator
  behind a door other than 30 s sets `REFLOW2_HUB_VIEW_BUDGET_MS` beside
  `REFLOW2_LOOP_STATUS_BUDGET_MS`; `loop_status`'s own `time_budget_ms`, passed
  through a hub, moves the view's budget with it.
