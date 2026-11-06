# Pre-play optimization and static image framing

## Video preparation

When an oversized video is selected, the application can offer
**Optimize, then apply** or **Play original**.

The optimization option uses FFmpeg to prepare a cached H.264 copy.
The original video is not modified. The current wallpaper remains
visible while preparation runs, and cached results may be reused.

Cancellation and failure handling should preserve the existing
wallpaper. This behavior requires verification on real Linux desktops.

## Static image framing

Static wallpapers provide these framing options:

- **Crop to fill**: fills the screen while preserving proportions.
- **Fit entire image**: shows the full image without distortion.
- **Stretch to screen**: fills the screen without preserving proportions.

Crop position and zoom can be adjusted, and framing preferences
are persisted where supported.

When monitor dimensions are unavailable, the application falls back
to the desktop backend's default behavior.

## Validation

These features are experimental. Test them with different monitor
resolutions, aspect ratios, Wayland/X11 sessions, and wallpaper backends.
FFmpeg is required for video preparation.
