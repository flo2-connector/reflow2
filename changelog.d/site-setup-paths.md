### Added

- **reflow2 has a public site, at flo2-connector.github.io/reflow2.** Both pages are in
  flo2.io's dark layout:
  - **The front page explains what a project brain is, in drawings.** It tells the story
    through a made-up school robotics team: where the knowledge lives, what goes in, one brain
    across repositories, troubleshooting, designing something new, and a change of requirements.
  - **A setup page gives three complete ways to set reflow2 up:** on your own computer with an
    agent that speaks MCP; in VS Code, where an organisation blocks third-party MCP servers; and
    as a server for a team, from the container image.

  `site/` holds the pages, and `.github/workflows/pages.yml` publishes them on each push to main
  that touches them. By the owner's decision of 2026-10-05, pages for someone who does not have
  reflow2 yet are written by hand. Any page that shows a real design's content stays generated
  from that design. **What to do:** nothing. A maintainer sets the repository's Pages source to
  "GitHub Actions" once, so the first deploy has a site to publish to.
