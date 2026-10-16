### Added

- **A conclusion says whether it is proven, and each step says who confirmed it.**
  `add_inference` takes three optional fields:
  - `confirmed_by`: who confirmed the step, as a contributor id or a proof checker's name;
  - `confirmed_how`: `person`, `checker`, or `unconfirmed` when omitted;
  - `confirmed_at`: when, as an ISO 8601 timestamp.

  `get_node` on a step shows them in `inference_reading.confirmation`. On a node a step
  concludes, `get_node` adds `proof_status`:
  - `proven`: every step in at least one holding derivation is confirmed;
  - `shown`: a derivation holds but a step in it is unconfirmed;
  - `contested`: a derivation holds but an objection stands against the conclusion;
  - `unproven`: no derivation holds, or an objection that holds refutes the conclusion.

  A counterexample is a `CONTRADICTS` from the counterexample to the statement it breaks.
  **Nothing changes for a design that records no `Inference`:** `proof_status` is absent.
  **Upgrade note:** the schema gains three `Inference` properties, so a store this release has
  opened is stamped with them.
