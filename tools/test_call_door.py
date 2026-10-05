#!/usr/bin/env python3
"""The `--call` door, driven the way a VS Code agent drives it.

    python3 tools/test_call_door.py            # needs `cargo build -p reflow2-mcp`
    REFLOW2_BIN=... python3 tools/test_call_door.py

WHY THIS EXISTS. Where an organisation blocks MCP, a VS Code agent reaches
reflow2 only by running terminal commands, from the instructions
`reflow2 init --harness vscode-cli` writes into `.github/`. Nothing tested that
route: the served text says "call X", the door turns that into a command, and a
drift between them is found by a person at work, as the 2026-10-02 field report
was (art:vscode-call-door-field-report-2026-10-02, limitation 11, idea 11;
req:init-installs-the-terminal-route-for-vs-code-and-update-keeps-it-current).

So this sets a scratch project up with the real installer, puts the real
`reflow2` command on PATH (the wrapper `reflow2 install` writes), and then does
what the installed files tell an agent to do, checking what comes back:

  - discovery: `reflow2 read --list`, `find_tools`, `list_skills`, every skill
    stub's own `get_skill` command, `describe_schema`, and the served copy of
    the instructions file;
  - every runnable `reflow2 read …` the instructions file shows, run as written;
  - a write on a free design, with prose on stdin that names remote-shell words;
  - the refusal shapes: `read` refusing a writer by name (exit 1), a missing
    argument naming the tool and the field (exit 2);
  - a held design: a write refused with nothing written, a read answered from a
    snapshot that says so;
  - the export kept current: by the write itself where a file is named, and by
    the Stop hook where nothing names one;
  - the hooks as VS Code runs them: the hook file's command through `sh -c`, a
    SessionStart that hands loop_status to the model, a Stop that counts the
    door write and nudges once.

Standard library only. FAILS, rather than skipping, when the binary is not
built: this is a CI gate, and a gate that passes having run nothing is the
failure tools/test_run_ci_gates.py exists to forbid.
"""

from __future__ import annotations

import importlib.util
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile
import time
import unittest

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parent
sys.path.insert(0, str(HERE))
from reflow2_bin import default_bin  # noqa: E402  (one binary for every gate)

BINARY = pathlib.Path(default_bin())
KIT = REPO / "getting-started"


def _load(name: str):
    spec = importlib.util.spec_from_file_location(name, HERE / f"{name}.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


class Door(unittest.TestCase):
    """One scratch project, set up once, driven call by call."""

    @classmethod
    def setUpClass(cls):
        if not BINARY.exists():
            raise AssertionError(
                f"{BINARY} is not built. This probe drives the real binary: "
                f"cargo build -p reflow2-mcp (or set REFLOW2_BIN)."
            )
        if shutil.which("sh") is None:
            raise AssertionError("no POSIX sh: the `reflow2` command is a shell script")
        cls.root = pathlib.Path(tempfile.mkdtemp(prefix="reflow2-call-door-"))
        cls.project = cls.root / "doorproj"
        cls.project.mkdir()
        subprocess.run(["git", "init", "-q", "-b", "main", str(cls.project)], check=True)
        # The `reflow2` command exactly as the machine installer writes it.
        installer = _load("reflow2_install")
        bindir = cls.root / "bin"
        bindir.mkdir()
        wrapper = bindir / "reflow2"
        wrapper.write_text(installer.WRAPPER.format(kit=KIT, binary=BINARY.resolve()))
        wrapper.chmod(0o755)
        cls.env = dict(os.environ, PATH=f"{bindir}{os.pathsep}{os.environ.get('PATH', '')}")
        cls.env.pop("RUST_LOG", None)
        # The real installer, with the route named, as a person would run it.
        setup = subprocess.run(
            [sys.executable, str(HERE / "reflow2_init.py"), str(cls.project),
             "--harness", "vscode-cli", "--binary", str(BINARY.resolve())],
            capture_output=True, text=True, env=cls.env, timeout=300,
        )
        if setup.returncode != 0:
            raise AssertionError(f"reflow2 init failed:\n{setup.stdout}\n{setup.stderr}")
        cls.setup_out = setup.stdout
        cls.record = cls.project / "docs" / "design" / "doorproj.json"

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.root, ignore_errors=True)

    # ---- helpers -------------------------------------------------------------

    def sh(self, command: str, stdin: str | None = None, timeout: int = 120):
        """Run one command line the way VS Code's terminal tool does."""
        return subprocess.run(["sh", "-c", command], input=stdin, capture_output=True,
                              text=True, cwd=self.project, env=self.env, timeout=timeout)

    def ok(self, command: str, stdin: str | None = None) -> dict:
        r = self.sh(command, stdin)
        self.assertEqual(r.returncode, 0, f"`{command}` exited {r.returncode}:\n{r.stderr}")
        return json.loads(r.stdout)

    def hook(self, event: dict, command: str | None = None):
        """Run the hook file's own command, as VS Code would."""
        doc = json.loads((self.project / ".github" / "hooks" / "reflow2.json").read_text())
        name = event["hook_event_name"]
        command = command or doc["hooks"][name][0]["command"]
        event = {"timestamp": "2026-10-05T12:00:00Z", "cwd": str(self.project), **event}
        return self.sh(command, stdin=json.dumps(event))

    # ---- the installed files ---------------------------------------------------

    def test_01_init_installed_the_route(self):
        github = self.project / ".github"
        for rel in ("instructions/reflow2.instructions.md", "hooks/reflow2.json"):
            self.assertTrue((github / rel).exists(), f"{rel} missing:\n{self.setup_out}")
        self.assertTrue(self.record.exists(), "init writes the shareable record")

    # ---- discovery ----------------------------------------------------------

    def test_02_discovery(self):
        split = self.ok("reflow2 read --list")
        self.assertIn("loop_status", split["read"]["tools"])
        self.assertIn("add_requirement", split["write"]["tools"])

        found = self.ok("""reflow2 read find_tools '{"query": "capture a new requirement"}'""")
        self.assertIn("add_requirement", json.dumps(found))

        listed = self.ok("reflow2 read list_skills")
        served = {s["name"] for s in listed["skills"]}
        stubs = {d.name for d in (self.project / ".github" / "skills").iterdir() if d.is_dir()}
        self.assertEqual(stubs, served, "one stub per served skill, no more and no fewer")

        schema = self.ok("""reflow2 read describe_schema '{"tool": "add_decision"}'""")
        self.assertIn("add_decision", json.dumps(schema))

        served_route = self.ok(
            """reflow2 read get_instructions '{"section": "vscode-terminal-route"}'""")
        installed = (self.project / ".github" / "instructions" /
                     "reflow2.instructions.md").read_text()
        init = _load("reflow2_init")
        self.assertEqual(served_route["instructions"], init.strip_owned_marker(installed),
                         "the file init wrote is the text this binary serves")

    def test_03_every_skill_stub_routes_to_a_skill_that_answers(self):
        pattern = re.compile(r"`(reflow2 read get_skill '\{\"name\": \"[a-z0-9-]+\"\}')`")
        for stub in sorted((self.project / ".github" / "skills").glob("*/SKILL.md")):
            m = pattern.search(stub.read_text())
            self.assertIsNotNone(m, f"{stub.parent.name}: no get_skill command")
            got = self.ok(m.group(1))
            self.assertEqual(got.get("name"), stub.parent.name, m.group(1))
            self.assertGreater(len(got.get("body", "")), 200, f"{stub.parent.name}: no body")

    def test_04_the_instructions_commands_run_as_written(self):
        # Served text that says a command works must stay true through the door
        # (dec:one-reflow2-serves-both-routes-mcp-and-the-call-door). Every
        # inline `reflow2 read …` with no placeholder is run exactly as shown.
        text = (self.project / ".github" / "instructions" / "reflow2.instructions.md").read_text()
        commands = [c for c in re.findall(r"`(reflow2 read [^`]+)`", text)
                    if "<" not in c and "..." not in c]
        self.assertGreaterEqual(len(commands), 3, commands)
        for command in commands:
            self.ok(command)

    # ---- writes and refusals ----------------------------------------------------

    def test_05_a_write_on_a_free_design_with_prose_on_stdin(self):
        # The prose names ssh and rsync on purpose: limitation 20's guard
        # blocked exactly this. Through stdin it never enters the command text.
        r = self.sh("reflow2 write add_requirement --args - <<'EOF'\n"
                    '{"id": "req:door-probe", "name": "Door probe", "statement": '
                    '"Backups go over ssh with rsync, every night."}\n'
                    "EOF")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("req:door-probe", r.stdout)
        node = self.ok("""reflow2 read get_node '{"id": "req:door-probe"}'""")
        self.assertIn("rsync", json.dumps(node))

    def test_06_read_refuses_a_writer_by_name(self):
        r = self.sh("""reflow2 read add_requirement '{"id": "req:x", "name": "x", "statement": "x"}'""")
        self.assertEqual(r.returncode, 1, r.stderr)
        self.assertIn("reflow2 write add_requirement", r.stderr)
        self.assertIn("Nothing was opened", r.stderr)

    def test_07_a_missing_argument_names_the_tool_and_the_field(self):
        r = self.sh("""reflow2 write add_requirement '{"id": "req:half"}'""")
        self.assertEqual(r.returncode, 2, f"stdout={r.stdout}\nstderr={r.stderr}")
        self.assertIn("add_requirement", r.stderr)
        self.assertIn("statement", r.stderr)

    def test_08_a_held_design_refuses_writes_and_reads_from_a_snapshot(self):
        holder = subprocess.Popen(
            [str(BINARY), "--graph-path", ".reflow2/graph"], cwd=self.project,
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, bufsize=1, env={**self.env, "RUST_LOG": "error"},
        )
        try:
            holder.stdin.write(json.dumps({
                "jsonrpc": "2.0", "id": 1, "method": "initialize",
                "params": {"protocolVersion": "2025-06-18", "capabilities": {},
                           "clientInfo": {"name": "holder", "version": "1"}}}) + "\n")
            holder.stdin.flush()
            self.assertIn("result", json.loads(holder.stdout.readline()))
            r = self.sh("""reflow2 write add_requirement '{"id": "req:while-held", """
                        """"name": "Held", "statement": "Written while held."}'""")
            self.assertEqual(r.returncode, 1, f"stdout={r.stdout}\nstderr={r.stderr}")
            read = self.sh("""reflow2 read get_node '{"id": "req:door-probe"}'""")
            self.assertEqual(read.returncode, 0, read.stderr)
            self.assertIn("snapshot", read.stderr.lower(), "a snapshot read says so")
        finally:
            holder.terminate()
            holder.wait(timeout=30)
        gone = self.sh("""reflow2 read get_node '{"id": "req:while-held"}'""")
        self.assertNotEqual(gone.returncode, 0, "the refused write wrote nothing")

    # ---- the export kept current --------------------------------------------------

    def test_09_a_write_naming_its_file_keeps_the_export_current(self):
        r = self.sh(f"reflow2 --export-to {self.record.relative_to(self.project)} "
                    """write add_requirement '{"id": "req:exported", "name": "Exported", """
                    """"statement": "Lands in the record."}'""")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("req:exported", self.record.read_text())

    def test_10_the_hooks_load_the_loop_count_the_door_and_export(self):
        session = f"door-{os.getpid()}"
        start = self.hook({"hook_event_name": "SessionStart", "session_id": session,
                           "source": "new"})
        self.assertEqual(start.returncode, 0, start.stderr)
        context = json.loads(start.stdout)["hookSpecificOutput"]["additionalContext"]
        self.assertIn("loop_status", context)
        self.assertIn("reflow2 read", context)

        # A door write with no MCP config naming a file: stderr says so, and
        # the record is behind until the Stop hook writes it.
        r = self.sh("reflow2 write add_requirement --args - <<'EOF'\n"
                    '{"id": "req:by-the-hook", "name": "By the hook", '
                    '"statement": "Exported at the end of the turn."}\n'
                    "EOF")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertNotIn("req:by-the-hook", self.record.read_text())

        stop = self.hook({"hook_event_name": "Stop", "session_id": session,
                          "stop_hook_active": False})
        self.assertEqual(stop.returncode, 0, stop.stderr)
        spec = json.loads(stop.stdout)["hookSpecificOutput"]
        self.assertEqual((spec["hookEventName"], spec["decision"]), ("Stop", "block"))
        self.assertIn("1 graph write(s)", spec["reason"])
        self.assertIn("req:by-the-hook", self.record.read_text(),
                      "the Stop hook kept the committed record current")

        again = self.hook({"hook_event_name": "Stop", "session_id": session,
                           "stop_hook_active": True})
        self.assertEqual((again.returncode, again.stdout), (0, ""), "it nudges once")

    def test_11_a_missing_reflow2_warns_and_never_blocks(self):
        # VS Code reads exit 2 as a BLOCKING error. With no `reflow2` on PATH
        # the hook's own command must come back as a plain warning.
        doc = json.loads((self.project / ".github" / "hooks" / "reflow2.json").read_text())
        command = doc["hooks"]["Stop"][0]["command"]
        r = subprocess.run([shutil.which("sh"), "-c", command], input="{}",
                           capture_output=True, text=True, cwd=self.project,
                           env=dict(self.env, PATH="/nonexistent"))
        self.assertEqual(r.returncode, 1, r.stderr)


if __name__ == "__main__":
    started = time.time()
    result = unittest.main(verbosity=2, exit=False).result
    print(f"\ncall door: {result.testsRun} checks in {time.time() - started:.1f}s")
    sys.exit(0 if result.wasSuccessful() and result.testsRun else 1)
