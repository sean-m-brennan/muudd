#!/usr/bin/env bash
# Rebuild the Ethne brag video: composition/ -> brag.mp4 -> poster -> brag.webm.
#
# The render needs Node >= 22 (the system Node is 18; the user manages node with
# nvm), the pinned hyperframes CLI (composition/package.json), and its cache of
# chrome-headless-shell and fonts under ~/.cache/hyperframes.
#
# npm_config_ignore_scripts is required behind the sandbox proxy: the CLI pulls
# in onnxruntime-node (its Kokoro TTS backend), whose postinstall fetches a
# binary straight from github.com and ignores http_proxy, so the whole install
# aborts with ECONNREFUSED. Nothing here needs TTS -- the narration WAVs are
# already in composition/assets/voice -- so skipping postinstalls costs nothing.
# Drop it if you ever need to regenerate the voice track.
#
# `check` runs with --timeout 30000: the default 10s navigation timeout is not
# enough to start the headless browser on a loaded box, and the resulting
# failure is the browser, not the composition.
#
#   ./build.sh                 render, poster, transcode
#   SKIP_RENDER=1 ./build.sh   poster + transcode an existing brag.mp4 only
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

if [[ "${SKIP_RENDER:-0}" != 1 ]]; then
    ( cd "$here/composition"
      npm run check
      npm run render -- --quality delivery --output "$here/brag.mp4" )
fi

test -s "$here/brag.mp4"

# Poster: the settled `cargo tree` frame -- the one moment in the video that is
# real output rather than authored type, and the frame worth being the idle
# thumbnail. Baked in as frame 0 so every platform's grabber picks it up.
ffmpeg -y -loglevel warning -ss 39.6 -i "$here/brag.mp4" -frames:v 1 -q:v 2 "$here/brag.jpg"
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
