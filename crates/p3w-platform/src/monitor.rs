/// Desktop-independent information about a monitor.
#[derive(Debug, Clone, PartialEq)]
pub struct Monitor {
    /// Hyprland monitor ID, used by activewindow.monitor.
    pub id: i64,
    pub name: String,
    pub description: String,
    pub width: u32,
    pub height: u32,
    pub refresh_rate: f64,
    pub x: i32,
    pub y: i32,
    pub focused: bool,
}

/// Contract implemented by desktop-specific monitor adapters.
pub trait MonitorProvider {
    fn monitors(&self) -> Result<Vec<Monitor>, String>;
}
