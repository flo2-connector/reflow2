### Added

- **A relation can point at a node in another design.** On `review_relations`, and on
  `related_to` when adding a decision, pass `other_design` (that design's id) with `other_id` (the
  node's id there), and optionally `other_name`. The design records a typed reference to the far
  node (`xref:<design>:<node>`) and draws the relation to it.
  - Search's `linked` shows which design the linked node lives in.
  - A ripple that reaches the reference names that design.
  - `upstream_status` lists every reference, and reports a link into a design this one does not
    declare (`link_into_undeclared_design`). It also reports a link whose far design has changed
    since the link was made (`link_far_end_moved`); making the link again acknowledges it.
  - `loop_status` names both under `cross_design_links`.

  **What to do:** nothing. Where you used to write "see the requirement in flo2" as prose, you can
  now record it as a link. Decomposing a requirement across designs is not available yet.
