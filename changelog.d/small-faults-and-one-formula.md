### Changed

- **What rests on an undocumented report now reads `conditional` on it.** Before, it read
  `supported`. It reads `supported` once a checksum-pinned source DOCUMENTS the report. This
  follows `dec:idea-what-rests-on-unsourced-evidence-reads-supported-or-conditional`, option (b),
  and changes what #782 built.

### Added

- **One formula, one record per occurrence.** A statement can name the formula it is an
  occurrence of (`same_formula_as`, on `add_requirement` and `add_constraint`), and
  `reasoning_report` groups the occurrences under `formulas`. A ⊥ shared by several reductios
  still cannot be one record, because that closes a circle.
- **A principle can say it is silent on a case** (`silent_on`), and the read-back shows it.
- **`add_constraint` takes `source`, and `direction: setpoint`.** A setpoint is a value held, such
  as a test parameter. It is never a budget, so it is never exceeded.
- **One answer can link from every record it became**: `answer_question`'s `also_records`.
- **A self-asked question can be withdrawn by its own id**: `withdraw_question` takes
  `question_id`. An unknown id is refused, and the refusal names the questions that exist.

### Fixed

- A stated foundation is no longer listed under `unsupported` as well.
- A step's generated name now follows its premises when one is added or deleted.
- A statement's `source` text is now said not to count as citing (`source_note`).
- `reasoning_report` now uses the whole budget it is given, and its note names that budget, never
  "a budget of 0".
- `loop_status.session` now says when a server that has seen only this call could not count a
  session (`counted: false`), instead of reporting zero writes.
- A computed check with a kept calculator record now counts as an executable form, and as a run
  fed back.
- `get_node` now says why every foundation holds while no derivation does
  (`foundations_and_proof`).
- A walk from a check no longer crosses a GOVERNED_BY.
- A settled decision's own "PROPOSED…, awaiting the owner" is now caught by
  `settled_question_prose`.
- Through a hub, a node name equal to its id is no longer prefixed.
