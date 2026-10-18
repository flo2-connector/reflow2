### Fixed

- **A claim's `proof_status.total_steps` counts each step under it once.** A
  claim with two lines of proof that share steps used to report the SUM of its
  lines, so a shared step counted twice: on proof trial rung 2 the uniqueness
  claim read 47 steps while the theorem built on it read 41, and on rung 1 the
  theorem read 26 steps in a design holding 25. `total_steps` is now the number
  of distinct steps under the claim's holding lines, and `confirmed_steps` and
  `unconfirmed_steps` split that same set (uniqueness reads 40, rung 1's theorem
  21). `proven` is still judged line by line: a line is proven only when every
  one of its own steps, shared ones included, is confirmed. A refuted claim that
  a step concludes now counts its steps too, where it used to read 0. Every
  `proof_status` now carries `counts`, a sentence saying what the numbers mean.
  **What to do:** nothing, unless you
  pinned a summed number; a reader of `total_steps` gets the smaller, distinct
  count.
- **`reasoning_report` with no `claim` reads every claim of the reasoning,
  refuted ones included.** A conjecture that only a counterexample points to was
  never read in the whole-design walk, because the walk followed an objection
  only from the claim it is against. Read over the whole design, the walk now
  also follows an objection to its target, and any claim the walk from the top
  conclusions does not reach (a concluded node, a statement with a
  `statement_kind`, or either end of an objection) becomes a root of its own, so
  a bare conjecture refuted by a bare counterexample is read from the
  conjecture. A read under one `claim` is unchanged: it reads what is under that
  claim. **What to do:** a client that asked about each objected-to claim one at
  a time to find refuted ones can read them from the whole-design reply.
