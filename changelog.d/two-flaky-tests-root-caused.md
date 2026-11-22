### Fixed

- **A read through a hub's address now arrives within the budget it was asked
  at, whatever its size.** When a member's reply came back over the budget
  with the hub's prefixes counted, the hub asked again with the budget lowered
  by the overshoot. A member lists whole rows until the next would pass, so its
  reply could already sit below that lower budget, and four asks in a row
  changed nothing: CI saw one arrive 9 characters over. The hub now lowers from
  what the member answered, and a held reply settles within two re-asks.
  Nothing to do: the reply is the same shape, and its `view_note` still says
  the design was asked at a lower budget.
- **The same search on a design nobody has written to now scores its hits the
  same every time.** The search index used to reload itself on a thread of its
  own, after every write and whenever the search engine finished tidying its
  files in the background, and nothing checked that the reload it kept was the
  newest. So a search asked twice in a row could score one hit 1.1349 and then
  1.1407, the score over the design as it stood one write earlier; and the
  tidying, which drops the old copies a rewrite leaves behind (they count in a
  hit's score), moved scores with nothing written. The index now changes only
  when the design is written. Scores can still move after a write, as they
  always could. Nothing to do.
