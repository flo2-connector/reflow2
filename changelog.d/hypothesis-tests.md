### Added

- **A claim supported by a statistical hypothesis test reads "supported at α", never shown or
  proven.** `add_verification` takes `hypothesis_block`: the `hypothesis_test` object of the tested
  value in a flo2-calc 0.10.0 kept record (record schema 10), copied whole. The check is then
  computed, so it counts only through its kept record, and a test that rejects H0 reads
  "supported at α = 0.05 (p = 0.0397, the test, n = 10, 10)". Every claim resting on it reads
  `supported` in `proof_status`, `statement_reading` and `reasoning_report`, however its steps
  are confirmed; a deductive line to the same claim is still preferred. A test that does not
  reject H0 reads "not rejected at α" and supports nothing, never "H0 true". The test's
  assumptions are written as Constraints the check rests on (`con:<check>-assumption-<n>`), so an
  objection can attack each. A test used as an objection contests a claim and never refutes it.
  `get_node` on the check carries `test_reading`.
- **A change to a test's criteria after its data arrived is reported.** The first
  `add_verification` with `hypothesis_block` locks the criteria as they stand
  (`Verification.criteria_at_data`): the α Constraint the record names, the hypothesis the check
  verifies, the test's Decision it is governed by, and the record's own test and α. A later
  change to any of them raises `criteria_changed_after_data`, with both values and the
  ChangeEvents that made it. Nothing is refused. **What to do:** link the hypothesis, α and the
  test's Decision to the check before attaching the data; acknowledge the gap with the reason
  when a change was legitimate.
- **α is spent like a budget across a family of tests.** A test whose record names an α
  Constraint with a `limit` draws CONSTRAINS from it with the α it was run at as its
  `contribution`, so `budget_report` sums the family and a test added later is visibly over. A
  test run at more than the share drawn for it reads unknown. The `design-a-proof` skill has the
  recipe: Bonferroni shares rounded down, and how Holm and Benjamini–Hochberg are recorded.
