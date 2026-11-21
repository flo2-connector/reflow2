### Added

- **`add_interface` names who provides and who consumes the interface, in the same call.** It
  takes `provided_by` (components) and `consumed_by` (components, or actors outside the design), and
  draws PROVIDES and CONSUMES exactly as `provides` and `consumes` do. An id naming nothing is
  refused, and nothing is written. On a revise, a side is added and none is removed. The two edge
  tools still work as before. On flo2.io they ran next to each other in 31 of 34 and 33 of 35 runs
  and never alone, so recording a seam took three calls where it now takes one.
