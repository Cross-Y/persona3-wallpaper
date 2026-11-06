#!/usr/bin/env bash
set -euo pipefail

lower="$(printf '%s:%s:%s' "${XDG_CURRENT_DESKTOP:-}" "${XDG_SESSION_DESKTOP:-}" "${DESKTOP_SESSION:-}" | tr '[:upper:]' '[:lower:]')"
detected=()
has_process() { command -v pgrep >/dev/null 2>&1 && pgrep -x "$1" >/dev/null 2>&1; }

if [[ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" || "$lower" == *hyprland* ]]; then detected+=("Hyprland"); fi
if [[ "$lower" == *gnome* ]] || has_process gnome-shell; then detected+=("GNOME"); fi
if [[ "$lower" == *kde* || "$lower" == *plasma* ]] || has_process plasmashell; then detected+=("KDE Plasma"); fi
if [[ "$lower" == *xfce* ]] || has_process xfce4-session; then detected+=("XFCE"); fi
if [[ "$lower" == *cinnamon* ]] || has_process cinnamon-session; then detected+=("Cinnamon"); fi
if [[ "$lower" == *mate* ]] || has_process mate-session; then detected+=("MATE"); fi
if [[ "$lower" == *lxqt* ]] || has_process lxqt-session; then detected+=("LXQt"); fi
if [[ "$lower" == *sway* ]] || has_process sway; then detected+=("Sway"); fi
if [[ "$lower" == *noctalia* ]] || has_process noctalia || has_process noctalia-shell; then detected+=("Noctalia shell"); fi

if ((${#detected[@]})); then
  printf 'Detected desktop/session: %s\n' "$(IFS=' + '; echo "${detected[*]}")"
else
  printf 'Detected desktop/session: generic Linux session\n'
fi
printf 'XDG_CURRENT_DESKTOP=%s\n' "${XDG_CURRENT_DESKTOP:-<unset>}"
printf 'XDG_SESSION_DESKTOP=%s\n' "${XDG_SESSION_DESKTOP:-<unset>}"
printf 'DESKTOP_SESSION=%s\n' "${DESKTOP_SESSION:-<unset>}"
printf 'Wayland=%s  X11=%s\n' "${WAYLAND_DISPLAY:-<unset>}" "${DISPLAY:-<unset>}"
