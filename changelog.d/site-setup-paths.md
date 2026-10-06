### Added

- **reflow2 has a public setup page, at flo2-connector.github.io/reflow2.** It gives three
  complete ways to set reflow2 up, in flo2.io's dark layout:
  - on your own computer, with an agent that speaks MCP;
  - in VS Code, where an organisation blocks third-party MCP servers;
  - as a server for a team, from the container image.
  `site/` holds the page, and `.github/workflows/pages.yml` publishes it on each push to main
  that touches it. The setup steps are written by hand, by the owner's decision of 2026-10-05.
  Any page saying what reflow2 is or how its design works stays generated from the design.
  **What to do:** nothing. A maintainer sets the repository's Pages source to "GitHub Actions"
  once, so the first deploy has a site to publish to.
