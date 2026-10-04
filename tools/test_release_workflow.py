#!/usr/bin/env python3
"""The release workflow's shape, pinned — GitHub issue #617.

#617 (Alex) asked for two things from the release: a linux/amd64 + linux/arm64
image on GHCR, and the image's digest in the release notes so a consumer can
pin it immutably. A third came with it: since v0.76.0 the image's own
description, the Dockerfile and docker/build.sh all said reflow2 has NO
AUTHENTICATION, which stopped being true when `--http-oidc-issuer` shipped.

⭐ WHY A TEST OF A WORKFLOW FILE. The release workflow runs on a tag, a few
times a month, and the expensive mistakes in it are ORDERINGS and CONDITIONS,
not syntax: five releases (v0.27.0 … v0.31.0) published an image that never
started because the check that ran verified the wrong property, and two more
(v0.60.0, v0.60.1) looked complete and shipped no image because the cut read
the assets and the image is not one. Nothing about either was visible until a
tag. So this reads the workflow and asserts the properties a tag would
otherwise be the first to test:

  - linux-arm64 is built NATIVELY, on an arm runner, not cross-compiled;
  - each architecture's image is smoke-tested BEFORE anything is pushed, in the
    same job, on its own architecture (no QEMU);
  - the merge refuses an index that does not list both platforms — and the
    script that does it is DRIVEN here, against a stand-in registry, both ways;
  - publish waits for the image and requires the arm64 asset;
  - the release notes carry the index digest, written idempotently and
    appended to whatever notes a release already has — DRIVEN here too;
  - a dry run can never log in, push, merge or publish;
  - the image no longer says NO AUTHENTICATION, and names the flag that
    replaced it — a flag this checks the binary actually has;
  - tools/install.sh maps Linux aarch64 to the new asset — DRIVEN, with a
    stand-in `uname` and `gh`, so no network is touched.

Hermetic: no network, no Docker, no registry. PyYAML is required (the CI job
installs it for the schema step) and its absence is a FAILURE, never a skip.
"""

from __future__ import annotations

import json
import os
import re
import stat
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
RELEASE = REPO / ".github/workflows/release.yml"
DOCKERFILE = REPO / "Dockerfile"
BUILD_SH = REPO / "docker/build.sh"
PUBLISH_INDEX = REPO / "docker/publish-index.sh"
NOTES_SH = REPO / "docker/image-release-notes.sh"
INSTALL_SH = REPO / "tools/install.sh"
MAIN_RS = REPO / "crates/reflow2-mcp/src/main.rs"

# The ONE condition every writing step and job carries. A tag push has no
# `inputs` at all, so the push case is named explicitly, and a dispatch writes
# only when dry_run is literally false.
PUBLISH_GATE = "github.event_name == 'push' || inputs.dry_run == false"

try:
    import yaml  # type: ignore
except ImportError:  # pragma: no cover - the CI job installs it
    print("FAIL  PyYAML is required (python3 -m pip install pyyaml); refusing to pass unread")
    sys.exit(1)


def workflow() -> dict:
    doc = yaml.safe_load(RELEASE.read_text(encoding="utf-8"))
    # YAML 1.1 reads the bare key `on` as the boolean True.
    if "on" not in doc and True in doc:
        doc["on"] = doc.pop(True)
    return doc


def jobs() -> dict:
    return workflow()["jobs"]


def steps(job: dict) -> list[dict]:
    return job.get("steps", [])


def step_text(step: dict) -> str:
    """Everything a step does, as one string: its uses, run and with."""
    parts = [str(step.get("uses", "")), str(step.get("run", ""))]
    parts += [f"{k}={v}" for k, v in (step.get("with") or {}).items()]
    return "\n".join(parts)


def norm(expr) -> str:
    s = str(expr or "").strip()
    if s.startswith("${{") and s.endswith("}}"):
        s = s[3:-2].strip()
    return re.sub(r"\s+", " ", s)


def matrix_rows(job: dict) -> list[dict]:
    return ((job.get("strategy") or {}).get("matrix") or {}).get("include") or []


def image_job() -> tuple[str, dict]:
    """The per-architecture image job: the one that runs docker/smoke.sh."""
    found = [
        (name, job)
        for name, job in jobs().items()
        if any("docker/smoke.sh" in step_text(s) for s in steps(job))
    ]
    assert len(found) == 1, f"expected exactly one job that smoke-tests the image, found {[n for n, _ in found]}"
    return found[0]


def merge_job() -> tuple[str, dict]:
    found = [(n, j) for n, j in jobs().items() if j.get("name") == "container image"]
    assert len(found) == 1, (
        "the job named `container image` must exist exactly once — the release-cut procedure "
        "reads the job by that name (fact:the-cut-verified-the-assets-and-never-the-image-2026-09-14)"
    )
    return found[0]


def writes(step: dict) -> bool:
    """Does this step write to a registry or log into one?"""
    t = step_text(step)
    return (
        "docker/login-action" in t
        or "push=true" in t
        or "--push" in t
        or "imagetools create" in t
        or "publish-index.sh" in t
    )


def make_exe(path: Path, body: str) -> None:
    path.write_text(body, encoding="utf-8")
    path.chmod(path.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


# ─────────────────────────────────────────────────────────────────────────────
# 1. linux-arm64 is a native build
# ─────────────────────────────────────────────────────────────────────────────


def test_linux_arm64_is_built_natively_on_an_arm_runner() -> None:
    rows = {r.get("name"): r for r in matrix_rows(jobs()["binaries"])}
    assert "linux-arm64" in rows, f"no linux-arm64 row in the binaries matrix: {sorted(rows)}"
    arm = rows["linux-arm64"]
    assert arm.get("runner") == "ubuntu-22.04-arm", (
        "linux-arm64 must build on GitHub's ubuntu-22.04-arm runner: native, and 22.04 for "
        f"glibc parity with linux-x86_64 and the ubuntu:22.04 base — got {arm.get('runner')!r}"
    )
    assert arm.get("target") == "aarch64-unknown-linux-gnu", arm
    assert rows.get("linux-x86_64", {}).get("runner") == "ubuntu-22.04", (
        "linux-x86_64 must stay on ubuntu-22.04 (the glibc floor the image base matches)"
    )
    build = "\n".join(step_text(s) for s in steps(jobs()["binaries"]))
    assert "cargo build --release -p reflow2-mcp --target ${{ matrix.target }}" in build, build
    # What the job RUNS, not what its comments say: a comment explaining why
    # there is no cross build must not read as one.
    for cross in ("cross build", "cargo zigbuild", "gcc-aarch64-linux-gnu", "AARCH64_UNKNOWN_LINUX_GNU_LINKER",
                  "setup-qemu-action"):
        assert cross not in build + json.dumps(jobs()["binaries"].get("env") or {}), (
            f"`{cross}` is a cross-compile; linux-arm64 is built natively on an arm runner"
        )


# ─────────────────────────────────────────────────────────────────────────────
# 2. each architecture's image starts before anything is pushed
# ─────────────────────────────────────────────────────────────────────────────


def test_each_per_arch_image_is_smoke_tested_before_any_push_in_the_same_job() -> None:
    name, job = image_job()
    rows = {r.get("arch"): r for r in matrix_rows(job)}
    assert set(rows) == {"amd64", "arm64"}, (
        f"the image job `{name}` must build one image per architecture (amd64, arm64); got {sorted(rows)}"
    )
    assert rows["amd64"].get("runner") == "ubuntu-22.04", rows["amd64"]
    assert rows["arm64"].get("runner") == "ubuntu-22.04-arm", (
        "the arm64 image is built and smoke-tested ON an arm runner — that is what makes the "
        f"smoke test prove the arm64 image starts, with no emulation: {rows['arm64']}"
    )
    assert rows["amd64"].get("binary") == "linux-x86_64" and rows["arm64"].get("binary") == "linux-arm64", rows
    assert "${{ matrix.runner }}" in str(job.get("runs-on")), job.get("runs-on")
    assert "setup-qemu-action" not in RELEASE.read_text(encoding="utf-8"), (
        "no QEMU: each runner builds only its own platform"
    )

    st = steps(job)
    download = [s for s in st if "download-artifact" in str(s.get("uses", ""))]
    assert download and "binary-${{ matrix.binary }}" in str((download[0].get("with") or {}).get("name")), (
        "each architecture's job must wrap THAT architecture's release binary"
    )
    smoke = [i for i, s in enumerate(st) if "docker/smoke.sh" in step_text(s)]
    pushes = [i for i, s in enumerate(st) if writes(s)]
    assert smoke, "no smoke test in the image job"
    assert pushes, "the image job never pushes — where does the per-arch image go?"
    assert smoke[0] < min(pushes), (
        f"the smoke test (step {smoke[0]}) must run BEFORE the first login or push (step {min(pushes)}) — "
        "a smoke test after the push tells you what you shipped; this one exists to stop you shipping it"
    )
    gate = step_text(st[smoke[0]])
    assert "--platform linux/${{ matrix.arch }}" in gate or '--platform "linux/${ARCH}"' in gate, gate
    assert "--load" in gate and "push" not in gate.replace("pushed", ""), (
        f"the gate builds a LOCAL image (--load) and pushes nothing: {gate}"
    )

    pushed = "\n".join(step_text(st[i]) for i in pushes)
    assert "push-by-digest=true" in pushed and "name-canonical=true" in pushed, (
        "each architecture pushes BY DIGEST, untagged — only the merge tags anything"
    )
    assert "--metadata-file" in pushed, "the pushed digest must be read from the build's metadata, not guessed"
    for label in ("org.opencontainers.image.version", "org.opencontainers.image.revision",
                  "org.opencontainers.image.source", "org.opencontainers.image.description"):
        assert label in pushed, f"the pushed image lost its OCI label {label}"
    uploads = [s for s in st if "upload-artifact" in str(s.get("uses", ""))]
    assert uploads and "digest" in str((uploads[0].get("with") or {}).get("name")), (
        "each architecture's digest must reach the merge as an artifact"
    )


# ─────────────────────────────────────────────────────────────────────────────
# 3. the merge verifies both platforms (and the script is driven both ways)
# ─────────────────────────────────────────────────────────────────────────────

FAKE_DOCKER = r'''#!/usr/bin/env python3
"""A stand-in for `docker buildx imagetools`, backed by a JSON file."""
import json, os, sys
state_path = os.environ["FAKE_STATE"]
log = open(os.environ["FAKE_LOG"], "a")
log.write(json.dumps(sys.argv[1:]) + "\n")
args = sys.argv[1:]
assert args[:2] == ["buildx", "imagetools"], args
platforms = json.loads(os.environ["FAKE_PLATFORMS"])  # {digest: "os/arch"}
state = json.load(open(state_path)) if os.path.exists(state_path) else {}

def index_of(sources):
    manifests = []
    for src in sources:
        d = src.split("@", 1)[1]
        os_, arch = platforms[d].split("/")
        manifests.append({"mediaType": "application/vnd.oci.image.manifest.v1+json",
                          "digest": d, "size": 1, "platform": {"os": os_, "architecture": arch}})
        manifests.append({"mediaType": "application/vnd.oci.image.manifest.v1+json",
                          "digest": "sha256:" + "e" * 64, "size": 1,
                          "platform": {"os": "unknown", "architecture": "unknown"}})
    return {"schemaVersion": 2, "mediaType": "application/vnd.oci.image.index.v1+json",
            "manifests": manifests}

if args[2] == "create":
    rest, tags, sources, dry = args[3:], [], [], False
    i = 0
    while i < len(rest):
        if rest[i] in ("--tag", "-t"):
            tags.append(rest[i + 1]); i += 2; continue
        if rest[i] == "--dry-run":
            dry = True; i += 1; continue
        sources.append(rest[i]); i += 1
    idx = index_of(sources)
    if dry:
        print(json.dumps(idx, indent=2)); sys.exit(0)
    # A registry that ends up serving something other than what was composed.
    drop = os.environ.get("FAKE_DROP_ON_PUSH")
    if drop:
        idx = dict(idx, manifests=[m for m in idx["manifests"]
                                   if f"{m['platform']['os']}/{m['platform']['architecture']}" != drop])
    for t in tags:
        state[t] = idx
    json.dump(state, open(state_path, "w"))
    sys.exit(0)
if args[2] == "inspect":
    ref = args[3]
    if ref not in state:
        print(f"ERROR: {ref}: not found", file=sys.stderr); sys.exit(1)
    idx = state[ref]
    if "--raw" in args:
        print(json.dumps(idx, indent=2)); sys.exit(0)
    if "--format" in args:
        print(json.dumps({"mediaType": idx["mediaType"], "digest": "sha256:" + "d" * 64, "size": 1})); sys.exit(0)
sys.exit(f"fake docker: unhandled {args}")
'''

AMD = "sha256:" + "a" * 64
ARM = "sha256:" + "b" * 64


def run_publish_index(digests: list[str], platforms: dict[str, str], **extra_env: str):
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        make_exe(tmp / "docker", FAKE_DOCKER)
        out = tmp / "github_output"
        env = dict(
            os.environ,
            PATH=f"{tmp}:{os.environ['PATH']}",
            FAKE_STATE=str(tmp / "state.json"),
            FAKE_LOG=str(tmp / "log.jsonl"),
            FAKE_PLATFORMS=json.dumps(platforms),
            GITHUB_OUTPUT=str(out),
            **extra_env,
        )
        r = subprocess.run(
            ["bash", str(PUBLISH_INDEX), "registry.test/o/r/reflow2-mcp", "9.9.9", *digests],
            capture_output=True, text=True, env=env, cwd=REPO,
        )
        calls = [json.loads(line) for line in (tmp / "log.jsonl").read_text().splitlines()] \
            if (tmp / "log.jsonl").exists() else []
        state = json.loads((tmp / "state.json").read_text()) if (tmp / "state.json").exists() else {}
        output = out.read_text() if out.exists() else ""
        return r, calls, state, output


def test_the_merge_verifies_both_platforms() -> None:
    name, job = merge_job()
    needs = job.get("needs") or []
    needs = [needs] if isinstance(needs, str) else needs
    image_name, _ = image_job()
    assert image_name in needs, f"the merge must need the per-arch image job `{image_name}`: {needs}"
    body = "\n".join(step_text(s) for s in steps(job))
    assert "docker/publish-index.sh" in body, "the merge must create and verify the index with docker/publish-index.sh"
    outputs = job.get("outputs") or {}
    assert any("digest" in k for k in outputs), f"the merge must expose the index digest as a job output: {outputs}"

    assert PUBLISH_INDEX.exists(), f"{PUBLISH_INDEX.relative_to(REPO)} does not exist"
    src = PUBLISH_INDEX.read_text(encoding="utf-8")
    assert "linux/amd64" in src and "linux/arm64" in src, "the index check must name both platforms"

    # Both platforms: one index, both tags, and the digest reaches the job output.
    r, calls, state, output = run_publish_index([AMD, ARM], {AMD: "linux/amd64", ARM: "linux/arm64"})
    assert r.returncode == 0, f"publish-index.sh failed on a good pair:\n{r.stdout}\n{r.stderr}"
    for tag in ("registry.test/o/r/reflow2-mcp:9.9.9", "registry.test/o/r/reflow2-mcp:latest"):
        assert tag in state, f"{tag} was not created: {sorted(state)}"
    creates = [c for c in calls if c[2] == "create" and "--dry-run" not in c]
    assert len(creates) == 1, f"the version and latest must be ONE create (one index): {creates}"
    assert "digest=sha256:" + "d" * 64 in output, f"the index digest is not in GITHUB_OUTPUT:\n{output}"
    assert "published registry.test/o/r/reflow2-mcp:9.9.9" in r.stdout, (
        "keep the `published <IMAGE>:<VERSION>` line — the cut reads the container job's log for it"
    )

    # One platform missing: refused, and NOTHING was tagged — :latest never moves to a half image.
    r, calls, state, _ = run_publish_index([AMD], {AMD: "linux/amd64"})
    assert r.returncode != 0, f"publish-index.sh accepted an index with no linux/arm64:\n{r.stdout}"
    assert "linux/arm64" in (r.stdout + r.stderr), f"the refusal must name the missing platform:\n{r.stderr}"
    assert not state, f"a refused index must tag nothing, but these were created: {sorted(state)}"

    # Two digests of the SAME platform: still refused.
    other = "sha256:" + "c" * 64
    r, _, state, _ = run_publish_index([AMD, other], {AMD: "linux/amd64", other: "linux/amd64"})
    assert r.returncode != 0 and not state, "two amd64 digests are not an amd64+arm64 index"

    # The registry serves something other than what was composed: the read-back
    # catches it, and no digest reaches the release notes.
    r, _, _, output = run_publish_index([AMD, ARM], {AMD: "linux/amd64", ARM: "linux/arm64"},
                                        FAKE_DROP_ON_PUSH="linux/arm64")
    assert r.returncode != 0 and "digest=" not in output, (
        f"the pushed tag must be read back and checked, not trusted:\n{r.stdout}\n{r.stderr}"
    )

    # Not a digest at all: refused before any registry call.
    r, calls, _, _ = run_publish_index(["latest", ARM], {ARM: "linux/arm64"})
    assert r.returncode != 0 and not calls, "a source that is not sha256:<64 hex> must be refused before any call"


# ─────────────────────────────────────────────────────────────────────────────
# 4. publish waits for the image and requires the arm64 asset
# ─────────────────────────────────────────────────────────────────────────────


def test_publish_needs_the_image_and_requires_the_arm64_asset() -> None:
    pub = jobs()["publish"]
    needs = pub.get("needs") or []
    merge_name, _ = merge_job()
    for n in ("binaries", "kit", merge_name):
        assert n in needs, f"publish must need `{n}`: {needs}"
    body = "\n".join(step_text(s) for s in steps(pub))
    assert "reflow2-mcp-linux-arm64.tar.gz" in body, "publish must require the linux-arm64 asset before going live"
    for s in steps(pub):
        if "download-artifact" in str(s.get("uses", "")):
            w = s.get("with") or {}
            assert w.get("name") or w.get("pattern"), (
                "an unfiltered download pulls the per-arch DIGEST artifacts into dist/ too, and "
                f"`gh release upload dist/*` would attach them as release assets: {s}"
            )
            assert "digest" not in str(w.get("name", "")) + str(w.get("pattern", "")), s


# ─────────────────────────────────────────────────────────────────────────────
# 5. the release notes carry the digest — idempotently, appended
# ─────────────────────────────────────────────────────────────────────────────

D1 = "sha256:" + "1" * 64
D2 = "sha256:" + "2" * 64
IMG = "ghcr.io/sligara7/reflow2/reflow2-mcp"


def notes(body: str, digest: str, version: str = "9.9.9") -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["bash", str(NOTES_SH), IMG, version, digest, "linux/amd64,linux/arm64"],
        input=body, capture_output=True, text=True, cwd=REPO,
    )


def test_the_release_notes_carry_the_index_digest() -> None:
    pub = jobs()["publish"]
    st = steps(pub)
    body = "\n".join(step_text(s) for s in st)
    assert "docker/image-release-notes.sh" in body, "publish must write the image section into the notes"
    assert "needs.container.outputs.digest" in json.dumps(pub), (
        "the notes must be written from the merge job's index digest output"
    )
    m_notes = body.find("image-release-notes.sh")
    m_live = body.find("--draft=false")
    assert 0 <= m_notes < m_live, "the image section must be written BEFORE the release goes live"

    assert NOTES_SH.exists(), f"{NOTES_SH.relative_to(REPO)} does not exist"
    existing = "## What changed\n\nReal notes the cut wrote.\n"
    r = notes(existing, D1)
    assert r.returncode == 0, r.stderr
    once = r.stdout
    assert once.startswith(existing.rstrip("\n")), f"the existing notes must be kept, first, verbatim:\n{once}"
    assert f"{IMG}@{D1}" in once and f"{IMG}:9.9.9" in once, once
    # Idempotent: the same input twice is the same output.
    again = notes(once, D1).stdout
    assert again == once, f"a re-run must not stack a second section:\n{again}"
    # A re-run with a NEW digest replaces the section, and the notes survive once.
    replaced = notes(once, D2).stdout
    assert f"{IMG}@{D2}" in replaced and D1 not in replaced, replaced
    assert replaced.count("Real notes the cut wrote.") == 1, replaced
    # Notes GitHub hands back with CRLF line endings (web-edited) still match the markers.
    crlf = notes(once.replace("\n", "\r\n"), D2).stdout
    assert crlf.count("reflow2:container-image:begin") == 1 and D1 not in crlf, crlf
    # Empty notes get just the section.
    assert notes("", D1).stdout.lstrip().startswith("<!-- reflow2:container-image:begin -->")
    # A begin marker with no end would swallow every note after it: refused, not "fixed".
    broken = existing + "\n<!-- reflow2:container-image:begin -->\nhalf a section\n\nMore real notes.\n"
    r = notes(broken, D1)
    assert r.returncode != 0 and not r.stdout, "an unterminated section must be refused, never truncated"
    # Not a digest: refused.
    assert notes(existing, "latest").returncode != 0


# ─────────────────────────────────────────────────────────────────────────────
# 6. a dry run can never write anywhere
# ─────────────────────────────────────────────────────────────────────────────


def test_dry_run_gates_every_push_merge_and_publish_fail_safe() -> None:
    wf = workflow()
    inputs = wf["on"]["workflow_dispatch"]["inputs"]
    dry = inputs.get("dry_run") or {}
    assert dry.get("type") == "boolean" and dry.get("default") is False, f"dry_run must be a boolean defaulting to false: {dry}"
    assert not inputs["tag"].get("required"), "tag must be optional (a dry run needs none)"
    assert "push" in wf["on"] and wf["on"]["push"].get("tags") == ["v*"], "a tag push still releases"

    merge_name, merge = merge_job()
    for jn in (merge_name, "publish"):
        assert norm(jobs()[jn].get("if")) == PUBLISH_GATE, (
            f"job `{jn}` must carry exactly `if: {PUBLISH_GATE}` — got {jobs()[jn].get('if')!r}"
        )
    for jn, job in jobs().items():
        job_gated = norm(job.get("if")) == PUBLISH_GATE
        for s in steps(job):
            if writes(s) or ("upload-artifact" in str(s.get("uses", "")) and "digest" in json.dumps(s)):
                assert job_gated or norm(s.get("if")) == PUBLISH_GATE, (
                    f"step `{s.get('name') or s.get('uses')}` in job `{jn}` writes to a registry and is "
                    f"not gated by `{PUBLISH_GATE}`"
                )
            if "gh release" in str(s.get("run", "")):
                assert job_gated, f"`gh release` in job `{jn}` runs outside the publish gate"

    kit = "\n".join(step_text(s) for s in steps(jobs()["kit"]))
    assert "DRY_RUN" in kit and "does not match Cargo.toml version" in kit, (
        "kit must keep the tag == Cargo version check, and skip it on a dry run"
    )
    assert re.search(r"needs\s+`?tag`?|names its tag|requires a tag|needs a tag", kit, re.I), (
        "a release dispatch without a tag must fail and SAY so"
    )


# ─────────────────────────────────────────────────────────────────────────────
# 7. the image says what v0.76.0 made true
# ─────────────────────────────────────────────────────────────────────────────


def test_the_image_description_no_longer_says_no_authentication() -> None:
    text = RELEASE.read_text(encoding="utf-8")
    m = re.search(r"org\.opencontainers\.image\.description=", text)
    assert m, "the image lost its description label"
    desc_src = json.dumps(workflow())
    assert "NO AUTHENTICATION" not in text, "release.yml still describes the image as having NO AUTHENTICATION"
    assert "--http-oidc-issuer" in desc_src, "the image description must name --http-oidc-issuer"
    for path in (DOCKERFILE, BUILD_SH):
        t = path.read_text(encoding="utf-8")
        assert "NO AUTHENTICATION" not in t and "reflow2 has none" not in t, (
            f"{path.relative_to(REPO)} still says reflow2 has no authentication — false since v0.76.0"
        )
    df = DOCKERFILE.read_text(encoding="utf-8")
    for needle in ("--http-oidc-issuer", "REFLOW2_OIDC_ISSUER", "--http-trusted-gateway",
                   "DNS-rebinding", "TLS"):
        assert needle in df, f"the Dockerfile's auth section must say {needle!r}"
    # Every flag and variable the image's documentation tells an operator to use
    # must exist in the binary — the class that shipped five dead images.
    rs = MAIN_RS.read_text(encoding="utf-8")
    for flag in ("http-oidc-issuer", "http-public-url", "http-contributor-id", "http-trusted-gateway",
                 "http-allow-host"):
        assert f'long = "{flag}"' in rs, f"--{flag} is named in the image docs and not declared in main.rs"
    for var in ("REFLOW2_OIDC_ISSUER", "REFLOW2_PUBLIC_URL", "REFLOW2_CONTRIBUTOR_ID", "REFLOW2_TRUSTED_GATEWAY"):
        assert f'env = "{var}"' in rs, f"{var} is named in the image docs and clap does not read it"
    # The ENTRYPOINT is what makes `docker run <image> --http-oidc-issuer …` reach the binary.
    entry = re.search(r"^ENTRYPOINT .*?(?=\n\S|\Z)", df, re.S | re.M)
    assert entry and '\\"$@\\"' in entry.group(0), "the ENTRYPOINT no longer forwards its arguments"


# ─────────────────────────────────────────────────────────────────────────────
# 8. install.sh maps Linux aarch64
# ─────────────────────────────────────────────────────────────────────────────


def run_install(os_name: str, arch: str) -> subprocess.CompletedProcess[str]:
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        bin_ = tmp / "bin"
        bin_.mkdir()
        make_exe(bin_ / "uname", f'#!/bin/sh\ncase "$1" in -s) echo {os_name};; -m) echo {arch};; esac\n')
        # Stand-ins that fail: nothing is fetched, and the asset NAME is in the message.
        make_exe(bin_ / "gh", "#!/bin/sh\nexit 1\n")
        make_exe(bin_ / "curl", "#!/bin/sh\nexit 22\n")
        env = dict(os.environ, PATH=f"{bin_}:/usr/bin:/bin", HOME=str(tmp / "home"))
        return subprocess.run(["sh", str(INSTALL_SH)], capture_output=True, text=True, env=env, cwd=tmp)


def test_install_sh_maps_linux_aarch64() -> None:
    for arch in ("aarch64", "arm64"):
        r = run_install("Linux", arch)
        both = r.stdout + r.stderr
        assert "no prebuilt binary" not in both, f"install.sh refuses Linux/{arch}:\n{both}"
        assert "reflow2-mcp-linux-arm64.tar.gz" in both, f"Linux/{arch} must fetch the linux-arm64 asset:\n{both}"
        assert r.returncode != 0, "the stand-in download fails, so the install must too"
    r = run_install("Linux", "x86_64")
    assert "reflow2-mcp-linux-x86_64.tar.gz" in r.stdout + r.stderr, r.stdout + r.stderr
    r = run_install("Linux", "riscv64")
    assert "no prebuilt binary for Linux/riscv64" in r.stderr, r.stderr


# ─────────────────────────────────────────────────────────────────────────────
# 9. the kit carries every sibling the gate imports
# ─────────────────────────────────────────────────────────────────────────────


def test_the_kit_ships_the_design_reader_beside_the_gate() -> None:
    """reflow2_check.py reads the saved design through tools/design_io.py — a
    SIBLING import — and exits 2 without it. A kit that shipped the gate alone
    would turn every consumer's CI red with "this kit is incomplete" the day
    they upgraded (the item layout, 2026-10-03)."""
    text = RELEASE.read_text(encoding="utf-8")
    assert "cp tools/reflow2_check.py kit-stage/reflow2-kit/tools/" in text
    assert "cp tools/design_io.py kit-stage/reflow2-kit/tools/" in text, (
        "release.yml ships reflow2_check.py without design_io.py, which it imports")
    gate = (REPO / "tools" / "reflow2_check.py").read_text(encoding="utf-8")
    assert "import design_io" in gate, "the gate no longer imports the reader; drop this test"


def main() -> int:
    tests = [
        test_the_kit_ships_the_design_reader_beside_the_gate,
        test_linux_arm64_is_built_natively_on_an_arm_runner,
        test_each_per_arch_image_is_smoke_tested_before_any_push_in_the_same_job,
        test_the_merge_verifies_both_platforms,
        test_publish_needs_the_image_and_requires_the_arm64_asset,
        test_the_release_notes_carry_the_index_digest,
        test_dry_run_gates_every_push_merge_and_publish_fail_safe,
        test_the_image_description_no_longer_says_no_authentication,
        test_install_sh_maps_linux_aarch64,
    ]
    failed = 0
    for t in tests:
        try:
            t()
            print(f"PASS  {t.__name__}")
        except (AssertionError, KeyError, TypeError, FileNotFoundError) as e:
            print(f"FAIL  {t.__name__}: {type(e).__name__}: {e}")
            failed += 1
    print(f"\n{len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
