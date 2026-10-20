### Added

- **A what-if on a principle that an argument assumes now reads every position on it.**
  `propagate_from` on a principle that the steps take up in `assumed` used to say nothing about
  the positions. Each step on the principle now gets a row:
  - the position reads `unsupported` when every line of it rested on the principle;
  - it reads `still_supported` when a line remains, and `still_holds_under` names the remaining
    lines.

  For example, on the trolley design, dropping act consequentialism leaves the footbridge "push"
  verdict unsupported, while the bystander "turn" verdict still stands on double effect. A
  reductio whose supposition is dropped still holds.
- **A what-if says what a contested claim loses, and which objections come back.** Each row now
  says:
  - what its claim read before the change (`conclusion_was`);
  - which of the claim's routes the change never reaches (`untouched_routes`).

  A claim that was only contested, and that the change leaves with nothing, is followed onward
  like any claim that falls. An objection whose answering reply the change takes away is listed
  under `objections_returning`, and what it attacks is read again.
- **A check whose conditions lose their support reads stale.** A check constrained by reasoned
  claims (a test setting concluded from an assessment) carries `check_conditions` on `get_node`
  and in `reasoning_report`. Each condition is read, and the check is marked stale when one no
  longer holds outright. A passing stale check holds nothing up as a premise. `propagate_from`
  names the checks a change leaves stale (`checks_gone_stale`).
- **What has lost its reason now is one read.** `loop_status` counts and names, across the whole
  design and not only as a what-if's change:
  - requirements whose reason to exist is lost, or stands on acceptance alone;
  - accepted decisions none of whose chains holds;
  - stale checks.

### Changed

- **A test whose criteria moved after its data reads unknown until the owner acknowledges the
  change.** A changed α, test or hypothesis no longer only reports the
  `criteria_changed_after_data` gap: the test supports nothing until that gap is acknowledged
  with a reason. Every write still lands. **What to do:** to accept a change made on purpose,
  acknowledge the gap with your reason.

### Fixed

- **A claim derived under a supposition that now reads refuted says so.** It no longer reads as a
  conditional that still holds, and an objection derived that way no longer stands.
- **A what-if on a premise reaches only what rests on it.** It no longer fans out to every sibling
  of the parent requirement that the premise decomposes.
- **A requirement's read-back follows its reason to exist down to the evidence.** The read-back
  now walks each governing decision's chain, and `rests_on` lists the reports underneath it.
  `reason_to_exist` names them (`chain_rests_on`, with any recorded `confidence`). A claim that
  another claim replaces no longer keeps a decision's chain holding.
