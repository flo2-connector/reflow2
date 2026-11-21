### Changed

- **`search_design` takes `budget_chars`, and a search through a hub's address is held to it
  whole.** Through a hub of 29 designs a search at `limit` 2 answered 5,746 characters, 1,314 of
  them hits, and nothing held the reply. Over the budget, the per-design lists give way first,
  then whole hits from the tail; every count stays, and `budget` says what was withheld.
- **A search over several designs carries `searched` and `found_in`.** `searched` is the one total
  of nodes the searches ran over (the key a single design's search already answers with), and
  `found_in` names only the designs that returned hits, with their counts.

### Deprecated

Three per-design lists in a `search_design` reply over several designs (through a hub's address,
or with `tracked_designs`) are going in the next release. They list every design whatever the
answer, so the reply grew with the number of designs. Each still arrives in this release, and a
reply that carries one says so under `deprecation_notice`.

- **`search_design`'s `view`.** It names the same designs as `designs_searched`. **What to do:**
  read `designs_searched`, which names them with the hub first.
- **`search_design`'s `searched_by_design`.** **What to do:** read `searched`, the total the
  searches ran over; a design whose search failed is named in `failed`.
- **`search_design`'s `found_by_design`.** **What to do:** read `found_in`, which names each design
  that returned hits with its count; a design searched and absent from it returned none.
