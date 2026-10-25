# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: Nushell Interactive Shell Configuration
# ============================================================================
# ==============================================================================
#  NUSHELL GOD MODE - CONFIGURATION
#  Target: Precision Visuals | Zero-Lag Prompt
# ==============================================================================

# --- 1. CATPPUCCIN MOCHA COLOR SCHEME ---
let catppuccin_mocha = {
    separator: "#9399b2"
    leading_trailing_space_bg: { attr: "n" }
    header: { fg: "#a6e3a1" attr: "b" }
    empty: "#89b4fa"
    bool: {|| if $in { "#a6e3a1" } else { "#f38ba8" } }
    int: "#94e2d5"
    filesize: {|e| (if $e == 0b { "#a6adc8" } else { "#89b4fa" }) }
    duration: "#a6adc8"
    date: {|| (date now) - $in | (if $in < 1hr { "#f38ba8" } else if $in < 6hr { "#fab387" } else if $in < 1day { "#f9e2af" } else { "#a6e3a1" }) }
    range: "#a6adc8"
    float: "#94e2d5"
    string: "#cdd6f4"
    nothing: "#a6adc8"
    binary: "#a6adc8"
    cell-path: "#a6adc8"
    row_index: { fg: "#a6e3a1" attr: "b" }
    record: "#cdd6f4"
    list: "#cdd6f4"
    block: "#cdd6f4"
    hints: "#585b70"
    search_result: { fg: "#f38ba8" bg: "#1e1e2e" }
    shape_and: { fg: "#f5c2e7" attr: "b" }
    shape_binary: { fg: "#f5c2e7" attr: "b" }
    shape_block: { fg: "#89b4fa" attr: "b" }
    shape_bool: "#89dceb"
    shape_closure: { fg: "#94e2d5" attr: "b" }
    shape_custom: "#a6e3a1"
    shape_datetime: { fg: "#89dceb" attr: "b" }
    shape_directory: "#89dceb"
    shape_external: "#89dceb"
    shape_externalarg: { fg: "#a6e3a1" attr: "b" }
    shape_filepath: "#89dceb"
    shape_flag: { fg: "#89b4fa" attr: "b" }
    shape_float: { fg: "#f5c2e7" attr: "b" }
    shape_garbage: { fg: "#11111b" bg: "#f38ba8" attr: "b" }
    shape_globpattern: { fg: "#89dceb" attr: "b" }
    shape_int: { fg: "#f5c2e7" attr: "b" }
    shape_internalcall: { fg: "#89dceb" attr: "b" }
    shape_list: { fg: "#89dceb" attr: "b" }
    shape_literal: "#89b4fa"
    shape_match_pattern: "#a6e3a1"
    shape_matching_brackets: { attr: "u" }
    shape_nothing: "#89dceb"
    shape_operator: "#f9e2af"
    shape_or: { fg: "#f5c2e7" attr: "b" }
    shape_pipe: { fg: "#f5c2e7" attr: "b" }
    shape_range: { fg: "#f9e2af" attr: "b" }
    shape_record: { fg: "#89dceb" attr: "b" }
    shape_redirection: { fg: "#f5c2e7" attr: "b" }
    shape_signature: { fg: "#a6e3a1" attr: "b" }
    shape_string: "#a6e3a1"
    shape_string_interpolation: { fg: "#89dceb" attr: "b" }
    shape_table: { fg: "#89b4fa" attr: "b" }
    shape_variable: "#f5c2e7"
    shape_vardecl: "#f5c2e7"
}

# --- 2. OH-MY-POSH NATIVE INTEGRATION ---
# Using the "Instant Prompt" cached method for zero-lag startup.
# NOTE: `source` requires parse-time constant paths in Nushell.
const cfg_dir = ($nu.default-config-dir)
const cache_dir = ($nu.home-dir | path join ".cache")
const omp_init_file = ($cache_dir | path join "oh-my-posh.nu")
const zoxide_init_file = ($cache_dir | path join "zoxide-init.nu")
const carapace_file = ($cfg_dir | path join "carapace.nu")
const fzf_file = ($cfg_dir | path join "modules" "fzf.nu")
const conda_file = ($cfg_dir | path join "modules" "conda.nu")
const additions_file = ($cfg_dir | path join "config-additions.nu")
let omp_config = $env.POSH_THEME

# Regenerate the init script if missing OR if the theme file changed since caching.
let omp_needs_refresh = if ($omp_init_file | path exists) and ($omp_config | path exists) {
    ((ls $omp_config | get 0 modified) > (ls $omp_init_file | get 0 modified))
} else {
    not ($omp_init_file | path exists)
}
if $omp_needs_refresh {
    ^oh-my-posh init nu --config $omp_config --print | save -f $omp_init_file
}

if not ($zoxide_init_file | path exists) {
    let z_init = (^zoxide init nushell --cmd cd)
    $"($z_init)\nexport alias z = __zoxide_z\nexport alias zi = __zoxide_zi\n" | save -f $zoxide_init_file
}

# Source the cached init script for instantaneous prompt rendering
source $omp_init_file

# --- 3. CORE SHELL CONFIGURATION ---
$env.config = {
    show_banner: false
    edit_mode: vi
    cursor_shape: {
        emacs: line
        vi_insert: line
        vi_normal: block
    }
    color_config: $catppuccin_mocha
    highlight_resolved_externals: true
    use_ansi_coloring: true

    # History: SQLite, no per-Enter fsync (sync_on_enter=false is the fast path).
    history: {
        file_format: "sqlite"
        max_size: 100_000
        sync_on_enter: false
        isolation: true
    }

    buffer_editor: "notepad"
    footer_mode: "auto"

    # Optimized Table Rendering
    table: {
        mode: rounded
        index_mode: always
        show_empty: true
        padding: { left: 1, right: 1 }
    }

    # High-Performance Completions
    completions: {
        algorithm: "fuzzy"
        case_sensitive: false
        quick: true
        partial: true
        external: {
            enable: true
            max_results: 20
            completer: {|buffer|
                let parts = ($buffer | split row " ")
                carapace-complete ($parts | first) ...($parts | rest)
            }
        }
    }

    # FZF Interactive Autocompletion, History Search & Navigation Keybindings
    keybindings: [
        {
            name: fzf_history
            modifier: control
            keycode: char_r
            mode: [emacs, vi_normal, vi_insert]
            event: {
                send: ExecuteHostCommand
                cmd: "fzf-history-widget"
            }
        }
        {
            name: fzf_file
            modifier: control
            keycode: char_t
            mode: [emacs, vi_normal, vi_insert]
            event: {
                send: ExecuteHostCommand
                cmd: "fzf-file-widget"
            }
        }
        {
            name: fzf_cd
            modifier: alt
            keycode: char_c
            mode: [emacs, vi_normal, vi_insert]
            event: {
                send: ExecuteHostCommand
                cmd: "fzf-cd-widget"
            }
        }
        {
            name: fzf_complete
            modifier: control
            keycode: space
            mode: [emacs, vi_normal, vi_insert]
            event: {
                send: ExecuteHostCommand
                cmd: "fzf-complete-widget"
            }
        }
    ]
}

# --- 4. ZOXIDE & CARAPACE & FZF ---
# NOTE: source at top level — defs inside try/if blocks are block-scoped and lost.
source $zoxide_init_file
source $carapace_file
source $fzf_file

# --- 5. EXTRA INTEGRATIONS ---
# Zero-lag conda wrapper + completions and clipboard helpers.
source $conda_file
source $additions_file

# --- 6. MUSIC — mpd under ~/Music + the ncmpc TUI (same `music` as pwsh/xonsh) ---
def mpd-up [] {
    ((mpc status | complete).exit_code == 0)
}

def --env music [...args: string] {
    let music_dir = ("~/Music" | path expand)

    if not (mpd-up) {
        # mpd has no daemon mode on Windows; start it detached in a hidden window.
        try {
            ^pwsh -NoLogo -NoProfile -Command $"start-process mpd -WorkingDirectory '($music_dir)' -WindowStyle hidden"
        } catch {
            print "failed to launch mpd"
        }
        for _ in 0..14 {
            sleep 300ms
            if (mpd-up) { break }
        }
    }

    if not (mpd-up) {
        print "mpd is not responding on 127.0.0.1:6600"
        return
    }

    let prev = $env.PWD
    cd $music_dir
    try {
        ^ncmpc ...$args
    } catch {
        null  # ncmpc's own stderr already showed; never strand the user in ~/Music
    }
    cd $prev
}

# --- 7. WINDOWS RICE INTERACTIVE TUI & MANAGER ---
def --env window-rice [
    --reload (-r)                # Reload Windows Rice environment (Komorebi, GlazeWM, YASB)
    --verify (-v)                # Run verification check
    --update (-c)                # Check updates from git origin (-c or -u)
    --check-updates (-u)         # Check updates from git origin
    --link-all                   # Link all configs
    --unlink (-x)                # Unlink configs cleanly and restore backups
    --unlink-all                 # Unlink all configs cleanly without trace and restore backups
    --install-missing            # Install missing packages
    --wallpapers (-w)            # Install 8K wallpapers
    --uninstall-wallpapers       # Uninstall 8K wallpapers
    --bootstrap (-b)             # Bootstrap Scoop, Choco, UV, Vcpkg
    ...args: string              # Additional arguments
] {
    let installer = ($nu.home-dir | path join ".config" "installer" "target" "release" "rice-installer.exe")
    let cargo_toml = ($nu.home-dir | path join ".config" "installer" "Cargo.toml")
    let install_ps1 = ($nu.home-dir | path join ".config" "install.ps1")

    if $reload {
        print "Reloading Windows Rice Environment..."
        try { ^komorebic replace-configuration ($nu.home-dir | path join ".config" "komorebi" "komorebi.json") }
        try { ^glazewm command wm-reload-config }
        let yasb_setup = ($nu.home-dir | path join ".config" "yasb" "tools" "setup" "yasb-setup.ps1")
        if ($yasb_setup | path exists) {
            ^pwsh -NoProfile -ExecutionPolicy Bypass -File $yasb_setup
        }
        print "Windows Rice environment reloaded!"
        return
    }

    if not ($installer | path exists) and ($cargo_toml | path exists) and (which cargo | is-not-empty) {
        print "Building Windows Rice TUI..."
        ^cargo build --release --manifest-path $cargo_toml --quiet
    }

    mut pass_args = []
    if $verify { $pass_args = ($pass_args | append "--verify") }
    if $update or $check_updates { $pass_args = ($pass_args | append "--update") }
    if $link_all { $pass_args = ($pass_args | append "--link-all") }
    if $unlink { $pass_args = ($pass_args | append "--unlink") }
    if $unlink_all { $pass_args = ($pass_args | append "--unlink-all") }
    if $install_missing { $pass_args = ($pass_args | append "--install-missing") }
    if $wallpapers { $pass_args = ($pass_args | append "--wallpapers") }
    if $uninstall_wallpapers { $pass_args = ($pass_args | append "--uninstall-wallpapers") }
    if $bootstrap { $pass_args = ($pass_args | append "--bootstrap") }
    if ($args | is-not-empty) { $pass_args = ($pass_args | append $args) }

    if ($installer | path exists) {
        ^$installer ...$pass_args
    } else if ($install_ps1 | path exists) {
        ^pwsh -NoProfile -File $install_ps1 ...$pass_args
    } else {
        print "rice-installer not found."
    }
}
alias rice = window-rice
