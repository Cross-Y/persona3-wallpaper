#!/usr/bin/env bash
# Read-only report. Uses no distro-specific package managers and changes no files.
set -euo pipefail

printf 'Persona 3 Wallpaper — Linux compatibility check\n\n'
if [[ -r /etc/os-release ]]; then
  distro="$(sed -n 's/^PRETTY_NAME=//p' /etc/os-release | head -n 1)"
  distro="${distro%\"}"; distro="${distro#\"}"
  printf 'Distribution: %s\n' "${distro:-Linux}"
else
  printf 'Distribution: Linux (os-release unavailable)\n'
fi

printf 'Session: %s\n' "${XDG_SESSION_TYPE:-unknown}"
printf 'Desktop tokens: %s\n' "${XDG_CURRENT_DESKTOP:-${XDG_SESSION_DESKTOP:-${DESKTOP_SESSION:-unknown}}}"
printf 'Hyprland session: %s\n' "$(if [[ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ]]; then echo yes; else echo no; fi)"
printf '\nXDG paths (raw overrides; empty means defaults):\n'
for key in XDG_CONFIG_HOME XDG_DATA_HOME XDG_STATE_HOME XDG_RUNTIME_DIR XDG_DATA_DIRS; do
  printf '  %s=%s\n' "$key" "${!key:-<default>}"
done

printf '\nProgram checks (available / unavailable):\n'
for binary in hyprctl swaymsg swaybg mpvpaper gsettings xfconf-query pcmanfm-qt qdbus6 qdbus-qt6 qdbus gdbus fc-list fc-cache; do
  if command -v "$binary" >/dev/null 2>&1; then
    printf '  %-18s available\n' "$binary"
  else
    printf '  %-18s unavailable\n' "$binary"
  fi
done

printf '\nWallpaper tools by desktop: Hyprland/Sway=swaybg+mpvpaper; GNOME/Cinnamon/MATE=gsettings; Plasma=qdbus/gdbus; XFCE=xfconf-query; LXQt=pcmanfm-qt.\n'
printf 'Desktop themes/fonts: XDG/GTK/Fontconfig integration and native adapters when available.\n'
printf 'A missing command is a capability warning, not an instruction to install a specific package.\n'
