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
        Keybind { keys: "Alt + ←/→/↑/↓", desc: "Focus window directionally (arrow keys)", category: "Navigation", cmd: "" },

        // Move Window
        Keybind { keys: "Alt + Shift + H", desc: "Move window to the left", category: "Move Window", cmd: "komorebic move left" },
        Keybind { keys: "Alt + Shift + L", desc: "Move window to the right", category: "Move Window", cmd: "komorebic move right" },
        Keybind { keys: "Alt + Shift + K", desc: "Move window up", category: "Move Window", cmd: "komorebic move up" },
        Keybind { keys: "Alt + Shift + J", desc: "Move window down", category: "Move Window", cmd: "komorebic move down" },
        Keybind { keys: "Alt + Shift + ←/→/↑/↓", desc: "Move window directionally (arrow keys)", category: "Move Window", cmd: "" },

        // Multi-Monitor
        Keybind { keys: "Alt + ,", desc: "Focus previous monitor", category: "Multi-Monitor", cmd: "komorebic cycle-monitor previous" },
        Keybind { keys: "Alt + .", desc: "Focus next monitor", category: "Multi-Monitor", cmd: "komorebic cycle-monitor next" },
        Keybind { keys: "Alt + Shift + ,", desc: "Move window to previous monitor", category: "Multi-Monitor", cmd: "komorebic cycle-move-to-monitor previous" },
        Keybind { keys: "Alt + Shift + .", desc: "Move window to next monitor", category: "Multi-Monitor", cmd: "komorebic cycle-move-to-monitor next" },

        // Window State
        Keybind { keys: "Alt + F", desc: "Toggle window fullscreen / maximize", category: "Window State", cmd: "komorebic toggle-maximize" },
        Keybind { keys: "Alt + Shift + F", desc: "Toggle monocle mode", category: "Window State", cmd: "komorebic toggle-monocle" },
        Keybind { keys: "Alt + T", desc: "Toggle floating / tiling mode", category: "Window State", cmd: "komorebic toggle-tiling" },
        Keybind { keys: "Alt + Q", desc: "Close active window", category: "Window State", cmd: "komorebic close" },

        // Workspaces
        Keybind { keys: "Alt + 1 .. 9", desc: "Switch to workspace 1 to 9", category: "Workspaces", cmd: "" },
        Keybind { keys: "Alt + Shift + 1 .. 9", desc: "Move window to workspace 1 to 9 & follow", category: "Workspaces", cmd: "" },

        // Launchers & Apps
        Keybind { keys: "Win + K", desc: "Open Keybindings cheatsheet (Omarchy style)", category: "Launchers", cmd: "" },
        Keybind { keys: "Alt + Return", desc: "Launch Otty terminal", category: "Launchers", cmd: "otty" },
        Keybind { keys: "Alt + Shift + Return", desc: "Launch Windows Terminal (wt.exe)", category: "Launchers", cmd: "wt.exe" },
        Keybind { keys: "Alt + Shift + B", desc: "Launch Zen Browser", category: "Launchers", cmd: "cmd /c start zen" },
        Keybind { keys: "Alt + Z", desc: "Launch Zen Browser (quick)", category: "Launchers", cmd: "cmd /c start zen" },
        Keybind { keys: "Alt + O", desc: "Launch Visual Studio Code", category: "Launchers", cmd: "code" },
        Keybind { keys: "Alt + Shift + C", desc: "Launch GNU Emacs", category: "Launchers", cmd: "C:\\PROGRA~1\\Emacs\\emacs-31.1\\bin\\runemacs.exe" },
        Keybind { keys: "Alt + Shift + E", desc: "Launch File Explorer", category: "Launchers", cmd: "explorer.exe" },
        Keybind { keys: "Alt + M", desc: "Launch Kew Music Player (24px)", category: "Launchers", cmd: "cmd /c start wt.exe -p KewMusic" },
        Keybind { keys: "Alt + Shift + M", desc: "Launch CLIamp Music Player (25px)", category: "Launchers", cmd: "cmd /c start wt.exe -p CLIampMusic" },
        Keybind { keys: "Alt + PrintScreen", desc: "Launch Snipping Tool", category: "Launchers", cmd: "SnippingTool.exe" },

        // System & Window Management
        Keybind { keys: "Alt + R", desc: "Enter window resize mode (h/j/k/l, Esc to exit)", category: "System & WM", cmd: "" },
        Keybind { keys: "Alt + Shift + R", desc: "Reload Komorebi & GlazeWM config", category: "System & WM", cmd: "komorebic replace-configuration C:\\Users\\haris\\.config\\komorebi\\komorebi.json" },
        Keybind { keys: "Alt + Shift + W", desc: "Retile all windows in current workspace", category: "System & WM", cmd: "komorebic retile" },
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
pub fn filter_keybinds(all: &[Keybind], query: &str) -> Vec<usize> {
    if query.trim().is_empty() {
        return (0..all.len()).collect();
    }
    let mut matches: Vec<(usize, i32)> = all
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| match_score(item, query).map(|s| (idx, s)))
        .collect();

    // Stable sort by score descending
    matches.sort_by(|a, b| b.1.cmp(&a.1));
    matches.into_iter().map(|(idx, _)| idx).collect()
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
        assert_eq!(f3.len(), 2);
    }

    #[test]
    fn fuzzy_subsequence_works() {
        let all = all_keybinds();
        let res = filter_keybinds(&all, "fllscrn");
        assert!(!res.is_empty());
        assert_eq!(all[res[0]].keys, "Alt + F");
    }
}
