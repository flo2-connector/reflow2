#!/usr/bin/env python3
"""What one open design costs in memory, and what writing to it costs in time.

    python3 tools/measure_open_design_memory.py --bin target/release/reflow2-mcp \
        [--designs 4] [--source docs/design/reflow2.json] [--writes 300] [--out result.json]

WHY THIS EXISTS. `ver:the-per-open-design-cost-is-measured` states the method in
prose: run one server, attach one design, read resident memory, attach a second,
read again; the delta is the per-store cost and the intercept the per-process
cost. A method in prose is re-derived by hand every time, and
`req:one-open-design-costs-a-deliberate-amount-of-memory` asks for a
measurement BEFORE and AFTER any change to the store's memory settings, on the
same design, with the write throughput traded away measured beside the memory
saved. That needs the same instrument twice. This is that instrument.

WHAT IT DOES:
  1. builds a registry root holding N copies of a real design (by default
     reflow2's own), each imported under its own graph_id so the registry sees
     N designs, not one;
  2. starts `--registry-root` over HTTP and reads the server's resident memory
     (VmRSS) and its peak (VmHWM) before any design is open;
  3. opens each design in turn (initialize + a full-text search, so its store
     and index are really in use) and reads memory after each;
  4. writes to the first design (`--writes` requirements, one call each) and
     reads memory again, timing the writes;
  5. prints, and optionally saves, the readings as JSON.

The slope of RSS over designs opened is the per-open-design cost; the reading
before any design is the per-process intercept. Writes grow memtables, which is
where a write-buffer budget shows.

⚠️ READ THE NUMBERS AS ONE MACHINE, ONE RUN. RSS includes allocator retention
(fact:the-servers-resident-memory-is-export-retention-not-rocksdb-2026-09-13);
compare runs of this tool against each other, with the same inputs, rather than
against numbers taken another way.
"""
import argparse
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import threading
import time
import urllib.request


def rss(pid):
    out = {}
    with open(f"/proc/{pid}/status") as f:
        for line in f:
            # RssAnon is memory the process owns; RssFile is file-backed pages
            # (memory-mapped index and table files) the OS can reclaim under
            # pressure. VmRSS is their sum, so read all three.
            if line.startswith(("VmRSS:", "VmHWM:", "RssAnon:", "RssFile:")):
                k, v = line.split(":", 1)
                out[k] = int(v.split()[0]) // 1024  # MB
    return out


def post(port, path, body, session=None):
    headers = {
        "content-type": "application/json",
        "accept": "application/json, text/event-stream",
    }
    if session:
        headers["mcp-session-id"] = session
    req = urllib.request.Request(
        f"http://127.0.0.1:{port}{path}", data=json.dumps(body).encode(), headers=headers
    )
    with urllib.request.urlopen(req, timeout=600) as r:
        sid = r.headers.get("mcp-session-id")
        raw = r.read().decode()
    return sid, raw


def initialize(port, path):
    sid, _ = post(
        port,
        path,
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "measure-memory", "version": "1"},
            },
        },
    )
    post(port, path, {"jsonrpc": "2.0", "method": "notifications/initialized"}, sid)
    return sid


def call(port, path, sid, tool, args, n=9):
    return post(
        port,
        path,
        {"jsonrpc": "2.0", "id": n, "method": "tools/call", "params": {"name": tool, "arguments": args}},
        sid,
    )[1]


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--bin", required=True)
    ap.add_argument("--designs", type=int, default=4)
    ap.add_argument("--source", default="docs/design/reflow2.json")
    ap.add_argument("--writes", type=int, default=300)
    ap.add_argument("--out")
    ap.add_argument("--keep", action="store_true", help="keep the temporary registry root")
    ap.add_argument(
        "--no-search",
        action="store_true",
        help="open each design with initialize only, so opening and searching can be told apart",
    )
    ap.add_argument(
        "--server-arg",
        action="append",
        default=[],
        help="an extra flag for the server, e.g. --server-arg=--store-memory=8 (repeatable)",
    )
    a = ap.parse_args()

    binary = os.path.abspath(a.bin)
    version = subprocess.run([binary, "--version"], capture_output=True, text=True).stdout.strip()
    doc = json.load(open(a.source))
    root = tempfile.mkdtemp(prefix="reflow2-measure-memory-")
    ids = []
    try:
        for i in range(a.designs):
            gid = f"measure-{i}"
            d = dict(doc)
            d["graph_id"] = gid
            for k in ("content_hash", "prev_content_hash", "taken_at"):
                d.pop(k, None)
            src = os.path.join(root, f"src-{i}.json")
            json.dump(d, open(src, "w"))
            store = os.path.join(root, f"design-{i}", ".reflow2", "graph")
            os.makedirs(store)
            r = subprocess.run(
                [binary, "--graph-path", store, "--import", src],
                capture_output=True,
                text=True,
            )
            if r.returncode != 0:
                sys.exit(f"import {i} failed: {r.stderr[-600:]}")
            os.remove(src)
            ids.append(json.load(open(os.path.join(root, f"design-{i}", ".reflow2", "graph.id.json")))["graph_id"])

        server = subprocess.Popen(
            [binary, "--registry-root", root, "--http", "127.0.0.1:0",
             "--registry-max-open", str(a.designs + 1), *a.server_arg],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            text=True,
        )
        port_box = {}

        def drain():
            for line in server.stderr:
                m = re.search(r"http://127\.0\.0\.1:(\d+)", line)
                if m and "port" not in port_box:
                    port_box["port"] = int(m.group(1))

        threading.Thread(target=drain, daemon=True).start()
        deadline = time.time() + 90
        while "port" not in port_box:
            if time.time() > deadline or server.poll() is not None:
                sys.exit("the server never printed its address")
            time.sleep(0.1)
        port = port_box["port"]

        readings = [{"step": "no design open", **rss(server.pid)}]
        sessions = []
        for i, gid in enumerate(ids):
            path = f"/g/{gid}/"
            sid = initialize(port, path)
            if a.no_search:
                # initialize alone does not open the store; one cheap call does.
                call(port, path, sid, "design_identity", {})
            else:
                for q in ("export graph", "requirement status", "release cut", "session", "backup"):
                    call(port, path, sid, "search_design", {"query": q, "limit": 20})
            sessions.append((path, sid))
            readings.append({"step": f"{i + 1} design(s) open", **rss(server.pid)})

        path, sid = sessions[0]
        t0 = time.time()
        for w in range(a.writes):
            call(
                port,
                path,
                sid,
                "add_requirement",
                {
                    "id": f"req:measure-write-{w}",
                    "name": f"Measurement write {w}",
                    "statement": "Written by tools/measure_open_design_memory.py to grow the memtables. " * 8,
                },
                n=100 + w,
            )
        elapsed = time.time() - t0
        readings.append({"step": f"after {a.writes} writes to design 1", **rss(server.pid)})
        server.terminate()
        server.wait(timeout=30)

        def slope_of(key):
            xs = [r[key] for r in readings[1 : 1 + len(ids)]]
            return round((xs[-1] - xs[0]) / (len(xs) - 1), 1) if len(xs) > 1 else None

        slope = slope_of("VmRSS")
        result = {
            "binary": version,
            "server_args": a.server_arg,
            "source": a.source,
            "source_nodes": len(doc["nodes"]),
            "designs": a.designs,
            "readings_mb": readings,
            "per_open_design_mb": slope,
            "per_open_design_anon_mb": slope_of("RssAnon"),
            "per_open_design_file_mb": slope_of("RssFile"),
            "intercept_mb": readings[0]["VmRSS"],
            "writes": a.writes,
            "write_seconds": round(elapsed, 2),
            "writes_per_second": round(a.writes / elapsed, 1),
        }
        print(json.dumps(result, indent=1))
        if a.out:
            json.dump(result, open(a.out, "w"), indent=1)
    finally:
        if not a.keep:
            shutil.rmtree(root, ignore_errors=True)


if __name__ == "__main__":
    main()
