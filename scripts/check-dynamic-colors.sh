#!/usr/bin/env bash
# Read-only checks; never modifies GTK, Plasma, GNOME or Noctalia settings.
set -u
printf 'Persona 3 Wallpaper — Dynamic Color Diagnostics\n'
printf 'Session: %s · Desktop: %s\n' "${XDG_SESSION_TYPE:-unknown}" "${XDG_CURRENT_DESKTOP:-unknown}"
for command_name in ffmpeg gsettings plasma-apply-colorscheme kreadconfig6 noctalia; do
    if command -v "$command_name" >/dev/null 2>&1; then printf '%-27s available\n' "$command_name";
    else printf '%-27s not found\n' "$command_name"; fi
done
printf '\nThe app generates palettes at ~/.config/persona3-wallpaper/dynamic-palette.json\n'
printf 'Noctalia v5+: disable its own wallpaper under Wallpaper > General > Enabled.\n'
printf 'Dynamic colors and target integrations are controlled from Appearance.\n'
