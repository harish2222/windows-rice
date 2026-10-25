//! Data model for the keybindings cheatsheet.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Keybind {
    pub keys: &'static str,
    pub desc: &'static str,
    pub category: &'static str,
    pub cmd: &'static str,
}

pub fn all_keybinds() -> Vec<Keybind> {
    vec![
        // Navigation (Focus)
        Keybind { keys: "Alt + H", desc: "Focus window to the left", category: "Navigation", cmd: "komorebic focus left" },
        Keybind { keys: "Alt + L", desc: "Focus window to the right", category: "Navigation", cmd: "komorebic focus right" },
        Keybind { keys: "Alt + K", desc: "Focus window above", category: "Navigation", cmd: "komorebic focus up" },
        Keybind { keys: "Alt + J", desc: "Focus window below", category: "Navigation", cmd: "komorebic focus down" },
        Keybind { keys: "Alt + Left", desc: "Focus window to the left (arrow)", category: "Navigation", cmd: "komorebic focus left" },
        Keybind { keys: "Alt + Right", desc: "Focus window to the right (arrow)", category: "Navigation", cmd: "komorebic focus right" },
        Keybind { keys: "Alt + Up", desc: "Focus window above (arrow)", category: "Navigation", cmd: "komorebic focus up" },
        Keybind { keys: "Alt + Down", desc: "Focus window below (arrow)", category: "Navigation", cmd: "komorebic focus down" },

        // Move Window
        Keybind { keys: "Alt + Shift + H", desc: "Move window to the left", category: "Move Window", cmd: "komorebic move left" },
        Keybind { keys: "Alt + Shift + L", desc: "Move window to the right", category: "Move Window", cmd: "komorebic move right" },
        Keybind { keys: "Alt + Shift + K", desc: "Move window up", category: "Move Window", cmd: "komorebic move up" },
        Keybind { keys: "Alt + Shift + J", desc: "Move window down", category: "Move Window", cmd: "komorebic move down" },
        Keybind { keys: "Alt + Shift + Left", desc: "Move window to the left (arrow)", category: "Move Window", cmd: "komorebic move left" },
        Keybind { keys: "Alt + Shift + Right", desc: "Move window to the right (arrow)", category: "Move Window", cmd: "komorebic move right" },
        Keybind { keys: "Alt + Shift + Up", desc: "Move window up (arrow)", category: "Move Window", cmd: "komorebic move up" },
        Keybind { keys: "Alt + Shift + Down", desc: "Move window down (arrow)", category: "Move Window", cmd: "komorebic move down" },

        // Multi-Monitor
        Keybind { keys: "Alt + ,", desc: "Focus previous monitor", category: "Multi-Monitor", cmd: "komorebic cycle-monitor previous" },
        Keybind { keys: "Alt + .", desc: "Focus next monitor", category: "Multi-Monitor", cmd: "komorebic cycle-monitor next" },
        Keybind { keys: "Alt + Shift + ,", desc: "Move window to previous monitor", category: "Multi-Monitor", cmd: "komorebic cycle-move-to-monitor previous" },
        Keybind { keys: "Alt + Shift + .", desc: "Move window to next monitor", category: "Multi-Monitor", cmd: "komorebic cycle-move-to-monitor next" },

        // Window State & Layout Modes
        Keybind { keys: "Alt + F", desc: "Toggle window fullscreen / maximize", category: "Window State", cmd: "komorebic toggle-maximize" },
        Keybind { keys: "Alt + Shift + F", desc: "Toggle monocle layout", category: "Window State", cmd: "komorebic toggle-monocle" },
        Keybind { keys: "Alt + T", desc: "Toggle window floating / tiling", category: "Window State", cmd: "komorebic toggle-float" },
        Keybind { keys: "Alt + Shift + T", desc: "Toggle workspace tiling mode", category: "Window State", cmd: "komorebic toggle-tiling" },
        Keybind { keys: "Alt + Q", desc: "Close focused window", category: "Window State", cmd: "komorebic close" },

        // Stacking & Layout Modes
        Keybind { keys: "Alt + X", desc: "Cycle layout modes (BSP, Columns, Rows, Stacks, Grid)", category: "Layout & Stack", cmd: "komorebic cycle-layout next" },
        Keybind { keys: "Alt + Shift + X", desc: "Cycle previous layout mode", category: "Layout & Stack", cmd: "komorebic cycle-layout previous" },
        Keybind { keys: "Alt + S", desc: "Toggle window stacking vs dynamic tiling", category: "Layout & Stack", cmd: "komorebic toggle-window-container-behaviour" },
        Keybind { keys: "Alt + C / Alt + ]", desc: "Cycle stacked windows in container (forward)", category: "Layout & Stack", cmd: "komorebic cycle-stack next" },
        Keybind { keys: "Alt + Shift + C / Alt + [", desc: "Cycle stacked windows in container (reverse)", category: "Layout & Stack", cmd: "komorebic cycle-stack previous" },
        Keybind { keys: "Alt + A", desc: "Stack all windows on workspace", category: "Layout & Stack", cmd: "komorebic stack-all" },
        Keybind { keys: "Alt + Shift + A", desc: "Unstack all windows on workspace", category: "Layout & Stack", cmd: "komorebic unstack-all" },

        // Quick Resize Shortcuts
        Keybind { keys: "Alt + =", desc: "Quick expand window (both axes)", category: "Resize", cmd: "komorebic resize-axis horizontal-and-vertical increase" },
        Keybind { keys: "Alt + -", desc: "Quick shrink window (both axes)", category: "Resize", cmd: "komorebic resize-axis horizontal-and-vertical decrease" },
        Keybind { keys: "Alt + Shift + =", desc: "Equalize window sizes (retile)", category: "Resize", cmd: "komorebic retile" },

        // Workspaces (Focus)
        Keybind { keys: "Alt + 1", desc: "Switch to Workspace 1", category: "Workspaces", cmd: "komorebic focus-workspace 0" },
        Keybind { keys: "Alt + 2", desc: "Switch to Workspace 2", category: "Workspaces", cmd: "komorebic focus-workspace 1" },
        Keybind { keys: "Alt + 3", desc: "Switch to Workspace 3", category: "Workspaces", cmd: "komorebic focus-workspace 2" },
        Keybind { keys: "Alt + 4", desc: "Switch to Workspace 4", category: "Workspaces", cmd: "komorebic focus-workspace 3" },
        Keybind { keys: "Alt + 5", desc: "Switch to Workspace 5", category: "Workspaces", cmd: "komorebic focus-workspace 4" },
        Keybind { keys: "Alt + 6", desc: "Switch to Workspace 6", category: "Workspaces", cmd: "komorebic focus-workspace 5" },
        Keybind { keys: "Alt + 7", desc: "Switch to Workspace 7", category: "Workspaces", cmd: "komorebic focus-workspace 6" },
        Keybind { keys: "Alt + 8", desc: "Switch to Workspace 8", category: "Workspaces", cmd: "komorebic focus-workspace 7" },
        Keybind { keys: "Alt + 9", desc: "Switch to Workspace 9", category: "Workspaces", cmd: "komorebic focus-workspace 8" },

        // Workspaces (Move & Follow)
        Keybind { keys: "Alt + Shift + 1", desc: "Move window to Workspace 1 & follow", category: "Workspaces (Move)", cmd: "komorebic move-to-workspace 0" },
        Keybind { keys: "Alt + Shift + 2", desc: "Move window to Workspace 2 & follow", category: "Workspaces (Move)", cmd: "komorebic move-to-workspace 1" },
        Keybind { keys: "Alt + Shift + 3", desc: "Move window to Workspace 3 & follow", category: "Workspaces (Move)", cmd: "komorebic move-to-workspace 2" },
        Keybind { keys: "Alt + Shift + 4", desc: "Move window to Workspace 4 & follow", category: "Workspaces (Move)", cmd: "komorebic move-to-workspace 3" },
        Keybind { keys: "Alt + Shift + 5", desc: "Move window to Workspace 5 & follow", category: "Workspaces (Move)", cmd: "komorebic move-to-workspace 4" },
        Keybind { keys: "Alt + Shift + 6", desc: "Move window to Workspace 6 & follow", category: "Workspaces (Move)", cmd: "komorebic move-to-workspace 5" },
        Keybind { keys: "Alt + Shift + 7", desc: "Move window to Workspace 7 & follow", category: "Workspaces (Move)", cmd: "komorebic move-to-workspace 6" },
        Keybind { keys: "Alt + Shift + 8", desc: "Move window to Workspace 8 & follow", category: "Workspaces (Move)", cmd: "komorebic move-to-workspace 7" },
        Keybind { keys: "Alt + Shift + 9", desc: "Move window to Workspace 9 & follow", category: "Workspaces (Move)", cmd: "komorebic move-to-workspace 8" },

        // Launchers & Apps
        Keybind { keys: "Win + K", desc: "Toggle Keybindings cheatsheet (Omarchy style)", category: "Launchers", cmd: "" },
        Keybind { keys: "Alt + Return", desc: "Launch Otty terminal", category: "Launchers", cmd: "otty" },
        Keybind { keys: "Win + Return", desc: "Launch Windows Terminal (wt.exe)", category: "Launchers", cmd: "wt.exe" },
        Keybind { keys: "Alt + Shift + Return", desc: "Launch Windows Terminal (wt.exe)", category: "Launchers", cmd: "wt.exe" },
        Keybind { keys: "Alt + Z", desc: "Launch Zen Browser (quick)", category: "Launchers", cmd: "cmd /c start zen" },
        Keybind { keys: "Alt + Shift + B", desc: "Launch Zen Browser", category: "Launchers", cmd: "cmd /c start zen" },
        Keybind { keys: "Alt + O", desc: "Launch Visual Studio Code", category: "Launchers", cmd: "code" },
        Keybind { keys: "Alt + Shift + C", desc: "Launch GNU Emacs", category: "Launchers", cmd: "C:\\PROGRA~1\\Emacs\\emacs-31.1\\bin\\runemacs.exe" },
        Keybind { keys: "Alt + Shift + E", desc: "Launch File Explorer", category: "Launchers", cmd: "explorer.exe" },
        Keybind { keys: "Alt + M", desc: "Launch Kew Music Player (24px)", category: "Launchers", cmd: "cmd /c start wt.exe -p KewMusic" },
        Keybind { keys: "Alt + Shift + M", desc: "Launch CLIamp Music Player (25px)", category: "Launchers", cmd: "cmd /c start wt.exe -p CLIampMusic" },
        Keybind { keys: "Alt + PrintScreen", desc: "Launch Snipping Tool", category: "Launchers", cmd: "SnippingTool.exe" },

        // Window Resize Mode
        Keybind { keys: "Alt + R", desc: "Enter window resize mode", category: "Resize Mode", cmd: "" },
        Keybind { keys: "H / Left", desc: "[Resize Mode] Decrease width (horizontal axis)", category: "Resize Mode", cmd: "komorebic resize-axis horizontal decrease" },
        Keybind { keys: "L / Right", desc: "[Resize Mode] Increase width (horizontal axis)", category: "Resize Mode", cmd: "komorebic resize-axis horizontal increase" },
        Keybind { keys: "K / Up", desc: "[Resize Mode] Increase height (vertical axis)", category: "Resize Mode", cmd: "komorebic resize-axis vertical increase" },
        Keybind { keys: "J / Down", desc: "[Resize Mode] Decrease height (vertical axis)", category: "Resize Mode", cmd: "komorebic resize-axis vertical decrease" },
        Keybind { keys: "Shift + H/L/K/J", desc: "[Resize Mode] Micro-adjust individual edge", category: "Resize Mode", cmd: "komorebic resize-edge" },
        Keybind { keys: "Space", desc: "[Resize Mode] Reset / retile layout", category: "Resize Mode", cmd: "komorebic retile" },
        Keybind { keys: "Esc / Enter / Alt + R", desc: "[Resize Mode] Exit resize mode", category: "Resize Mode", cmd: "" },

        // System & Window Management
        Keybind { keys: "Alt + Shift + R", desc: "Reload Komorebi & GlazeWM config", category: "System & WM", cmd: "komorebic replace-configuration C:\\Users\\haris\\.config\\komorebi\\komorebi.json" },
        Keybind { keys: "Alt + Shift + W", desc: "Retile all windows in workspace", category: "System & WM", cmd: "komorebic retile" },
        Keybind { keys: "Alt + Shift + P", desc: "Toggle Komorebi tiling pause", category: "System & WM", cmd: "komorebic toggle-pause" },
    ]
}

/// Match score for query. Higher score = better match.
pub fn match_score(item: &Keybind, query: &str) -> Option<i32> {
    if query.trim().is_empty() {
        return Some(0);
    }
    let q = query.trim().to_lowercase();
    let k = item.keys.to_lowercase();
    let d = item.desc.to_lowercase();
    let c = item.category.to_lowercase();

    if k.starts_with(&q) {
        return Some(100);
    }
    if d.starts_with(&q) {
        return Some(90);
    }
    if k.contains(&q) {
        return Some(80);
    }
    if d.contains(&q) {
        return Some(70);
    }
    if c.contains(&q) {
        return Some(50);
    }

    // Subsequence matching
    let is_subseq = |src: &str| -> bool {
        let mut s_chars = src.chars();
        for qc in q.chars() {
            if !s_chars.any(|sc| sc == qc) {
                return false;
            }
        }
        true
    };

    if is_subseq(&d) {
        return Some(30);
    }
    if is_subseq(&k) {
        return Some(20);
    }

    None
}

/// Filter and sort keybindings by relevance.
#[allow(dead_code)]
pub fn filter_keybinds(all: &[Keybind], query: &str) -> Vec<usize> {
    filter_keybinds_with_category(all, query, None)
}

/// Distinct categories preserving source order.
pub fn categories(all: &[Keybind]) -> Vec<&'static str> {
    let mut list = Vec::new();
    for it in all {
        if !list.contains(&it.category) {
            list.push(it.category);
        }
    }
    list
}

/// Filter keybindings with optional category constraint.
pub fn filter_keybinds_with_category(
    all: &[Keybind],
    query: &str,
    category: Option<&str>,
) -> Vec<usize> {
    let q = query.trim();
    let mut indices: Vec<(usize, i32)> = all
        .iter()
        .enumerate()
        .filter(|(_, it)| {
            if let Some(cat) = category {
                if cat != "All" && it.category != cat {
                    return false;
                }
            }
            true
        })
        .filter_map(|(idx, item)| {
            if q.is_empty() {
                Some((idx, 0))
            } else {
                match_score(item, q).map(|score| (idx, score))
            }
        })
        .collect();

    if !q.is_empty() {
        indices.sort_by(|a, b| b.1.cmp(&a.1));
    }
    indices.into_iter().map(|(idx, _)| idx).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_returns_all() {
        let all = all_keybinds();
        assert_eq!(filter_keybinds(&all, "").len(), all.len());
    }

    #[test]
    fn query_filters_by_keys_and_desc() {
        let all = all_keybinds();
        let f1 = filter_keybinds(&all, "focus");
        assert!(!f1.is_empty());
        let f2 = filter_keybinds(&all, "alt+f");
        assert!(!f2.is_empty());
        let f3 = filter_keybinds(&all, "terminal");
        assert_eq!(f3.len(), 3);
    }

    #[test]
    fn fuzzy_subsequence_works() {
        let all = all_keybinds();
        let res = filter_keybinds(&all, "fllscrn");
        assert!(!res.is_empty());
        assert_eq!(all[res[0]].keys, "Alt + F");
    }

    #[test]
    fn category_filtering_works() {
        let all = all_keybinds();
        let nav = filter_keybinds_with_category(&all, "", Some("Navigation"));
        assert_eq!(nav.len(), 8);
        for &idx in &nav {
            assert_eq!(all[idx].category, "Navigation");
        }
        let term = filter_keybinds_with_category(&all, "terminal", Some("Launchers"));
        assert_eq!(term.len(), 3);
    }

    #[test]
    fn categories_list_is_complete() {
        let all = all_keybinds();
        let cats = categories(&all);
        assert!(cats.contains(&"Navigation"));
        assert!(cats.contains(&"Move Window"));
        assert!(cats.contains(&"Launchers"));
        assert!(cats.contains(&"Workspaces"));
    }
}
