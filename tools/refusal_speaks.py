#!/usr/bin/env python3
"""Every argument refusal names the TOOL and the FIELD PATH — checked ON THE WIRE.

🛑 THE REASON THIS GATE IS END-TO-END AND NOT A UNIT TEST.

reflow2 has intercepted `unknown field` deserialisation refusals in
`ReflowService::call_tool` since v0.51.0, to add one sentence about a client
whose tool list predates the server. On 2026-09-11 that branch was measured
against a real binary for the first time and **it had never fired**: rmcp 3
returns a deserialisation failure as `Ok(CallToolResponse::Complete)` carrying
`isError: true`, not as `Err`, so the `Err` arm reached nothing on the wire.
The test behind it called `stale_client_hint` as a pure function, so it passed
every day while the feature did nothing.

That is the whole argument for this file. A unit test proves the SENTENCE; only
a server proves the CALLER RECEIVES IT.

🛑 AND WHY IT IS GENERATED, NOT A HAND LIST (widened 2026-10-02).

Until 2026-10-02 this gate sent `{}` to each tool and one `unknown field` to
`loop_status`. So it could see a missing top-level field and nothing else, and
on 0.77.0 a WRONG-TYPED argument was refused on every tool with serde's bare
"invalid type: string …, expected a sequence", naming neither the tool nor the
field — a phrasing first met twenty days earlier and never probed
(`fact:root-cause-a-wrong-type-argument-is-refused-in-the-bare-serde-string-because-the-interception-matches-two-phrasings-2026-10-02`).
Arguments are now checked against the published schema before anything
deserialises them (`dec:idea-every-argument-refusal-names-the-tool-and-the-field-path`),
and this gate derives its probes from the same schemas, for every served tool:

  · `{}` — every required top-level field is named;
  · a wrong-typed value at EVERY typed location, top-level and nested;
  · an unknown key in EVERY closed object, top-level and inside items;
  · a value outside EVERY published enum;
  · an empty item wherever an item has required fields;

and requires each refusal to name the TOOL and the PATH (`related_to[0].evidence`),
never to be the bare deserialiser string, and — for a field — to quote the
field's own published description, or to say plainly that it has none.

Plus the transport: the reconnect advice reaches a session and never the
`--call` door, which reads the schema from its own binary on every call.

    python3 tools/refusal_speaks.py
    python3 tools/refusal_speaks.py --bin target/release/reflow2-mcp
"""
from __future__ import annotations
import argparse, json, pathlib, shutil, subprocess, sys, tempfile
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from smoke_mcp import Server
from reflow2_bin import default_bin  # noqa: E402  (one binary for every gate)

BARE = "failed to deserialize parameters"
UNKNOWN_KEY = "zz_no_such_field"

# Tools that cannot be probed with `{}` because the empty call is LEGAL for
# them — their required field has a schema default, or the router answers
# before deserialising. An entry is a claim that `{}` is not a missing-argument
# call, never that the obligation does not apply.
EXEMPT: dict[str, str] = {}


def refusal_text(res: dict) -> str | None:
    """The refusal a caller actually sees, or None if the call was not refused."""
    if "error" in res:
        return res["error"].get("message", "")
    res = res.get("result", {})
    if not res.get("isError"):
        return None
    blocks = res.get("content") or []
    return (blocks[0].get("text", "") if blocks else "") or ""


def resolve(root: dict, s):
    for _ in range(16):
        if not isinstance(s, dict):
            return s
        ref = s.get("$ref")
        if not (isinstance(ref, str) and ref.startswith("#/$defs/")):
            return s
        target = (root.get("$defs") or {}).get(ref[len("#/$defs/"):])
        if target is None:
            return s
        s = target
    return s


def types_of(s) -> list[str]:
    if not isinstance(s, dict):
        return []
    t = s.get("type")
    if isinstance(t, str):
        return [t]
    if isinstance(t, list):
        return [x for x in t if isinstance(x, str)]
    return []


def opening(root: dict, s) -> str | None:
    """The first four words of a field's description — its own, else its `$ref`'s."""
    if not isinstance(s, dict):
        return None
    d = s.get("description") or (resolve(root, s) or {}).get("description")
    if not isinstance(d, str):
        return None
    return " ".join(d.split()[:4]) or None


def place(steps: list, leaf):
    """`leaf` at `steps` (a key, or None for item 0) in an otherwise empty object."""
    v = leaf
    for s in reversed(steps):
        v = [v] if s is None else {s: v}
    return v


def path_of(steps: list) -> str:
    out = ""
    for s in steps:
        out += "[0]" if s is None else ("." if out else "") + s
    return out


def wrong_typed(allowed: list[str]):
    """A value whose JSON type `allowed` does not include. 7.5 is not an integer."""
    if not allowed:
        return None
    for t, v in (("string", "zz-wrong-type"), ("number", 7.5), ("boolean", True), ("array", []), ("object", {})):
        if t not in allowed:
            return v
    return None


def probes(root: dict) -> list[dict]:
    out: list[dict] = []

    def add(what, args, path, described):
        out.append({"what": what, "args": args, "path": path, "described": described})

    def walk(schema, at: list, depth: int) -> None:
        if depth > 6:
            return
        s = resolve(root, schema)
        if not isinstance(s, dict):
            return
        ts = types_of(s)
        if at:
            bad = wrong_typed(ts)
            if bad is not None:
                add("wrong type", place(at, bad), path_of(at), opening(root, schema))
            if isinstance(s.get("enum"), list) and "string" in ts:
                add("outside the enum", place(at, "zz-not-a-value"), path_of(at), opening(root, schema))
        if "object" in ts or "properties" in s:
            if s.get("additionalProperties") is False:
                add("unknown field", place(at + [UNKNOWN_KEY], 1), path_of(at + [UNKNOWN_KEY]), None)
            props = s.get("properties") or {}
            if at:
                for r in s.get("required") or []:
                    add("missing nested field", place(at, {}), path_of(at + [r]), opening(root, props.get(r)))
            for name, sub in props.items():
                walk(sub, at + [name], depth + 1)
        if "array" in ts and "items" in s:
            walk(s["items"], at + [None], depth + 1)

    walk(root, [], 0)
    return out


def door(binary: str, graph: str, tool: str, args: dict) -> tuple[int, str]:
    """One call through the `--call` door: its exit code and everything it printed."""
    p = subprocess.run(
        [binary, "--graph-path", graph, "--call", tool, "--args", json.dumps(args)],
        capture_output=True, text=True, timeout=120,
    )
    return p.returncode, p.stdout + p.stderr


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", default=None, help="reflow2-mcp binary (default: $REFLOW2_BIN, else this checkout's debug build)")
    a = ap.parse_args()
    a.bin = a.bin or default_bin()

    tmp = pathlib.Path(tempfile.mkdtemp(prefix="refusal-speaks-"))
    failures: list[str] = []
    probed = 0
    kinds: dict[str, int] = {}
    tools: list = []
    try:
        s = Server(a.bin, str(tmp / "graph"))
        try:
            tools = s.rpc("tools/list", {})["result"]["tools"]

            for t in tools:
                name = t["name"]
                root = t.get("inputSchema") or {}

                # Every required top-level field, from one empty call.
                required = root.get("required") or []
                if required and name not in EXEMPT:
                    probed += 1
                    kinds["missing field"] = kinds.get("missing field", 0) + 1
                    text = refusal_text(s.rpc("tools/call", {"name": name, "arguments": {}}))
                    if text is None:
                        failures.append(
                            f"{name}: called with {{}} and was NOT refused, though its schema "
                            f"declares required {required}. Either the schema is wrong or this "
                            f"belongs in EXEMPT with the reason."
                        )
                    elif text.startswith(BARE):
                        failures.append(f"{name}: bare deserialiser string — {text[:120]!r}")
                    else:
                        unnamed = [r for r in required if f"`{r}`" not in text]
                        if f"`{name}`" not in text or unnamed:
                            failures.append(f"{name} {{}}: does not name the tool and {unnamed} — {text[:160]!r}")

                for p in probes(root):
                    probed += 1
                    kinds[p["what"]] = kinds.get(p["what"], 0) + 1
                    text = refusal_text(s.rpc("tools/call", {"name": name, "arguments": p["args"]}))
                    where = f"{name} {json.dumps(p['args'])} [{p['what']}]"
                    if text is None:
                        failures.append(f"{where}: NOT refused")
                        continue
                    if text.startswith(BARE) or f"`{name}`" not in text or f"`{p['path']}`" not in text:
                        failures.append(f"{where}: does not name the tool and `{p['path']}` — {text[:200]!r}")
                        continue
                    if p["what"] != "unknown field":
                        words = p["described"]
                        if (words and words not in text) or (not words and "publishes no description" not in text):
                            failures.append(
                                f"{where}: does not quote the field's description ({words!r}) or say "
                                f"it has none — {text[:200]!r}"
                            )

            # The transport: a session may hold a stale tool list and is told to
            # reconnect — the sentence that was dead from v0.51.0 to 2026-09-11.
            text = refusal_text(s.rpc("tools/call", {"name": "loop_status", "arguments": {UNKNOWN_KEY: 1}})) or ""
            if "Reconnect" not in text:
                failures.append(
                    "loop_status: an UNKNOWN-field refusal over a session no longer carries the "
                    f"stale-client sentence — {text[:160]!r}."
                )
        finally:
            s.close()

        # …and the door, which reads the schema from its own binary on every call,
        # is never told to reconnect. It names the tool and the path, and exits 2:
        # an argument refusal is a tool result marked as an error. It runs on the
        # design the session above created (the server has released it): a read
        # through the door refuses a path that holds no design.
        code, out = door(a.bin, str(tmp / "graph"), "loop_status", {UNKNOWN_KEY: 1})
        if code != 2 or "`loop_status`" not in out or f"`{UNKNOWN_KEY}`" not in out:
            failures.append(f"--call loop_status: exit {code}, expected 2 naming the tool and the path — {out[:200]!r}")
        if "Reconnect" in out:
            failures.append(f"--call loop_status: the door is told to reconnect, which means nothing there — {out[:200]!r}")
        code, out = door(a.bin, str(tmp / "graph"), "external_dependency", {"name": "x", "components": "core"})
        if code != 2 or "`external_dependency`" not in out or "`components`" not in out:
            failures.append(f"--call external_dependency: a wrong type does not name the tool and field — exit {code}, {out[:200]!r}")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    print(f"  probed {probed} argument failure(s) across {len(tools)} tools: {kinds}")
    if probed < 1000:
        failures.append(f"only {probed} probes — the walk is not reaching the surface")
    if failures:
        print(f"  FAIL  {len(failures)} refusal(s) do not name the tool and the field path:")
        for f in failures:
            print(f"          {f}")
        return 1
    print("  PASS  every argument refusal names the tool, the path, and what the field is for")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
