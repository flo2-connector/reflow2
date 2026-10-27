### Fixed

- **The typed edge tools now name `draw_edges`'s real argument.** Since 0.75
  every typed edge tool's description (`allocate`, `satisfies`, `invalidates`
  and 29 more) ended "MANY AT ONCE: `draw_edges`, items {…}", but `draw_edges`
  takes `edges`, so an agent that followed the text was refused. The sentence
  now reads "`draw_edges` with `edges`: [{"tool": …, "arguments": {…}}]", and
  the argument's name is read from `draw_edges`' own schema, so a rename
  cannot leave it behind again. A new check reads every served description for
  an argument named in three shapes ("`tool` with `arg`", "`tool`'s `arg`
  argument", and a JSON example after a tool's name) and fails the build when
  the tool does not take it. **What to do:** nothing; a call that sent `items`
  should send `edges`.
