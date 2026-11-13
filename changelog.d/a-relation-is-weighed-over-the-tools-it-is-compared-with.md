### Fixed

- **`find_tools` no longer shows more relations each time the schema gains an
  edge type.** A relation comes back beside the tools only when it scores
  close to the top tool. The relation's words were weighed over the tools and
  the schema's edge types together, and the top tool's over the tools alone,
  so every added edge type raised every relation's score and none of the
  tools'. Two placeholder edge types, with words no query uses, took the share
  of plain tool searches carrying relations from 89 to 91 of 184 and failed
  its ceiling. Relations are now weighed over the tools, the same scale, and
  the same two types change no reply: 79 of 184 (42.9%) with or without them.
  The ceiling is lowered to 43.0%. Tool rankings do not change. One
  relation question in the corpus, "which capabilities does this user role
  work with", no longer finds `INTERACTS_WITH`: it was found only through the
  word "with", and it joins its sibling "connect a user role to the feature
  they use" among the known misses. **What to do:** nothing. A design whose
  schema adds its own edge types now gets the same relations back as before
  it added them.
