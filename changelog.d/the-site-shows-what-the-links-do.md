### Changed

- **reflow2's site now says what the links in a design are for, and shows how small decisions add up.**
  The home page used to read as though reflow2 only records decisions. It now says that each one is
  linked to what it rests on and what it affects. It lists the questions those links answer that a
  list of notes can't: what a change will reach, which needs nothing delivers yet, which decisions
  rest on a result that just failed, and what nobody has asked yet. A new section, "It adds up", charts
  reflow2's own design week by week, from 124 records on 2026-07-19 to over 7,000 now. The chart's
  numbers are counted from the design committed at each release tag when the site is published
  (`tools/site_counts.py`), never typed. The links each release lists for the files it ships are
  counted apart, so the size is not inflated. The site is not part of the binary: **nothing to do.**
