use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Category {
    WindowManagers,
    Terminals,
    Shells,
    Navigation,
    Media,
    System,
    Editors,
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
            Category::Editors => "Editors & IDEs",
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
            description: "GPU-accelerated cross-platform terminal emulator and multiplexer",
            binaries: &["wezterm.exe"],
            scoop_cmd: Some("scoop install wezterm"),
            winget_cmd: Some("winget install wez.wezterm"),
            choco_cmd: None,
            cargo_cmd: None,
            configs: vec![ConfigMapping {
                source_rel: "wezterm",
                target_pattern: ".config/wezterm",
                is_dir: true,
            }],
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
                target_pattern: ".config/nushell",
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
                    source_rel: "shells/xonsh",
                    target_pattern: ".config/xonsh",
                    is_dir: true,
                },
                ConfigMapping {
                    source_rel: "shells/xonsh/.xonshrc",
                    target_pattern: ".xonshrc",
                    is_dir: false,
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
        PackageItem {
            id: "forgum",
            name: "Forgum Engine",
            category: Category::System,
            description: "AI-enhanced developer workflow and orchestration assistant",
            binaries: &["forgum.exe", "forgum-engine.exe"],
            scoop_cmd: None,
            winget_cmd: None,
            choco_cmd: None,
            cargo_cmd: None,
            configs: vec![ConfigMapping {
                source_rel: "forgum",
                target_pattern: ".config/forgum",
                is_dir: true,
            }],
        },

        // Editors
        PackageItem {
            id: "nvim",
            name: "Neovim",
            category: Category::Editors,
            description: "Vim-fork focused on extensibility and usability with Lua configuration",
            binaries: &["nvim.exe"],
            scoop_cmd: Some("scoop install neovim"),
            winget_cmd: Some("winget install Neovim.Neovim"),
            choco_cmd: Some("choco install neovim"),
            cargo_cmd: None,
            configs: vec![
                ConfigMapping {
                    source_rel: "nvim",
                    target_pattern: ".config/nvim",
                    is_dir: true,
                },
                ConfigMapping {
                    source_rel: "nvim",
                    target_pattern: "AppData/Local/nvim",
                    is_dir: true,
                },
            ],
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
