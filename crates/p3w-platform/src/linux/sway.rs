//! Sway monitor discovery. Reads the compositor's output JSON, not xrandr.
use std::process::Command;
use crate::{monitor::{Monitor, MonitorProvider}};

pub struct SwayMonitorProvider;
impl MonitorProvider for SwayMonitorProvider {
    fn monitors(&self) -> Result<Vec<Monitor>, String> {
        let output = Command::new("swaymsg").args(["-t", "get_outputs", "-r"]).output()
            .map_err(|e| format!("Could not run swaymsg: {e}"))?;
        if !output.status.success() {
            return Err(format!("swaymsg could not query displays: {}", String::from_utf8_lossy(&output.stderr)));
        }
        let value: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("Invalid Sway display list: {e}"))?;
        let array = value.as_array().ok_or("Sway did not return a display array")?;
        let mut monitors = Vec::new();
        for item in array {
            if !item["active"].as_bool().unwrap_or(false) { continue; }
            let Some(name) = item["name"].as_str() else { continue; };
            let rect = &item["rect"];
            let mode = &item["current_mode"];
            monitors.push(Monitor {
                id: monitors.len() as i64,
                name: name.to_string(),
                description: item["make"].as_str().unwrap_or("").to_owned() + " " + item["model"].as_str().unwrap_or(name),
                width: mode["width"].as_u64().or_else(|| rect["width"].as_u64()).unwrap_or(0) as u32,
                height: mode["height"].as_u64().or_else(|| rect["height"].as_u64()).unwrap_or(0) as u32,
                refresh_rate: mode["refresh"].as_f64().unwrap_or(0.0) / 1000.0,
                x: rect["x"].as_i64().unwrap_or(0) as i32,
                y: rect["y"].as_i64().unwrap_or(0) as i32,
                focused: item["focused"].as_bool().unwrap_or(false),
            });
        }
        if monitors.is_empty() { Err("Sway reported no active displays".into()) } else { Ok(monitors) }
    }
}
