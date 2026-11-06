#!/usr/bin/env bash
set -euo pipefail

PID="${1:-}"
if [[ -z "$PID" ]]; then
    PID="$(pgrep -xo persona3-wallpa || true)"
fi
if [[ -z "$PID" || ! -r "/proc/$PID/status" ]]; then
    echo "Could not find persona3-wallpaper. Usage: $0 <PID>"
    echo "You can find the PID with: pgrep -a persona3-wallpaper"
    exit 1
fi

show_process() {
    local pid="$1"
    [[ -r "/proc/$pid/status" ]] || return 0
    local name rss pss
    name="$(awk '/^Name:/ {print $2}' "/proc/$pid/status")"
    rss="$(awk '/^VmRSS:/ {print $2 " kB"}' "/proc/$pid/status")"
    pss="$(awk '/^Pss:/ {print $2 " kB"}' "/proc/$pid/smaps_rollup" 2>/dev/null || echo 'unavailable')"
    printf 'PID %-8s %-22s RSS %-14s PSS %s\n' "$pid" "$name" "${rss:-unknown}" "$pss"
}

echo "Persona 3 Wallpaper memory (RSS includes shared libraries; PSS apportions shared pages):"
show_process "$PID"
echo
echo "Related direct child processes:"
children="$(pgrep -P "$PID" || true)"
if [[ -z "$children" ]]; then
    echo "  (none)"
else
    while read -r child; do show_process "$child"; done <<< "$children"
fi

echo
echo "Largest processes on the desktop (top 12):"
ps -eo pid,ppid,rss,comm,args --sort=-rss | head -n 13
