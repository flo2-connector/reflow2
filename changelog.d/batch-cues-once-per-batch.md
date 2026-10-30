### Changed

- **The step for when a batch of writes lands is offered once per batch, not on
  every write.** Six cues named that step on every write: `loop_hint/detect_gaps`,
  `loop_hint/get_skill:detect-and-ask`, `loop_hint/detect_defects`,
  `loop_hint/get_skill:check-health`, `loop_hint/loop_status` and
  `loop_hint/governed_by`. Measured on 0.94.0, over the six writer sessions of
  the recall re-run: detect_gaps was offered 97 times and followed 0,
  detect-and-ask 75 and 0, governed_by 31 and 0, detect_defects and check-health
  17 and 0 each, and loop_status 162 times, followed within five calls twice and
  otherwise only at the end of each round. The tools themselves are called where
  they are needed (flo2.io, 2026-09-24 to 10-30: detect_gaps 211 calls,
  loop_status 397, governed_by 226), so the cue was offered at the wrong moment.
  **What changed, exactly:** a write is still offered each of these cues on the
  first write of a batch. It is not offered one again while that cue was offered
  within the same session's last 25 calls and 15 minutes and nothing since has
  made the call it names; making that call (or the batch running past 25 calls
  or 15 minutes) ends the batch, and the next write offers it again. The session
  is the one the cue counts already use: the seat, and the `--call` door's calls
  as one sequence. It is decided from the design's usage ledger, so an
  in-memory design, which has none, offers every cue as before. Where a hint is
  only that step (add_requirement, create_node, add_component, add_design_rule
  with its power stated), the withheld reply's `loop_hint` reads "loop: the step
  for when this batch lands was named earlier in the batch and is not repeated
  on every write", which names no call. Where a hint also says something about
  the node just written, that half is said every time and only the batch half
  is dropped: replace_text keeps its settlement clause, add_capability keeps
  "wire satisfies", add_constraint keeps "wire it", an accepted decision keeps
  "it reads back as what was decided", link_artifact keeps its
  reconcile_artifacts clause. Each cue's id and words are unchanged when it is
  offered, so increment 6 compares like with like. `loop_hint/set_decision_status`
  (the owner-only cue 0.94.0 and 0.95.0 fixed) is not affected. **What to do:**
  nothing; run detect_gaps, detect_defects, loop_status when your batch lands,
  as the first write said.

### Added

- **add_change_event names its next step when the change touched a registered
  file.** Its reply carries `next_step`: accept the files' checksums against
  this change with set_artifact_checksums (disposition `design_updated`,
  `design_change_event_id` this change). On flo2.io, 227 of
  set_artifact_checksums' 327 runs came within two writes of add_change_event,
  207 of them right after it, and nothing in the reply named that step. Said
  only when an Artifact is among `affected` and not removed; counted as the cue
  `next_step/set_artifact_checksums`. Inside record_the_build, which runs that
  step itself, the change step's receipt does not repeat it.
