### Changed

- **Opening a design again reuses the search index kept with its store,
  instead of rebuilding it every time.** Every open used to rebuild the
  design's full-text index from the store: seconds on a large design, and on a
  hosted server, which closes a design after 15 minutes without a request, that
  cost landed on the request path of every reopen. A whole-hub search after a
  quiet spell paid it once per member. Now the index is reused when it is
  proven to match the store: its last commit carries a voucher naming the
  store's write sequence, an anchor kept in the store, this reflow2 version and
  the fields this build searches, and the open checks every one. Anything else
  is rebuilt, as before: an index that is missing, unreadable, written by
  another reflow2 version, or behind or ahead of its store (a store restored
  from a backup, or rewritten by a build that keeps no index), even when it
  holds as many documents as the store holds nodes. A search after a write
  still finds what the write added. An index that cannot be read is set aside
  and rebuilt rather than failing the open. **Nothing to do.** The first open
  of each design under this release rebuilds its index once (an index from an
  earlier release carries no voucher), and opens after that reuse it. A
  release from before this one opens these stores unchanged.
