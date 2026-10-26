### Fixed

- **A design with a long chain of reasoning no longer crashes the server.**
  get_node on the last claim of a chain about 3,900 steps deep, and loop_status
  or detect_gaps on a chain about 2,000 steps deep whose steps hold, ran out of
  stack and aborted the whole process, taking down every design it served (on
  flo2.io, one process serves them all). Two walks, the one that finds what a
  claim rests on and the one that counts the steps under a proof, went one
  level deeper for every step of the chain. They now keep their own list of
  where they are, so a chain of any depth is walked in the same small stack,
  and every answer is exactly what it was. The server's threads also get a
  stack chosen on purpose, 16 MiB, where they had the 2 MiB nobody chose: it
  reserves address space, and memory is used only by a call that goes deep.
  **Nothing to do.** Reading a whole design whose chains are thousands of
  steps deep is still slow (loop_status took 22 s on a chain 1,000 steps
  deep); that is a separate problem, which this does not change.
