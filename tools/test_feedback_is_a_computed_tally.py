#!/usr/bin/env python3
"""The usage ledger holds the verb and never the object, and `usage_report`
renders it — measured against the real binary, over the wire.

`req:feedback-on-reflow2-is-computed-from-a-record-the-server-kept`
(increment 437). The hook lives in the server's `call_tool`, which no Rust
integration test reaches: the mcp suite calls handler methods directly, and a
direct call never passes the one place every wire call passes. So this drives
`target/debug/reflow2-mcp` over stdio exactly as a harness does, makes calls of
every outcome class, and then reads the file the server left beside the store.

What it pins, and why each one:

  1. ONE LINE PER CALL, classed. An answered call is `ok`; a call the
     deserialiser refused is `refused/missing_argument`; a call a tool's own
     rule refused is `refused/refused`. The class table is the server's own
     phrasing and this is where its drift would show.
  2. THE VERB, NEVER THE OBJECT. The requirement written during the run has an
     id and a statement chosen to be unmistakable; neither appears anywhere in
     the ledger. This is the line `req:telemetry-carries-usage-never-design-content`
     draws, checked on bytes rather than on intent.
  3. THE HARNESS IS NAMED FROM THE HANDSHAKE. The client name and version this
     probe declares at `initialize` are on every line.
  4. `get_skill` KEEPS THE SKILL NAME and nothing else keeps any argument.
  5. THE REPORT CLOSES ITS WINDOW. A second report sees only what happened
     after the first one's marker — which is exactly one call: the first
     report itself, whose own ledger line lands after the marker it left.
  6. (An in-memory design's "no ledger" answer is pinned in the Rust suite,
     since the binary has no in-memory mode.)

Usage (from the repo root, after `cargo build -p reflow2-mcp`):

    python3 tools/test_feedback_is_a_computed_tally.py

Exits 0 when every pin holds, 1 otherwise. Standard library only.
"""

import json
import re
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
from reflow2_bin import default_bin  # noqa: E402  (one binary for every gate)

REPO = pathlib.Path(__file__).resolve().parent.parent
BINARY = pathlib.Path(default_bin())

CLIENT_NAME = "feedback-probe"
CLIENT_VERSION = "9.9.9"

# Chosen to be unmistakable if they ever leak into the ledger.
SECRET_ID = "req:the-secret-ingredient-nobody-may-see"
SECRET_TEXT = "SECRET SAUCE RECIPE the customer must never lose"


class Server:
    def __init__(self, *args):
        self.proc = subprocess.Popen(
            [str(BINARY), *args],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            bufsize=1,
            env={**os.environ, "RUST_LOG": "warn"},
        )
        self.ident = 0
        self.rpc(
            "initialize",
            {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": CLIENT_NAME, "version": CLIENT_VERSION},
            },
        )
        self.notify("notifications/initialized")

    def notify(self, method):
        self.proc.stdin.write(json.dumps({"jsonrpc": "2.0", "method": method}) + "\n")
        self.proc.stdin.flush()

    def rpc(self, method, params=None):
        self.ident += 1
        msg = {"jsonrpc": "2.0", "method": method, "id": self.ident}
        if params is not None:
            msg["params"] = params
        self.proc.stdin.write(json.dumps(msg) + "\n")
        self.proc.stdin.flush()
        line = self.proc.stdout.readline()
        if not line:
            raise SystemExit("server closed stdout")
        return json.loads(line)

    def call(self, name, arguments):
        return self.rpc("tools/call", {"name": name, "arguments": arguments})

    def close(self):
        self.proc.stdin.close()
        self.proc.terminate()
        try:
            self.proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.proc.kill()


def structured(reply):
    return reply.get("result", {}).get("structuredContent")


def main() -> int:
    if not BINARY.exists():
        print(f"SKIP: {BINARY} not built (cargo build -p reflow2-mcp)")
        return 0

    failures = []

    def check(name, ok, detail=""):
        print(("ok   " if ok else "FAIL ") + name + (f" — {detail}" if detail and not ok else ""))
        if not ok:
            failures.append(name)

    work = pathlib.Path(tempfile.mkdtemp(prefix="reflow2-feedback-tally-"))
    graph = work / "graph"
    ledger = work / "graph.usage.jsonl"
    try:
        s = Server("--graph-path", str(graph))
        # 1 · ok
        r = s.call(
            "add_requirement",
            {"id": SECRET_ID, "name": "The secret ingredient", "statement": SECRET_TEXT},
        )
        first_reply = r
        check("the requirement write itself succeeded", "error" not in r and not r["result"].get("isError"), str(r)[:200])
        # 2 · refused / missing_argument — the deserialiser, before any handler
        r = s.call("get_node", {})
        check("a call with no `id` is refused", r["result"].get("isError") is True, str(r)[:200])
        # 3 · refused / refused — a tool's own rule (a settling status with nobody's name)
        r = s.call(
            "add_decision",
            {
                "id": "dec:signed-by-nobody",
                "name": "Signed by nobody",
                "decision": "A choice landing past proposed with no approver.",
                "rationale": "To be refused.",
                "status": "accepted",
            },
        )
        check(
            "a settlement with nobody's name is refused",
            "error" in r or r["result"].get("isError") is True,
            str(r)[:200],
        )
        # 4 · a skill fetch — the one argument the ledger keeps
        r = s.call("get_skill", {"name": "where-am-i"})
        check("get_skill answered", "error" not in r and not r["result"].get("isError"))

        # ---- the ledger, read off disk ----
        check("the ledger exists beside the store", ledger.exists(), str(ledger))
        raw = ledger.read_text() if ledger.exists() else ""
        lines = [json.loads(l) for l in raw.splitlines() if l.strip()]
        calls = [l for l in lines if l.get("kind") == "call"]
        check("one line per call (4 calls)", len(calls) == 4, f"got {len(calls)}: {[l.get('tool') for l in calls]}")
        by_tool = {l["tool"]: l for l in calls}
        check("the answered call is `ok`", by_tool.get("add_requirement", {}).get("outcome") == "ok")
        check(
            "the bare call is `refused/missing_argument`",
            by_tool.get("get_node", {}).get("outcome") == "refused"
            and by_tool.get("get_node", {}).get("refusal") == "missing_argument",
            str(by_tool.get("get_node")),
        )
        check(
            "the nobody's-name settlement is `refused/refused`",
            by_tool.get("add_decision", {}).get("outcome") == "refused"
            and by_tool.get("add_decision", {}).get("refusal") == "refused",
            str(by_tool.get("add_decision")),
        )
        check("get_skill keeps the skill's name", by_tool.get("get_skill", {}).get("skill") == "where-am-i")
        check(
            "every line names the harness from the handshake",
            all(l.get("client") == CLIENT_NAME and l.get("client_version") == CLIENT_VERSION for l in calls),
            str(calls[:1]),
        )
        check("every line carries a duration and a seat", all("ms" in l and l.get("seat") for l in calls))
        # THE LINE THAT MATTERS
        check("the requirement's id is nowhere in the ledger", SECRET_ID not in raw)
        check("the requirement's statement is nowhere in the ledger", "SECRET SAUCE" not in raw)
        check("no argument name of the write is in the ledger", "statement" not in raw and "The secret ingredient" not in raw)

        # ---- the cues (cap:each-cue-carries-an-id-and-is-counted-by-whether-it-was-followed) ----
        offers = [l for l in lines if l.get("kind") == "cue"]
        check(
            "the handshake offers the first-read cue to this session",
            len(offers) == 1
            and offers[0].get("cues") == ["instructions/graph_report", "instructions/search_design"]
            and offers[0].get("seat") == calls[0].get("seat"),
            str(offers),
        )
        cued = [c for l in calls for c in l.get("cues", [])]
        check("a reply's cue ids are on its ledger line", bool(by_tool.get("add_requirement", {}).get("cues")), str(by_tool.get("add_requirement")))
        check(
            "a cue id is `<field>/<named call>`, reflow2's words only",
            all(re.fullmatch(r"[a-z_]+/[a-z_]+(:[a-z-]+)?", c) for c in cued) and SECRET_ID not in json.dumps(cued),
            str(cued),
        )
        meta = (first_reply.get("result") or {}).get("_meta") or {}
        check(
            "the reply carries the same ids in _meta",
            meta.get("reflow2/cues") == by_tool.get("add_requirement", {}).get("cues"),
            str(meta),
        )

        # ---- the report ----
        r = s.call("usage_report", {})
        rep = structured(r)
        check("usage_report answers structured", rep is not None, str(r)[:300])
        if rep:
            t = rep["tally"]
            check("the report counts the 4 calls", t["calls"] == 4, str(t["calls"]))
            check("refusals are counted by class", t["refusals_by_class"].get("missing_argument") == 1 and t["refusals_by_class"].get("refused") == 1, str(t["refusals_by_class"]))
            check("refusals are counted by tool", t["refusals_by_tool"].get("get_node", {}).get("missing_argument") == 1, str(t["refusals_by_tool"]))
            check("skills fetched are named", t["skills_fetched"].get("where-am-i") == 1, str(t["skills_fetched"]))
            check("the harness is listed", f"{CLIENT_NAME} {CLIENT_VERSION}" in t["clients"], str(t["clients"]))
            check("never_called is against the served surface", "loop_status" in t["never_called"] and "add_requirement" not in t["never_called"])
            env = rep["environment"]
            check("the environment names reflow2's version, OS, harness and design size",
                  bool(env.get("reflow2_version")) and bool(env.get("os")) and env.get("harness_last_connected") == f"{CLIENT_NAME} {CLIENT_VERSION}" and env.get("design_nodes", 0) >= 1,
                  str(env))
            check("the model is declared unknown to the server", "self-reported" in env.get("model", ""))
            check("the report left its marker", rep.get("marker_left") is True)
            cues = t.get("cues") or {}
            first = (cues.get("by_cue") or {}).get("instructions/graph_report") or {}
            check(
                "the report counts each cue offered and whether it was followed",
                first.get("offered") == 1 and first.get("followed_at_all") == 0
                and cues.get("follow_within_calls") == 5 and cues.get("offered", 0) >= 2,
                str(cues)[:400],
            )
            check("the secret is not in the report either", SECRET_ID not in json.dumps(rep) and "SECRET SAUCE" not in json.dumps(rep))

        # 5 · the window closed: a second report sees only the first report's own line
        r = s.call("usage_report", {"peek": True})
        rep2 = structured(r)
        check("a second report starts after the marker", rep2 is not None and rep2["tally"]["calls"] == 1 and rep2["tally"]["by_tool"] == {"usage_report": 1}, str(rep2 and rep2["tally"]["by_tool"]))
        check("the second report says where its window began", rep2 is not None and rep2["tally"]["window"].get("from") == "last_report")
        check("a peek leaves no marker", rep2 is not None and rep2.get("marker_left") is False)
        raw2 = ledger.read_text()
        check("peek wrote no marker line", raw2.count('"kind":"report"') == 1, str(raw2.count('"kind":"report"')))

        # 5b · A COUNTED REFUSING STUB (crates/reflow2-mcp/src/deprecation.rs,
        #      req:the-2026-10-30-fold-and-removal-candidates-ship-as-counted-
        #      refusing-stubs-then-december-decides): a call to a retired name
        #      changes nothing, answers with the call that does its job, and
        #      lands in the ledger UNDER ITS OWN NAME — the count the December
        #      re-count decides by.
        r = s.call("set_artifact_checksum", {"artifact_id": "art:none", "disposition": "baseline_established"})
        said = json.dumps(r)
        check("a stub call is refused", "error" in r or (r.get("result") or {}).get("isError") is True, said[:300])
        check("the stub names the call that does its job", "set_artifact_checksums" in said and "RETIRED" in said, said[:400])
        stub_lines = [json.loads(l) for l in ledger.read_text().splitlines() if l.strip()]
        stub_lines = [l for l in stub_lines if l.get("kind") == "call" and l.get("tool") == "set_artifact_checksum"]
        check("the stub call is in the ledger under its own name, as a refusal",
              len(stub_lines) == 1 and stub_lines[0].get("outcome") == "refused" and stub_lines[0].get("refusal") == "refused",
              str(stub_lines))
        r = s.call("usage_report", {"peek": True})
        rep3 = structured(r) or {}
        stubs = rep3.get("retired_stubs") or {}
        check("usage_report counts each stub, the called one and the uncalled ones",
              (stubs.get("counts") or {}).get("set_artifact_checksum") == 1
              and (stubs.get("counts") or {}).get("evidence_report") == 0
              and stubs.get("called") == 1 and stubs.get("re_count") == "2026-12-23",
              str(stubs)[:400])
        check("the stub's refusal is counted by tool",
              (rep3.get("tally") or {}).get("refusals_by_tool", {}).get("set_artifact_checksum", {}).get("refused") == 1,
              str((rep3.get("tally") or {}).get("refusals_by_tool")))
        s.close()

        # 6 · in-memory: the binary has no in-memory mode, so that pin lives in
        #     crates/reflow2-mcp/tests/feedback_is_a_computed_tally.rs, which
        #     calls the handler on `ReflowService::in_memory()` directly.

        # 7 · A BATCH CUE IS OFFERED ONCE PER BATCH, decided from this ledger
        #     (crates/reflow2-mcp/src/cues.rs BATCH_CUES; increment 7 of
        #     epoch:planned-cognition-plus-memory), and 8 · a process names its
        #     step. A fresh store, so its ledger starts empty.
        batch_check(check, work / "batch")
    finally:
        shutil.rmtree(work, ignore_errors=True)

    if failures:
        print(f"\nFAIL: {len(failures)} pin(s) broken: {failures}")
        return 1
    print("\nOK: the ledger holds the verb and never the object, and the report renders it.")
    return 0


# crates/reflow2-mcp/src/cues.rs WITHHELD_HINT, word for word.
WITHHELD = ("loop: the step for when this batch lands was named earlier in the batch and is not "
            "repeated on every write")


def batch_check(check, work):
    """On 0.94.0 detect_gaps was offered on 97 writes and followed on none, and
    loop_status on 162 and followed only at the end of each round: the step for
    when a batch lands is now offered on the batch's first write, withheld on
    the next ones, and offered again once the session acts on it."""
    work.mkdir(parents=True, exist_ok=True)
    ledger = work / "graph.usage.jsonl"
    s = Server("--graph-path", str(work / "graph"))
    try:
        def write(name, args):
            r = s.call(name, args)
            check(f"{name} {args.get('id') or args.get('node_id')} is answered",
                  "error" not in r and not r["result"].get("isError"), str(r)[:300])
            return structured(r) or {}

        def ledger_cues():
            lines = [json.loads(l) for l in ledger.read_text().splitlines() if l.strip()]
            return [l.get("cues", []) for l in lines if l.get("kind") == "call"]

        def req(i):
            return {"id": f"req:batch-{i}", "name": f"Batch {i}", "statement": f"The pump starts within {i} s."}

        h1 = write("add_requirement", req(1)).get("loop_hint", "")
        h2 = write("add_requirement", req(2)).get("loop_hint", "")
        h3 = write("add_requirement", req(3)).get("loop_hint", "")
        check("the first write of a batch is offered the batch step", "detect_gaps" in h1 and "loop_status" in h1, h1)
        check("the next writes of the batch are not offered it again, and are told so",
              h2 == WITHHELD and h3 == WITHHELD, f"{h2!r} / {h3!r}")
        cues = ledger_cues()
        check("the ledger records the offer once, under the same id",
              len(cues) == 3 and "loop_hint/detect_gaps" in cues[0]
              and not any("loop_hint/detect_gaps" in c for c in cues[1:]), str(cues))
        # Acting on it ends the batch: the next write is offered it again.
        s.call("detect_gaps", {})
        h4 = write("add_requirement", req(4)).get("loop_hint", "")
        check("acting on the batch step ends the batch: offered again", "detect_gaps" in h4, h4)
        # The loop_status half was just offered (by req 4); the settling half is
        # the owner-only cue (0.94.0, 0.95.0) and is said on every call.
        h5 = write("replace_text", {"node_id": "req:batch-1", "field": "statement",
                                    "new": "The pump starts within 2 s."}).get("loop_hint", "")
        check("replace_text keeps its settling clause and withholds only the batch step",
              "set_decision_status" in h5 and "loop_status" not in h5, h5)

        write("add_artifact", {"id": "art:batch-pump", "name": "pump.rs", "artifact_type": "code",
                               "location": "src/pump.rs"})
        base = {"change_type": "new_feature", "subject": "system", "summary": "Faster start.",
                "detected_at": "2026-10-30"}
        c1 = write("add_change_event", {"id": "chg:batch-file", "name": "file", **base,
                                        "affected": [{"node_id": "art:batch-pump"}]})
        check("a change that touched a registered file names the checksum step",
              "set_artifact_checksums" in (c1.get("next_step") or ""), str(c1)[:400])
        c2 = write("add_change_event", {"id": "chg:batch-words", "name": "words", **base,
                                        "affected": [{"node_id": "req:batch-2"}]})
        check("a change that touched no file names none", "next_step" not in c2, str(c2)[:400])
        cues = ledger_cues()
        check("the step cue is counted under its own id",
              "next_step/set_artifact_checksums" in cues[-2] and "next_step/set_artifact_checksums" not in cues[-1],
              str(cues[-2:]))
    finally:
        s.close()


if __name__ == "__main__":
    sys.exit(main())
