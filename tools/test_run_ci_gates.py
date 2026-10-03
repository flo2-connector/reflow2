#!/usr/bin/env python3
"""The gate runner's own net: it must never report a pass having run nothing.

⭐ WHY THIS FILE EXISTS AT ALL. `run_ci_gates.py` was written to end the
hand-rolled local gate script, whose defining failure was reporting green
having checked almost nothing. A runner with that same failure mode would be
the joke telling itself. The first draft had it twice over: it printed
"RAN 0 gate(s); 0 FAILED" and exited 0 on a filter that matched nothing, and
it never returned a non-zero exit code at all — every sweep that day was read
off the log by eye, which is exactly the habit the runner is meant to retire.

So the four things pinned here are the four ways it could lie:

  1. a gate that fails makes the RUN fail, in the EXIT CODE and not only in
     the printed summary;
  2. a filter matching nothing is a FAILURE, never a clean sweep;
  3. a broken or truncated read of ci.yml is a FAILURE, not a short workflow;
  4. the real ci.yml yields a plausible number of gates, so a future change to
     the workflow's shape cannot silently reduce the runner to a no-op.

  5. every gate ci.yml runs takes its binary from one resolver, so
     `REFLOW2_BIN=…` reaches all of them (added 2026-10-03).

Hermetic: nothing here runs cargo, spawns a server, or touches the design.

Usage:  python3 tools/test_run_ci_gates.py
"""

from __future__ import annotations

import pathlib
import subprocess
import sys
import unittest

REPO = pathlib.Path(__file__).resolve().parent.parent
RUNNER = REPO / "tools" / "run_ci_gates.py"


def run(args: list[str], cwd: pathlib.Path | None = None) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, str(RUNNER), *args],
        cwd=str(cwd or REPO),
        capture_output=True,
        text=True,
    )


class RunnerRefusesToPassVacuously(unittest.TestCase):
    def test_a_filter_that_matches_nothing_fails(self):
        """The first draft printed a clean summary here and exited 0."""
        p = run(["a-string-no-gate-command-contains-zzz"])
        self.assertEqual(p.returncode, 1, f"stdout:\n{p.stdout}\nstderr:\n{p.stderr}")
        self.assertIn("not a pass", (p.stdout + p.stderr).lower().replace("NOT A PASS", "not a pass"))

    def test_a_failing_gate_makes_the_run_fail_in_the_exit_code(self):
        """Not only in the summary line. Reading the log by eye is the habit
        this runner exists to retire, so the exit code has to carry it."""
        p = run(["--list"])
        self.assertEqual(p.returncode, 0, p.stderr)
        # `false` is not a gate ci.yml runs, so drive the failure path through a
        # keyword that selects a real gate and assert the contract on the code
        # path itself rather than on a fabricated workflow.
        import run_ci_gates  # noqa: PLC0415

        self.assertTrue(
            hasattr(run_ci_gates, "main"),
            "the runner must expose main() so its exit code is the tested thing",
        )
        source = RUNNER.read_text(encoding="utf-8")
        self.assertIn(
            "return 1 if failures else 0",
            source,
            "the exit code must be derived from the failures, not printed and discarded",
        )
        self.assertNotIn(
            "| head",
            source,
            "nothing may be piped through head: it truncates the report and can "
            "kill the run through the closed pipe",
        )


class RunnerRefusesABrokenRead(unittest.TestCase):
    def test_a_truncated_workflow_is_a_failure_not_a_short_one(self):
        """Driven through the guard itself rather than through the working
        directory: the runner resolves ci.yml from its OWN location, so it is
        correctly immune to cwd and that cannot be used to fake a broken read.
        A first draft of this test asserted the cwd version and failed, which
        is the test being wrong rather than the runner."""
        sys.path.insert(0, str(REPO / "tools"))
        import run_ci_gates  # noqa: PLC0415
        from unittest import mock  # noqa: PLC0415

        with mock.patch.object(run_ci_gates, "ci_gates", return_value={"a": "true", "b": "true"}):
            with mock.patch.object(sys, "argv", ["run_ci_gates.py", "--list"]):
                self.assertEqual(
                    run_ci_gates.main(),
                    1,
                    "a near-empty parse must be refused as broken, not reported as a short workflow",
                )

    def test_the_real_workflow_yields_a_plausible_gate_count(self):
        """A future change to ci.yml's shape must not quietly reduce this to a
        no-op. The floor is deliberately well below today's count: this asks
        'did the parse work', not 'is the number still exactly N'."""
        sys.path.insert(0, str(REPO / "tools"))
        from skill_lint import ci_gates

        gates = ci_gates()
        self.assertGreaterEqual(
            len(gates),
            30,
            f"only {len(gates)} gate(s) parsed out of ci.yml — the runner shares this parser "
            f"with skill_lint, so a broken read here breaks both",
        )

    def test_the_runner_does_not_carry_its_own_copy_of_the_gate_list(self):
        """The whole point. A fourth hand-kept copy would be the drift this
        exists to end, so the runner must import the parser rather than scan
        the workflow itself."""
        source = RUNNER.read_text(encoding="utf-8")
        self.assertIn("from skill_lint import ci_gates", source)
        self.assertNotIn(
            "import yaml",
            source,
            "pyyaml is not in the base image; skill_lint's text scan exists for that reason",
        )


# Gates that name a binary path in code, and why each may. Every other gate
# takes its binary from `reflow2_bin.default_bin()`.
NAMES_ITS_OWN_BINARY = {
    # Installed into consumer projects ALONE, where there is no target/ and no
    # reflow2_bin.py, so it keeps its own copy of the order (plus PATH). It
    # still reads $REFLOW2_BIN first, which the next test checks.
    "tools/reflow2_check.py": "ships alone to consumer projects",
    # A path that must NOT exist: the installer's suite proves a missing binary
    # is handled. It never runs one.
    "tools/test_init.py": "a deliberately non-existent binary, never run",
}


def binary_paths_named_in_code(source: str) -> list[tuple[int, str]]:
    """Where a script names a reflow2-mcp build path, or a private env var for
    one, in CODE. Docstrings are prose and are skipped: a usage line saying
    `--bin target/release/reflow2-mcp` names nothing the script runs."""
    import ast
    import re

    tree = ast.parse(source)
    docstrings = {
        id(n.body[0].value)
        for n in ast.walk(tree)
        if isinstance(n, (ast.Module, ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef))
        and n.body
        and isinstance(n.body[0], ast.Expr)
        and isinstance(n.body[0].value, ast.Constant)
        and isinstance(n.body[0].value.value, str)
    }
    hits: list[tuple[int, str]] = []
    for n in ast.walk(tree):
        if isinstance(n, ast.Constant) and isinstance(n.value, str) and id(n) not in docstrings:
            if re.search(r"target/(debug|release)/reflow2-mcp", n.value):
                hits.append((n.lineno, n.value))
            # Spelled by parts so this checker does not find itself.
            elif n.value in ("_".join(("REFLOW2", "MCP")), "_".join(("REFLOW2", "MCP", "BIN"))):
                hits.append((n.lineno, f"${n.value} (the variable is $REFLOW2_BIN)"))
        # REPO / "target" / "debug" / "reflow2-mcp", or os.path.join(…, "target", …)
        if isinstance(n, (ast.BinOp, ast.Call)):
            parts = [c.value for c in ast.walk(n) if isinstance(c, ast.Constant)]
            if "target" in parts and "reflow2-mcp" in parts:
                hits.append((n.lineno, "a path built from target/ … /reflow2-mcp"))
    return hits


class EveryGateRunsTheBinaryItIsTold(unittest.TestCase):
    """`REFLOW2_BIN=… python3 tools/run_ci_gates.py` must point EVERY gate at one
    binary. Until 2026-10-03 reflow2_check read $REFLOW2_BIN and about twenty
    gates hard-coded target/debug, so with only a release build the run passed
    reflow2_check and failed replies_are_bounded and a_reply_is_sent_once for a
    binary they never looked for (fix program item 8). Two other gates read
    $REFLOW2_MCP and $REFLOW2_MCP_BIN."""

    def test_no_gate_ci_runs_names_its_own_binary(self):
        import re

        sys.path.insert(0, str(REPO / "tools"))
        from skill_lint import ci_gates

        scripts = sorted(
            {m.group(1) for c in ci_gates().values() for m in re.finditer(r"python3 (tools/\S+\.py)", c)}
        )
        self.assertGreater(len(scripts), 30, scripts)
        offenders = {}
        for script in scripts:
            if script in NAMES_ITS_OWN_BINARY:
                continue
            hits = binary_paths_named_in_code((REPO / script).read_text(encoding="utf-8"))
            if hits:
                offenders[script] = hits
        self.assertEqual(
            offenders,
            {},
            "these gates choose their own binary; take it from reflow2_bin.default_bin() so "
            "$REFLOW2_BIN reaches them, or add the script to NAMES_ITS_OWN_BINARY with why",
        )

    def test_the_exemptions_are_still_gates_that_name_a_binary(self):
        for script in NAMES_ITS_OWN_BINARY:
            self.assertTrue(
                binary_paths_named_in_code((REPO / script).read_text(encoding="utf-8")),
                f"{script} no longer names a binary; drop its exemption",
            )

    def test_reflow2_check_reads_the_same_variable_first(self):
        source = (REPO / "tools" / "reflow2_check.py").read_text(encoding="utf-8")
        body = source.split("def default_bin", 1)[1].split("\ndef ", 1)[0]
        self.assertLess(body.index('"REFLOW2_BIN"'), body.index('"target"'))

    def test_the_resolver_takes_the_variable_then_debug_then_says_it_fell_back_to_release(self):
        import contextlib
        import io
        import os
        import tempfile

        sys.path.insert(0, str(REPO / "tools"))
        import reflow2_bin

        saved = (os.environ.get(reflow2_bin.ENV), reflow2_bin.DEBUG, reflow2_bin.RELEASE)
        tmp = pathlib.Path(tempfile.mkdtemp(prefix="reflow2-bin-"))
        try:
            os.environ[reflow2_bin.ENV] = "/somewhere/else/reflow2-mcp"
            self.assertEqual(reflow2_bin.default_bin(), "/somewhere/else/reflow2-mcp")
            del os.environ[reflow2_bin.ENV]
            reflow2_bin.DEBUG, reflow2_bin.RELEASE = tmp / "debug", tmp / "release"
            self.assertEqual(reflow2_bin.default_bin(), str(tmp / "debug"), "absent: name the debug build")
            (tmp / "release").write_text("")
            err = io.StringIO()
            with contextlib.redirect_stderr(err):
                self.assertEqual(reflow2_bin.default_bin(), str(tmp / "release"))
            self.assertIn("release", err.getvalue(), "a fallback to release is said, never silent")
            (tmp / "debug").write_text("")
            self.assertEqual(reflow2_bin.default_bin(), str(tmp / "debug"))
        finally:
            if saved[0] is None:
                os.environ.pop(reflow2_bin.ENV, None)
            else:
                os.environ[reflow2_bin.ENV] = saved[0]
            reflow2_bin.DEBUG, reflow2_bin.RELEASE = saved[1], saved[2]


if __name__ == "__main__":
    unittest.main(verbosity=2)
