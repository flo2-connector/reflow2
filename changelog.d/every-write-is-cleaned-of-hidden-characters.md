### Security

- **Every write is cleaned of hidden characters, and says so.** Text a write tool stores no
  longer keeps Unicode tag characters (an invisible alphabet), bidirectional overrides or
  zero-width formatting. Until now only text brought in through INGEST was cleaned, so on a design
  several people write to, someone else's hidden text reached your agent unchanged. A write that
  removed any says what and from which fields, under `sanitized` in its reply. The zero-width
  joiner emoji use is kept.
- **An id holding hidden characters is refused.** The store accepted them, so two ids that read
  the same could be different ids. Cleaning one would quietly address the other, so it is refused,
  naming the field. **What to do:** send the id without them.
- **An import reports hidden characters and stores the document as given.** An export's content
  is its identity, so `import_graph` does not rewrite it. Its reply names every field holding them
  under `hidden_characters`. **What to do:** clean them in the source and import it again if they
  should not be there.
