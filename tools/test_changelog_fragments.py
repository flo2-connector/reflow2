#!/usr/bin/env python3
"""The CHANGELOG fragment tool's own net (hermetic, stdlib only).

Pins what dec:idea-how-changelog-md-gets-written-once-the-design-is-the-record
(option b, settled 2026-10-03) asks of it: a fragment per PR, checked; the cut
assembles them into CHANGELOG.md under the new version, grouped by section, and
removes them; an entry written straight into [Unreleased] fails the check, and
two PRs adding fragments merge with plain git in either order.
"""

from __future__ import annotations

import os
import pathlib
import subprocess
import sys
import tempfile
import unittest

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import changelog_fragments as cf  # noqa: E402

TOOL = HERE / "changelog_fragments.py"

CHANGELOG = """# Changelog

Intro.

## [Unreleased]

## [0.1.0] — 2026-01-01

- The first release.
"""


def run(root: pathlib.Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run([sys.executable, str(TOOL), "--root", str(root), *args],
                          capture_output=True, text=True, timeout=60)


def git(cwd: pathlib.Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True, timeout=60)


class Fragments(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory(prefix="changelog-fragments-")
        self.root = pathlib.Path(self._tmp.name)
        (self.root / "CHANGELOG.md").write_text(CHANGELOG, encoding="utf-8")
        (self.root / "changelog.d").mkdir()
        (self.root / "changelog.d" / "README.md").write_text("# not a fragment\n")

    def tearDown(self):
        self._tmp.cleanup()

    def frag(self, name: str, text: str) -> None:
        (self.root / "changelog.d" / name).write_text(text, encoding="utf-8")

    def test_well_formed_fragments_pass_the_check(self):
        self.frag("a.md", "### Fixed\n\n- **A fix.** Do nothing.\n")
        r = run(self.root, "--check")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_an_unknown_section_or_stray_text_fails_the_check(self):
        self.frag("a.md", "### Improvements\n\n- nope\n")
        self.frag("b.md", "loose text\n### Added\n\n- ok\n")
        r = run(self.root, "--check")
        self.assertEqual(r.returncode, 1)
        self.assertIn("Improvements", r.stdout)
        self.assertIn("before the first", r.stdout)

    def test_an_entry_written_straight_into_unreleased_fails_the_check(self):
        (self.root / "CHANGELOG.md").write_text(
            CHANGELOG.replace("## [Unreleased]\n", "## [Unreleased]\n\n- direct entry\n"))
        r = run(self.root, "--check")
        self.assertEqual(r.returncode, 1)
        self.assertIn("fragment", r.stdout)

    def test_the_cut_assembles_by_section_and_removes_the_fragments(self):
        self.frag("b-second.md", "### Fixed\n\n- **Second fix.**\n")
        self.frag("a-first.md", "### Added\n\n- **A feature.**\n\n### Fixed\n\n- **First fix.**\n")
        r = run(self.root, "--cut", "0.2.0", "--date", "2026-10-04")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        text = (self.root / "CHANGELOG.md").read_text()
        self.assertIn("## [Unreleased]\n\n## [0.2.0] — 2026-10-04\n\n### Added", text)
        added, fixed = text.index("### Added"), text.index("### Fixed")
        self.assertLess(added, fixed, "Keep a Changelog order")
        self.assertLess(text.index("First fix"), text.index("Second fix"), "file-name order")
        self.assertLess(text.index("## [0.2.0]"), text.index("## [0.1.0]"))
        self.assertEqual(cf.unreleased_body(text), "", "the next release starts empty")
        left = sorted(p.name for p in (self.root / "changelog.d").iterdir())
        self.assertEqual(left, ["README.md"], "the cut removes what it used")
        self.assertEqual(run(self.root, "--check").returncode, 0)

    def test_a_malformed_fragment_stops_the_cut_and_writes_nothing(self):
        self.frag("a.md", "### Nope\n\n- x\n")
        before = (self.root / "CHANGELOG.md").read_text()
        r = run(self.root, "--cut", "0.2.0")
        self.assertEqual(r.returncode, 1)
        self.assertEqual((self.root / "CHANGELOG.md").read_text(), before)
        self.assertTrue((self.root / "changelog.d" / "a.md").exists())

    def test_two_branches_adding_fragments_merge_in_either_order_with_plain_git(self):
        repo = self.root
        for args in (("init", "-q", "-b", "main"), ("config", "user.email", "t@t"),
                     ("config", "user.name", "t"), ("add", "-A"), ("commit", "-qm", "base")):
            self.assertEqual(git(repo, *args).returncode, 0)
        for branch, name in (("one", "one.md"), ("two", "two.md")):
            git(repo, "checkout", "-q", "-b", branch, "main")
            self.frag(name, f"### Fixed\n\n- **{branch}.**\n")
            git(repo, "add", "-A")
            git(repo, "commit", "-qm", branch)
            git(repo, "checkout", "-q", "main")
        for order in (("one", "two"), ("two", "one")):
            git(repo, "checkout", "-q", "-B", "land", "main")
            for b in order:
                r = git(repo, "merge", "--no-edit", "-q", b)
                self.assertEqual(r.returncode, 0, f"{order}: {r.stdout}{r.stderr}")
            names = sorted(p.name for p in (repo / "changelog.d").iterdir())
            self.assertEqual(names, ["README.md", "one.md", "two.md"])
            git(repo, "checkout", "-q", "main")


if __name__ == "__main__":
    unittest.main(verbosity=2)
