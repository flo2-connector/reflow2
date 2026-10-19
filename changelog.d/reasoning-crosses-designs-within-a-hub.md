### Added

- **A premise in another design now reads as it reads there.** A step whose
  premise is a reference to a node in another design (`xref:<design>:<node>`,
  drawn by `review_relations` with `other_design`) used to read that premise as
  "reflow2 computes no truth for a Resource", so a claim citing a theorem proved
  in another design read unproven. On a registry server behind a trusted host
  (flo2.io, or a local `--registry-root` with a gateway or bearer gate), the far
  node is now read in its own design before each call, under that design's
  guard alone and only if the caller may read it: the premise carries the far
  node's truth, its proof status there limits a proof here (a far result that is
  only `shown` keeps a fully confirmed line here at `shown`), and what it holds
  only under there (an unverified citation, an open assumption) makes the claim
  here `conditional`. A far node that is gone reads **stale**, and a far design
  the caller may not read, or a server that reaches no other design, reads
  **unreachable**: both unknown, never holding. Nothing read across is stored.
- **Through a hub, a change walk continues into the designs that cite what it
  reached.** `propagate_from` and `propagate_change` called through a hub's
  address now carry the radius on into every design of the view (that the
  caller may read) holding a reference to a node the walk reached, and on from
  there. Each reference is named in `boundary_crossings` with its design's
  prefix, and each design's radius comes back under `across_designs`
  (`design`, `from_design`, `entered_at`, `radius`). A call that does not come
  through a hub's address is unchanged.
