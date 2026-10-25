# ==============================================================================
#  NUSHELL GOD MODE - NATIVE ARCHITECTURE ORCHESTRATOR
#  Target: Absolute Zero Lag | Native Oh-My-Posh Integration
# ==============================================================================

# --- 1. FAST PATH ENVIRONMENT ---
# Move expensive checks to once-per-session and streamline paths.
if ($nu.os-info.name == "windows") {
    $env.HOME = $env.USERPROFILE
}

# Optimized Path Prepending
$env.PATH = (
    $env.PATH
    | prepend ($env.HOME | path join "scoop" "shims")
    | prepend ($env.HOME | path join ".local" "bin")
    | prepend ($env.HOME | path join ".cargo" "bin")
    | uniq
)

# Standard XDG Base
$env.XDG_CONFIG_HOME = ($env.USERPROFILE | path join ".config")
$env.XDG_CACHE_HOME  = ($env.USERPROFILE | path join ".cache")
$env.XDG_DATA_HOME   = ($env.USERPROFILE | path join ".local" "share")
$env.XDG_STATE_HOME  = ($env.USERPROFILE | path join ".local" "state")

# --- 2. CONDA INTEGRATION (ZERO-LAG) ---
# Using the KV-Store approach from nu_scripts to avoid 'conda activate' lag.
# Zero-lag conda: the module lives under modules/conda.nu and
# is sourced from config.nu top level.

# --- 3. OH-MY-POSH THEME PINNING ---
# One theme shared by pwsh, nu and xonsh: half-life, as requested.
$env.POSH_THEME = ($env.HOME | path join ".config" "oh-my-posh" "half-life.omp.json")

# --- 4. VISUALS & UTILS ---
$env.EDITOR = "notepad"
$env.BAT_THEME = "Catppuccin Mocha"
$env.EZA_ICONS_AUTO = "1"

# --- 6. ZOXIDE, CARAPACE & FZF ENVIRONMENT ---
$env.CARAPACE_BRIDGES = "zsh,fish,bash,inshellisense"
$env._ZO_ECHO = "1"
$env._ZO_RESOLVE_SYMLINKS = "1"
$env._ZO_FZF_OPTS = "--height 15 --min-height 10 --reverse --preview-window=right:40% --border"
$env.FZF_DEFAULT_OPTS = "--height 15 --min-height 10 --layout=reverse --border --info=inline"

