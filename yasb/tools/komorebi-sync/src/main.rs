// ============================================================================
// Author -: HKDEVS
// Project -: Window-Rice
// Title -: YASB komorebi-sync Module (main)
// ============================================================================
//! Komorebi Workspace Window Count Synchronizer
//!
//! Subscribes to Komorebi state and keeps workspace names formatted with
//! window counts across all monitors (e.g., `1 (2)`, `2 (1)`, and clean `3` when empty).
//! This feeds directly into YASB Reborn's `{name}` workspace label without
//! modifying any YASB internals.

use serde::Deserialize;
use std::env;
use std::error::Error;
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::thread;
use std::time::Duration;

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Deserialize)]
pub struct KomorebiState {
    pub monitors: MonitorList,
}

#[derive(Debug, Deserialize)]
pub struct MonitorList {
    pub elements: Vec<Monitor>,
}

#[derive(Debug, Deserialize)]
pub struct Monitor {
    pub id: Option<i64>,
    pub workspaces: WorkspaceList,
}

#[derive(Debug, Deserialize)]
pub struct WorkspaceList {
    pub elements: Vec<Workspace>,
}

#[derive(Debug, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub containers: ContainerList,
    pub floating_windows: Option<WindowList>,
}

#[derive(Debug, Deserialize)]
pub struct ContainerList {
    pub elements: Vec<Container>,
}

#[derive(Debug, Deserialize)]
pub struct Container {
    pub windows: WindowList,
}

#[derive(Debug, Deserialize)]
pub struct WindowList {
    pub elements: Vec<serde_json::Value>,
}

/// Compute the desired workspace display name given zero-based index and window count.
pub fn calculate_workspace_name(ws_idx: usize, window_count: usize) -> String {
    let base_idx = ws_idx + 1;
    if window_count > 0 {
        format!("{base_idx} ({window_count})")
    } else {
        format!("{base_idx}")
    }
}

/// Synchronize all workspace names against current Komorebi state.
pub fn sync_workspace_names() -> Result<usize, Box<dyn Error>> {
    let output = Command::new("komorebic")
        .arg("state")
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;

    if !output.status.success() {
        return Ok(0);
    }

    let state: KomorebiState = serde_json::from_slice(&output.stdout)?;
    let mut updated = 0;

    for (mon_idx, monitor) in state.monitors.elements.iter().enumerate() {
        for (ws_idx, ws) in monitor.workspaces.elements.iter().enumerate() {
            let mut window_count = 0;
            for container in &ws.containers.elements {
                window_count += container.windows.elements.len();
            }
            if let Some(ref floats) = ws.floating_windows {
                window_count += floats.elements.len();
            }

            let desired = calculate_workspace_name(ws_idx, window_count);
            if ws.name != desired {
                let _ = Command::new("komorebic")
                    .args([
                        "workspace-name",
                        &mon_idx.to_string(),
                        &ws_idx.to_string(),
                        &desired,
                    ])
                    .creation_flags(CREATE_NO_WINDOW)
                    .output();
                updated += 1;
            }
        }
    }

    Ok(updated)
}

fn run_daemon() {
    let log_path = std::env::temp_dir().join("komorebi_sync.log");
    let mut consecutive_failures = 0;
    let mut iter = 0;
    loop {
        iter += 1;
        match sync_workspace_names() {
            Ok(updated) => {
                consecutive_failures = 0;
                let _ = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_path)
                    .and_then(|mut f| {
                        use std::io::Write;
                        writeln!(f, "Iter {iter}: updated {updated}")
                    });
            }
            Err(e) => {
                consecutive_failures += 1;
                let _ = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_path)
                    .and_then(|mut f| {
                        use std::io::Write;
                        writeln!(f, "Iter {iter} ERR: {e}")
                    });
            }
        }

        let sleep_ms = if consecutive_failures > 5 {
            2000
        } else {
            750
        };

        thread::sleep(Duration::from_millis(sleep_ms));
    }
}

fn main() {
    std::panic::set_hook(Box::new(|info| {
        let log_path = std::env::temp_dir().join("komorebi_sync_panic.log");
        let _ = std::fs::write(log_path, format!("PANIC: {info}\n"));
    }));

    let log_path = std::env::temp_dir().join("komorebi_sync.log");
    let _ = std::fs::write(&log_path, "Started\n");

    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--once") {
        match sync_workspace_names() {
            Ok(count) => {
                let _ = std::fs::write(&log_path, format!("Updated {count}\n"));
            }
            Err(e) => {
                let _ = std::fs::write(&log_path, format!("Error: {e}\n"));
                std::process::exit(1);
            }
        }
    } else {
        run_daemon();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_workspace_name_empty() {
        assert_eq!(calculate_workspace_name(0, 0), "1");
        assert_eq!(calculate_workspace_name(1, 0), "2");
        assert_eq!(calculate_workspace_name(8, 0), "9");
    }

    #[test]
    fn test_calculate_workspace_name_with_windows() {
        assert_eq!(calculate_workspace_name(0, 1), "1 (1)");
        assert_eq!(calculate_workspace_name(0, 2), "1 (2)");
        assert_eq!(calculate_workspace_name(1, 4), "2 (4)");
    }

    #[test]
    fn test_parse_state_snippet() {
        let json_data = r#"{
            "monitors": {
                "elements": [
                    {
                        "id": 65537,
                        "workspaces": {
                            "elements": [
                                {
                                    "name": "1",
                                    "containers": {
                                        "elements": [
                                            {
                                                "windows": {
                                                    "elements": [
                                                        {"title": "Terminal"}
                                                    ]
                                                }
                                            }
                                        ]
                                    },
                                    "floating_windows": {
                                        "elements": []
                                    }
                                }
                            ]
                        }
                    }
                ]
            }
        }"#;

        let state: KomorebiState = serde_json::from_str(json_data).unwrap();
        assert_eq!(state.monitors.elements.len(), 1);
        assert_eq!(state.monitors.elements[0].workspaces.elements.len(), 1);
        let ws = &state.monitors.elements[0].workspaces.elements[0];
        assert_eq!(ws.name, "1");
        assert_eq!(ws.containers.elements[0].windows.elements.len(), 1);
    }
}
