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
- **`reasoning_report` returns every claim whole, and pages through a long
  proof.** Each claim now carries its own words (`says`), each step its rule
  and warrant, and each premise its `name`; no text in a claim is cut. A reply
  over `budget_chars` returns fewer claims, in proof order, instead of every
  claim with its words blanked. The reply says where the page starts
  (`offset`), how many claims the walk holds (`total_claims`), how many are
  left (`remaining`) and where to read on (`next_offset`, passed back as the
  new `offset` argument). A `page` block names the `budget_chars` the reply
  was held to, so a budget lowered on the way shows in the result. The walk
  always reads the whole reasoning, up to 500 records, so `rests_on`,
  `stated_foundations` and the other summary lists are never one page's: a
  small `limit` no longer reads as resting on nothing. A walk that stops at
  500 names what it did not read in `not_walked`. `limit` now sets the page
  size, and `capped` now means the walk itself stopped. Measured on proof
  trial rung 2: at flo2.io's 27,267 characters the read-back cut 152 text
  fields to "…" and showed 25 of the 60 claims it walked; now it shows 14 of
  66 claims whole and reads on in pages. **What to do:** to read a whole
  proof, call again with `offset` set to `next_offset` until it is absent.
