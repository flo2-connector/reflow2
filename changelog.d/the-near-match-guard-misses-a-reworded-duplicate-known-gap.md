### Changed

- **A known gap in the near-match guard is now pinned by a test, switched off,
  and nothing about what the guard reports changes yet.** When a new node
  restates an existing one in other words, the guard can stay silent. Measured
  on 2026-11-01: a requirement restating a verified capability scored 313.8
  against its own text, the floor was half of that, and the capability (134.6)
  and the decision behind it (149.7) both fell under it. Four cuts on today's
  score were measured against a real design of 7,426 nodes and against copies
  of it at 1/3, 1/10 and 1/30 of that size. Each cut that caught the pair also
  raised false alarms at some size, from double to many times today's rate on
  the smaller copies, so none ships. The cut stays at half the new node's own score.
  `tests/a_reworded_duplicate_of_a_verified_capability_is_named.rs` pins the
  pair and is `#[ignore]` with its reason until a signal clears the bar. Its
  counterweight, an unrelated requirement that must get no warning, runs on
  every build. `tests/near_match_replay.rs` is the instrument that measured
  the cuts: run it with `--ignored` against a saved design (its header says
  how). **What to do:** nothing.
