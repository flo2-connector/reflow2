### Added

- **Evidence is recorded as evidence, with its grade.** `add_requirement` and `add_constraint` take
  `statement_kind` `evidence` (a report, study or measurement: "the report says X") and `judgement`
  (an analytic judgement drawn from evidence). They also take `source_reliability` (A–F),
  `information_credibility` (1–6), `reported_at` and `confidence` (low, moderate, high), stored as
  fields.
  - A piece of evidence that a checksum-pinned source DOCUMENTS reads `cited_as_evidence`. With no
    pinned source it reads `evidence_unverified`. Either way it holds as evidence, and what rests on
    it reads `supported`, never shown or proven.
  - `reasoning_report` lists every piece of evidence and every judgement under `evidence`, each with
    how it reads and its `grade`. Every claim carries its `grade`.
  - No statement of a line of reasoning — anything with a `statement_kind` — is listed by
    `loop_status` as delivered intent awaiting the owner's word. Before this, the capstone trial
    listed 7 analytic claims there, and the logic rung 19 lemmas.
- **Natural deduction records what the logic rung could not.**
  - `add_inference` takes `rule_node`: the node that states the step's rule. It is never a premise.
    The step holds only while its rule holds, a what-if on the rule reaches every step it rules, and
    the rule is never an uncited premise.
  - `add_inference` takes `establishes` (`entailment` ⊨ or `derivability` ⊢).
  - `add_inference` takes `conclusion_form` (`conjunction` or `biconditional`). A what-if row for
    such a step lists its `parts`, and which part still stands.
  - A premise named twice is used twice (A, A ⊢ A∧A). The second mention was dropped silently
    before.
  - A case split may name a disjunction assumed in the same subproof as `cases_exhaustive`. It is
    read as assumed there and drawn as an `assumed` premise, so the →I that discharges it holds.
  - A step that holds on an open assumption or an unverified citation reads
    `holds_as: conditional` at the step itself, and one resting only on evidence reads
    `holds_as: supported`.
  - Two lines stated independent that rest on the same cited result, or on the same piece of
    evidence, are not independent (`shared_citations`).
- **More writes say what they turn refuted.** `create_edges`, `draw_edges` and `add_decision`'s
  `related_to` now name in `now_refuted` what they turn refuted, as `create_edge` and
  `review_relations` already did.
