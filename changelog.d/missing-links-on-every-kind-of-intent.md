### Added

- **Requirements and accepted decisions linked to no other intent are noticed, not only open
  ideas.** A new gap finding, `unlinked_intent`, counts the requirements and accepted decisions
  that relate to no other requirement or idea. That means no review relation, no `DECOMPOSES`, no
  `GOVERNED_BY`, and no note saying their relations were reviewed. A capability satisfying a
  requirement does not count as a link. It is one low-severity finding with its denominator; on
  reflow2's own design it reads "134 of 890". It is not asked in a design with fewer than six
  live requirements and accepted decisions, where the whole intent is read at once. The
  `link-ideas` skill now works this backlog alongside `unreviewed_ideas`. **What to do:** nothing; work it a few at a time when it suits you.
- **Capturing a requirement offers what it relates to.** `capture-intent` now calls
  `relation_candidates` on a new requirement, puts the real candidates to you, and links the ones
  you confirm.
