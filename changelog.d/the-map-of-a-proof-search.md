### Added

- **A proof search reads as one page: `reasoning_report` with `map: true`.** Pass `claim` the
  target and the reply holds the target and how it reads, every hole under the claim it serves
  with its routes (open with what would refute it, failed with where and why, waiting with its
  stated promise and rank), each closed result in one line, the rounds since the last progress
  toward the target, the forks that hold an untried branch, and the untried branches anywhere on
  the chart, most promising first. It fits one reply at any size, naming in `page.withheld` what
  it held back, and carries no step's warrant: read each argument by id with `get_node`. Built
  after a 16-round proof trial on the Aaronson–Ambainis conjecture, where both arms spent their
  late rounds on holes that could not close the target.
- **A hole says what it serves, and what closing it does.** `add_decision` takes `hole`,
  `serves` and `closing_it` (`closes`, `narrows` or `restates`) on a hole's decision point. The
  map says along the path whether closing each hole could close the target; a hole that only
  narrows it is shown as such, and an unsaid link reads unsaid, never `closes`.
- **A route says what would refute it, and an idea can be charted before it is judged.**
  `register_alternative` and `add_artifact` take `refuter`, `waiting`, `promise` and
  `promise_rank` on a route under a hole. A charted idea needs no reason until it is worked.
- **Rounds are counted.** A round is recorded when it ends, as `record_finding` with
  `fact_type: search_round` about the target, and the map counts the ended rounds since progress
  that served the target. Work on a special case that only narrows the target does not count.
- **work-a-proof-hole starts each round from the map** and, after three rounds with no progress
  toward the target, goes back to the most promising untried branch on the chart and records the
  turn with its cause. It charts every candidate at a fork, one line each, and ends each round
  with a new-ideas pass. **What to do:** nothing for a recorded proof to keep reading as before;
  to use the map, give each hole's decision point `hole`, `serves` and `closing_it`, and record
  each round's end.
