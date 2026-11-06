# Desktop backends

Each backend implements the `p3w_core::WallpaperBackend` contract and owns
desktop-specific operations. Planned order:

1. Hyprland / mpvpaper (first migration target)
2. KDE Plasma
3. GNOME

Do not assume that one wallpaper mechanism works across all desktops.
