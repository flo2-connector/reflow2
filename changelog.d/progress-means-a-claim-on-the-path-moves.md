### Changed

- **A proof search's "rounds since progress" now counts only a claim on the path to the target
  moving.** The map's `rounds.since_progress` resets when the target, or the claim of a hole
  whose every link up to the target closes or restates, comes to hold, is refuted, or comes to
  rest on fewer premises. Steps closed and routes failed beneath an open claim on the path are
  still recorded, and no longer reset the count. In a proof trial the old count reset every
  round while the open lemma on the path stayed unchanged for four rounds, so the go-back rule
  never fired. `rounds.last_progress` and `rounds.this_round.progress` now name the claims that
  moved, with how each read before and after.
- **A round's finding records how the path read when the round ended.** `record_finding` with
  `fact_type: search_round` writes into the finding's `value` how each claim on the path reads at
  that moment, and the map compares each round with the one before. A `value` passed on such a
  finding is refused. A round the record cannot judge (one recorded before this release, or a
  claim on the path that reads settled with no earlier reading) stops the count: it reads
  `at_least`, or `null` when no later round could be judged, and `rounds.not_judged` lists those
  rounds. **What to do:** nothing; record each round with `record_finding` when it ends, as
  before, and leave `value` out.

### Added

- **work-a-proof-hole spends a fifth of each round on quick kills, and sieves new candidates.**
  Each round sets aside a slice of its budget (a fifth, as a starting guess) for the cheapest
  refuters of the unranked and lowest-ranked candidates anywhere on the chart, each time-boxed
  and recorded as a failed route; a barrier that rules out several is recorded against each.
  The design keeps a standing sieve of the field's known tight or extremal examples, each a
  statement with its source, and every new candidate is tested against it before it is ranked.
