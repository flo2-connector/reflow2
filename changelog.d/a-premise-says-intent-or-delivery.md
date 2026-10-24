### Changed

- **A design choice can read as supported before anything is built.** A premise
  of a reasoning step now says what it rests on: its owner's **intent** ("X is
  required") or its **delivery** ("X holds in the system"). In a step that
  concludes a Decision, a requirement, capability or interface premise is read
  as intent by default: an accepted requirement holds, a proposed one is
  unknown, a dropped one fails; a capability reads by the needs it satisfies,
  an interface by the decisions that govern it. Every other step keeps reading
  delivery, so proofs, arguments and verification arguments read as before.
  Until now a requirement premise read only its delivery, so a choice made at
  design time on accepted requirements read "unproven", was listed as having
  lost its reason, and raised a `conclusion_does_not_follow` gap (19 of 19
  choices in a trial design). **What to do:** nothing for a design choice. A
  decision that really rests on delivery (a release, a go/no-go) marks those
  premises with `add_inference`'s new `as_delivery`; `as_intent` marks the
  other way on any step.

### Added

- **`add_inference` takes `as_intent` and `as_delivery`**, each a list of the
  step's premises, written as `reads_as` on the premise's link. A step's
  read-back shows `reads_as` on each premise read as intent or marked
  delivery. `reasoning_report` and `loop_status` list a premise held on its
  owner's word (`held_on_the_owners_word`) apart from the uncited ones, and
  one nobody has accepted (`awaiting_the_owners_word`), which `loop_status`
  names under "WHAT AWAITS THE OWNER'S WORD".
- **The served text says how a computed result enters a chain**: as a passing
  check marked `computed` whose `location` names its kept calculator record,
  never the record (a file) itself as a premise.
