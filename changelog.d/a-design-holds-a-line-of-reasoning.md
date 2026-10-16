### Added

- **A line of reasoning reads back in one call: `reasoning_report`.** An argument for and
  against, an experiment's case, a paper's chain of claims or a legal case is recorded with
  reflow2's own vocabulary:
  - the claims, and the steps that support them (`add_inference`);
  - premises taken as given (a Constraint);
  - sources (an Artifact that `DOCUMENTS` a claim, with its `location`);
  - objections and replies (`CONTRADICTS`).

  For each claim, `reasoning_report` gives its computed truth, its steps with their rule, warrant
  and who confirmed them, its objections and how each stands, and its cited sources. Then it
  lists:
  - `rests_on`: the premises the reasoning rests on;
  - `uncited_premises`: those premises that cite no source;
  - `still_contested`: the points still in dispute;
  - `contested_by_design`: the disputes kept open on purpose;
  - `unsupported`: every claim with no support.

  Pass `claim` to read the reasoning under one claim, or omit it to read every top conclusion and
  every contested decision. Every reply carries `what_this_means`: structure is not truth, a
  source counts only when the design names it, and nothing in the reply is legal, medical or
  other professional advice. A claim with no support is listed here and raises no gap.
- **An objection counts as far as it holds.** An opposing `CONTRADICTS`, whether an objection or
  a counterexample, is weighed by its own truth:
  - one that holds (a passing check, a met requirement) refutes its target, and every step
    resting on the target is reported broken;
  - one not yet established leaves the target `unknown`, and its `proof_status` reads
    `contested`;
  - one that fails no longer counts. It fails when a reply that holds answers it, or when it is
    rejected.

  A reply is recorded the way the objection was: a `CONTRADICTS` from the reply to the objection.
  Two claims that object to each other, with nothing else to settle it, both read `unknown`.
  **What to do:** a counterexample meant to refute must itself hold, so record it as a check
  that passes or a requirement that is met.
- **A Decision can be kept open on purpose: status `contested`.** Both sides stay on the record
  and no verdict is owed, as in an argument for and against that is meant to stay one. Govern
  each side of the dispute by the decision (`governed_by`). Like `deferred`, it is the owner's
  word and not debt:
  - the loop does not count it as owed;
  - `detect_defects` does not report a contradiction between two nodes it governs, and counts
    them in `swept.suppressed_by_held_open` instead;
  - its own truth reads `unknown`.

  **Upgrade note:** `Decision.status` gains a value, so the schema stamp moves. An older binary
  opening a store that holds a `contested` decision does not know that value.
