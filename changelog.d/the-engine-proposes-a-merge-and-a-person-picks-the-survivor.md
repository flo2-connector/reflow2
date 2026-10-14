### Changed

- **Which node a merge keeps is now a person's choice: `propose_heal` recommends, and `apply_heal`
  keeps the node the person picks.** The old rule picked the survivor by provenance and then by
  node id, and read nothing of status, priority or connections. On reflow2's own design the id
  decided all 11 same-type pairs. It kept `ctr:anthony` (10 edges) over `who:ajs` (1,353), and a
  medium, accepted requirement over a critical, proposed one, and `apply_heal` refused the
  swapped operation a person would have used to correct it. Now each merge in a proposal carries
  a `survivor` block beside its existing fields: both `candidates`, each with its `reasons`
  (provenance, standing, priority, status, connections, and which is newer), a `recommendation`,
  the signal that `decided_by` it, and a `reason`. The merge's `keep_id` is the recommendation,
  and it is given only when a real difference separates the two. The ladder is provenance, then
  live over withdrawn, then priority, then settled over open status, then connections. Which is
  newer is shown and never weighed, and the node id is never a signal. **To keep the other node,**
  swap the merge's keep and remove, or set the operation's new `chosen_survivor` to the id to
  keep. Both are accepted, and everything `apply_heal` refused before is still refused.
- **A merge that nothing in the design separates is no longer applied on the alphabet.** Its
  `survivor.needs_choice` is `true`, its reason begins "no recommendation: a person must pick",
  and `apply_heal` refuses the whole proposal, before any write, until that operation carries
  `chosen_survivor`. **What to do:** a client that passes `propose_heal`'s proposal straight back
  will be refused when such a merge is in it. Ask the person which node to keep, set
  `chosen_survivor`, and apply again. To keep both nodes, take the operation out.
  `would_destroy` no longer says "THE ALPHABET CHOSE THE VICTIM". A recommended merge's entry
  says why that node is the one kept. A merge that needs a pick names both nodes, with the issue
  id as its `reference`, and says what keeping each would lose.
