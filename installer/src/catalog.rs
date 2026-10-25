use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Category {
    WindowManagers,
    Terminals,
    Shells,
    Navigation,
    Media,
    System,
    Wallpapers,
}

impl Category {
    pub fn as_str(&self) -> &'static str {
        match self {
            Category::WindowManagers => "Window Managers & Tiling",
            Category::Terminals => "Terminals & Consoles",
            Category::Shells => "Shells & Prompts",
            Category::Navigation => "Navigation & Autocomplete",
            Category::Media => "Audio & Visualizers",
            Category::System => "System & Monitoring",
            Category::Wallpapers => "Wallpapers & Visuals",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigMapping {
    pub source_rel: &'static str,
    pub target_pattern: &'static str,
    pub is_dir: bool,
}

#[derive(Debug, Clone)]
pub struct PackageItem {
    #[allow(dead_code)]
    pub id: &'static str,
    pub name: &'static str,
    pub category: Category,
    pub description: &'static str,
    pub binaries: &'static [&'static str],
    pub scoop_cmd: Option<&'static str>,
    pub winget_cmd: Option<&'static str>,
    pub choco_cmd: Option<&'static str>,
    pub cargo_cmd: Option<&'static str>,
    pub configs: Vec<ConfigMapping>,
}

pub fn get_catalog() -> Vec<PackageItem> {
    vec![
        // Window Managers
        PackageItem {
            id: "glazewm",
            name: "GlazeWM",
            category: Category::WindowManagers,
            description: "Tiling window manager for Windows with i3/bspwm keybindings",
            binaries: &["glazewm.exe"],
            scoop_cmd: Some("scoop install glazewm"),
            winget_cmd: Some("winget install GlazeWM.GlazeWM"),
            choco_cmd: None,
            cargo_cmd: None,
            configs: vec![ConfigMapping {
                source_rel: "glazewm",
                target_pattern: ".glzr/glazewm",
                is_dir: true,
            }],
        },
        PackageItem {
            id: "komorebi",
            name: "Komorebi",
            category: Category::WindowManagers,
            description: "Tiling window manager with BSP/stacking modes and whkd daemon",
            binaries: &["komorebi.exe", "komorebic.exe"],
            scoop_cmd: Some("scoop install komorebi"),
            winget_cmd: Some("winget install LGUG2Z.komorebi"),
            choco_cmd: None,
            cargo_cmd: None,
            configs: vec![ConfigMapping {
                source_rel: "komorebi",
                target_pattern: ".config/komorebi",
                is_dir: true,
            }],
        },
        PackageItem {
            id: "yasb",
            name: "YASB Status Bar",
            category: Category::WindowManagers,
            description: "Yet Another Status Bar with custom Saka, Pomodoro, and Audio widgets",
            binaries: &["yasb.exe"],
            scoop_cmd: Some("scoop install yasb"),
            winget_cmd: None,
            choco_cmd: None,
            cargo_cmd: None,
            configs: vec![ConfigMapping {
                source_rel: "yasb",
                target_pattern: ".config/yasb",
                is_dir: true,
            }],
        },
        PackageItem {
            id: "hotkeys",
            name: "Hotkeys Overlay",
            category: Category::WindowManagers,
            description: "Visual on-screen cheat sheet overlay for WM shortcuts",
            binaries: &[],
            scoop_cmd: None,
            winget_cmd: None,
            choco_cmd: None,
            cargo_cmd: None,
            configs: vec![ConfigMapping {
                source_rel: "hotkeys-overlay",
                target_pattern: ".config/hotkeys-overlay",
                is_dir: true,
            }],
        },

        // Terminals
        PackageItem {
            id: "windows-terminal",
            name: "Windows Terminal",
            category: Category::Terminals,
            description: "Microsoft Windows Terminal with Catppuccin Mocha, acrylic blur, font size 15",
            binaries: &["wt.exe"],
            scoop_cmd: None,
            winget_cmd: Some("winget install Microsoft.WindowsTerminal"),
            choco_cmd: None,
            cargo_cmd: None,
            configs: vec![
                ConfigMapping {
                    source_rel: "windows-terminal/settings.json",
                    target_pattern: "AppData/Local/Packages/Microsoft.WindowsTerminal_8wekyb3d8bbwe/LocalState/settings.json",
                    is_dir: false,
                },
                ConfigMapping {
                    source_rel: "windows-terminal/settings.json",
                    target_pattern: "AppData/Local/Microsoft/Windows Terminal/settings.json",
                    is_dir: false,
                },
            ],
        },
        PackageItem {
            id: "otty",
            name: "Otty Terminal",
            category: Category::Terminals,
            description: "Modern GPU-accelerated terminal with Windows 11 acrylic frosted glass",
            binaries: &["otty.exe"],
            scoop_cmd: Some("scoop install otty"),
            winget_cmd: Some("winget install otty"),
            choco_cmd: None,
            cargo_cmd: None,
            configs: vec![ConfigMapping {
                source_rel: "otty",
                target_pattern: ".config/otty",
                is_dir: true,
            }],
        },
        PackageItem {
            id: "wezterm",
            name: "WezTerm",
            category: Category::Terminals,
            description: "GPU-accelerated terminal emulator with native Win32 acrylic backdrop and size 15 font",
            binaries: &["wezterm.exe"],
            scoop_cmd: Some("scoop install wezterm"),
            winget_cmd: Some("winget install wez.wezterm"),
            choco_cmd: None,
            cargo_cmd: None,
            configs: vec![
                ConfigMapping {
                    source_rel: "wezterm",
                    target_pattern: ".config/wezterm",
                    is_dir: true,
                },
                ConfigMapping {
                    source_rel: "wezterm/wezterm.lua",
                    target_pattern: ".wezterm.lua",
                    is_dir: false,
                },
            ],
        },
        PackageItem {
            id: "alacritty",
            name: "Alacritty",
            category: Category::Terminals,
            description: "Fast OpenGL terminal emulator styled with Catppuccin Mocha, size 15 font, and underline cursor",
            binaries: &["alacritty.exe"],
            scoop_cmd: Some("scoop install alacritty"),
            winget_cmd: Some("winget install Alacritty.Alacritty"),
            choco_cmd: Some("choco install alacritty"),
            cargo_cmd: None,
            configs: vec![
                ConfigMapping {
                    source_rel: "alacritty",
                    target_pattern: ".config/alacritty",
                    is_dir: true,
                },
                ConfigMapping {
                    source_rel: "alacritty/alacritty.toml",
                    target_pattern: "AppData/Roaming/alacritty/alacritty.toml",
                    is_dir: false,
                },
            ],
        },
        PackageItem {
            id: "rio",
            name: "Rio Terminal",
            category: Category::Terminals,
            description: "Hardware-accelerated terminal emulator focused on speed and simplicity",
            binaries: &["rio.exe"],
            scoop_cmd: Some("scoop install rio"),
            winget_cmd: None,
            choco_cmd: None,
            cargo_cmd: Some("cargo install rio --locked"),
            configs: vec![ConfigMapping {
                source_rel: "rio",
                target_pattern: ".config/rio",
                is_dir: true,
            }],
        },

        // Shells
        PackageItem {
            id: "pwsh",
            name: "PowerShell 7",
            category: Category::Shells,
            description: "Cross-platform automation and configuration tool/framework",
            binaries: &["pwsh.exe"],
            scoop_cmd: Some("scoop install pwsh"),
            winget_cmd: Some("winget install Microsoft.PowerShell"),
            choco_cmd: Some("choco install powershell-core"),
            cargo_cmd: None,
            configs: vec![ConfigMapping {
                source_rel: "shells/powershell",
                target_pattern: "Documents/PowerShell",
                is_dir: true,
            }],
        },
        PackageItem {
            id: "nushell",
            name: "Nushell",
            category: Category::Shells,
            description: "A modern shell that treats output as structured data tables",
            binaries: &["nu.exe"],
            scoop_cmd: Some("scoop install nu"),
            winget_cmd: Some("winget install Nushell.Nushell"),
            choco_cmd: None,
            cargo_cmd: Some("cargo install nu"),
            configs: vec![ConfigMapping {
                source_rel: "shells/nushell",
                target_pattern: "AppData/Roaming/nushell",
                is_dir: true,
            }],
        },
        PackageItem {
            id: "xonsh",
            name: "Xonsh",
            category: Category::Shells,
            description: "Python-powered, cross-platform, Unix-gazing shell language and command prompt",
            binaries: &["xonsh.exe"],
            scoop_cmd: Some("scoop install python; pip install xonsh[full]"),
            winget_cmd: None,
            choco_cmd: None,
            cargo_cmd: None,
            configs: vec![
                ConfigMapping {
                    source_rel: "shells/xonsh/.xonshrc",
                    target_pattern: ".xonshrc",
                    is_dir: false,
                },
                ConfigMapping {
                    source_rel: "shells/xonsh",
                    target_pattern: "AppData/Roaming/xonsh",
                    is_dir: true,
                },
            ],
        },
        PackageItem {
            id: "oh-my-posh",
            name: "Oh My Posh",
            category: Category::Shells,
            description: "Prompt theme engine for any shell with Catppuccin Mocha palette",
            binaries: &["oh-my-posh.exe"],
            scoop_cmd: Some("scoop install oh-my-posh"),
            winget_cmd: Some("winget install JanDeDobbeleer.OhMyPosh"),
            choco_cmd: None,
            cargo_cmd: None,
            configs: vec![ConfigMapping {
                source_rel: "oh-my-posh",
                target_pattern: ".config/oh-my-posh",
                is_dir: true,
            }],
        },

        // Navigation & Completion
        PackageItem {
            id: "carapace",
            name: "Carapace Completer",
            category: Category::Navigation,
            description: "Multi-shell multi-command completion generator with 60+ curated specs",
            binaries: &["carapace.exe"],
            scoop_cmd: Some("scoop install carapace-bin"),
            winget_cmd: Some("winget install rsteube.carapace-bin"),
            choco_cmd: None,
            cargo_cmd: None,
            configs: vec![
                ConfigMapping {
                    source_rel: "carapace",
                    target_pattern: ".config/carapace",
                    is_dir: true,
                },
                ConfigMapping {
                    source_rel: "carapace/specs",
                    target_pattern: "AppData/Roaming/carapace/specs",
                    is_dir: true,
                },
            ],
        },
        PackageItem {
            id: "zoxide",
            name: "Zoxide",
            category: Category::Navigation,
            description: "Smarter cd command that remembers your most visited directories",
            binaries: &["zoxide.exe"],
            scoop_cmd: Some("scoop install zoxide"),
            winget_cmd: Some("winget install ajeetdsouza.zoxide"),
            choco_cmd: Some("choco install zoxide"),
            cargo_cmd: Some("cargo install zoxide --locked"),
            configs: vec![],
        },
        PackageItem {
            id: "fzf",
            name: "FZF",
            category: Category::Navigation,
            description: "General-purpose command-line fuzzy finder",
            binaries: &["fzf.exe"],
            scoop_cmd: Some("scoop install fzf"),
            winget_cmd: Some("winget install junegunn.fzf"),
            choco_cmd: Some("choco install fzf"),
            cargo_cmd: None,
            configs: vec![],
        },
        PackageItem {
            id: "zellij",
            name: "Zellij Multiplexer",
            category: Category::Navigation,
            description: "Terminal workspace with battery-included layouts and plugins",
            binaries: &["zellij.exe"],
            scoop_cmd: Some("scoop install zellij"),
            winget_cmd: None,
            choco_cmd: None,
            cargo_cmd: Some("cargo install --locked zellij"),
            configs: vec![ConfigMapping {
                source_rel: "zellij",
                target_pattern: ".config/zellij",
                is_dir: true,
            }],
        },

        // Media & Visualizers
        PackageItem {
            id: "kew",
            name: "Kew Music Player",
            category: Category::Media,
            description: "Lightweight terminal music player with Apple Music layouts, lyrics, visualizer, and Catppuccin themes",
            binaries: &["kew.exe"],
            scoop_cmd: Some("scoop install kew"),
            winget_cmd: None,
            choco_cmd: None,
            cargo_cmd: None,
            configs: vec![
                ConfigMapping {
                    source_rel: "kew",
                    target_pattern: "AppData/Roaming/kew",
                    is_dir: true,
                },
                ConfigMapping {
                    source_rel: "kew",
                    target_pattern: ".config/kew",
                    is_dir: true,
                },
            ],
        },
        PackageItem {
            id: "cliamp",
            name: "CLIamp Music Player",
            category: Category::Media,
            description: "Terminal audio player with Winamp aesthetics, visualizer, Discord RPC, and radio streams",
            binaries: &["cliamp.exe"],
            scoop_cmd: Some("scoop install cliamp"),
            winget_cmd: None,
            choco_cmd: None,
            cargo_cmd: None,
            configs: vec![
                ConfigMapping {
                    source_rel: "cliamp",
                    target_pattern: ".config/cliamp",
                    is_dir: true,
                },
                ConfigMapping {
                    source_rel: "cliamp",
                    target_pattern: "AppData/Roaming/cliamp",
                    is_dir: true,
                },
            ],
        },
        PackageItem {
            id: "cava",
            name: "Cava Audio Visualizer",
            category: Category::Media,
            description: "Console-based Audio Visualizer for ALSA, Pulseaudio, and WASAPI",
            binaries: &["cava.exe"],
            scoop_cmd: Some("scoop install cava"),
            winget_cmd: None,
            choco_cmd: None,
            cargo_cmd: None,
            configs: vec![ConfigMapping {
                source_rel: "cava",
                target_pattern: ".config/cava",
                is_dir: true,
            }],
        },
        PackageItem {
            id: "mpd",
            name: "MPD & ncmpc Ecosystem",
            category: Category::Media,
            description: "Music Player Daemon client-server stack with WASAPI output and ncmpc curses client",
            binaries: &["mpd.exe", "mpc.exe", "ncmpc.exe"],
            scoop_cmd: Some("scoop install mpd mpc ncmpc"),
            winget_cmd: None,
            choco_cmd: None,
            cargo_cmd: None,
            configs: vec![
                ConfigMapping {
                    source_rel: "mpd",
                    target_pattern: ".config/mpd",
                    is_dir: true,
                },
                ConfigMapping {
                    source_rel: "ncmpc",
                    target_pattern: ".config/ncmpc",
                    is_dir: true,
                },
                ConfigMapping {
                    source_rel: "ncmpc/config",
                    target_pattern: ".ncmpc/config",
                    is_dir: false,
                },
            ],
        },

        // System & Monitoring
        PackageItem {
            id: "fastfetch",
            name: "Fastfetch",
            category: Category::System,
            description: "Fast system information display tool styled with Catppuccin Mocha",
            binaries: &["fastfetch.exe"],
            scoop_cmd: Some("scoop install fastfetch"),
            winget_cmd: Some("winget install Fastfetch-cli.Fastfetch"),
            choco_cmd: Some("choco install fastfetch"),
            cargo_cmd: None,
            configs: vec![ConfigMapping {
                source_rel: "fastfetch",
                target_pattern: ".config/fastfetch",
                is_dir: true,
            }],
        },

        // Wallpapers & Visuals
        PackageItem {
            id: "wallpapers",
            name: "8K Wallpaper Collection",
            category: Category::Wallpapers,
            description: "1,774 curated 8K/5K wallpapers across Distros, Apple macOS, Minimal Rice & Curated Art (HKDEVS edition)",
            binaries: &[],
            scoop_cmd: None,
            winget_cmd: None,
            choco_cmd: None,
            cargo_cmd: None,
            configs: vec![],
        },
    ]
}

pub fn resolve_target_path(pattern: &str) -> PathBuf {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    if pattern.starts_with("Documents/") {
        let onedrive_docs = home.join("OneDrive").join(pattern);
        if onedrive_docs.parent().map(|p| p.exists()).unwrap_or(false) {
            return onedrive_docs;
        }
        return home.join(pattern);
    }

    if pattern.starts_with("AppData/Roaming/") {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let rel = pattern.strip_prefix("AppData/Roaming/").unwrap_or(pattern);
            return PathBuf::from(appdata).join(rel);
        }
    }

    if pattern.starts_with("AppData/Local/") {
        if let Some(localappdata) = std::env::var_os("LOCALAPPDATA") {
            let rel = pattern.strip_prefix("AppData/Local/").unwrap_or(pattern);
            return PathBuf::from(localappdata).join(rel);
        }
    }

    home.join(pattern)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_catalog_no_duplicate_ids() {
        let catalog = get_catalog();
        let mut seen = HashSet::new();
        for item in &catalog {
            assert!(
                seen.insert(item.id),
                "Duplicate package ID found in catalog: {}",
                item.id
            );
        }
    }

    #[test]
    fn test_catalog_excludes_removed_items() {
        let catalog = get_catalog();
        for item in &catalog {
            assert_ne!(item.id, "forgum", "Forgum must not be in catalog");
            assert_ne!(item.id, "neovim", "Neovim must not be in catalog");
            assert_ne!(item.id, "nvim", "nvim must not be in catalog");
            assert_ne!(item.id, "rammap", "rammap must not be in catalog");
            assert_ne!(item.id, "configstore", "configstore must not be in catalog");
            assert_ne!(item.id, "freebuff-desktop", "freebuff-desktop must not be in catalog");
            assert_ne!(item.id, "kilo", "kilo must not be in catalog");
            assert_ne!(item.id, "opencode", "opencode must not be in catalog");
        }
    }

    #[test]
    fn test_catalog_source_paths_exist() {
        let catalog = get_catalog();
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("repo root")
            .to_path_buf();

        for item in &catalog {
            for config in &item.configs {
                let src_path = repo_root.join(config.source_rel);
                assert!(
                    src_path.exists(),
                    "Config source path does not exist for package '{}': {:?}",
                    item.id,
                    src_path
                );
            }
        }
    }

    #[test]
    fn test_resolve_target_path() {
        let appdata = resolve_target_path("AppData/Roaming/alacritty/alacritty.toml");
        assert!(appdata.to_string_lossy().contains("alacritty.toml"));

        let localappdata = resolve_target_path("AppData/Local/Packages/Microsoft.WindowsTerminal/settings.json");
        assert!(localappdata.to_string_lossy().contains("settings.json"));

        let dotconfig = resolve_target_path(".config/wezterm");
        assert!(dotconfig.to_string_lossy().contains("wezterm"));
    }
}
