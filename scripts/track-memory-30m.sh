#!/usr/bin/env bash
# Linux: measure the application process (not mpvpaper) over time.
set -euo pipefail

PID="${1:-$(pgrep -xo persona3-wallpa 2>/dev/null || true)}"
DURATION="${2:-1800}"
INTERVAL="${3:-30}"
OUTPUT="${4:-p3w-memory-$(date +%Y%m%d-%H%M%S).csv}"

if [[ ! "$PID" =~ ^[0-9]+$ ]] || [[ ! -r "/proc/$PID/status" ]]; then
    echo "PID not found. Usage: $0 <PID> [duration_sec=1800] [interval_sec=30] [output.csv]" >&2
    echo "Find the PID with: pgrep -af persona3-wallpaper" >&2
    exit 1
fi
if [[ ! "$DURATION" =~ ^[0-9]+$ ]] || [[ ! "$INTERVAL" =~ ^[0-9]+$ ]] || (( DURATION < 1 || INTERVAL < 1 )); then
    echo "Duration and interval must be positive integers." >&2
    exit 1
fi
if [[ ! -r "/proc/$PID/smaps_rollup" ]]; then
    echo "Cannot read /proc/$PID/smaps_rollup (use your own PID, without sudo)." >&2
    exit 1
fi

HZ="$(getconf CLK_TCK)"
START="$(date +%s)"
PREVIOUS_TS=0
PREVIOUS_TICKS=0

# /proc/PID/stat has a parenthesized name which may contain spaces or ')'.
process_ticks() {
    awk '{ sub(/^.*\) /, ""); n = split($0, fields, /[[:space:]]+/); if (n < 13) exit 1; print fields[12] + fields[13] }' "/proc/$PID/stat"
}

printf 'timestamp_utc,elapsed_seconds,rss_kib,pss_kib,uss_kib,cpu_percent\n' > "$OUTPUT"
echo "Measuring PID $PID for $DURATION s, every $INTERVAL s → $OUTPUT"
echo "Open Library → URL → Library at least once, then leave the app in the background."

while true; do
    if [[ ! -r "/proc/$PID/status" || ! -r "/proc/$PID/smaps_rollup" ]]; then
        echo "The process exited; measurement was incomplete." >&2
        break
    fi
    NOW="$(date +%s)"
    ELAPSED=$(( NOW - START ))
    RSS="$(awk '/^VmRSS:/ {print $2; found=1; exit} END {if (!found) print 0}' "/proc/$PID/status")"
    read -r PSS USS < <(awk '
        /^Pss:/ {pss=$2}
        /^Private_Clean:/ || /^Private_Dirty:/ || /^Private_Hugetlb:/ {private+=$2}
        END {printf "%.0f %.0f\n", pss, private}
    ' "/proc/$PID/smaps_rollup")
    TICKS="$(process_ticks)"
    CPU="0.00"
    if (( PREVIOUS_TS > 0 && NOW > PREVIOUS_TS )); then
        CPU="$(awk -v delta="$((TICKS - PREVIOUS_TICKS))" -v hz="$HZ" -v seconds="$((NOW - PREVIOUS_TS))" 'BEGIN {printf "%.2f", 100*delta/hz/seconds}')"
    fi
    printf '%s,%s,%s,%s,%s,%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$ELAPSED" "$RSS" "$PSS" "$USS" "$CPU" >> "$OUTPUT"
    printf 't=%4ss   RSS=%5.1f MiB   PSS=%5.1f MiB   USS=%5.1f MiB   CPU=%s%%\n' "$ELAPSED" "$(awk -v k="$RSS" 'BEGIN {print k/1024}')" "$(awk -v k="$PSS" 'BEGIN {print k/1024}')" "$(awk -v k="$USS" 'BEGIN {print k/1024}')" "$CPU"
    PREVIOUS_TS="$NOW"
    PREVIOUS_TICKS="$TICKS"
    if (( ELAPSED >= DURATION )); then break; fi
    REMAINING=$(( DURATION - ELAPSED ))
    if (( REMAINING < INTERVAL )); then sleep "$REMAINING"; else sleep "$INTERVAL"; fi
done

awk -F, 'NR==2 {rss0=$3; pss0=$4; uss0=$5} NR>1 {rss=$3; pss=$4; uss=$5; samples++}
END {
  if (samples < 1) {print "No valid samples."; exit}
  printf "\nSUMMARY (%d samples)\n", samples
  printf "  RSS: initial %.1f MiB, final %.1f MiB, delta %+.1f MiB\n", rss0/1024, rss/1024, (rss-rss0)/1024
  printf "  PSS: initial %.1f MiB, final %.1f MiB, delta %+.1f MiB\n", pss0/1024, pss/1024, (pss-pss0)/1024
  printf "  USS: initial %.1f MiB, final %.1f MiB, delta %+.1f MiB\n", uss0/1024, uss/1024, (uss-uss0)/1024
  print "  One isolated increase does not prove a leak: compare repeated runs of the same workflow."
}' "$OUTPUT"
