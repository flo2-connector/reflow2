### Changed

- **Relation suggestions no longer treat the author as a reason.** `relation_candidates` weighs a
  shared neighbour by how rare it is, the way it already weighed shared words. A neighbour that
  nearly every record touches, such as the person who wrote most of the design or the project,
  now counts for nothing. Before, "both relate to <author>" was the top reason on almost every
  suggestion. With no `pool_type`, a requirement or decision is now compared with requirements AND
  decisions, so a requirement is offered the idea it grew from. Measured on reflow2's own design:
  for one requirement, the share of its top 10 suggestions from its actual family went from 2 to
  10. **What to do:** nothing; pass `pool_type` if you want the old same-type ranking.

### Added

- **A search hit lists the records linked to it.** `search_design` hits carry `linked`: each
  record directly linked by a review relation or `DECOMPOSES`, with the relation and its direction,
  up to 8. A family whose pieces are linked comes back together, even when only one of them
  contains your words.
