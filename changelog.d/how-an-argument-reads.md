### Added

- **Statement kinds for an argument: `objection`, `reply`, `unstated_premise` and `method_rule`.**
  These are on `add_requirement` and `add_constraint`. An objection reads how it stands
  (`stands`, `refutes`, `answered` or `conceded`). A reply reads `answers`, `stands` or
  `concedes`. An unstated premise is made explicit and is not taken as given unless it says so.
  A rule of method is taken as given by choice.
- **A step that is not an inference: rule `immediate`.** A claim grasped directly, by intuition,
  self-evidence or a self-verifying act, is a step that reads "not an inference". It may name no
  premise, and an objection can still aim at it.
- **A reply that concedes one reading and moves to another.** Draw a CONTRADICTS with
  `alignment: conceding` and `moves_to` from the reply to the objection. The conceded reading
  reads `objected: conceded` and supports nothing. The objection names the reply and the move.
- **The owner's chosen reading of a dispute kept open.** A `governed_by` from the chosen side
  to the dispute Decision carries `ruling: held`. The read-back marks that side `held` and the
  others `rival`. Read on the Decision, `reasoning_report` lists its `positions`, each with what
  it rests on.
- **A write that turns a claim refuted says so.** This applies to `review_relations`,
  `create_edge`, `add_requirement` and `add_constraint`. The reply names, under `now_refuted`, a
  claim it turned refuted or a step it turned defeated or conceded. For a claim, it also says
  how to keep the two as an open dispute instead.

### Fixed

- **A conclusion held up only by disputes reads `contested`, not `unproven`.** A claim reads
  `contested` when no route holds, but a route would hold if the objections standing against
  its steps or premises were settled. It names what stands against each such route
  (`contested_by`). A foundation taken as given under a standing objection now reads contested,
  and so does what rests on it. The read-back's `still_contested` counts them.
- **A claim's read-back shows a dispute kept open on its steps.** A `contested` Decision that
  governs a claim's rival readings is listed under the claim's `contested_by_design`.
- **What a claim rests on is its own support.** Under one claim, `rests_on` and
  `uncited_premises` no longer list the premises of objections against it. Those premises are
  listed apart, under `against_rests_on`. A step used as a premise is walked into and is never
  an uncited premise.
- **An argument is held in its project, and is asked an argument's questions.** A Project's
  scope reaches every step of the design's reasoning, and what those steps conclude and rest on.
  A design with steps and nothing to build, whose Project's domain names an argument or a
  proof, is no longer asked for an artifact standard or a closure criterion over its "live
  requirements".
