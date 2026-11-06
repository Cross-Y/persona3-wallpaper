use std::collections::{HashMap, HashSet};
use std::process::Command;

use crate::monitor::{Monitor, MonitorProvider};

/// Adapter for detecting monitors through Hyprland.
pub struct HyprlandMonitorProvider;

impl HyprlandMonitorProvider {
    pub fn new() -> Self {
        Self
    }

    /// Return names of monitors with at least one window
    /// mapped to a visible workspace (including floating, pinned, and special workspaces).
    /// Does not depend on which window has focus.
    pub fn occupied_monitor_names(&self) -> Result<HashSet<String>, String> {
        let monitors_output = Command::new("hyprctl")
            .args(["monitors", "-j"])
            .output()
            .map_err(|error| format!("Could not query monitors: {error}"))?;
        if !monitors_output.status.success() {
            return Err(format!(
                "Hyprland could not list monitors: {}",
                String::from_utf8_lossy(&monitors_output.stderr)
            ));
        }
        let monitors_json: serde_json::Value = serde_json::from_slice(&monitors_output.stdout)
            .map_err(|error| format!("Invalid monitor JSON: {error}"))?;

        // Monitor ID -> workspaces actually visible on that monitor.
        let mut visible_workspaces: HashMap<i64, (String, HashSet<i64>)> = HashMap::new();
        for monitor in monitors_json
            .as_array()
            .ok_or("Invalid monitor list")?
        {
            let Some(monitor_id) = monitor.get("id").and_then(serde_json::Value::as_i64) else {
                continue;
            };
            let Some(monitor_name) = monitor.get("name").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let (_, workspaces) = visible_workspaces
                .entry(monitor_id)
                .or_insert_with(|| (monitor_name.to_string(), HashSet::new()));
            if let Some(id) = monitor
                .get("activeWorkspace")
                .and_then(|workspace| workspace.get("id"))
                .and_then(serde_json::Value::as_i64)
            {
                workspaces.insert(id);
            }
            // A special workspace may be visible over the regular workspace.
            if let Some(id) = monitor
                .get("specialWorkspace")
                .and_then(|workspace| workspace.get("id"))
                .and_then(serde_json::Value::as_i64)
            {
                if id != 0 {
                    workspaces.insert(id);
                }
            }
        }

        let clients_output = Command::new("hyprctl")
            .args(["clients", "-j"])
            .output()
            .map_err(|error| format!("Could not query windows: {error}"))?;
        if !clients_output.status.success() {
            return Err(format!(
                "Hyprland could not list windows: {}",
                String::from_utf8_lossy(&clients_output.stderr)
            ));
        }
        let clients_json: serde_json::Value = serde_json::from_slice(&clients_output.stdout)
            .map_err(|error| format!("Invalid window JSON: {error}"))?;

        let mut occupied = HashSet::new();
        for client in clients_json
            .as_array()
            .ok_or("Invalid window list")?
        {
            // mapped=false usually means the window is minimized or not visible.
            if !client
                .get("mapped")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
                continue;
            }
            if client
                .get("hidden")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
                continue;
            }
            let Some(monitor_id) = client.get("monitor").and_then(serde_json::Value::as_i64) else {
                continue;
            };
            let Some(workspace_id) = client
                .get("workspace")
                .and_then(|workspace| workspace.get("id"))
                .and_then(serde_json::Value::as_i64)
            else {
                continue;
            };
            if let Some((monitor_name, workspaces)) = visible_workspaces.get(&monitor_id) {
                // Pinned windows remain visible when changing workspaces.
                let pinned = client
                    .get("pinned")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                if pinned || workspaces.contains(&workspace_id) {
                    occupied.insert(monitor_name.clone());
                }
            }
        }
        Ok(occupied)
    }
}

impl Default for HyprlandMonitorProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MonitorProvider for HyprlandMonitorProvider {
    fn monitors(&self) -> Result<Vec<Monitor>, String> {
        let output = Command::new("hyprctl")
            .args(["monitors", "-j"])
            .output()
            .map_err(|error| format!("Could not run hyprctl: {error}"))?;

        if !output.status.success() {
            return Err(format!(
                "Hyprland returned an error: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let json: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("Invalid monitor JSON: {error}"))?;

        let outputs = json
            .as_array()
            .ok_or_else(|| "Hyprland response is not a list".to_string())?;

        let mut monitors = Vec::new();

        for output in outputs {
            let id = output["id"]
                .as_i64()
                .ok_or_else(|| "Monitor has no ID".to_string())?;

            let name = output["name"]
                .as_str()
                .ok_or_else(|| "Monitor has no name".to_string())?;

            let width = output["width"]
                .as_u64()
                .ok_or_else(|| format!("Invalid width on {name}"))?;

            let height = output["height"]
                .as_u64()
                .ok_or_else(|| format!("Invalid height on {name}"))?;

            let x = output["x"]
                .as_i64()
                .ok_or_else(|| format!("Invalid X position on {name}"))?;

            let y = output["y"]
                .as_i64()
                .ok_or_else(|| format!("Invalid Y position on {name}"))?;

            let refresh_rate = output["refreshRate"]
                .as_f64()
                .ok_or_else(|| format!("Invalid refresh rate on {name}"))?;

            let focused = output["focused"].as_bool().unwrap_or(false);

            let description = output["description"].as_str().unwrap_or(name);

            monitors.push(Monitor {
                id,
                name: name.to_string(),
                description: description.to_string(),
                width: u32::try_from(width)
                    .map_err(|_| format!("Width out of range on {name}"))?,
                height: u32::try_from(height)
                    .map_err(|_| format!("Height out of range on {name}"))?,
                refresh_rate,
                x: i32::try_from(x).map_err(|_| format!("X position out of range on {name}"))?,
                y: i32::try_from(y).map_err(|_| format!("Y position out of range on {name}"))?,
                focused,
            });
        }

        Ok(monitors)
    }
}
