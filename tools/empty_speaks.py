#!/usr/bin/env python3
"""An empty answer says which empty it is — the class contract, checked on the wire.

Three passes, all on the wire because the obligation is about what a consumer
receives:

  ① NO-ARGUMENT reads on an EMPTY store — "nothing here" must say whether that
    is an all-clear or an absence.
  ② ARG-TAKING reads given a DELIBERATELY ABSENT id — "nothing found" must say
    whether the referent is missing or merely has nothing. Either a refusal
    naming the id, or an empty reply carrying the sentence, is a pass; a bare
    `{"value":null}` / `{"count":0}` / `{}` is not.
  ③ QUERY-TAKING reads given words NOTHING matches — "nothing matched" must say
    over how much it looked: `searched` (how many it ran over) or a sentence.
    `{"hits": []}` from 0 searched and from 6,000 searched are opposite facts.

All take the tool list from `tools/list`, so a new tool joins the class the
moment it is served rather than when somebody remembers.

Pass ③ was added 2026-10-02, after this gate passed while `search_design`
answered `{"hits": [], "limit": 10, "stale": []}` from an index that held
nothing — a copy of a held design, read through `--call` — for a word the
design held (fact:root-cause-a-held-design-answers-search-through-the-door-with-a-false-nothing-matched-2026-10-02).
The gate missed it three ways: pass ② skips a read whose required input is
free text, by its own rule below; `is_empty` did not know `hits: []`; and no
pass asked a query-taking read anything at all.

Measured 2026-09-11 before any fix. Pass ①: 39 no-arg read-only tools, 10
empty on an empty design, 1 saying which empty (open_questions), 9 bare. A
bare zero from `hierarchy_issues` reads exactly like a clean design; from
`manual_work_report` exactly like nobody did work by hand — its own
description says the opposite, and the description is not the reply. Pass ②:
13 id-taking reads probed, 8 refused well, 1 spoke, 4 bare — and
`readiness_report` did something worse than bare, answering `ungated` with a
full summary about a subject that does not exist.

    python3 tools/empty_speaks.py            # exit 0 when every empty speaks
    python3 tools/empty_speaks.py --bin target/release/reflow2-mcp
"""
from __future__ import annotations
import argparse, json, os, pathlib, shutil, sys, tempfile
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from smoke_mcp import Server

SPEAKS = ("empty_because", "loop_hint")

# A STRUCTURED answer to "which empty" counts, and is better than prose.
# `propagate_from` returns `unknown_seeds` and `unclaimed_findings` returns
# `unknown_events`, each NAMING the id that resolved to nothing — which is
# exactly the fact a bare zero withholds, in a form a machine can read. A
# non-empty list here is a pass; an empty one says nothing and is not.
NAMES_THE_MISSING = ("unknown_seeds", "unknown_events", "unknown_ids")

# A count of what was LOOKED AT is the structured answer for a search: `0`
# included, because "searched 0" is exactly the fact a bare empty hides.
SAYS_WHAT_IT_SEARCHED = ("searched",)

# Words no design and no tool description holds — pass ③'s question.
NOTHING_MATCHES = "zzqxv wqqzx"

def is_empty(v) -> bool:
    if not isinstance(v, dict):
        return False
    if v == {}:
        return True
    if v.get("count") == 0 or v.get("items") == [] or v.get("hits") == []:
        return True
    if "value" in v and v["value"] is None:
        return True
    if "node" in v and v["node"] is None:
        return True
    return False

ABSENT = "zz:deliberately-absent"

def probe_args(schema: dict):
    """Arguments that make an arg-taking read look for something ABSENT.

    Returns None unless at least one required parameter is id-shaped — a tool
    whose required input is a path, a document or a free-text query has no
    "referent that does not exist" to probe for, and forcing one would test
    something else.
    """
    props = schema.get("properties") or {}
    args, id_shaped = {}, False
    for p in schema.get("required") or []:
        spec = props.get(p, {})
        t = spec.get("type")
        t = t if isinstance(t, str) else (t[0] if isinstance(t, list) else None)
        if (p == "id" or p.endswith("_id")) and t == "string":
            args[p], id_shaped = ABSENT, True
        elif (p.endswith("_ids") or p.endswith("_keys")) and t == "array":
            args[p], id_shaped = [ABSENT], True
        elif spec.get("enum"):
            legal = [x for x in spec["enum"] if x]
            if not legal:
                return None
            args[p] = legal[0]
        elif t == "string":
            args[p] = "x"
        elif t == "number":
            args[p] = 1.0
        elif t == "array":
            args[p] = []
        else:
            return None
    return args if id_shaped else None


def query_args(schema: dict):
    """Arguments that ask a query-taking read for words nothing matches.

    None unless `query` is a required string and every other required
    parameter is one this pass can fill without inventing a referent.
    """
    props = schema.get("properties") or {}
    req = schema.get("required") or []
    if "query" not in req:
        return None
    t = (props.get("query") or {}).get("type")
    t = t if isinstance(t, str) else (t[0] if isinstance(t, list) else None)
    if t != "string":
        return None
    if any(p != "query" for p in req):
        return None
    return {"query": NOTHING_MATCHES}


def speaks(v: dict) -> bool:
    return (
        any(isinstance(v.get(k), str) and v[k].strip() for k in SPEAKS)
        or any(isinstance(v.get(k), list) and v[k] for k in NAMES_THE_MISSING)
        or any(isinstance(v.get(k), int) and not isinstance(v.get(k), bool)
               for k in SAYS_WHAT_IT_SEARCHED)
    )


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", default="target/debug/reflow2-mcp")
    a = ap.parse_args()
    store = tempfile.mkdtemp(prefix="reflow2-empty-")
    s = Server(a.bin, store)
    bare, spoke, skipped = [], [], []
    queried = []
    try:
        listed = s.rpc("tools/list", {})["result"]["tools"]
        for t in sorted(listed, key=lambda t: t["name"]):
            ro = (t.get("annotations") or {}).get("readOnlyHint")
            req = (t.get("inputSchema") or {}).get("required") or []
            if not ro:
                continue
            if req:
                args = probe_args(t.get("inputSchema") or {})
                if args is None:
                    args = query_args(t.get("inputSchema") or {})
                    if args is None:
                        continue
                    queried.append(t["name"])
            else:
                args = {}
            resp = s.rpc("tools/call", {"name": t["name"], "arguments": args})
            res = resp.get("result") or {}
            if "error" in resp or res.get("isError"):
                # A refusal naming the absent referent IS the answer — better
                # than an empty reply, and the pattern budget_report and
                # flow_report already set.
                spoke.append(t["name"]) if args else skipped.append((t["name"], "refused"))
                continue
            v = res.get("structuredContent")
            if v is None:
                skipped.append((t["name"], "prose")); continue
            if not is_empty(v):
                continue
            if speaks(v):
                spoke.append(t["name"])
            else:
                bare.append((t["name"], json.dumps(v)[:80]))
    finally:
        s.close(); shutil.rmtree(store, ignore_errors=True)
    print(f"empty on an empty design: {len(spoke) + len(bare)}   speaking: {len(spoke)}   BARE: {len(bare)}"
          f"   (skipped: {len(skipped)} refused/prose)")
    print(f"query-taking reads asked for words nothing matches (pass ③): {', '.join(queried) or 'NONE'}")
    if "search_design" not in queried:
        # The pass exists for this tool; a schema change that drops it from
        # the pass must fail here rather than leave the pass asking nobody.
        print("\nFAIL: pass ③ did not ask `search_design` anything — the read it was added for.")
        return 1
    for n, b in bare:
        print(f"  BARE  {n:<28} {b}")
    if bare:
        print("\nFAIL: an empty reply must say which empty it is — `empty_because` (what was swept and "
              "why nothing came back), `searched` (how many it ran over), or the loop-debt "
              "`loop_hint`. A bare zero reads as a pass.")
        return 1
    print("OK: every empty answer says which empty it is.")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
