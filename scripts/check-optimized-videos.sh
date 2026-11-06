#!/usr/bin/env bash
# Read-only inspection of monitor-aware optimized video cache.
set -euo pipefail
cache="${XDG_CACHE_HOME:-$HOME/.cache}/persona3-wallpaper/optimized-video"
if [[ ! -d "$cache" ]]; then
    printf '%s\n' "No optimized videos have been generated yet: $cache"
    exit 0
fi
printf 'Video optimization cache: %s\n' "$cache"
find "$cache" -maxdepth 1 -type f -name '*.mp4' -printf '%s\t%p\n' | sort -nk1 | while IFS=$'\t' read -r bytes file; do
    printf '\n%s (%.1f MiB)\n' "$file" "$(awk -v n="$bytes" 'BEGIN{print n/1024/1024}')"
    if command -v ffprobe >/dev/null 2>&1; then
        ffprobe -v error -select_streams v:0 -show_entries stream=codec_name,width,height,avg_frame_rate -of default=noprint_wrappers=1 "$file" || true
    fi
done
