//! On-demand procfs resource sampling for the system tray (no background thread).
use std::{sync::Mutex, time::Instant};
use libc;
static RESOURCE_SAMPLE: Mutex<Option<(u64, Instant)>> = Mutex::new(None);

/// Returns the current process resource usage for the tray tooltip.
/// RSS/PSS are read from procfs; CPU is calculated from process CPU-time deltas.
pub(crate) fn current_resource_usage() -> String {
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
        let rss_kb = status.lines().find_map(|line| {
            line.strip_prefix("VmRSS:")?.split_whitespace().next()?.parse::<u64>().ok()
        }).unwrap_or(0);
        let smaps = std::fs::read_to_string("/proc/self/smaps_rollup").unwrap_or_default();
        let pss_kb = smaps.lines().find_map(|line| {
            line.strip_prefix("Pss:")?.split_whitespace().next()?.parse::<u64>().ok()
        }).unwrap_or(rss_kb);

        let cpu_ticks = std::fs::read_to_string("/proc/self/stat").ok().and_then(|stat| {
            // comm is parenthesized and may contain spaces, so split after its final ')'.
            let fields = stat.rsplit_once(')')?.1.split_whitespace().collect::<Vec<_>>();
            let user = fields.get(11)?.parse::<u64>().ok()?;
            let system = fields.get(12)?.parse::<u64>().ok()?;
            Some(user.saturating_add(system))
        }).unwrap_or(0);
        let now = Instant::now();
        let cpu_percent = {
            let mut previous = RESOURCE_SAMPLE.lock().ok();
            if let Some(ref mut guard) = previous {
                let value = guard.as_ref().and_then(|(old_ticks, old_time)| {
                    let elapsed = now.duration_since(*old_time).as_secs_f64();
                    if elapsed <= 0.0 { return None; }
                    let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
                    if hz <= 0 { return None; }
                    Some(cpu_ticks.saturating_sub(*old_ticks) as f64 / hz as f64 / elapsed * 100.0)
                }).unwrap_or(0.0);
                **guard = Some((cpu_ticks, now));
                value
            } else { 0.0 }
        };

        let format_memory = |kb: u64| -> String {
            if kb >= 1024 * 1024 { format!("{:.2} GiB", kb as f64 / 1024.0 / 1024.0) }
            else { format!("{} MiB", (kb + 512) / 1024) }
        };
        format!("Persona 3 Wallpaper\nRAM: {} RSS · {} PSS\nCPU: {:.1}%", format_memory(rss_kb), format_memory(pss_kb), cpu_percent)
    }
    #[cfg(not(target_os = "linux"))]
    { "Persona 3 Wallpaper\nResource metrics unavailable on this platform".to_string() }
}
