#!/usr/bin/env bash
#
# Write the container image's section into a release's notes. GitHub issue #617:
# a consumer pins the image by its index digest, so the notes must carry it.
#
# Usage (a filter — the notes as they stand on stdin, the new notes on stdout):
#   gh release view vX --json body -q .body \
#     | docker/image-release-notes.sh <image> <version> <sha256:digest> [platforms] \
#     > notes.md
#
# ⭐ APPENDED, AND REPLACED ON A RE-RUN — NEVER OVERWRITTEN. The release may have
# been created by the cut with real notes, so everything outside the markers is
# kept verbatim, first. The section sits between two HTML comments (invisible on
# the release page), and a re-run removes the old section before appending the
# new one, so running this twice with one digest changes nothing and running it
# with a new digest leaves exactly one section, naming the new one.
#
# ⚠️ A BEGIN MARKER WITH NO END IS REFUSED, NOT REPAIRED. Stripping from an
# unmatched begin to the end of the notes would delete every note after it, so
# that case exits non-zero and prints nothing for the caller to write back.
# Notes edited on the web come back with CRLF line endings; the markers are
# matched with the carriage return ignored, and the text keeps its own endings.
set -euo pipefail

image="${1:?usage: docker/image-release-notes.sh <image> <version> <sha256:digest> [platforms]}"
version="${2:?usage: docker/image-release-notes.sh <image> <version> <sha256:digest> [platforms]}"
digest="${3:?usage: docker/image-release-notes.sh <image> <version> <sha256:digest> [platforms]}"
platforms="${4:-linux/amd64,linux/arm64}"
[[ "$digest" =~ ^sha256:[0-9a-f]{64}$ ]] \
  || { echo "image-release-notes: '$digest' is not a digest (sha256:<64 hex>)" >&2; exit 2; }

begin='<!-- reflow2:container-image:begin -->'
end='<!-- reflow2:container-image:end -->'

# The notes with any earlier section removed. Command substitution drops the
# trailing newlines, which is what makes a re-run byte-identical.
kept="$(awk -v b="$begin" -v e="$end" '
  { line = $0; sub(/\r$/, "", line) }
  !skip && line == b { skip = 1; next }
  skip && line == e  { skip = 0; next }
  !skip              { print }
  END { if (skip) exit 3 }
')" || {
  echo "image-release-notes: the notes open a '${begin}' section that never closes — refusing rather than dropping everything after it" >&2
  exit 1
}

if [ -n "$kept" ]; then
  printf '%s\n\n' "$kept"
fi
bt='`'
platforms_md="${platforms//,/${bt}, ${bt}}"
cat <<EOF
${begin}
## Container image

One multi-platform index (\`${platforms_md}\`):

\`\`\`
docker pull ${image}:${version}
\`\`\`

To pin it immutably, use the index digest. A tag can be moved; a digest cannot:

\`\`\`
${image}@${digest}
\`\`\`
${end}
EOF
