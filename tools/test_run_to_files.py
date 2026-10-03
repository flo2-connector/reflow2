#!/usr/bin/env python3
"""The run-to-files converter's own net: a real run must reach the design as
per-file outcomes, and nothing it cannot place may read as a pass."""

from __future__ import annotations

import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import run_to_files as conv  # noqa: E402


def tree() -> pathlib.Path:
    root = pathlib.Path(tempfile.mkdtemp(prefix="run2files-"))
    for crate in ("alpha-core", "alpha-mcp"):
        d = root / "crates" / crate
        (d / "src").mkdir(parents=True)
        (d / "tests").mkdir()
        (d / "Cargo.toml").write_text("[package]\n")
        (d / "src" / "lib.rs").write_text("")
    (root / "crates/alpha-core/tests/good.rs").write_text("")
    (root / "crates/alpha-mcp/tests/bad.rs").write_text("")
    (root / "crates/alpha-mcp/tests/broken.rs").write_text("")
    (root / "crates/alpha-mcp/tests/crashed.rs").write_text("")
    # build output must never be searched or matched
    (root / "target/debug").mkdir(parents=True)
    (root / "target/debug/Cargo.toml").write_text("")
    return root


ESC = "\x1b[1m\x1b[92m"
CARGO = f"""\
{ESC}     Running\x1b[0m unittests src/lib.rs (target/debug/deps/alpha_core-0123abcd)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     Running unittests src/lib.rs (target/debug/deps/alpha_mcp-4567abcd)
test result: ok. 1 passed; 0 failed
{ESC}     Running\x1b[0m tests/good.rs (target/debug/deps/good-89abcdef)
test result: ok. 2 passed; 0 failed
     Running tests/bad.rs (target/debug/deps/bad-89abcdef)
test result: FAILED. 1 passed; 1 failed
     Running tests/crashed.rs (target/debug/deps/crashed-89abcdef)
error: test failed, to rerun pass `-p alpha-mcp --test crashed`
error: 3 targets failed:
    `-p alpha-mcp --test bad`
    `-p alpha-mcp --test broken`
    `-p alpha-mcp --test crashed`
"""


class Cargo(unittest.TestCase):
    def setUp(self):
        self.root = tree()
        self.out: dict[str, str] = {}
        self.unresolved = conv.from_cargo(CARGO, self.root, self.out)

    def test_each_target_is_placed_in_its_own_crate(self):
        self.assertEqual(self.out["crates/alpha-core/src/lib.rs"], "passed")
        self.assertEqual(self.out["crates/alpha-mcp/src/lib.rs"], "passed")
        self.assertEqual(self.out["crates/alpha-core/tests/good.rs"], "passed")
        self.assertEqual(self.unresolved, [])

    def test_colour_codes_from_a_ci_log_do_not_hide_a_target(self):
        # The two coloured lines are the first unittests and good.rs.
        self.assertIn("crates/alpha-core/tests/good.rs", self.out)

    def test_a_failed_target_is_failed(self):
        self.assertEqual(self.out["crates/alpha-mcp/tests/bad.rs"], "failed")

    def test_a_target_that_never_started_is_blocked_not_dropped(self):
        # It did not compile, so none of its tests ran: that says nothing about
        # the code under test, and `failed` would say it is broken.
        self.assertEqual(self.out["crates/alpha-mcp/tests/broken.rs"], "blocked")

    def test_a_target_that_started_and_printed_no_result_is_failed(self):
        # It ran and died (a crash or an abort): that is a failure of the code.
        self.assertEqual(self.out["crates/alpha-mcp/tests/crashed.rs"], "failed")

    def test_build_output_is_never_matched(self):
        self.assertFalse(any(k.startswith("target/") for k in self.out))


class Junit(unittest.TestCase):
    def test_worst_outcome_per_file_and_unplaced_cases_are_counted(self):
        root = tree()
        xml = root / "r.xml"
        xml.write_text(
            """<testsuite>
  <testcase classname="a" name="one" file="tests/a.py"/>
  <testcase classname="a" name="two" file="tests/a.py"><failure/></testcase>
  <testcase classname="b" name="one" file="./tests/b.py"><skipped/></testcase>
  <testcase classname="c" name="nofile"/>
</testsuite>"""
        )
        out: dict[str, str] = {}
        no_file = conv.from_junit(str(xml), root, out)
        self.assertEqual(out, {"tests/a.py": "failed", "tests/b.py": "skipped"})
        self.assertEqual(no_file, 1)

    def test_an_error_before_the_body_ran_is_blocked_and_one_while_it_ran_is_failed(self):
        # fact:root-cause-a-check-that-did-not-run-reads-did-not-work-as-designed-because-only-failing-is-loud-2026-10-02:
        # pytest writes a collection error as <error message="collection
        # failure">, and every <error> used to become `failed`.
        root = tree()
        xml = root / "r.xml"
        xml.write_text(
            """<testsuite>
  <testcase classname="" name="tests.test_c" file="tests/c.py"><error message="collection failure">ImportError</error></testcase>
  <testcase classname="d" name="one" file="tests/d.py"><error message="failed on setup with &quot;fixture&quot;">E</error></testcase>
  <testcase classname="e" name="one" file="tests/e.py"><error message="java.lang.NullPointerException">at Foo</error></testcase>
  <testcase classname="f" name="one" file="tests/f.py"/>
  <testcase classname="f" name="two" file="tests/f.py"><error message="collection failure"/></testcase>
  <testcase classname="g" name="one" file="tests/g.py"><failure/></testcase>
  <testcase classname="g" name="two" file="tests/g.py"><error message="collection failure"/></testcase>
</testsuite>"""
        )
        out: dict[str, str] = {}
        conv.from_junit(str(xml), root, out)
        self.assertEqual(
            out,
            {
                "tests/c.py": "blocked",
                "tests/d.py": "blocked",
                "tests/e.py": "failed",
                "tests/f.py": "blocked",  # a file one of whose cases never ran did not pass
                "tests/g.py": "failed",  # and a failure is never hidden by one
            },
        )

    def test_the_outcomes_are_the_ones_the_reconcile_accepts(self):
        # verify.rs OBSERVED_OUTCOMES — the reconcile refuses anything else.
        root = pathlib.Path(__file__).resolve().parent.parent
        src = (root / "crates/reflow2-core/src/verify.rs").read_text()
        line = next(l for l in src.splitlines() if l.startswith("pub const OBSERVED_OUTCOMES"))
        declared = line.split("= &[", 1)[1].split("]", 1)[0]
        self.assertEqual(sorted(conv.RANK), sorted(x.strip('" ') for x in declared.split(",")))


class Python(unittest.TestCase):
    def test_exit_code_is_the_result(self):
        root = tree()
        (root / "tools").mkdir()
        (root / "tools/test_ok.py").write_text("raise SystemExit(0)\n")
        (root / "tools/test_no.py").write_text("raise SystemExit(1)\n")
        out: dict[str, str] = {}
        conv.from_python("tools/test_*.py", root, out)
        self.assertEqual(out, {"tools/test_no.py": "failed", "tools/test_ok.py": "passed"})


class Worst(unittest.TestCase):
    def test_a_later_pass_never_hides_an_earlier_failure(self):
        out: dict[str, str] = {}
        conv.worst(out, "f", "failed")
        conv.worst(out, "f", "passed")
        conv.worst(out, "f", "skipped")
        conv.worst(out, "f", "blocked")
        self.assertEqual(out["f"], "failed")




class Refusals(unittest.TestCase):
    def test_a_log_with_no_test_target_is_refused_not_read_as_clean(self):
        root = tree()
        empty = root / "empty.log"
        empty.write_text("")
        self.assertEqual(conv.main(["--cargo", str(empty), "--root", str(root)]), 2)

    def test_the_converter_is_not_named_like_a_test(self):
        # A `test_*.py` glob must never pick up the converter itself — it did
        # on 2026-09-23 and recorded a usage error as a failed test.
        self.assertFalse(pathlib.Path(conv.__file__).name.startswith("test_"))


if __name__ == "__main__":
    unittest.main(verbosity=2)
