### Added

- **The owner can rule that an experiment's result refutes a claim, and it
  reads refuted by that ruling.** A result (a statement of kind `evidence`, or
  a statistical test) still only CONTESTS what it goes against on its own. The
  owner settles it with acts reflow2 already serves: a Decision
  (`add_decision`), optionally concluded by a step from the result and the
  prediction it missed (`add_inference`), that CONCEDES the result
  (`review_relations` CONTRADICTS with `alignment: conceding`), accepted and
  signed by the owner (`set_decision_status` `accepted` with `approver`). While
  that ruling is in force, the claim the result goes against reads `refuted`,
  and its reading names the ruling, who signed it, the result and the
  conditions: the model and idealizations held on purpose (`held_as`) beneath
  the claim and the ruling's step. A hypothesis (statement of kind
  `hypothesis`) discharged into the missed prediction, or derived under, reads
  refuted by the same ruling. The objection carries the ruling whole
  (`ruling`); `reasoning_report` lists each ruling in `refuted_by_ruling`;
  `loop_status`'s `reasoning` names them in `rulings` / `rulings_in_force`; and
  `propagate_from` / `propagate_change` name each ruling a change takes the
  force of in `rulings_lost` (the Decision itself, or a held model or
  idealization it rests on). Withdraw it (`set_decision_status` `rejected`, or
  delete the concession) and the result only contests again. **What to do:**
  nothing for an existing design. To rule, follow the design-a-phenomenon
  skill, section 6.
- **A claim a result supports, resting otherwise only on what is held on
  purpose, reads `supported` given it.** When a line under a claim rests on a
  documented result and otherwise only on premises held as a `model` or a
  `principle`, its `proof_status` reads `supported` with those premises in
  `given` (and reasoning_report's claim reads `truth: supported` with `given`;
  the step's `holds_as` reads `supported`), never `proven`. Before, it read
  plain `conditional`. A claim that also rests on an undischarged supposition,
  an unverified citation, an undocumented report or a bridge reads as before.
  **What to do:** nothing; a design that holds nothing as a model or a
  principle reads exactly as before.

### Changed

- **A Decision that concedes a result counts only as a ruling.** Unsigned, or
  not accepted, it grants nothing and the result contests; before, any
  holding reply's concession of a result refuted what the result went against.
  A concession of any other objection, and a concession of a result by a
  statement that is not a Decision, read as before. No design in the trial
  fixtures or reflow2's own design holds a Decision conceding a result.
