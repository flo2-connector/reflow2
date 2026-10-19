### Fixed

- **`add_decision` stores the status it is given.** With `status: rejected` or
  `superseded` it stored the Decision `proposed` — and, when an `approver` was
  passed, drew the owner's signature on it all the same, so a proposed Decision
  carried an approval. Only the decisive statuses (`accepted`, `deferred`,
  `contested`) were ever written, because the same check that decides whether
  the owner's name is needed also decided whether the status was written. Both
  constructors that take an approver (`add_decision`, `add_requirement`) now
  store exactly the status given, on a create and on a revise (a revise back to
  `proposed` was silently ignored too). A status the schema does not know is
  refused, naming the legal ones, before anything is written. An `approver`
  passed with no `status`, or with `proposed`, has nothing to sign: it is no
  longer drawn, and the reply says so under `approver_not_drawn`. **What to
  do:** nothing. To record the owner's word, pass the status they chose with
  their name.
- **Re-drawing an edge keeps the properties the call leaves out, for every typed
  edge helper.** `satisfies` already did, and `depends_on` did only when given
  `argued_by`; the other 23 helpers `draw_edges` runs (`allocate`, `verifies`,
  `realizes`, `governed_by`, `deploy_to`, `violates_rule`, …) replaced the whole
  property bag, so a bare re-draw erased an allocation's weight, a check's
  coverage, a realization's completeness, a granted variance's status or a
  governing note. What a call names still replaces that one property.
  **What to do:** to clear a property, delete the edge and draw it again, or set
  the whole bag with `create_edge`, which still replaces.
