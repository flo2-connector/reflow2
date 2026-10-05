### Added

- **A guide to using reflow2 from VS Code's Copilot agent where an organisation blocks third-party MCP
  servers: [docs/using-reflow2-in-vscode-without-mcp.md](docs/using-reflow2-in-vscode-without-mcp.md).**
  It grew from a field log kept while working that way, and it is now checked against v0.78.0. It covers:
  - the terminal door: `reflow2 read <tool>` and `reflow2 write <tool>`, `--call`, and what each exit code
    means;
  - one VS Code setting that approves every read and leaves each write asking;
  - a user-scope instructions file that teaches the agent the door, until `reflow2 init` installs the route
    for VS Code;
  - a table of what was measured on main;
  - the limitations that still hold, each with the finding behind it;
  - where each of the field report's 19 ideas is recorded and how far it has got.

  **What to do:** if you drive reflow2 from VS Code's terminal, follow the guide's instructions-file list. It
  drops two steps that v0.78.0 made unnecessary: re-reading a node after a refused write, and exporting by
  hand after each write.
