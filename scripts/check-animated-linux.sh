#!/usr/bin/env bash
# Diagnostic only; never changes settings, installs packages or starts players.
set -u
printf '%s\n' 'Persona 3 Wallpaper — Animated Wallpaper Diagnostics'
printf 'Desktop: %s\n' "${XDG_CURRENT_DESKTOP:-unknown}"
printf 'Session: %s\n' "${XDG_SESSION_TYPE:-unknown}"
for cmd in mpvpaper swaybg mpv xwinwrap gdbus gsettings gnome-extensions; do
  if command -v "$cmd" >/dev/null 2>&1; then
    printf '%-19s available\n' "$cmd"
  else
    printf '%-19s missing\n' "$cmd"
  fi
done
if command -v gnome-extensions >/dev/null 2>&1; then
  if gnome-extensions list --enabled 2>/dev/null | grep -Fxq 'wallpaperengine@waylandwe'; then
    echo 'Compatible GNOME extension: enabled'
  else
    echo 'Compatible GNOME extension: not detected/enabled'
  fi
fi
root=${XDG_DATA_HOME:-"$HOME/.local/share"}
if [[ -f "$root/plasma/wallpapers/luisbocanegra.smart.video.wallpaper.reborn/metadata.json" ]] ||
   [[ -f /usr/share/plasma/wallpapers/luisbocanegra.smart.video.wallpaper.reborn/metadata.json ]]; then
  echo 'Compatible KDE plugin: found in standard paths'
else
  echo 'Compatible KDE plugin: not found in standard paths (may be in another XDG_DATA_DIRS location)'
fi
echo 'This check does NOT verify graphical playback. See docs/ANIMATED_LINUX.md.'
