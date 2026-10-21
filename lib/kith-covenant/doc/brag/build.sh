#!/usr/bin/env bash
# Rebuild the Kith-Covenant brag video: composition/ -> brag.mp4 -> brag.jpg -> brag.webm.
#
# The render needs the pinned hyperframes CLI (composition/package.json) plus its
# cache of chrome-headless-shell and fonts under ~/.cache/hyperframes.
#
# Node: the system node is 18 and hyperframes needs >= 22, so the nvm-managed
# v22 is put on PATH below.
#
# npm_config_ignore_scripts is required behind the sandbox proxy: the CLI pulls in
# onnxruntime-node (its Kokoro TTS backend), whose postinstall fetches a binary
# straight from github.com and ignores http_proxy, so the whole install aborts
# with ECONNREFUSED. Nothing here needs TTS -- the narration WAVs are already in
# composition/assets/voice -- so skipping postinstalls costs us nothing. Drop this
# if you ever need to regenerate the voice track, and export
# HYPERFRAMES_PYTHON="$HOME/.cache/hyperframes/tts-python" as well.
#
# check runs with --timeout 30000: the default 10s navigation timeout is flaky
# when several renders share the machine, and a slow browser start is not a
# composition defect.
#
#   ./build.sh                 check, render, poster, transcode
#   SKIP_RENDER=1 ./build.sh   poster + transcode from an existing brag.mp4
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Node >= 22 is required (the system Node is 18; Sean manages Node with nvm).
# Resolve the newest installed major >= 22 rather than pinning one patch release,
# so this keeps working after an nvm upgrade.
if ! node --version 2>/dev/null | grep -qE '^v(2[2-9]|[3-9][0-9])\.'; then
    node_bin=$(ls -d "$HOME"/.nvm/versions/node/v* 2>/dev/null \
        | sed 's|.*/v||' | sort -t. -k1,1n -k2,2n -k3,3n \
        | awk -F. '$1 >= 22' | tail -1)
    [[ -n "$node_bin" ]] || { echo "need Node >= 22; none found under ~/.nvm" >&2; exit 1; }
    export PATH="$HOME/.nvm/versions/node/v$node_bin/bin:$PATH"
fi
export npm_config_ignore_scripts=true

HF="npx --yes hyperframes@0.8.47"
POSTER_AT="${POSTER_AT:-21.4}"   # absolute-measures scene, both gap readings settled

if [[ "${SKIP_RENDER:-0}" != 1 ]]; then
    ( cd "$here/composition"
      $HF check --timeout 30000
      $HF render --quality delivery --output "$here/brag.mp4" )
fi

test -s "$here/brag.mp4"

# Poster: a deliberately chosen settled frame, then baked as frame 0 so the
# platforms that regenerate thumbnails server-side pick it up.
ffmpeg -y -loglevel warning -ss "$POSTER_AT" -i "$here/brag.mp4" \
    -frames:v 1 -q:v 2 "$here/brag.jpg"
ffmpeg -y -loglevel warning -i "$here/brag.mp4" -i "$here/brag.jpg" \
    -filter_complex "[0:v][1:v]overlay=0:0:enable='eq(n,0)'[v]" \
    -map "[v]" -map 0:a? -c:v libx264 -crf 18 -preset slow -pix_fmt yuv420p \
    -c:a copy -movflags +faststart "$here/brag.poster.mp4"
mv "$here/brag.poster.mp4" "$here/brag.mp4"

ffmpeg -y -loglevel warning -i "$here/brag.mp4" \
    -c:v libvpx-vp9 -crf 32 -b:v 0 -row-mt 1 \
    -c:a libopus -b:a 128k \
    "$here/brag.webm"

ls -lh "$here/brag.mp4" "$here/brag.jpg" "$here/brag.webm"
