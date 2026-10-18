### Changed

- **A near-match of another type, or between two statements of a proof, no
  longer refuses a write.** `add_requirement`, `add_capability`,
  `add_component`, `add_design_rule` and `add_decision` write the node and name
  each match, with its type, in `search_first.near_matches` on the reply. They
  refuse only for a match of the same type outside a line of reasoning, with
  the same three answers as before (sharpen, `distinct_from`, `replaces`). A
  line of reasoning means a statement carrying a `statement_kind`, or an
  Inference. The hand-kept list of fourteen exempt type pairs is gone, and a
  class test pins the rule. Measured on proof trial rung 2 (the fundamental
  theorem of arithmetic, 0.86.0): replayed against the rest of its design, 45
  of its 49 statements were refused, and 42 had been recorded with
  `distinct_from` lists of up to 18 ids; now all 49 are written with none.
  **What to do:** nothing. A `distinct_from` you pass is still kept on the node.
  Read the `search_first` note on a write; if a match covers the same need,
  revise or link it.
