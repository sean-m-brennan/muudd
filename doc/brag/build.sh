#!/usr/bin/env bash
# Rebuild the MUUDD brag video: composition/ -> brag.mp4 -> poster -> brag.webm.
#
# The render needs Node >= 22 (the system Node is 18; the user manages node with
# nvm) plus the hyperframes cache of chrome-headless-shell and fonts under
# ~/.cache/hyperframes.
#
# npm_config_ignore_scripts is required behind the sandbox proxy: the CLI pulls in
# onnxruntime-node (its Kokoro TTS backend), whose postinstall fetches a binary
# straight from github.com and ignores http_proxy, so the whole install aborts
# with ECONNREFUSED. Nothing here needs TTS -- the narration WAVs are already in
# composition/assets/voice -- so skipping postinstalls costs us nothing. Drop this
# if you ever need to regenerate the voice track.
#
# `check` is pinned to --timeout 30000: the default 10s navigation timeout is
# flaky when several renders are running on the same box, and the failure is the
# headless browser being slow to start rather than anything in the composition.
#
#   ./build.sh                 check, render, poster, transcode
#   SKIP_RENDER=1 ./build.sh   re-do poster + transcode from an existing brag.mp4
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
export HYPERFRAMES_PYTHON="$HOME/.cache/hyperframes/tts-python"

# The poster: the settled ledger frame, fiber against mesh, every row landed.
POSTER_AT=33.2

if [[ "${SKIP_RENDER:-0}" != 1 ]]; then
    ( cd "$here/composition"
      npx --yes hyperframes@0.8.47 check --timeout 30000
      npx --yes hyperframes@0.8.47 render --quality delivery --output "$here/brag.mp4" )
fi

test -s "$here/brag.mp4"

# Poster, pulled full-res from the render itself so it lines up pixel for pixel.
ffmpeg -y -loglevel error -ss "$POSTER_AT" -i "$here/brag.mp4" \
    -frames:v 1 -q:v 2 "$here/brag.jpg"

# Bake the poster as frame 0: a bare .mp4 has no poster attribute, and every
# platform's thumbnail grabber takes frame 0. Only the first frame's pixels are
# replaced -- same duration, same frame count, audio copied through.
ffmpeg -y -loglevel error -i "$here/brag.mp4" -i "$here/brag.jpg" \
    -filter_complex "[0:v][1:v]overlay=0:0:enable='eq(n,0)'[v]" \
    -map "[v]" -map 0:a? \
    -c:v libx264 -crf 18 -preset slow -pix_fmt yuv420p \
    -c:a copy -movflags +faststart "$here/brag.poster.mp4"
mv "$here/brag.poster.mp4" "$here/brag.mp4"

ffmpeg -y -loglevel warning -i "$here/brag.mp4" \
    -c:v libvpx-vp9 -crf 32 -b:v 0 -row-mt 1 \
    -c:a libopus -b:a 128k \
    "$here/brag.webm"

ls -lh "$here/brag.mp4" "$here/brag.jpg" "$here/brag.webm"
