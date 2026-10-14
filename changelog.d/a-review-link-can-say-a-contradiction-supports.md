### Fixed

- **A relation drawn with `review_relations` can now say a contradiction supports.** Every
  CONTRADICTS it drew was stored `alignment: opposing`, the schema default, because a link had no
  field for anything else and the tool wrote only `evidence`. A writer who meant "these two
  corroborate each other" wrote that into the evidence, and `detect_defects` reported the edge as a
  contradiction. In reflow2's own design 10 edges said "supporting" in their evidence and stored
  `opposing`; 9 were reported. A link now takes an optional `alignment` (`opposing` or
  `supporting`), and the same goes for every other property a review relation's schema gives a
  default: CAUSES `basis` and `validation_status`, DUPLICATES `basis`, DEPENDS_ON `optional` and
  `weight_basis`. `add_decision`'s `related_to` takes the same fields. A value the schema does not
  allow, or a property the link's relation does not declare, is refused, naming the legal ones. A
  value that differs from an edge already stored is refused rather than reported as already
  present. Nothing is read out of the evidence.

  **What to do:** nothing, if your links say nothing new: a link without these fields stores the
  same defaults as before, and every existing argument and reply is unchanged. To correct a
  CONTRADICTS stored `opposing` that you meant as a corroboration, call `create_edge` with the same
  ends and props carrying `alignment: "supporting"` AND the stored `evidence` again, because
  `create_edge` replaces the edge's properties.
