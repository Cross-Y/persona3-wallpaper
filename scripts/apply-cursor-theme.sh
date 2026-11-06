#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "Usage: $0 THEME [SIZE]" >&2
  exit 2
fi

theme="$1"
size="${2:-24}"
if [[ ! "$theme" =~ ^[A-Za-z0-9._-]+$ ]]; then
  echo "Invalid cursor theme name." >&2
  exit 2
fi
if [[ ! "$size" =~ ^[0-9]+$ ]] || (( size < 1 || size > 256 )); then
  echo "Cursor size must be between 1 and 256." >&2
  exit 2
fi

found=0
for root in "${HOME}/.local/share/icons" "${HOME}/.icons" /usr/local/share/icons /usr/share/icons; do
  if [[ -d "$root/$theme/cursors" ]]; then found=1; break; fi
done
if (( ! found )); then
  echo "Warning: theme '$theme' was not found in standard Xcursor icon directories; applying settings anyway." >&2
fi

applied=()
active_desktop="${XDG_CURRENT_DESKTOP:-}"
active_desktop="${active_desktop,,}"
run_if_available() {
  local label="$1"; shift
  if "$@" >/dev/null 2>&1; then applied+=("$label"); return 0; fi
  return 1
}

if [[ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ]] && command -v hyprctl >/dev/null 2>&1; then
  run_if_available Hyprland hyprctl setcursor "$theme" "$size" || true
fi
if [[ "$active_desktop" == *sway* ]] && command -v swaymsg >/dev/null 2>&1; then
  run_if_available Sway swaymsg seat seat0 xcursor_theme "$theme" "$size" || true
fi
if [[ "$active_desktop" == *gnome* ]] && command -v gsettings >/dev/null 2>&1; then
  run_if_available GNOME gsettings set org.gnome.desktop.interface cursor-theme "$theme" || true
fi
if [[ "$active_desktop" == *cinnamon* ]] && command -v gsettings >/dev/null 2>&1; then
  run_if_available Cinnamon gsettings set org.cinnamon.desktop.interface cursor-theme "$theme" || \
    run_if_available Cinnamon gsettings set org.gnome.desktop.interface cursor-theme "$theme" || true
fi
if [[ "$active_desktop" == *kde* || "$active_desktop" == *plasma* ]]; then
  if command -v kwriteconfig6 >/dev/null 2>&1; then
    run_if_available 'KDE Plasma' kwriteconfig6 --file kcminputrc --group Mouse --key cursorTheme "$theme" || true
  elif command -v kwriteconfig5 >/dev/null 2>&1; then
    run_if_available 'KDE Plasma' kwriteconfig5 --file kcminputrc --group Mouse --key cursorTheme "$theme" || true
  fi
fi
if [[ "$active_desktop" == *xfce* ]] && command -v xfconf-query >/dev/null 2>&1; then
  run_if_available XFCE xfconf-query -c xsettings -p /Gtk/CursorThemeName -n -t string -s "$theme" || \
    run_if_available XFCE xfconf-query -c xsettings -p /Gtk/CursorThemeName -s "$theme" || true
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required for the generic GTK/Xcursor settings fallback." >&2
  exit 1
fi
python3 - "$theme" "$size" <<'PY'
import os, pathlib, sys

theme, size = sys.argv[1], sys.argv[2]
home = pathlib.Path.home()
config = pathlib.Path(os.environ.get("XDG_CONFIG_HOME", home / ".config"))

def set_ini(path, section, key, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    lines = path.read_text().splitlines() if path.exists() else []
    header = f"[{section}]"
    try:
        start = next(i for i, line in enumerate(lines) if line.strip() == header)
    except StopIteration:
        if lines and lines[-1].strip(): lines.append("")
        lines.extend([header, f"{key}={value}"])
    else:
        end = next((i for i in range(start + 1, len(lines)) if lines[i].strip().startswith("[") and lines[i].strip().endswith("]")), len(lines))
        match = next((i for i in range(start + 1, end) if lines[i].split("=", 1)[0].strip() == key and "=" in lines[i]), None)
        if match is None: lines.insert(end, f"{key}={value}")
        else: lines[match] = f"{key}={value}"
    path.write_text("\n".join(lines) + "\n")

for version in ("gtk-3.0", "gtk-4.0"):
    set_ini(config / version / "settings.ini", "Settings", "gtk-cursor-theme-name", theme)
    set_ini(config / version / "settings.ini", "Settings", "gtk-cursor-theme-size", size)
set_ini(home / ".icons/default/index.theme", "Icon Theme", "Inherits", theme)

xresources = home / ".Xresources"
lines = xresources.read_text().splitlines() if xresources.exists() else []
for key, value in (("Xcursor.theme", theme), ("Xcursor.size", size)):
    prefix = key + ":"
    match = next((i for i, line in enumerate(lines) if line.lstrip().startswith(prefix)), None)
    if match is None: lines.append(f"{key}: {value}")
    else: lines[match] = f"{key}: {value}"
xresources.write_text("\n".join(lines) + "\n")
PY

if [[ -n "${DISPLAY:-}" ]] && command -v xrdb >/dev/null 2>&1; then
  xrdb -merge "$HOME/.Xresources" >/dev/null 2>&1 || true
fi
applied+=("GTK3/GTK4" "Xcursor default theme" "Xresources")
printf 'Applied cursor theme %s (size %s) through: %s\n' "$theme" "$size" "$(IFS=', '; echo "${applied[*]}")"
echo 'Some applications/compositors may require a restart or a new login session.'
