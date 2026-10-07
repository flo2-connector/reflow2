### Fixed

- **A plain `get_instructions` returns the working instructions again.** From v0.78.0 the served
  document was over 30,000 bytes, and `get_instructions` withheld any document longer than its
  30,000 default limit. So the call the handshake tells every session to make first returned a table of
  contents and no instructions. Nothing failed, and a test recorded as guarding this had never
  been committed. Now a call with no `budget_chars` returns the whole document however long it
  grows. A client that names its cap still gets the section list instead of half a document. A
  test pins both. **What to do:** nothing. If your agent has been reading
  the instructions a section at a time, a plain call works again.
- **Every project is told to find a failure's cause before fixing it.** The two-step root-cause
  rule reached only reflow2's own repository. On any failure, search the design for the exact
  error first. The moment you are about to write down *why*, in the design or in a reply, read
  the `root-cause` skill and follow it. The rule is now served in the instructions, in the
  handshake every MCP agent sees, and in the VS Code terminal route's instructions file. The
  handshake is the only one of the three that reaches an agent without a call. **What to do:** on
  the VS Code terminal route, re-run `reflow2 init . --harness vscode-cli` to refresh
  `.github/instructions/reflow2.instructions.md`. MCP projects need nothing.
