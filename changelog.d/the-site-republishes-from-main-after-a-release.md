### Fixed

- **The site republishes from main after each release, instead of trying from the
  release tag and being refused.** 0.84.0 made each `v*` tag republish the site, so
  the home page's counts of reflow2's own design would take in the release just
  cut. The `github-pages` environment admits deployments from `main` alone, on
  purpose, so the v0.84.0 tag's run failed: "Tag "v0.84.0" is not allowed to deploy
  to github-pages due to environment protection rules." The counts came out right
  that once only because main's run at the cut's merge started six seconds before
  the tag was pushed. Now `pages.yml` has no tag trigger, and the release workflow's
  last job, `site`, dispatches it on `main` once the release is published. That job
  holds only `actions: write`, a dry run skips it, and a failure in it cannot fail
  or undo the release. Nothing to do for anyone who uses reflow2.
