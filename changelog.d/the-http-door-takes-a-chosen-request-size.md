### Added

- **The largest request the HTTP door takes is a setting:
  `--http-max-request-bytes SIZE` (or `REFLOW2_HTTP_MAX_REQUEST_BYTES`).**
  SIZE is a whole number of bytes, or one followed by `KiB`, `MiB` or `GiB`
  (`16MiB`), from 64 KiB to 512 MiB. A request is held whole in memory for the
  length of the call, so a value outside that is refused at startup with the
  bounds, and the banner says the limit when it is not the default. It reaches
  every HTTP surface: one design, the shared daemon, a registry of designs and
  the degraded surface. **Unset, nothing changes:** the limit stays the
  transport library's 4 MiB, which until now was the only one there was, so a
  document sent to `import_graph` over HTTP could be no larger. **A request
  over the limit is answered `413` in words that say what would have worked**:
  the limit in bytes and MiB, the flag and its environment twin, and that
  nothing in it was read. The library's own words named the limit and not the
  setting. stdio and the `--call` door take no request bound and ignore it.
  **What to do:** nothing, unless you serve over HTTP and need larger requests;
  then set it, and give the machine the memory one such call holds.
