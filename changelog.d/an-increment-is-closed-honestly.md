### Added

- **`close_increment` closes an increment that will not be finished, honestly, on the owner's
  word.** Until now an increment could only stay `planned`, listed as open at every session's
  start and end, or be marked `arrived`, which is false. It now closes three more ways:
  `deferred` (still meant, later), `dropped` (no longer meant; `reason` is required) and
  `superseded` (by the increment `superseded_by` names). No unfinished item is defaulted:
  `leftovers` answers for every item scheduled for it and not delivered. Each one `moves` to
  another open increment, keeping its modality, is `dropped` from this one with its reason, or,
  on a deferral only, `stays` with it. A dropped item keeps its own status, so closing a plan
  retires no intent. One missing, doubled or unknown answer refuses the whole call, naming every
  problem, and nothing is written. When an item leaves and the plan is not on the record yet,
  `recorded_in` names an arrived epoch to snapshot the plan in first, so `arrival_delta` on the
  closed increment reads each item as moved or dropped rather than the plan as always smaller.
  The close writes one dated ChangeEvent with every fate and reason, linked to what moved.
  `approver` is required, and the tool declares that every call settles (`_meta`
  `reflow2/settles`), so a gateway signing for its caller reads the rule. **What to do:**
  nothing; to close one, call `close_increment`.
- **`DesignEpoch.status` gains `deferred`, `dropped` and `superseded`**, with two new properties,
  `close_reason` and `superseded_by`. These are new values and properties, not new types, so the
  schema stamp does not move. A closed increment never happened, so `snapshot_before_change`
  refuses to record history into one, as it refuses a planned one.
- **`loop_status` and `graph_report` count deferred increments apart** (`open_increments` →
  `deferred`). A deferred increment is closed for now and not listed; `set_epoch_status`
  `planned` reopens it, which clears its `close_reason` and `superseded_by` (the close's
  ChangeEvent keeps both).
- **The plan-increments skill has a step for an increment that will not be finished**, and
  where-am-i points to it.

### Changed

- **`set_epoch_status` and `add_epoch` refuse `deferred`, `dropped` and `superseded`**, naming
  `close_increment`; their `status` still offers only `planned` and `arrived`. `set_epoch_status`
  also refuses to mark a dropped or superseded increment `arrived` without reopening it first,
  since what was unfinished in it was moved or dropped at the close.
- **A closed increment is the owner's settle** in the authority check
  (`tools/check_intent_authority.py` and `reflow2_core::intent::SETTLING`): one written without
  an `AUTHORED_BY` approver fails the design gate, as an unsigned accepted requirement does.
  **What to do:** nothing; no design held one before this release.
