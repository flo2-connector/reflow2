### Added

- **`loop_status` answers within a time budget, and names what it did not
  compute.** On a design of 8,696 nodes it took 38.1 s on flo2.io's hardware,
  past the 30 s its gateway waits, so the call every session is told to make
  answered nothing. It now computes its floor first (questions, decisions,
  checks, divergences, follow-ups, breaches, kept outputs), then the five
  whole-design analyses cheapest first: the confirmation ledger, the
  reasoning, whether a real result ever came back, gap detection, defect
  detection. When the budget runs out, the analyses it did not reach are
  listed in `not_computed`, each with the fields it left null and the call
  that computes it alone (`detect_gaps`, `detect_defects`,
  `confirmation_ledger`, `reasoning_report`; `loop_closure` has none, so a
  larger `time_budget_ms`); a line for
  each leads `next`; and the answer is never `clean`. A new `time_budget`
  block says what the call was given and spent, and whether the answer is
  whole. The default is 20 s, two thirds of flo2.io's door; the clock starts
  when the call arrives, so a wait for the design is spent from it, and a
  hub's view spends one budget across its designs. **What to do:** nothing.
  Pass `time_budget_ms` to raise or lower it for one call (`0` is the floor
  alone), or set `REFLOW2_LOOP_STATUS_BUDGET_MS` on a server behind a
  different door.

- **A long read stops when its caller has gone.** `loop_status`,
  `detect_gaps` and `detect_defects` check between their detectors whether
  the request was cancelled, and stop there instead of running to the end
  while holding the design. rmcp cancels a request when its client sends
  `notifications/cancelled` for it (any transport), or when a stateless HTTP
  client (MCP 2026-07-28) disconnects. It does NOT on an HTTP session
  (before 2026-07-28): a client there that gives up must send
  `notifications/cancelled`. **What to do:** a client that times a call out
  should send that notification.

### Changed

- **`loop_status`'s `unsurfaced_gaps`, `answered_with_open_gap`,
  `answered_naming_their_record`, `structural_defects`, `unexamined_claims`
  and `loop_closure` read `null` when their analysis was not computed this
  call**, never `0`. A whole answer reads exactly as before. **What to do:** a
  script that adds these up should treat `null` as "not known from this
  answer", and read `not_computed`.

**In your setup.** The `--call` setup gets all of it: `reflow2 read loop_status`
answers within 20 s, and `reflow2 read loop_status '{"time_budget_ms": 0}'`
prints the floor alone at once, with `not_computed` naming the five analyses.
A one-shot call has no caller to give up, so the second entry is for a
server. Needs flo2.io: nothing, once flo2.io's engine moves to this release;
flo2.io's gateway then sends `notifications/cancelled` when it stops waiting.
