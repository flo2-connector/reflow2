#!/usr/bin/env python3
"""reflow2's design on flo2.io, and the copy of it CI reads — with the one-PR guard.

    python3 tools/reflow2_design_copy.py hold     # before a PR's first design write
    python3 tools/reflow2_design_copy.py copy     # last, after `git merge origin/main`
    python3 tools/reflow2_design_copy.py status   # what is in flight, and who holds it

WHY THIS EXISTS. reflow2's own design moves onto flo2.io
(dec:idea-reflow2s-own-design-moves-onto-flo2-io-or-stays-local, settled
2026-10-09, option c1). CI still reads the committed item layout at
docs/design/reflow2/, so a PR that changes the design commits a COPY of the
hosted design, taken last. flo2 has done this since 2026-09-28 with its own
ops/flo2-design-copy; this is reflow2's, for the item layout. It retires with the
copy itself, at combined position 11
(epoch:planned-flo2-then-reflow2-move-and-the-photocopy-retires).

THE ONE-PR GUARD. With one live design, PR B's design writes are in the design
before B merges, so PR A's copy would carry B's accepted checksums against files
A's branch does not have, and A's design gate would fail on B's work. Anthony,
2026-10-09, chose "One design PR at a time": one design-writing PR in flight, its
copy taken last after merging main, and a copy that REFUSES while another PR's
changes are in the live design.

  - The hold is a claim on the claim board: `claim_region` on `proj:reflow2`, depth
    0, note `design PR: <branch>`. Claims are advisory and never block a write
    (crates/reflow2-core/src/claims.rs), so the refusal lives HERE, in the step
    every design PR must pass to reach main. A hold claim never enters a copy.
  - "In flight" is everything the live design holds that main's copy does not,
    hold claims aside. Nothing in flight means every holder's work has merged,
    so a leftover hold is stale and anyone may hold next.
  - `hold` says whether this branch may begin writing: yes when nothing is in
    flight, or this branch already holds it. It refuses while another branch
    holds it, or while writes nobody holds are in flight (an abandoned PR, or
    writes made outside one) — `--adopt` takes those into this PR deliberately.
  - `copy` refuses on the same grounds, and also until the branch contains
    main, so the copy is taken against what it will merge into.

The guard binds whoever runs it. A session that writes without holding is the
gap it cannot close: claims are advisory by design, and the live design records
who wrote, not on which branch.

HOW THE COPY IS MADE. The engine on flo2.io's droplet is asked for the whole
design (127.0.0.1:8900 over ssh, a read; nothing on flo2.io changes). The hold
claims are dropped, the rest is imported into a throwaway store by THIS repo's
reflow2 (--bin, else REFLOW2_BIN, else this checkout's build, else PATH),
and exported into docs/design/reflow2/, so each item's lineage and the stamp are
this repo's. Measured 2026-10-10 on the real design (6,976 nodes, 47,571 edges):
the import-export round trip is exact, both on 0.81.1 and from a 0.81.0 export
into 0.81.1, and writes no item when nothing changed. If a round trip ever does
change content (a schema default the source never stated, `fact:bl-198`), the
copy says which values and exits 2 rather than letting them pass as the design's.

`--from PATH` reads a saved design (either form) instead of flo2.io: the hermetic
tests use it, and so does the break-glass path in AGENTS.md when flo2.io cannot
serve the design.

Standard library only.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import design_io  # noqa: E402

HOLD_SEED = "proj:reflow2"
HOLD_PREFIX = "design PR: "
CLAIMS = "CLAIMS"
# flo2.io's droplet, as every flo2 ops script names it (FLO2_HOST_SSH overrides),
# so one known_hosts entry serves them all.
DEFAULT_HOST = "root@67.205.173.71"
ENGINE_PORT = 8900
SHOW = 12  # items named per group before "and N more"

# The droplet side of the read: ask the engine for the whole design and print it.
# It runs there with the box's python3, so it is stdlib only and self-contained.
REMOTE = r"""
import json, os, sys, urllib.request
URL = f"http://127.0.0.1:{os.environ['PORT']}/g/{os.environ['ID']}/mcp"
HEADERS = {"Content-Type": "application/json", "Accept": "application/json, text/event-stream"}

def post(body, sid=None):
    h = dict(HEADERS)
    if sid:
        h["Mcp-Session-Id"] = sid
    r = urllib.request.urlopen(urllib.request.Request(URL, json.dumps(body).encode(), h), timeout=300)
    sid = r.headers.get("Mcp-Session-Id") or sid
    raw = r.read().decode()
    msgs = [json.loads(l[5:]) for l in raw.splitlines() if l.startswith("data:") and l[5:].strip()]
    if not msgs and raw.strip().startswith("{"):
        msgs = [json.loads(raw)]
    return sid, msgs

sid, _ = post({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
    "protocolVersion": "2025-06-18", "capabilities": {},
    "clientInfo": {"name": "reflow2-design-copy", "version": "1"}}})
post({"jsonrpc": "2.0", "method": "notifications/initialized"}, sid)
_, msgs = post({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
                "params": {"name": "export_graph", "arguments": {}}}, sid)
result = [m for m in msgs if m.get("id") == 2][0]["result"]
if result.get("isError"):
    sys.exit("export refused: " + result["content"][0]["text"][:400])
doc = result.get("structuredContent")
if not doc or "nodes" not in doc:
    doc = json.loads(result["content"][0]["text"])
json.dump(doc, sys.stdout)
"""


class Refused(Exception):
    """A guard or precondition said no; the message says why and what to do."""


# ---- reading the two designs ----------------------------------------------


def fetch_live(host: str, design_id: str) -> dict:
    """The whole hosted design, read from the droplet's engine over ssh."""
    try:
        r = subprocess.run(
            ["ssh", "-o", "BatchMode=yes", host,
             f"ID={design_id} PORT={ENGINE_PORT} python3 -"],
            input=REMOTE, capture_output=True, text=True, timeout=600)
    except (OSError, subprocess.SubprocessError) as e:
        raise Refused(f"could not reach {host}: {e}") from e
    if r.returncode != 0:
        raise Refused(f"{host} did not hand back design {design_id}: "
                      f"{(r.stderr or r.stdout).strip()[-600:]}")
    return json.loads(r.stdout)


def git(root: str, *args: str) -> str:
    r = subprocess.run(["git", *args], cwd=root, capture_output=True, text=True)
    if r.returncode != 0:
        raise Refused(f"git {' '.join(args)}: {r.stderr.strip()[:300]}")
    return r.stdout.strip()


def committed_design(root: str, base: str) -> dict:
    doc = design_io.load_design_at(root, base, design_io.ITEMS_REL)
    if doc is None:
        raise Refused(f"{base} carries no design at {design_io.ITEMS_REL}/")
    return doc


# ---- the guard: pure functions of the two designs --------------------------


def holds(doc: dict) -> list[dict]:
    """Every hold on the design: a claim on HOLD_SEED, with the branch its note names."""
    out = []
    for e in doc.get("edges", []):
        if e.get("edge_type") != CLAIMS or e.get("to_id") != HOLD_SEED:
            continue
        props = e.get("properties") or {}
        note = str(props.get("note") or "")
        branch = None
        if note.startswith(HOLD_PREFIX):
            rest = note[len(HOLD_PREFIX):].split()
            branch = rest[0] if rest else None
        out.append({"contributor": e.get("from_id"), "branch": branch,
                    "claimed_at": props.get("claimed_at"), "note": note})
    return out


def without_holds(doc: dict) -> dict:
    """The design with its hold claims removed — what a copy carries."""
    out = dict(doc)
    out["edges"] = [e for e in doc.get("edges", [])
                    if not (e.get("edge_type") == CLAIMS and e.get("to_id") == HOLD_SEED)]
    out["content_hash"] = design_io.design_hash(out)
    return out


def _keyed(doc: dict) -> tuple[dict, dict]:
    nodes = {(n["node_type"], n["node_id"]): n for n in doc.get("nodes", [])}
    edges = {(e["edge_type"], e["from_id"], e["to_id"]): e for e in doc.get("edges", [])}
    return nodes, edges


def in_flight(live: dict, main: dict) -> dict:
    """What `live` holds that `main` does not, item by item. Pass `live` without
    its holds: a hold claim is not work."""
    ln, le = _keyed(live)
    mn, me = _keyed(main)

    def split(a: dict, b: dict) -> dict:
        canon = design_io.canonical
        return {
            "added": sorted(k for k in a if k not in b),
            "removed": sorted(k for k in b if k not in a),
            "changed": sorted(k for k in a if k in b and canon(a[k]) != canon(b[k])),
        }

    return {"nodes": split(ln, mn), "edges": split(le, me)}


def flight_size(flight: dict) -> int:
    return sum(len(v) for part in flight.values() for v in part.values())


def judge(verb: str, me: str, hold_list: list[dict], flight: dict, adopt: bool) -> list[str]:
    """Whether `me` may hold or copy. Returns what to say; raises Refused when not."""
    n = flight_size(flight)
    others = [h for h in hold_list if h["branch"] != me]
    mine = [h for h in hold_list if h["branch"] == me]

    def who(h: dict) -> str:
        b = h["branch"] or f"an unnamed holder (note: {h['note'][:80]!r})"
        return f"{b} ({h['contributor']}, since {h['claimed_at'] or 'an unrecorded time'})"

    if n == 0:
        said = ["Nothing is in flight: the live design is main's copy."]
        if others or mine:
            said.append("Leftover hold(s), all merged and stale: "
                        + "; ".join(who(h) for h in hold_list) + ".")
        if verb == "hold" and not mine:
            said.append(claim_instruction(me))
        return said

    if others:
        held = "; ".join(who(h) for h in others)
        if not adopt:
            raise Refused(
                f"{n} item(s) are in flight and the design is held by {held}. One design PR at a "
                "time: wait for that PR to merge (then nothing is in flight and the hold is "
                "stale), or, if it is abandoned, run this again with --adopt to take its "
                "items into this PR, knowing that PR can then no longer copy.")
        return [f"ADOPTING {n} item(s) held by {held}. They ride in this PR's copy.",
                claim_instruction(me) if not mine else "This branch holds the design too."]

    if not mine:
        if not adopt:
            raise Refused(
                f"{n} item(s) are in the live design that main's copy does not have, and NOBODY "
                "holds them: a PR that wrote without holding, an abandoned PR, or writes made "
                "outside a PR. Read them below. Take them into this PR deliberately with "
                "--adopt, or undo them on flo2.io first.")
        return [f"ADOPTING {n} unheld item(s). They ride in this PR's copy.",
                claim_instruction(me)]

    return [f"This branch, {me}, holds the design; {n} item(s) are in flight."]


def claim_instruction(me: str) -> str:
    args = json.dumps({"contributor_id": "<you>", "seed_id": HOLD_SEED, "depth": 0,
                       "note": f"{HOLD_PREFIX}{me}"})
    return ("HOLD IT NOW, before your first design write — claim_region on reflow2's design "
            f"on flo2.io: {args}. A hold nobody can see is not a hold.")


def describe(flight: dict, live: dict, main: dict) -> list[str]:
    """In-flight items in words: change events first, since they name the work."""
    ln, _ = _keyed(live)
    mn, _ = _keyed(main)
    lines = []
    for kind in ("added", "changed", "removed"):
        keys = flight["nodes"][kind]
        if not keys:
            continue
        keys = sorted(keys, key=lambda k: (k[0] != "ChangeEvent", k))
        lines.append(f"  nodes {kind}: {len(keys)}")
        for k in keys[:SHOW]:
            node = (ln.get(k) or mn.get(k) or {}).get("properties") or {}
            name = str(node.get("name") or "")[:100]
            lines.append(f"    {k[0]:<13} {k[1]}" + (f" — {name}" if name else ""))
        if len(keys) > SHOW:
            lines.append(f"    … and {len(keys) - SHOW} more")
    for kind in ("added", "changed", "removed"):
        keys = flight["edges"][kind]
        if not keys:
            continue
        by_type: dict = {}
        for k in keys:
            by_type[k[0]] = by_type.get(k[0], 0) + 1
        lines.append(f"  edges {kind}: {len(keys)} ("
                     + ", ".join(f"{t} {c}" for t, c in sorted(by_type.items())) + ")")
    return lines


# ---- writing the copy ------------------------------------------------------


def find_binary(root: str, given: str | None) -> str:
    """The binary that writes the copy, found in reflow2_check.py's order: this
    branch's own build before whatever is on PATH."""
    for cand in (given, os.environ.get("REFLOW2_BIN"),
                 os.path.join(root, "target", "debug", "reflow2-mcp"),
                 os.path.join(root, "target", "release", "reflow2-mcp"),
                 shutil.which("reflow2-mcp")):
        if cand and os.access(cand, os.X_OK):
            return os.path.abspath(cand)
    raise Refused("no reflow2 binary: pass --bin, set REFLOW2_BIN, or build this checkout")


def write_copy(binary: str, doc: dict, root: str) -> dict:
    """Import `doc` into a throwaway store with `binary` and export it into the
    checkout's layout. Returns the export's receipt."""
    tmp = tempfile.mkdtemp(prefix="reflow2-design-copy-")
    try:
        src = os.path.join(tmp, "design.json")
        with open(src, "w", encoding="utf-8") as fh:
            json.dump(doc, fh)
        graph = os.path.join(tmp, "store", ".reflow2", "graph")
        r = subprocess.run([binary, "--graph-path", graph, "--import", src],
                           capture_output=True, text=True)
        if r.returncode != 0:
            raise Refused("the import into a throwaway store failed:\n"
                          + (r.stderr or r.stdout)[-2000:])
        out = os.path.join(root, design_io.ITEMS_REL) + "/"
        r = subprocess.run(
            [binary, "--graph-path", graph, "--call", "export_graph",
             "--args", json.dumps({"path": out, "overwrite": True})],
            cwd=root, capture_output=True, text=True)
        if r.returncode != 0:
            raise Refused("the export into the layout failed:\n" + (r.stderr or r.stdout)[-2000:])
        return json.loads(r.stdout)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def round_trip_changes(source: dict, written: dict) -> list[str]:
    """What the written copy says that the source did not: values a round trip
    invented (`fact:bl-198`) or lost. Empty when the copy is exact."""
    flight = in_flight(written, source)
    if flight_size(flight) == 0:
        return []
    return describe(flight, written, source)


# ---- the verbs -------------------------------------------------------------


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("verb", choices=["hold", "copy", "status"])
    ap.add_argument("--from", dest="source", metavar="PATH",
                    help="a saved design (either form) instead of flo2.io")
    ap.add_argument("--host", default=os.environ.get("FLO2_HOST_SSH", DEFAULT_HOST),
                    help=f"ssh target of flo2.io's droplet (default {DEFAULT_HOST}, or FLO2_HOST_SSH)")
    ap.add_argument("--base", default="origin/main", help="the branch the PR merges into")
    ap.add_argument("--no-fetch", action="store_true", help="do not `git fetch` the base first")
    ap.add_argument("--branch", help="this PR's branch (default: the checked-out one)")
    ap.add_argument("--bin", help="the reflow2 binary that writes the copy (copy only)")
    ap.add_argument("--adopt", action="store_true",
                    help="take in-flight items nobody else may still be working on into this PR")
    ap.add_argument("--root", default=design_io.REPO, help=argparse.SUPPRESS)
    args = ap.parse_args(argv)
    root = os.path.abspath(args.root)

    try:
        if not args.no_fetch and args.base.startswith("origin/"):
            git(root, "fetch", "-q", "origin", args.base.split("/", 1)[1])
        me = args.branch or git(root, "rev-parse", "--abbrev-ref", "HEAD")
        main_doc = committed_design(root, args.base)
        design_id = main_doc.get("graph_id") or ""

        if args.source:
            live = design_io.load_design(args.source)
            print(f"1/4 design {design_id}, from {args.source}")
        else:
            print(f"1/4 design {design_id}, from {args.host}'s engine")
            live = fetch_live(args.host, design_id)
        if live.get("graph_id") != design_id:
            raise Refused(f"what came back is design {live.get('graph_id')!r}, not {design_id!r}")
        hold_list = holds(live)
        bare = without_holds(live)
        flight = in_flight(bare, main_doc)
        n = flight_size(flight)
        print(f"2/4 {len(live['nodes'])} nodes, {len(live['edges'])} edges; "
              f"{n} item(s) in flight against {args.base}; "
              f"{len(hold_list)} hold(s): "
              + (", ".join(f"{h['branch']} ({h['contributor']})" for h in hold_list) or "none"))
        detail = describe(flight, bare, main_doc)

        if args.verb == "status":
            print("\n".join(detail) if detail else "  nothing in flight")
            return 0

        try:
            said = judge(args.verb, me, hold_list, flight, args.adopt)
        except Refused:
            print("\n".join(detail))
            raise
        for line in said:
            print(f"    {line}")
        if args.verb == "hold":
            if detail:
                print("\n".join(detail))
            return 0

        # copy
        if subprocess.run(["git", "merge-base", "--is-ancestor", args.base, "HEAD"],
                          cwd=root).returncode != 0:
            raise Refused(f"this branch does not contain {args.base}. Run `git merge {args.base}` "
                          "first: the copy is taken last, against what the PR will merge into.")
        dirty = git(root, "status", "--porcelain", "--", design_io.ITEMS_REL)
        if dirty:
            raise Refused(f"{design_io.ITEMS_REL}/ has uncommitted changes, which the copy would "
                          "replace. Commit or discard them first.")
        binary = find_binary(root, args.bin)
        version = subprocess.run([binary, "--version"], capture_output=True, text=True).stdout.strip()
        print(f"3/4 {version} imports it into a throwaway store and writes {design_io.ITEMS_REL}/")
        receipt = write_copy(binary, bare, root)
        written = design_io.load_design(os.path.join(root, design_io.ITEMS_REL))
        items = receipt.get("items") or {}
        changed = round_trip_changes(bare, written)
        if changed:
            print("4/4 ⚠️  THE COPY DIFFERS FROM THE DESIGN ON flo2.io — the round trip changed it:")
            print("\n".join(changed))
            print("    These are values nobody wrote on flo2.io. Do not commit them as the design's "
                  "unless you mean the record to assert them; `git checkout -- "
                  f"{design_io.ITEMS_REL}` undoes the copy.")
            return 2
        print(f"4/4 {receipt.get('wrote')}: {items.get('written', 0)} written, "
              f"{items.get('changed', 0)} changed, {items.get('deleted', 0)} deleted, "
              f"{items.get('unchanged', 0)} unchanged — the same design as flo2.io "
              f"({written['content_hash']}), hold claims aside")
        if detail:
            print("    This PR's copy carries:")
            print("\n".join(detail))
        return 0
    except Refused as e:
        sys.stdout.flush()
        print(f"reflow2-design-copy: REFUSED: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
