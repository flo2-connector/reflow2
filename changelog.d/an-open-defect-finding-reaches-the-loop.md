### Fixed

- **A defect you recorded now shows up in `loop_status` and `detect_gaps` until
  something closes it.** A finding recorded with `fact_type: defect` (by
  `record_finding`, the root-cause skill, or a check that measured a limit
  broken) was read by nothing in the loop, so a design could hold "the roof is
  3 7/16 in over its height limit" or "the barn is 12 ft from the well against
  100 ft" and `loop_status` still said nothing was open. Each open defect
  finding is now an `open_defect_finding` gap, and `loop_status` counts them in
  a `next` line of their own and lists the newest 20 in `defect_findings`
  (`defect_findings_open` is the whole count). **What to do:** for each one,
  draw `invalidates` from the change or the decision that fixed it, set
  `valid_to` if it simply lapsed, or acknowledge its gap with your reason if it
  stands on purpose. A decision that only rests on the finding does not close
  it. A design with many old defect records will see a large count the first
  time; most are fixes that never drew `invalidates`.
