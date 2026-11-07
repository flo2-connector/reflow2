### Fixed

- **`reasoning_report` never answers zero claims because one claim is larger
  than the budget.** A claim with many failed routes under it could read at
  39,000 characters: at `budget_chars` 3,000 the read answered "0 of 28 claims"
  with `next_offset` 0, and through a reply door of about 30,000 characters it
  could not be read whole at any budget. Such a claim now comes as its **head**
  (what it says, how it reads and why, and everything it carries but its parts)
  with `in_parts` counting its steps of support, failed routes and objections,
  and as many of those **parts** as fit, each whole, in `parts` (each naming
  the list it belongs to in `in`). Every page keeps to its budget. **What to
  do:** read on through a claim's parts with the new `part_offset` argument
  (the claim's `in_parts.next_part_offset`, with the same `offset`);
  `next_offset` still goes on to the next claim, so a reader that follows only
  it still reads every claim's head. A claim whose words alone are larger than
  the budget is still answered with no claim and the budget it needs.
- **No tool draws an edge from a node to itself where that means nothing.**
  `register_alternative`, called again for an alternative already registered
  under a decision (to move where its export lives, say), drew "X CONTRADICTS
  X"; `record_finding` with `caused_by` equal to its own id drew "X CAUSES X";
  and `create_edge` and `create_edges` drew either. The schema now declares, per
  edge type, the seventeen relations that cannot hold from a node to itself
  (`irreflexive: true`: CONTRADICTS, CAUSES, DEPENDS_ON, DUPLICATES,
  EVOLVES_INTO, DECOMPOSES, EQUIVALENT_TO, the two links of a step and the rest
  that a review already refused, TRIGGERS excepted, since a step of a process
  may retry itself), and the graph's one edge write refuses such an edge on
  every door, naming the relation. Registering an alternative again is now an
  update that keeps its siblings' CONTRADICTS. `describe_schema`, asked about a
  pair, says `irreflexive` on the relations it applies to. **What to do:**
  nothing for new work. A design that already holds such an edge still imports
  whole: the import keeps it and names it in `self_loops_kept`, and
  `detect_defects` reports a node recorded as contradicting itself as what it
  is; remove one with `delete_edge`.
