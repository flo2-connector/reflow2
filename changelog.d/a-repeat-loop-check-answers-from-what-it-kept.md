### Changed

- **`loop_status` asked again with no write since answers from what the engine kept.** On
  reflow2's own design (8,916 nodes) a repeat ask took 3.1 s and now takes 0.1 s, measured
  2026-11-13 on one machine with the 0.98.0 engine beside this one. Until now only gap and
  defect detection were kept between asks; whether each check ever came back with a real
  result, the list of checks, the confirmation ledger and the reasoning debt were worked out
  again every time, from records no write had touched. Each is now worked out once per state
  of the design, and a whole answer is kept per person asked about. Any write drops all of
  it, so nothing is served stale. `detect_defects` asked again with no write went from 1.6 s
  to under 0.05 s, and a `loop_status` with `time_budget_ms: 0` from 0.63 s to 0.35 s. **The
  first ask after a write is not faster** (8.6 to 9.2 s on that design, as before): that
  needs the checks to look only at what changed, which is recorded and not built. What is
  kept lives in the running engine and uses memory there; it is gone when the design is
  closed. **What to do:** nothing.

### Fixed

- **`detect_gaps` asked twice with no write between could answer the second call from the
  first call's readings of another design.** Since 0.89.0 a premise that lives in another
  design reads as it reads there, from readings each call brings. The kept gap scan was only
  dropped by a write to this design, so a call that brought different readings (the other
  design changed, or the caller can read different designs) got the gaps worked out for the
  call before, until the next write here. Shown at the core library; how often it was met in
  use was not measured. The engine now notes which readings its kept results were worked out
  under and drops them when a call brings different ones. **What to do:** nothing. If a
  design that rests on another one showed a gap that did not match the other design, ask
  again on this version.
