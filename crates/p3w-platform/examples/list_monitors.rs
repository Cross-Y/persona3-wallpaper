use p3w_platform::{linux::hyprland::HyprlandMonitorProvider, MonitorProvider};

fn main() {
    let provider = HyprlandMonitorProvider::new();

    match provider.monitors() {
        Ok(monitors) => {
            println!("Detected monitors: {}\n", monitors.len());

            for monitor in monitors {
                println!("Monitor: {}", monitor.name);
                println!("Description: {}", monitor.description);
                println!("Resolution: {}x{}", monitor.width, monitor.height);
                println!("Frecuencia: {:.2} Hz", monitor.refresh_rate);
                println!("Position: {}, {}", monitor.x, monitor.y);
                println!("Enfocado: {}", monitor.focused);
                println!("-----------------------------");
            }
        }
        Err(error) => {
            eprintln!("Error detecting monitors: {error}");
            std::process::exit(1);
        }
    }
}
