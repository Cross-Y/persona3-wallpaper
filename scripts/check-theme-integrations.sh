#!/usr/bin/env bash
# Read-only diagnostic. Does not change themes or restart processes.
set -u
printf 'Persona 3 Wallpaper — Color Integration Check\n'
printf 'Session: %s | Desktop: %s\n' "${XDG_SESSION_TYPE:-not detected}" "${XDG_CURRENT_DESKTOP:-not detected}"
for binary in ffmpeg gsettings plasma-apply-colorscheme noctalia qt5ct qt6ct kvantummanager waybar rofi wofi; do
    if command -v "$binary" >/dev/null 2>&1; then printf ' %-25s available\n' "$binary"
    else printf ' %-25s not found\n' "$binary"; fi
done
cfg="${XDG_CONFIG_HOME:-$HOME/.config}"
for dir in gtk-3.0 gtk-4.0 qt5ct qt6ct waybar rofi wofi; do
    if [ -d "$cfg/$dir" ]; then printf ' %-25s config present\n' "$dir"; fi
done
printf 'The new adapters are enabled manually in Appearance.\n'
printf 'qt5ct/qt6ct: select Persona 3 Wallpaper once; Kvantum support depends on the theme.\n'
