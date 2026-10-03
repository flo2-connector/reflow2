#!/usr/bin/env python3
"""Tool confusability — the instrument behind the rank ratchet
(dec:idea-prune-the-tool-surface-to-an-orthogonal-essential-set, 2026-09-18).

An offline replica of find_tools' scorer over tools/toolsnaps (name,
description, parameters). Run after `python3 tools/toolsnap.py --update`:

    python3 tools/tool_confusability.py              # the report
    python3 tools/tool_confusability.py --validate   # agree with the live ranker, or exit 1
    python3 tools/tool_confusability.py --baseline   # validate, then rewrite the ratchet fixture

⭐ IT SCORES WHAT find_tools SCORES, NOT WHAT tools/list SENDS. The toolsnaps
are tools/list's output, and tools/list adds to the router's tools at list
time: the bulk-route sentence on the typed edge helpers ("MANY AT ONCE:
`draw_edges` …", bulk_edges.rs `name_the_bulk_route`), the `echo` parameter
on every write (receipt.rs `declare_echo`), and on a design with lessons the
lessons block. find_tools ranks the router's tools, which carry none of them.
Until 2026-10-03 the replica scored them anyway: `edges` had a document
frequency of 49 here against 25 live, and the replica agreed with the live
ranker on 186 of 193 corpus queries at top 1 and 115 at an exact top 5 while
this header claimed 185 of 185
(fact:the-confusability-replica-ranks-a-bulk-door-sentence-find-tools-never-sees-2026-10-03).
`load_tools` now strips each of them, and `--validate` measures the agreement
against the live binary instead of a file someone saved once: it exits 1 on
any top-1 disagreement, and CI runs it, so the next thing tools/list adds is
caught the day it is added. `--baseline` validates first and refuses to write
a fixture from rankings the live ranker does not give.

The report: which tools are NOT ranked first for their own corpus query and
who beats them, the mutually confusable pairs, and the crowders (tools that
sit in the most other tools' top 5). With `.reflow2/graph.usage.jsonl` present
it also says which served tools this project has never called — one project's
ledger, weak evidence, never a verdict (BL-155).

The Rust test beside the corpus is the authority; the replica can miss a tie-broken pair, so `--baseline` never removes a pair the test found — merge by hand.

Description words count once (has_word is boolean) and a name match scores
five times a description hit, so a tool beaten on a NAME match cannot be
rescued by words; it belongs in the baseline with that reason.
"""
import functools, json, glob, math, os, re, sys

# Refusing stubs for renamed tools (service.rs DEPRECATED_TOOLS): find_tools
# never offers them, so the replica leaves them out too.
DEPRECATED = {"record_change", "manual_work_report"}

# What tools/list adds to a router tool's description at list time, and
# find_tools therefore never scores. Each pattern names where it is added.
LISTED_ONLY_TEXT = [
    # bulk_edges.rs name_the_bulk_route: " MANY AT ONCE: `draw_edges`, items
    # {"tool": "<name>", "arguments": {…}}, same checks, all or nothing."
    re.compile(r" MANY AT ONCE: `draw_edges`, items \{.*?\}\}, same checks, all or nothing\."),
    # lessons.rs: the design's lessons for this tool, appended after a blank line.
    re.compile(r"\n\n⭐ LESSONS THIS DESIGN HOLDS FOR .*\Z", re.S),
]
# receipt.rs declare_echo: every write's PUBLISHED schema carries `echo`; the
# router's schema, which find_tools reads, does not.
LISTED_ONLY_PARAMS = {"echo"}

def router_view(desc, params):
    """A tool as find_tools sees it: tools/list's additions taken back off."""
    for pattern in LISTED_ONLY_TEXT:
        desc = pattern.sub("", desc)
    return desc, [p for p in params if p not in LISTED_ONLY_PARAMS]

def load_tools(snapdir="tools/toolsnaps", overrides=None):
    tools = {}
    for f in glob.glob(snapdir + "/*.json"):
        d = json.load(open(f))
        name = d["name"]
        if name in DEPRECATED:
            continue
        desc = d.get("description") or ""
        params = list((d.get("inputSchema", {}) or {}).get("properties", {}).keys())
        tools[name] = router_view(desc, params)
    if overrides:
        for k, v in overrides.items():
            if k in tools:
                tools[k] = (v, tools[k][1])
    return tools

def terms_of(query):
    return [t for t in re.split(r"[^0-9a-z_]", query.lower()) if t]

@functools.lru_cache(maxsize=None)
def _words(hay):
    return frozenset(re.split(r"[^0-9a-zA-Z]", hay))

def has_word(hay, term):
    return term in _words(hay)

def weights(terms, tools):
    n = len(tools)
    out = []
    for t in terms:
        df = sum(1 for name, (desc, _) in tools.items() if t in name.lower() or has_word(desc.lower(), t))
        out.append((t, max(math.log((n + 1) / (df + 1)), 0.0)))
    return out

def score(name, desc, params, weighted):
    nl, dl = name.lower(), desc.lower()
    s = 0.0
    for t, w in weighted:
        if nl == t: s += 8.0 * w
        elif t in nl: s += 5.0 * w
        elif any(p.startswith(t) for p in nl.split("_")): s += 1.5 * w
        if has_word(dl, t): s += 2.0 * w
        if any(has_word(p.lower(), t) for p in params): s += 1.0 * w
    return s / math.log(2.0 + len(dl) / 200.0)

def rank(query, tools, limit=5):
    w = weights(terms_of(query), tools)
    scored = [(score(n, d, p, w), n) for n, (d, p) in tools.items()]
    scored = [(s, n) for s, n in scored if s > 0]
    scored.sort(key=lambda x: (-x[0], x[1]))
    return scored[:limit]

def report(baseline=False):
    corpus = json.load(open("crates/reflow2-mcp/tests/fixtures/find_tools_corpus.json"))["queries"]
    tools = load_tools()
    top = {t: [n for _, n in rank(q, tools)] for t, q in sorted(corpus.items())}
    not_first = [t for t, l in top.items() if not l or l[0] != t]
    mutual = sorted({tuple(sorted((t, u))) for t, l in top.items() for u in l if u != t and u in corpus and t in top.get(u, [])})
    crowd = {}
    for t, l in top.items():
        for u in l:
            if u != t: crowd[u] = crowd.get(u, 0) + 1
    print(f"served with a query: {len(corpus)}  |  not ranked first for their own job: {len(not_first)}  |  mutual pairs: {len(mutual)}")
    for t in not_first:
        r = rank(corpus[t], tools)
        print(f"  {t:34s} first={r[0][1]} ({r[0][0]:.1f})  own rank={next((i+1 for i,(_,n) in enumerate(r) if n==t), '>5')}")
    print("crowders:", sorted(crowd.items(), key=lambda kv: -kv[1])[:10])
    try:
        calls = {}
        for line in open(".reflow2/graph.usage.jsonl"):
            try: rec = json.loads(line)
            except Exception: continue
            t = rec.get("tool")
            if t: calls[t] = calls.get(t, 0) + 1
        never = sorted(set(tools) - set(calls))
        print(f"never called in this project's ledger ({sum(calls.values())} calls): {len(never)} — weak evidence, one project")
    except FileNotFoundError:
        pass
    if baseline:
        path = "crates/reflow2-mcp/tests/fixtures/find_tools_rank_baseline.json"
        doc = json.load(open(path)); doc["not_first"] = not_first
        kept = [tuple(p) for p in doc.get("mutual", [])]
        doc["mutual"] = [list(m) for m in sorted(set(mutual) | set(kept))]
        json.dump(doc, open(path, "w"), indent=1); print("baseline rewritten:", path)

def live_rankings(binary, corpus):
    """Ask the live binary's find_tools every corpus query, in one session on
    an empty design (the toolsnaps are taken on one too, so no lessons)."""
    import shutil, tempfile
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    from smoke_mcp import Server
    store = tempfile.mkdtemp(prefix="reflow2-confusability-")
    s = Server(binary, os.path.join(store, "graph"))
    try:
        out = {}
        for t, q in sorted(corpus.items()):
            r = s.call("find_tools", {"query": q, "limit": 5})
            out[t] = [x["tool"] for x in r["items"]]
        return out
    finally:
        s.close()
        shutil.rmtree(store, ignore_errors=True)

def validate(binary):
    """Agreement with the live ranker. Returns the number of top-1 disagreements."""
    corpus = json.load(open("crates/reflow2-mcp/tests/fixtures/find_tools_corpus.json"))["queries"]
    tools = load_tools()
    live = live_rankings(binary, corpus)
    top1, top5 = [], 0
    for t, q in sorted(corpus.items()):
        mine = [n for _, n in rank(q, tools)]
        if mine[:1] != live[t][:1]:
            top1.append((t, mine[:3], live[t][:3]))
        if mine == live[t]:
            top5 += 1
    print(f"replica vs live ({binary}): top-1 agree {len(corpus) - len(top1)}/{len(corpus)}, "
          f"exact top-5 agree {top5}/{len(corpus)}")
    for t, mine, theirs in top1:
        print(f"  DISAGREE {t}: replica {mine} — live {theirs}")
    if top1:
        print("FAIL: the replica ranks differently from find_tools, so its report and its "
              "--baseline describe a ranker that is not served. Find what the replica scores "
              "that find_tools does not (or the other way round) and fix load_tools.")
    return len(top1)

def binary_arg():
    if "--bin" in sys.argv:
        return sys.argv[sys.argv.index("--bin") + 1]
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    from reflow2_bin import default_bin
    return default_bin()

if __name__ == "__main__":
    if "--validate" in sys.argv:
        sys.exit(1 if validate(binary_arg()) else 0)
    if "--baseline" in sys.argv and validate(binary_arg()):
        print("refusing to rewrite the baseline from rankings the live ranker does not give")
        sys.exit(1)
    report(baseline="--baseline" in sys.argv)
    sys.exit(0)
