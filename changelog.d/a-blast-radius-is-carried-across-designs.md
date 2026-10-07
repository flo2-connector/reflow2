### Added

- **A change's impact can be followed from one design into the next.** A blast radius
  (`propagate_change`, `propagate_from`) now names the `design` it ran in. A row that belongs to
  another design names that design: a mirrored node, or an interface a `uses` pin names.
  `continue_in` lists the member designs to carry the radius on in, and the ids to start from
  there. `propagate_from` also takes `arriving_from` (another design's id) with `interfaces` (that
  radius's `interfaces_reached`) instead of seeds. The design's own pins then say where the ripple
  enters: across an interface it uses, or up at what requires the part that changed.
  `arrived: false` means nothing records a way in. The `/hub` skill uses this to carry a change
  from member to member and to report every design it reached. `mirror_surface` now records which
  design each mirrored node came from (`mirrored_from`). **What to do:** nothing, for a single
  design. In a hub, state each member's relation first (the `relation_not_stated` findings): a
  ripple follows only the relations that are recorded.
