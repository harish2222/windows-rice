//! Persistent state for saka-popup.
//!
//! Remembers the user's chosen layout mode (`Portrait`, `Landscape`, `Compact`)
//! and script (`English`, `Telugu`) across launches by persisting to
//! `~/.config/yasb/saka-state.json`.

use std::path::{Path, PathBuf};
use saka::Script;
use crate::layout::LayoutMode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppState {
    pub mode: LayoutMode,
    pub script: Script,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mode: LayoutMode::Portrait,
            script: Script::Latin, // English
        }
    }
}

pub fn state_file_path() -> PathBuf {
    let home = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".config").join("yasb").join("saka-state.json")
}

pub fn load_state(path: &Path) -> AppState {
    let Ok(content) = std::fs::read_to_string(path) else {
        return AppState::default();
    };
    let mut state = AppState::default();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("\"mode\":") {
            if line.contains("landscape") {
                state.mode = LayoutMode::Landscape;
            } else if line.contains("compact") {
                state.mode = LayoutMode::Compact;
            } else if line.contains("portrait") {
                state.mode = LayoutMode::Portrait;
            }
        } else if line.starts_with("\"script\":") {
            if line.contains("telugu") {
                state.script = Script::Telugu;
            } else if line.contains("english") || line.contains("latin") {
                state.script = Script::Latin;
            }
        }
    }
    state
}

pub fn save_state(path: &Path, state: &AppState) {
    let mode_str = match state.mode {
        LayoutMode::Portrait => "portrait",
        LayoutMode::Landscape => "landscape",
        LayoutMode::Compact => "compact",
    };
    let script_str = match state.script {
        Script::Telugu => "telugu",
        _ => "english",
    };
    let content = format!(
        "{{\n  \"mode\": \"{}\",\n  \"script\": \"{}\"\n}}\n",
        mode_str, script_str
    );
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, content);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_round_trip() {
        let temp_dir = std::env::temp_dir().join("saka_test_state");
        let _ = std::fs::create_dir_all(&temp_dir);
        let path = temp_dir.join("test_state.json");

        for mode in [LayoutMode::Portrait, LayoutMode::Landscape, LayoutMode::Compact] {
            for script in [Script::Latin, Script::Telugu] {
                let s = AppState { mode, script };
                save_state(&path, &s);
                let loaded = load_state(&path);
                assert_eq!(loaded, s);
            }
        }
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&temp_dir);
    }

    #[test]
    fn missing_file_returns_default() {
        let non_existent = Path::new("non_existent_saka_state_file_12345.json");
        let s = load_state(non_existent);
        assert_eq!(s, AppState::default());
    }
}
