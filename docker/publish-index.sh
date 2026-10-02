#!/usr/bin/env bash
#
# Join per-architecture images, pushed untagged by digest, into ONE multi-platform
# index; tag it <version> and latest; and refuse unless the index lists every
# required platform. GitHub issue #617.
#
# Usage:
#   docker/publish-index.sh <image> <version> <sha256:digest>...
#   docker/publish-index.sh ghcr.io/sligara7/reflow2/reflow2-mcp 0.77.0 sha256:aaa… sha256:bbb…
#
# Environment:
#   REQUIRED_PLATFORMS   space-separated os/arch the index must list
#                        (default: "linux/amd64 linux/arm64")
#   GITHUB_OUTPUT        if set, image=, version=, digest= and platforms= are
#                        appended to it, for the release notes
#
# Prints `published <image>:<version>` and `index <image>@<digest>` on success.
# The release-cut procedure reads the first line in the `container image` job's
# log (fact:the-cut-verified-the-assets-and-never-the-image-2026-09-14).
#
# ⭐ CHECKED BEFORE ANYTHING IS TAGGED, THEN AGAIN ON WHAT WAS TAGGED.
#   1. `imagetools create --dry-run` composes the index WITHOUT pushing it, and
#      the platforms are checked on that. A missing architecture therefore tags
#      nothing: `latest` stays on the last good index instead of moving to a
#      half one, which a check after the push could only report.
#   2. `<version>` and `latest` are created in ONE call, so they are one index
#      with one digest — never two builds that happen to share a name.
#   3. The PUSHED tag is read back from the registry and checked the same way,
#      and `latest` must resolve to the same digest.
#
# ⚠️ THIS SAYS WHAT THE INDEX LISTS, NOT THAT ANY IMAGE IN IT STARTS. That is
# docker/smoke.sh's job, run per architecture in release.yml BEFORE the push. A
# "pullable" check passed on five releases whose image never started; this one
# is stronger than that check and is still not that gate.
#
# Testable against a throwaway local registry (registry:2 on localhost:5000),
# and driven with a stand-in `docker` by tools/test_release_workflow.py.
set -euo pipefail

usage() { echo "usage: docker/publish-index.sh <image> <version> <sha256:digest>..." >&2; exit 2; }
[ "$#" -ge 3 ] || usage
image="$1"; version="$2"; shift 2
required="${REQUIRED_PLATFORMS:-linux/amd64 linux/arm64}"

[ -n "$image" ] && [ -n "$version" ] || usage
sources=()
for d in "$@"; do
  # A tag here would let the index name whatever that tag points at by the
  # time it is read; a digest names one image forever.
  [[ "$d" =~ ^sha256:[0-9a-f]{64}$ ]] \
    || { echo "publish-index: '$d' is not a digest (sha256:<64 hex>); refusing before touching the registry" >&2; exit 2; }
  sources+=("${image}@${d}")
done

is_index() {
  case "$(jq -r '.mediaType // empty' <<< "$1")" in
    application/vnd.oci.image.index.v1+json|application/vnd.docker.distribution.manifest.list.v2+json) return 0 ;;
    *) return 1 ;;
  esac
}

# os/arch of every image manifest in an index, attestations (unknown/unknown)
# left out; a variant (arm64's v8) is not part of the comparison.
platforms_of() {
  jq -r '.manifests[]? | select(.platform != null and .platform.os != "unknown")
         | "\(.platform.os)/\(.platform.architecture)"' <<< "$1" | sort -u
}

# check <what> <index json> — refuse unless every required platform is listed.
check() {
  local what="$1" json="$2" have listed missing=""
  is_index "$json" || { echo "publish-index: ${what} is not a multi-platform index:" >&2; echo "$json" >&2; return 1; }
  have="$(platforms_of "$json")"
  listed="${have//$'\n'/ }"
  for p in $required; do
    grep -qx "$p" <<< "$have" || missing="${missing} ${p}"
  done
  if [ -n "$missing" ]; then
    echo "publish-index: ${what} is missing${missing} — it lists: ${listed:-nothing}" >&2
    return 1
  fi
  echo "publish-index: ${what} lists ${listed}"
}

tags=(--tag "${image}:${version}" --tag "${image}:latest")

# 1. Compose it, push nothing, and check what it WOULD be.
planned="$(docker buildx imagetools create --dry-run "${tags[@]}" "${sources[@]}")"
check "the index these digests make" "$planned" \
  || { echo "publish-index: refusing — nothing was tagged, and ${image}:latest has not moved" >&2; exit 1; }

# 2. One index, both tags.
docker buildx imagetools create "${tags[@]}" "${sources[@]}"

# 3. Read back what the registry now serves under the tag, and check it again.
pushed="$(docker buildx imagetools inspect "${image}:${version}" --raw)"
check "${image}:${version} as the registry serves it" "$pushed"
digest="$(docker buildx imagetools inspect "${image}:${version}" --format '{{json .Manifest}}' | jq -r '.digest // empty')"
latest="$(docker buildx imagetools inspect "${image}:latest" --format '{{json .Manifest}}' | jq -r '.digest // empty')"
[[ "$digest" =~ ^sha256:[0-9a-f]{64}$ ]] || { echo "publish-index: no digest for ${image}:${version}: '${digest}'" >&2; exit 1; }
[ "$latest" = "$digest" ] \
  || { echo "publish-index: ${image}:latest is ${latest:-unresolved}, not ${digest} — the two tags are not one index" >&2; exit 1; }

platforms="$(platforms_of "$pushed" | paste -sd, -)"
echo "published ${image}:${version}"
echo "index ${image}@${digest} (${platforms})"
if [ -n "${GITHUB_OUTPUT:-}" ]; then
  {
    echo "image=${image}"
    echo "version=${version}"
    echo "digest=${digest}"
    echo "platforms=${platforms}"
  } >> "$GITHUB_OUTPUT"
fi
