### Added

- **A design pin can say how the other design stands to this one.** `external_dependency` takes
  `relation`: `part_of` (the other design is a part of this one; intent flows down to it, status
  flows up) and/or `uses` (a peer this design uses). For `uses`, `interfaces` names the
  interfaces the use crosses. `upstream_status` lists every design pin under `members` with its
  relation, or "not stated". It also reports a member linked by nothing a ripple could follow:
  `relation_not_stated`, or `no_interface_to_follow` for a `uses` link with no mirrored Interface
  here. `loop_status` names these under `members_unlinked`, even when nothing is watched. The
  `/hub` skill asks for the relation when a member joins. **What to do:** in a hub, answer the
  `relation_not_stated` findings once per member: re-declare each pin with its `relation`, and
  for a peer its `interfaces`, then mirror each interface with the link-projects skill. Pins that
  name no design are never asked.
