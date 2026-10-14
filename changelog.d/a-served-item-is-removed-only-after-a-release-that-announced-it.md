### Added

- **Anything a client can call or read is removed only after a release that
  announced it.** reflow2 now has one place that declares a deprecation, and
  what a client sees while something is going: a deprecated tool's or argument's
  description opens with "DEPRECATED in <version>, removed in the next release:
  <what to do>", `describe_schema` marks a deprecated node type, property or edge
  type with `deprecated` and lists every deprecation under `deprecations`, and a
  call that uses a deprecated item gets a `deprecation_notice` in its reply. The
  item still works exactly as before; only the next release may remove it, and a
  tool removed then still answers its old name with what to do, for that one
  release. Nothing is deprecated yet, so tools/list and every reply are unchanged.
  **What to do:** nothing now. When a release deprecates something, its
  changelog says so under "Deprecated", with what to do.
