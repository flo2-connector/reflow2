### Fixed

- **`loop_status` is fast again on a large design.** On reflow2's own design (about 7,000
  nodes) it took 30 s locally and 31–46 s on flo2.io's server. That is over the 30 s a hosted
  call is allowed, and over the 2,000 ms limit set for a warm `loop_status`. Nearly all of it
  was the confirmation ledger: it re-read an artifact's whole history once for every
  capability that artifact realizes, about 940,000 reads where 4,000 would do. It now reads
  each artifact and each change once. Its answer is unchanged, and the ledger drops from 17 s
  to 0.26 s. `confirmation_ledger` itself is faster by the same amount. A test now fails if
  the ledger's reads grow with capabilities × history again. **What to do:** nothing.
