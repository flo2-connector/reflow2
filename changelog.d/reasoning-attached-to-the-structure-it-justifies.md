### Added

- **A requirement reads its reason to exist.** `get_node` on a Requirement a
  Decision governs, and `reasoning_report` on it, carry `reason_to_exist`: each
  governing Decision with the owner's `acceptance`, the verdict of the chain
  that concludes it beside it, and its `support` (`accepted_and_its_chain_holds`,
  `accepted_without_a_holding_chain`, `not_settled`, `withdrawn` ...), read
  apart from whether the requirement is delivered. A `parks` ruling is not a
  reason to exist. It used to say only "delivered by the roll-up".
- **A change that breaks a Decision's support names what it governs.**
  `propagate_from` (and `propagate_change`) carry `reasons_to_exist_lost`: every
  Decision the change leaves without a holding line (a seed, or a conclusion
  whose row no longer reads `still_supported`, `accepted_without_a_holding_chain`
  included) with every Requirement GOVERNED_BY it, named. They were reached only
  as distance rings before.
- **An edge names the claim that argues for it.** `SATISFIES` and `DEPENDS_ON`
  take `argued_by`, a typed link to a claim (a statement a step concludes),
  refused at write unless it names a node of the design. `satisfies` and
  `depends_on` take `argued_by`, and `satisfies` takes `evidence`, the edge's
  declared field it could not set before. The claim's verdict is read beside
  the edge: `argument` on `get_node`'s edges listing and on the write's reply,
  `argues_for` on the claim in `reasoning_report`, and `arguments_lost` on a
  change walk that breaks the claim. A claim deleted later is reported by
  `detect_defects` (`dangling_reference`).
- **Delivery argued along its own edge reads `circular`.** A claim "X
  satisfies Y" resting on Y, where Y holds only because X delivers it along that
  very SATISFIES, reads `circular` beside the edge, with the reason.

### Changed

- **`derived` lineage covers the owner's words chosen through a Decision.** It
  read as "technical necessity nobody asked for" alone, so a requirement
  stating an option the owner chose was left `original`. No value was added and
  no stored requirement changes; the schema, `set_requirement_lineage` and
  `decomposes` say it. **What to do:** set such a requirement `derived` and keep
  its GOVERNED_BY to the Decision.
- **Re-drawing a `satisfies`, or a `depends_on` with `argued_by`, keeps what the
  edge carried.** A re-draw used to write only the properties it was given, so
  adding one to an existing edge would have dropped the rest (a `partial`
  coverage among them). It now keeps every property the call leaves out.
  `depends_on` with no `argued_by` writes as before.
- **capture-intent, brainstorm and the served instructions say how to record a
  requirement's reason to exist and how to argue for an edge.**
