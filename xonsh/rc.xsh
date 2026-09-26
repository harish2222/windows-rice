#!/usr/bin/env xonsh
# ==============================================================================
#  █████╗  ██████╗ ███████╗██╗  ██╗ ██████╗ ███████╗
# ██╔══██╗██╔═══██╗██╔════╝╚██╗██╔╝██╔═══██╗██╔════╝
# ███████║██║   ██║███████╗ ╚███╔╝ ██║   ██║███████╗
# ██╔══██║██║   ██║╚════██║ ██╔██╗ ██║   ██║╚════██║
# ██║  ██║╚██████╔╝███████║██╔╝ ██╗╚██████╔╝███████║
# ╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚══════╝
# ==============================================================================
# .xonshrc — Xonsh Shell Configuration
# Author: @HKDEVS | Organization: @HKDevloops
# Font: CaskaydiaCove Nerd Font (primary), JetBrains Mono Nerd Font (secondary)
# Platform: Windows (Scoop) | Shell: Xonsh
# Version: 1.0.0 | License: MIT
# ==============================================================================
# This config uses pure Python for all logic, with xonsh syntax only inside
# try/except blocks for aliases and shell-specific settings.
# ==============================================================================

# ==============================================================================
# SECTION 1: EARLY IMPORTS
# ==============================================================================
import os
import sys
import shutil
import subprocess
import pathlib
from pathlib import Path
from typing import Optional, List, Dict, Any, Union

# ==============================================================================
# SECTION 2: ENVIRONMENT SETUP
# ==============================================================================

# --- Helper: resolve home directory ---
_HOME = str(Path.home())
_USER = os.environ.get("USER", os.environ.get("USERNAME", "user"))
_IS_WINDOWS = sys.platform == "win32"
_IS_MSYS = "MSYSTEM" in os.environ
_IS_WSL = "WSL_DISTRO_NAME" in os.environ

# --- PATH setup ---
def _build_path() -> str:
    """Build the complete PATH string with all custom directories."""
    paths = []

    # User local bin
    paths.append(str(Path(_HOME) / ".local" / "bin"))

    # Scoop shims (Windows)
    if _IS_WINDOWS:
        scoop_shims = str(Path(_HOME) / "scoop" / "shims")
        scoop_apps = str(Path(_HOME) / "scoop" / "apps" / "bin")
        scoop_carapace = str(Path(_HOME) / "scoop" / "apps" / "carapace" / "current")
        paths.extend([scoop_shims, scoop_apps, scoop_carapace])

    # Cargo / Rust
    paths.append(str(Path(_HOME) / ".cargo" / "bin"))

    # Go
    go_path = os.environ.get("GOPATH", str(Path(_HOME) / "go"))
    paths.append(str(Path(go_path) / "bin"))

    # Pipx
    paths.append(str(Path(_HOME) / ".local" / "pipx" / "bin"))

    # PlatformIO
    pio_core = os.environ.get("PLATFORMIO_CORE_DIR", str(Path(_HOME) / ".platformio"))
    paths.append(str(Path(pio_core) / "penv" / "bin"))

    # Node modules
    paths.append(str(Path.cwd() / "node_modules" / ".bin"))

    # Python user scripts
    paths.append(str(Path(_HOME) / ".local" / "share" / "python" / "scripts"))

    # Nix profile (if available)
    paths.append(str(Path(_HOME) / ".nix-profile" / "bin"))
    paths.append("/nix/var/nix/profiles/default/bin")

    # Existing PATH
    existing = os.environ.get("PATH", "")
    paths.append(existing)

    # Join and deduplicate while preserving order
    seen = set()
    unique = []
    for p in paths:
        if p and p not in seen:
            seen.add(p)
            unique.append(p)
    return os.pathsep.join(unique)

$PATH = _build_path()

# --- XDG Directories ---
$XDG_CONFIG_HOME = os.environ.get("XDG_CONFIG_HOME", str(Path(_HOME) / ".config"))
$XDG_CACHE_HOME = os.environ.get("XDG_CACHE_HOME", str(Path(_HOME) / ".cache"))
$XDG_DATA_HOME = os.environ.get("XDG_DATA_HOME", str(Path(_HOME) / ".local" / "share"))
$XDG_STATE_HOME = os.environ.get("XDG_STATE_HOME", str(Path(_HOME) / ".local" / "state"))
$XDG_BIN_HOME = os.environ.get("XDG_BIN_HOME", str(Path(_HOME) / ".local" / "bin"))

# Ensure XDG directories exist
for _xdg_dir in [$XDG_CONFIG_HOME, $XDG_CACHE_HOME, $XDG_DATA_HOME, $XDG_STATE_HOME, $XDG_BIN_HOME]:
    try:
        Path(_xdg_dir).mkdir(parents=True, exist_ok=True)
    except Exception:
        pass

# --- Terminal Detection ---
def _detect_terminal() -> dict:
    """Detect terminal emulator and capabilities."""
    info = {
        "term": os.environ.get("TERM", "unknown"),
        "term_program": os.environ.get("TERM_PROGRAM", ""),
        "colorterm": os.environ.get("COLORTERM", ""),
        "truecolor": False,
        "unicode": True,
        "name": "unknown",
    }

    # Detect specific terminals
    tp = info["term_program"].lower()
    term = info["term"].lower()

    if "rio" in tp or "rio" in term:
        info["name"] = "Rio"
        info["truecolor"] = True
        info["unicode"] = True
    elif "windowsterminal" in tp or "wt" in term or "WT_SESSION" in os.environ:
        info["name"] = "Windows Terminal"
        info["truecolor"] = True
        info["unicode"] = True
    elif "wezterm" in tp:
        info["name"] = "WezTerm"
        info["truecolor"] = True
        info["unicode"] = True
    elif "alacritty" in tp or "alacritty" in term:
        info["name"] = "Alacritty"
        info["truecolor"] = True
        info["unicode"] = True
    elif "kitty" in tp:
        info["name"] = "Kitty"
        info["truecolor"] = True
        info["unicode"] = True
    elif "ghostty" in tp:
        info["name"] = "Ghostty"
        info["truecolor"] = True
        info["unicode"] = True
    elif "konsole" in tp:
        info["name"] = "Konsole"
        info["truecolor"] = True
    elif "iterm" in tp:
        info["name"] = "iTerm2"
        info["truecolor"] = True
    elif info["colorterm"] in ("truecolor", "24bit"):
        info["truecolor"] = True

    return info

_TERMINAL = _detect_terminal()

# Set terminal environment variables
if _TERMINAL["truecolor"]:
    $COLORTERM = "truecolor"
$TERM_PROGRAM = _TERMINAL["term_program"] or _TERMINAL["name"]

# --- Editor Resolution ---
def _resolve_editor() -> str:
    """Resolve the best available editor."""
    editors = ["nvim", "helix", "micro", "code", "nano", "vi"]
    for ed in editors:
        if shutil.which(ed):
            return ed
    return "nano"

$EDITOR = os.environ.get("EDITOR", _resolve_editor())
$VISUAL = $EDITOR

# If nvim is available, set it as MANPAGER too
if shutil.which("nvim"):
    $MANPAGER = "nvim +Man!"
    $MANWIDTH = "999"
elif shutil.which("bat"):
    $MANPAGER = "sh -c 'col -bx | bat -l man -p'"

# --- Zoxide Config ---
$ZOXIDE_NO_MSG = "1"
$_ZO_ECHO = "1"  # xonsh needs echo mode for cd integration

# --- Carapace Config ---
$CARAPACE_BRACKETS = "1"
$CARAPACE_BRIDGES = "zsh,fish,bash,inshellisense"

# --- FZF Config with Catppuccin Mocha Colors ---
$FZF_DEFAULT_OPTS = (
    " --height=40%"
    " --layout=reverse"
    " --border=rounded"
    " --margin=1"
    " --padding=1"
    " --info=inline"
    " --prompt='  '"
    " --pointer='>'"
    " --marker='+'"
    " --header-first"
    " --multi"
    " --cycle"
    " --color=bg+:#363a4f,bg:#1e1e2e,spinner:#f5e0dc,hl:#f38ba8"
    " --color=fg:#cdd6f4,header:#f38ba8,info:#cba6f7,pointer:#f5e0dc"
    " --color=marker:#b4befe,fg+:#cdd6f4,prompt:#cba6f7,hl+:#f38ba8"
    " --color=selected-bg:#45475a,border:#89b4fa,gutter:#1e1e2e"
)

$FZF_CTRL_T_OPTS = (
    " --preview='bat -n --color=always {} 2>/dev/null || cat {}'"
    " --preview-window=right,60%,wrap,follow"
    " --bind='ctrl-/:toggle-preview'"
    " --bind='ctrl-y:execute-silent(echo {} | clip.exe)+abort'"
)

$FZF_ALT_C_OPTS = (
    " --preview='eza --tree --level=2 --icons=always --color=always {} 2>/dev/null || ls {}'"
    " --preview-window=right,50%,wrap,follow"
)

$FZF_CTRL_R_OPTS = (
    " --preview='echo {}'"
    " --preview-window=down,3,wrap,follow"
    " --bind='ctrl-y:execute-silent(echo {q} | clip.exe)+abort'"
)

# Use fd as the default finder for fzf
if shutil.which("fd"):
    $FZF_DEFAULT_COMMAND = "fd --type f --hidden --follow --exclude .git --exclude node_modules --exclude .venv"
    $FZF_CTRL_T_COMMAND = "fd --type f --hidden --follow --exclude .git --exclude node_modules"
    $FZF_ALT_C_COMMAND = "fd --type d --hidden --follow --exclude .git --exclude node_modules"

# --- Bat Config (Catppuccin Mocha) ---
$BAT_THEME = "Catppuccin Mocha"
$BAT_STYLE = "full"
$BAT_PAGER = "less -RF"

# --- Eza Config ---
$EZA_ICONS_OPTION = "fancy"
$EZA_COLORS_OPTION = "gi=1:da=1:sn=32:sb=32:uu=38;5;220:un=38;5;220:gu=38;5;220:gn=38;5;220:df=38;5;40:ds=38;5;40:ln=35:lp=2:lp=35"

# --- Oh-My-Posh Theme Config ---
$POSH_THEME = str(Path(_HOME) / "Documents" / "PowerShell" / "lambda_new.omp.json")
$SHELL_THEME = os.environ.get("SHELL_THEME", "catppuccin")

# --- Dev Environment Variables ---
# Python
$PYTHONSTARTUP = str(Path(_HOME) / ".config" / "python" / "startup.py")
$PYTHONIOENCODING = "utf-8"
$PYTHONDONTWRITEBYTECODE = "1"
$PYTHONUNBUFFERED = "1"
$PIP_NO_INPUT = "1"
$PIP_DISABLE_PIP_VERSION_CHECK = "1"

# Rust
$RUST_SRC_PATH = str(Path(_HOME) / ".rustup" / "toolchains" / "stable-x86_64-pc-windows-msvc" / "lib" / "rustlib" / "src" / "rust" / "library")
$CARGO_HOME = os.environ.get("CARGO_HOME", str(Path(_HOME) / ".cargo"))
$RUSTUP_HOME = os.environ.get("RUSTUP_HOME", str(Path(_HOME) / ".rustup"))

# Go
$GOPATH = os.environ.get("GOPATH", str(Path(_HOME) / "go"))
$GOBIN = str(Path(str($GOPATH)) / "bin")

# Java
_java_home_candidates = [
    str(Path(_HOME) / "scoop" / "apps" / "openjdk" / "current"),
    str(Path(_HOME) / "scoop" / "apps" / "temurin-jdk" / "current"),
    os.environ.get("JAVA_HOME", ""),
]
for _jhc in _java_home_candidates:
    if _jhc and Path(_jhc).exists():
        $JAVA_HOME = _jhc
        break

# PlatformIO
$PLATFORMIO_CORE_DIR = os.environ.get("PLATFORMIO_CORE_DIR", str(Path(_HOME) / ".platformio"))

# Node
$NODE_OPTIONS = "--max-old-space-size=4096"
$NPM_CONFIG_PREFIX = str(Path(_HOME) / ".local")

# --- Windows-Specific Paths ---
if _IS_WINDOWS:
    $SCOOP = os.environ.get("SCOOP", str(Path(_HOME) / "scoop"))
    $SCOOP_GLOBAL = os.environ.get("SCOOP_GLOBAL", "C:\\ProgramData\\scoop")
    $APPDATA = os.environ.get("APPDATA", str(Path(_HOME) / "AppData" / "Roaming"))
    $LOCALAPPDATA = os.environ.get("LOCALAPPDATA", str(Path(_HOME) / "AppData" / "Local"))
    # Windows Terminal settings
    $WT_SESSION = os.environ.get("WT_SESSION", "")
    # Enable virtual terminal processing for colors
    $TERM = "xterm-256color"

# --- Nix (if available) ---
if Path("/nix/var/nix/profiles/default").exists():
    $NIX_PROFILES = "/nix/var/nix/profiles/default " + str(Path(_HOME) / ".nix-profile")

# --- Misc ---
$LANG = "en_US.UTF-8"
$LC_ALL = "en_US.UTF-8"
$LESS = "-RF -i -j.3 -K -M"
$LESSHISTFILE = "-"
$PAGER = "less"
$READNULLCMD = "bat --paging=always --style=full"
$TIME_STYLE = "long-iso"


# ==============================================================================
# SECTION 3: UTILITY FUNCTIONS (40+)
# ==============================================================================

def test_command(cmd: str) -> bool:
    """Test if a command is available in PATH."""
    return shutil.which(cmd) is not None

def touch(path: str) -> None:
    """Create an empty file or update its modification time."""
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.touch(exist_ok=True)

def mkcd(path: str) -> None:
    """Create a directory and cd into it."""
    p = Path(path)
    p.mkdir(parents=True, exist_ok=True)
    os.chdir(str(p))

def ff(pattern: str = ".", **kwargs) -> None:
    """Find files using fd (fast find)."""
    if test_command("fd"):
        args = ["fd", pattern]
        for k, v in kwargs.items():
            args.extend([f"--{k}", str(v)])
        subprocess.run(args)
    else:
        for f in Path(".").rglob(pattern):
            print(f)

def pubip() -> None:
    """Get public IP address."""
    try:
        if test_command("xh"):
            result = subprocess.run(["xh", "--body", "https://ifconfig.me"], capture_output=True, text=True)
        elif test_command("curl"):
            result = subprocess.run(["curl", "-s", "https://ifconfig.me"], capture_output=True, text=True)
        else:
            result = subprocess.run(["wget", "-qO-", "https://ifconfig.me"], capture_output=True, text=True)
        print(result.stdout.strip())
    except Exception as e:
        print(f"Error getting public IP: {e}")

def admin(*args) -> None:
    """Run command with administrator privileges."""
    if _IS_WINDOWS:
        if args:
            subprocess.run(["powershell", "-Command", f"Start-Process {' '.join(args)} -Verb RunAs"])
        else:
            subprocess.run(["powershell", "-Command", "Start-Process powershell -Verb RunAs"])
    else:
        if test_command("sudo"):
            cmd = ["sudo"] + list(args) if args else ["sudo", "su"]
            subprocess.run(cmd)
        else:
            print("No sudo available")

def uptime_cmd() -> None:
    """Show system uptime."""
    if _IS_WINDOWS:
        subprocess.run(["powershell", "-Command", "(Get-CimInstance Win32_OperatingSystem).LastBootUpTime"])
    elif test_command("uptime"):
        subprocess.run(["uptime"])
    else:
        print("uptime command not available")

def unzip_cmd(archive: str, dest: str = ".") -> None:
    """Extract archive (zip, tar.gz, tar.bz2, etc.)."""
    if test_command("ouch"):
        subprocess.run(["ouch", "decompress", archive, "--dir", dest])
    else:
        a = archive.lower()
        if a.endswith(".zip"):
            shutil.unpack_archive(archive, dest)
        elif a.endswith((".tar.gz", ".tgz")):
            subprocess.run(["tar", "xzf", archive, "-C", dest])
        elif a.endswith((".tar.bz2", ".tbz2")):
            subprocess.run(["tar", "xjf", archive, "-C", dest])
        elif a.endswith(".tar.xz"):
            subprocess.run(["tar", "xJf", archive, "-C", dest])
        else:
            print(f"Unsupported archive format: {archive}")

def grep(pattern: str, path: str = ".", *args) -> None:
    """Search using ripgrep."""
    if test_command("rg"):
        subprocess.run(["rg", pattern, path] + list(args))
    else:
        subprocess.run(["grep", "-rn", pattern, path] + list(args))

def diskfree() -> None:
    """Show disk usage."""
    if test_command("dust"):
        subprocess.run(["dust"])
    else:
        if _IS_WINDOWS:
            subprocess.run(["powershell", "-Command", "Get-PSDrive -PSProvider FileSystem | Format-Table Name,Used,Free,Root"])
        else:
            subprocess.run(["df", "-h"])

def sed_replace(pattern: str, replacement: str, *files) -> None:
    """Replace text in files using sd (fast sed)."""
    if test_command("sd"):
        subprocess.run(["sd", pattern, replacement] + list(files))
    else:
        for f in files:
            content = Path(f).read_text()
            import re
            new_content = re.sub(pattern, replacement, content)
            Path(f).write_text(new_content)

def which_cmd(cmd: str) -> None:
    """Find the location of a command."""
    result = shutil.which(cmd)
    if result:
        print(result)
    else:
        print(f"{cmd} not found")

def export_env(key: str, value: str) -> None:
    """Export an environment variable."""
    os.environ[key] = value
    print(f"Exported {key}={value}")

def pkill_cmd(pattern: str) -> None:
    """Kill processes matching pattern."""
    if test_command("procs"):
        subprocess.run(["procs", "--no-header", pattern], capture_output=True)
    if _IS_WINDOWS:
        subprocess.run(["powershell", "-Command", f"Get-Process -Name '*{pattern}*' | Stop-Process -Force"])
    else:
        subprocess.run(["pkill", "-f", pattern])

def pgrep_cmd(pattern: str) -> None:
    """Find processes matching pattern."""
    if test_command("procs"):
        subprocess.run(["procs", pattern])
    else:
        subprocess.run(["pgrep", "-fla", pattern])

def head_cmd(file: str, n: int = 10) -> None:
    """Show first N lines of a file."""
    if test_command("bat"):
        subprocess.run(["bat", "--header", f":{n}", "--line-range", f"0:{n}", file])
    else:
        with open(file) as f:
            for i, line in enumerate(f):
                if i >= n:
                    break
                print(line, end="")

def tail_cmd(file: str, n: int = 10) -> None:
    """Show last N lines of a file."""
    subprocess.run(["tail", "-n", str(n), file])

def nf() -> None:
    """Run neofetch/fastfetch."""
    if test_command("fastfetch"):
        subprocess.run(["fastfetch"])
    elif test_command("neofetch"):
        subprocess.run(["neofetch"])
    else:
        print(f"System: {os.uname() if hasattr(os, 'uname') else 'N/A'}")

def trash(path: str) -> None:
    """Move file/directory to trash instead of deleting."""
    if test_command("trash-cli"):
        subprocess.run(["trash-cli", "put", path])
    elif test_command("trash-put"):
        subprocess.run(["trash-put", path])
    elif _IS_WINDOWS:
        subprocess.run(["powershell", "-Command", f"Add-Type -AssemblyName Microsoft.VisualBasic; [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteFile('{Path(path).resolve()}', 'OnlyErrorDialogs', 'SendToRecycleBin')"])
    else:
        # Fallback: move to ~/.local/share/Trash/files
        trash_dir = Path(_HOME) / ".local" / "share" / "Trash" / "files"
        trash_dir.mkdir(parents=True, exist_ok=True)
        shutil.move(path, str(trash_dir / Path(path).name))

def docs(cmd: str) -> None:
    """Read documentation for a command."""
    if test_command("glow"):
        # Try to find and render markdown docs
        md_paths = [
            str(Path(_HOME) / ".config" / f"{cmd}" / "README.md"),
            str(Path(_HOME) / ".local" / "share" / "doc" / f"{cmd}" / "README.md"),
        ]
        for mp in md_paths:
            if Path(mp).exists():
                subprocess.run(["glow", mp])
                return
    # Fallback to man or tldr
    if test_command("tealdeer"):
        subprocess.run(["tldr", cmd])
    else:
        subprocess.run(["man", cmd])

def dtop() -> None:
    """Open lazydocker (Docker TUI)."""
    if test_command("lazydocker"):
        subprocess.run(["lazydocker"])
    elif test_command("docker"):
        subprocess.run(["docker", "ps"])
    else:
        print("Docker not available")

def k9() -> None:
    """Open k9s (Kubernetes TUI)."""
    if test_command("k9s"):
        subprocess.run(["k9s"])
    elif test_command("kubectl"):
        subprocess.run(["kubectl", "get", "pods", "-A"])
    else:
        print("Kubernetes tools not available")

def la(path: str = ".") -> None:
    """List all files (including hidden) with details."""
    if test_command("eza"):
        subprocess.run(["eza", "--all", "--long", "--header", "--git", "--icons=always", path])
    else:
        subprocess.run(["ls", "-la", path])

def ll(path: str = ".") -> None:
    """List files with details."""
    if test_command("eza"):
        subprocess.run(["eza", "--long", "--header", "--git", "--icons=always", "--group-directories-first", path])
    else:
        subprocess.run(["ls", "-l", path])

def sysinfo() -> None:
    """Show system information."""
    if test_command("fastfetch"):
        subprocess.run(["fastfetch"])
    else:
        import platform
        info = {
            "System": platform.system(),
            "Node": platform.node(),
            "Release": platform.release(),
            "Version": platform.version(),
            "Machine": platform.machine(),
            "Processor": platform.processor(),
            "Python": platform.python_version(),
        }
        for k, v in info.items():
            print(f"  {k}: {v}")

def flushdns() -> None:
    """Flush DNS cache."""
    if _IS_WINDOWS:
        subprocess.run(["ipconfig", "/flushdns"], shell=True)
    elif sys.platform == "darwin":
        subprocess.run(["sudo", "dscacheutil", "-flushcache"])
        subprocess.run(["sudo", "killall", "-HUP", "mDNSResponder"])
    else:
        subprocess.run(["sudo", "resolvectl", "flush-caches"])

def cpy(path: str) -> None:
    """Copy file content to clipboard."""
    if _IS_WINDOWS:
        subprocess.run(["clip.exe"], stdin=open(path, "rb"))
    elif test_command("xclip"):
        subprocess.run(["xclip", "-selection", "clipboard", "-i", path])
    elif test_command("xsel"):
        subprocess.run(["xsel", "--clipboard", "--input"], stdin=open(path, "r"))
    elif test_command("pbcopy"):
        subprocess.run(["pbcopy"], stdin=open(path, "r"))
    else:
        print("No clipboard utility available")

def pst() -> None:
    """Paste from clipboard."""
    if _IS_WINDOWS:
        subprocess.run(["powershell.exe", "-Command", "Get-Clipboard"])
    elif test_command("xclip"):
        subprocess.run(["xclip", "-selection", "clipboard", "-o"])
    elif test_command("xsel"):
        subprocess.run(["xsel", "--clipboard", "--output"])
    elif test_command("pbpaste"):
        subprocess.run(["pbpaste"])
    else:
        print("No clipboard utility available")

def edit_profile() -> None:
    """Open the xonsh config file in editor."""
    config_path = str(Path(_HOME) / ".config" / "xonsh" / "rc.xsh")
    editor = os.environ.get("EDITOR", "nano")
    subprocess.run([editor, config_path])

def invoke_profile() -> None:
    """Reload the xonsh configuration."""
    config_path = str(Path(_HOME) / ".config" / "xonsh" / "rc.xsh")
    print("Reloading rc.xsh...")
    # In xonsh, we can source the file
    try:
        exec(compile(open(config_path).read(), config_path, "exec"))
    except Exception as e:
        print(f"Error reloading config: {e}")

def download_file(url: str, output: str = "") -> None:
    """Download a file using aria2c (fast) or curl."""
    if test_command("aria2c"):
        cmd = ["aria2c", "-x", "16", "-s", "16", "-k", "1M"]
        if output:
            cmd.extend(["-o", output])
        cmd.append(url)
        subprocess.run(cmd)
    elif test_command("curl"):
        cmd = ["curl", "-L", "--progress-bar"]
        if output:
            cmd.extend(["-o", output])
        cmd.append(url)
        subprocess.run(cmd)
    else:
        print("No download tool available")

def fetch_url(url: str) -> None:
    """Fetch and display URL content."""
    if test_command("xh"):
        subprocess.run(["xh", url])
    elif test_command("curl"):
        subprocess.run(["curl", "-sL", url])
    else:
        print("No HTTP client available")

def debug_message() -> None:
    """Display startup debug information."""
    import datetime
    now = datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    print(f"\n{'='*60}")
    print(f"  Xonsh Shell | @HKDEVS | @HKDevloops")
    print(f"  Loaded: {now}")
    print(f"  Terminal: {_TERMINAL['name']}")
    print(f"  Truecolor: {_TERMINAL['truecolor']}")
    print(f"  Editor: {$EDITOR}")
    print(f"  Theme: {$SHELL_THEME}")
    print(f"  Python: {__import__('sys').version.split()[0]}")
    print(f"  Type 'show_help' for commands & shortcuts")
    print(f"{'='*60}\n")

def clear_cache() -> None:
    """Clear shell and tool caches."""
    caches_to_clear = [
        str(Path(_HOME) / ".cache" / "carapace"),
        str(Path(_HOME) / ".cache" / "fzf"),
        str(Path(_HOME) / ".local" / "share" / "atuin" / "history.db"),
    ]
    for cache_path in caches_to_clear:
        p = Path(cache_path)
        if p.exists():
            if p.is_dir():
                shutil.rmtree(p)
            else:
                p.unlink()
            print(f"Cleared: {cache_path}")
    # Zoxide rebuild
    if test_command("zoxide"):
        subprocess.run(["zoxide", "init", "xonsh"])
    print("Cache cleared.")

def update_xonsh() -> None:
    """Update xonsh and core tools."""
    print("Updating xonsh...")
    subprocess.run([sys.executable, "-m", "pip", "install", "--upgrade", "xonsh[full]"])
    if test_command("zoxide"):
        print("Updating zoxide...")
        subprocess.run(["cargo", "install", "zoxide", "--force"])
    if test_command("carapace"):
        print("Updating carapace...")
        subprocess.run(["carapace", "--update"])
    print("Update complete.")

def show_help() -> None:
    """Show comprehensive help for all custom functions, aliases, and builtins."""
    help_text = f"""
{'='*80}
  XONSH SHELL CONFIGURATION — COMPLETE REFERENCE
  Author: @HKDEVS | Organization: @HKDevloops
  Version: 1.0.0 | Font: CaskaydiaCove Nerd Font
{'='*80}

{'─'*80}
  CUSTOM UTILITY FUNCTIONS
{'─'*80}

  File Operations:
    touch(path)              Create empty file / update mtime
    mkcd(path)               Create directory and cd into it
    trash(path)              Move file to trash (safe delete)
    unzip_cmd(archive, dest) Extract any archive format
    compress(files, out)     Compress files (ouch)
    decompress(archive, out) Decompress archive (ouch)
    cpy(path)                Copy file content to clipboard
    pst()                    Paste from clipboard
    hexview(file)            Hex dump viewer (hexyl)

  Search & Find:
    ff(pattern)              Find files with fd
    grep(pattern, path)      Search with ripgrep
    which_cmd(cmd)           Locate a command
    sed_replace(pat, rep, *f)Replace in files (sd)
    sr(pattern, repl, *files)Fast find & replace (sd)
    jsonq(expr)              Query JSON with jq
    yamql(expr)              Query YAML with yq

  System Information:
    sysinfo()                Show system information
    nf()                     Neofetch / fastfetch
    diskfree()               Show disk usage
    uptime_cmd()             Show system uptime
    pubip()                  Get public IP address
    flushdns()               Flush DNS cache
    psx()                    Process viewer (procs)
    sys()                    System monitor (bottom)
    disk()                   Disk usage analyzer (dust)
    net()                    Network monitor (bandwhich)
    trace(host)              Network traceroute (trippy)

  Process Management:
    pkill_cmd(pattern)       Kill processes by pattern
    pgrep_cmd(pattern)       Find processes by pattern
    admin(*args)             Run with admin privileges

  File Viewing:
    la(path)                 List all files (with hidden)
    ll(path)                 List files with details
    head_cmd(file, n)        Show first N lines
    tail_cmd(file, n)        Show last N lines
    md(file)                 Render markdown (glow)
    cheat()                  Cheat sheets (navi)
    tldr(cmd)                Simplified man pages

  Navigation & Directories:
    ..                       cd .. (parent directory)
    ...                      cd ../.. (grandparent)
    z                        Smart cd with zoxide

  Network & Download:
    download_file(url, out)  Download file (aria2c)
    fetch_url(url)           Fetch URL content
    dl(url, out)             Fast download (aria2c)
    http(*args)              HTTP client (xh)

  Development Tools:
    pyrun(file)              Run Python script
    rustbuild()              Build Rust project (cargo)
    cbuild()                 Build C project (make)
    javabuild()              Build Java project (gradle/maven)
    npmdev()                 Run npm dev server
    pioupload()              PlatformIO upload
    stmbuild()               Build STM32Cube project

  Developer TUIs:
    fm()                     File manager (yazi)
    glg()                    Git TUI (lazygit)
    gd()                     Git UI (gitui)
    dk()                     Docker TUI (lazydocker)
    note(*args)              Note taking (zk)
    calc(expr)               Calculator (kalker)
    bench(cmd)               Benchmark tool (hyperfine)
    cloc(path)               Count lines of code (tokei)
    pass(*args)              Password manager (gopass)
    music()                  Music player (termusic)
    difft(*args)             Diff tool (difftastic)
    dua_interactive(path)    Disk usage analyzer (dua i)
    watchx(cmd)              Watch & run (watchexec)
    taskr(*args)             Task runner (just)

  Shell Configuration:
    edit_profile()           Open .xonshrc in editor
    invoke_profile()         Reload .xonshrc
    clear_cache()            Clear all caches
    update_xonsh()           Update xonsh & tools
    theme(name)              Switch theme (catppuccin/monokai/dark+/atom)
    test_command(cmd)        Test if command is available

  Git Shortcuts:
    gs                       git status
    ga(*args)                git add
    gc(msg)                  git commit -m
    gpush()                  git push
    gpull()                  git pull
    gcl(url)                 git clone
    g()                      git
    gcom(msg)                git commit -m (conventional)
    lazyg()                  lazygit
    gp()                     git push

  File Content:
    la(path)                 List all (eza --all --long)
    ll(path)                 List long (eza --long)
    grep(pattern, path)      ripgrep search

{'─'*80}
  XONSH BUILTINS & NATIVE COMMANDS
{'─'*80}

  Navigation:
    cd, pushd, popd, dirs, pwd

  File Operations:
    mkdir, rmdir, ls, ln, mv, cp, rm, chmod, chown, touch,
    cat, head, tail, echo, print, tee, chdir

  Environment:
    env, export, unset, set, var, let, read, readonly,
    local, global, nonlocal

  Process & Job Control:
    exec, exit, quit, kill, jobs, bg, fg, wait, suspend,
    disown, nohup, &, |, &&, ||, ;;

  Shell Control:
    source, alias, unalias, aliases, help, history,
    return, break, continue, pass, True, False, None

  Testing & Logic:
    test, [, [[, true, false, and, or, not, in, is,
    if, else, elif, for, while, with, case, select

  Functions & Classes:
    def, class, lambda, yield, return, super,
    property, staticmethod, classmethod

  Exception Handling:
    try, except, finally, raise, assert, del

  Async:
    async, await

  Import:
    import, from, as

  Xonsh-Specific Syntax:
    !(cmd)           Run command, capture output (like $(...) in bash)
    $(cmd)           Capture command output as string
    ![cmd]           Run command in subprocess mode
    $[cmd]           Capture command output, split by lines
    @(expr)          Python expression in subprocess context
    &(...)           Run command in background (threaded)
    ^(...)           Run command with elevated priority
    xp               xonsh prompt
    xonsh-pragma     Pragma directives for xonsh
    macro val        Macro value expansion
    trace            Trace mode for debugging

  Xonsh Built-in Objects:
    $XONSH_CONFIG_DIR   Config directory path
    $XONSH_DATA_DIR     Data directory path
    $XONSH_CACHE_DIR    Cache directory path
    $XONSH_INTERACTIVE  Whether shell is interactive
    $XONSH_HISTORY_FILE History file path
    $XONSH_VERSION      Xonsh version string
    aliases              Dictionary of all aliases
    builtins.__xonsh__   Xonsh runtime environment
    XonshSession         Session object

  Subprocess Modes:
    !(...)  captured-subprocess: returns CommandPipeline
    $(...)  captured-subprocess: returns str (stdout)
    ![...]  uncaptured-subprocess: directly to terminal
    $[...]  list of str (split stdout by newlines)

  Redirectors:
    >, >>, <, o>, e>, o>>, e>>, a>, a>>, &>, &>>

  Pipe Operators:
    |   standard pipe
    |e  pipe stderr
    |a  pipe all (stdout + stderr)

{'─'*80}
  KEYBINDINGS (EMACS MODE — DEFAULT)
{'─'*80}

  Navigation:
    Ctrl+A    Move to beginning of line
    Ctrl+E    Move to end of line
    Ctrl+F    Move forward one character
    Ctrl+B    Move back one character
    Alt+F     Move forward one word
    Alt+B     Move back one word

  Editing:
    Ctrl+D    Delete character / exit
    Ctrl+H    Delete previous character (backspace)
    Ctrl+K    Kill to end of line
    Ctrl+U    Kill to beginning of line
    Ctrl+W    Kill previous word
    Ctrl+Y    Yank (paste)
    Alt+Y     Yank pop
    Ctrl+T    Transpose characters
    Alt+T     Transpose words

  History:
    Ctrl+R    Reverse search history
    Ctrl+S    Forward search history
    Ctrl+P    Previous history (up)
    Ctrl+N    Next history (down)
    Alt+.     Insert last argument from previous command

  Completion:
    Tab       Complete
    Alt+=     Insert possible completions

  Control:
    Ctrl+C    Interrupt / cancel
    Ctrl+Z    Suspend
    Ctrl+L    Clear screen
    Ctrl+J    New line (enter)

  FZF Integration:
    Ctrl+T    Find files (fzf + fd)
    Ctrl+R    Search history (fzf)
    Alt+C     Change directory (fzf + fd)

  Zoxide Integration:
    z <query> Smart cd (zoxide)
    zi        Interactive directory selection

{'─'*80}
  ALIASES
{'─'*80}

  Modern Replacements:
    cat    → bat              (syntax-highlighted cat)
    ls     → eza              (modern ls with icons)
    find   → fd               (fast find)
    grep   → rg               (fast grep)
    diff   → delta            (syntax-aware diff)
    ps     → procs            (modern ps)
    top    → btm              (system monitor)
    du     → dust             (disk usage)
    sed    → sd               (fast sed)
    curl   → xh               (modern HTTP client)
    nano   → micro            (modern nano)
    vim    → $EDITOR          (configured editor)
    ..     → cd ..            (parent directory)
    ...    → cd ../..         (grandparent directory)

  Developer Shortcuts:
    fm     → yazi             (file manager)
    glg    → lazygit          (git TUI)
    gd     → gitui            (git UI)
    sys    → btm              (system monitor)
    disk   → dust             (disk analyzer)
    net    → bandwhich        (network monitor)
    dk     → lazydocker       (docker TUI)
    note   → zk               (note manager)
    calc   → kalker           (calculator)
    cheat  → navi             (cheatsheets)
    tldr   → tealdeer         (simplified man)
    bench  → hyperfine        (benchmarking)
    cloc   → tokei            (code stats)
    md     → glow             (markdown reader)
    hexview→ hexyl            (hex viewer)
    pass   → gopass           (password manager)
    music  → termusic         (music player)
    dl     → aria2c           (downloader)
    neofetch→ fastfetch       (system info)
    psx    → procs            (process viewer)
    difft  → difftastic       (diff tool)
    compress→ ouch compress   (compression)
    decompress→ ouch decompress (decompression)
    http   → xh               (HTTP client)
    jsonq  → jq               (JSON query)
    yamql  → yq               (YAML query)
    dua_interactive→ dua i   (disk analyzer)
    sr     → sd               (search & replace)
    watchx → watchexec        (watch & execute)
    taskr  → just              (task runner)
    trace  → trippy           (traceroute)

  Git Shortcuts:
    gs     → git status
    ga     → git add
    gc     → git commit -m
    gpush  → git push
    gpull  → git pull
    gcl    → git clone
    g      → git
    gcom   → git commit (conventional)
    lazyg  → lazygit
    gp     → git push

{'─'*80}
  CARAPACE COMPLETION
{'─'*80}

  Carapace provides completions for 600+ commands including deep
  subcommands. Key completions available:

    git, docker, kubectl, cargo, npm, pip, go, python,
    rsync, ssh, tar, gzip, make, cmake, gcc, g++,
    java, javac, gradle, mvn, dotnet, rustup,
    gh, hub, code, nvim, vim, micro, helix,
    eza, bat, fd, rg, delta, dust, procs, btm,
    fzf, zoxide, oh-my-posh, scoop, choco, winget,
    just, task, glow, gum, jq, yq, xh, sd,
    lazygit, lazydocker, yazi, navi, tealdeer,
    zk, gopass, hyperfine, tokei, hexyl, dua,
    ouch, aria2c, bandwhich, trippy, difftastic,
    kalker, termusic, fastfetch, watchexec, etc.

  Init: eval $(carapace xonsh --no-banner)

{'─'*80}
  THEME SWITCHING
{'─'*80}

  Current theme: {$SHELL_THEME}

  Available themes:
    catppuccin   Catppuccin Mocha (default) — warm dark theme
    monokai      Monokai — classic dark theme
    dark+        Dark+ — VS Code dark+ variant
    atom         Atom One Dark — Atom editor theme

  Usage:
    theme("catppuccin")    Switch to Catppuccin Mocha
    theme("monokai")       Switch to Monokai
    theme("dark+")         Switch to Dark+
    theme("atom")          Switch to Atom One Dark

  Affected settings:
    BAT_THEME              Bat syntax highlighting theme
    SHELL_THEME            Shell theme environment variable
    FZF_DEFAULT_OPTS       FZF color scheme
    POSH_THEME             Oh-My-Posh prompt theme

{'─'*80}
  TOOLS INSTALLED (Scoop)
{'─'*80}

  Shell & Prompt:     oh-my-posh, carapace, zoxide, fzf, atuin, mcfly, navi
  File & Search:      fd, bat, eza, ripgrep, delta, dust, dua, ouch, sd
  Dev & Build:        just, hyperfine, tokei, difftastic, watchexec
  Git:                lazygit, delta, gitui
  Network:            xh, aria2, bandwhich, trippy
  TUI:                yazi, bottom, zellij, broot, lf
  Data:               jq, yq, hexyl
  Docs:               glow, gum, tealdeer
  Security:           gopass
  System:             fastfetch, procs
  Misc:               kalker, zk, termusic, chafa, usql, yt-dlp

{'='*80}
"""
    print(help_text)

# --- Git Shortcuts ---
def gs(*args) -> None:
    """Git status."""
    subprocess.run(["git", "status"] + list(args))

def ga(*args) -> None:
    """Git add."""
    if not args:
        args = (".",)
    subprocess.run(["git", "add"] + list(args))

def gc(msg: str = "", *args) -> None:
    """Git commit with message."""
    if msg:
        subprocess.run(["git", "commit", "-m", msg] + list(args))
    else:
        subprocess.run(["git", "commit"] + list(args))

def gpush(*args) -> None:
    """Git push."""
    subprocess.run(["git", "push"] + list(args))

def gpull(*args) -> None:
    """Git pull."""
    subprocess.run(["git", "pull"] + list(args))

def gcl(url: str, *args) -> None:
    """Git clone."""
    subprocess.run(["git", "clone", url] + list(args))

def g(*args) -> None:
    """Git shorthand."""
    subprocess.run(["git"] + list(args))

def gcom(msg: str, *args) -> None:
    """Git commit with conventional commit style."""
    subprocess.run(["git", "commit", "-m", msg] + list(args))

def lazyg() -> None:
    """Open lazygit."""
    if test_command("lazygit"):
        subprocess.run(["lazygit"])
    else:
        print("lazygit not installed")

def gp(*args) -> None:
    """Git push shorthand."""
    subprocess.run(["git", "push"] + list(args))


# ==============================================================================
# SECTION 4: ADDITIONAL DEVELOPER FUNCTIONS
# ==============================================================================

def pyrun(file: str, *args) -> None:
    """Run a Python script."""
    subprocess.run([sys.executable, file] + list(args))

def rustbuild(*args) -> None:
    """Build Rust project with cargo."""
    if test_command("cargo"):
        subprocess.run(["cargo", "build"] + list(args))
    else:
        print("cargo not installed")

def cbuild(*args) -> None:
    """Build C project with make."""
    if Path("Makefile").exists() or Path("makefile").exists():
        subprocess.run(["make"] + list(args))
    elif Path("CMakeLists.txt").exists():
        if not Path("build").exists():
            Path("build").mkdir()
        subprocess.run(["cmake", "-B", "build"])
        subprocess.run(["cmake", "--build", "build"])
    else:
        print("No Makefile or CMakeLists.txt found")

def javabuild(*args) -> None:
    """Build Java project."""
    if Path("build.gradle").exists() or Path("build.gradle.kts").exists():
        if test_command("gradle"):
            subprocess.run(["gradle", "build"] + list(args))
        elif Path("gradlew").exists():
            subprocess.run(["./gradlew", "build"] + list(args))
    elif Path("pom.xml").exists():
        if test_command("mvn"):
            subprocess.run(["mvn", "compile"] + list(args))
    else:
        print("No Gradle or Maven project found")

def npmdev(*args) -> None:
    """Run npm dev server."""
    if Path("package.json").exists():
        subprocess.run(["npm", "run", "dev"] + list(args))
    else:
        print("No package.json found")

def pioupload(*args) -> None:
    """PlatformIO upload."""
    if test_command("pio"):
        subprocess.run(["pio", "run", "-t", "upload"] + list(args))
    else:
        print("PlatformIO not installed")

def stmbuild(*args) -> None:
    """Build STM32CubeIDE project."""
    if test_command("stm32cubeidec"):
        subprocess.run(["stm32cubeidec"] + list(args))
    else:
        # Try make-based build
        if Path("Makefile").exists():
            subprocess.run(["make", "all"] + list(args))
        else:
            print("STM32CubeIDE CLI not found and no Makefile present")

def fm(path: str = ".") -> None:
    """File manager (yazi)."""
    if test_command("yazi"):
        subprocess.run(["yazi", path])
    elif test_command("lf"):
        subprocess.run(["lf", path])
    elif test_command("broot"):
        subprocess.run(["broot", path])
    else:
        print("No file manager (yazi/lf/broot) installed")

def glg() -> None:
    """Open lazygit."""
    if test_command("lazygit"):
        subprocess.run(["lazygit"])
    else:
        print("lazygit not installed")

def gd() -> None:
    """Open gitui."""
    if test_command("gitui"):
        subprocess.run(["gitui"])
    else:
        print("gitui not installed")

def sys_mon() -> None:
    """System monitor (bottom)."""
    if test_command("btm"):
        subprocess.run(["btm"])
    elif test_command("bottom"):
        subprocess.run(["bottom"])
    else:
        subprocess.run(["top"])
aliases["sys"] = sys_mon

def disk(path: str = ".") -> None:
    """Disk usage analyzer (dust)."""
    if test_command("dust"):
        subprocess.run(["dust", path])
    else:
        subprocess.run(["du", "-sh", path])

def net() -> None:
    """Network monitor (bandwhich)."""
    if test_command("bandwhich"):
        subprocess.run(["bandwhich"])
    else:
        if _IS_WINDOWS:
            subprocess.run(["netstat", "-an"])
        else:
            subprocess.run(["ss", "-tuln"])

def dk() -> None:
    """Docker TUI (lazydocker)."""
    if test_command("lazydocker"):
        subprocess.run(["lazydocker"])
    elif test_command("docker"):
        subprocess.run(["docker", "ps", "-a"])
    else:
        print("Docker not available")

def note(*args) -> None:
    """Note taking (zk)."""
    if test_command("zk"):
        subprocess.run(["zk"] + list(args))
    else:
        print("zk not installed")

def calc(expr: str = "") -> None:
    """Calculator (kalker)."""
    if test_command("kalker"):
        if expr:
            subprocess.run(["kalker", expr])
        else:
            subprocess.run(["kalker"])
    else:
        if expr:
            try:
                result = eval(expr)
                print(result)
            except Exception as e:
                print(f"Error: {e}")
        else:
            print("kalker not installed")

def cheat() -> None:
    """Cheat sheets (navi)."""
    if test_command("navi"):
        subprocess.run(["navi"])
    else:
        print("navi not installed")

def tldr(cmd: str = "") -> None:
    """Simplified man pages (tealdeer)."""
    if test_command("tldr"):
        if cmd:
            subprocess.run(["tldr", cmd])
        else:
            subprocess.run(["tldr", "--list"])
    else:
        print("tealdeer not installed")

def bench(cmd: str, *args) -> None:
    """Benchmark tool (hyperfine)."""
    if test_command("hyperfine"):
        subprocess.run(["hyperfine", cmd] + list(args))
    else:
        print("hyperfine not installed")

def cloc(path: str = ".", *args) -> None:
    """Count lines of code (tokei)."""
    if test_command("tokei"):
        subprocess.run(["tokei", path] + list(args))
    else:
        print("tokei not installed")

def md(file: str = "", *args) -> None:
    """Render markdown (glow)."""
    if test_command("glow"):
        if file:
            subprocess.run(["glow", file] + list(args))
        else:
            subprocess.run(["glow"])
    else:
        print("glow not installed")

def hexview(file: str) -> None:
    """Hex dump viewer (hexyl)."""
    if test_command("hexyl"):
        subprocess.run(["hexyl", file])
    else:
        subprocess.run(["xxd", file])

def pass_cmd(*args) -> None:
    """Password manager (gopass)."""
    if test_command("gopass"):
        subprocess.run(["gopass"] + list(args))
    else:
        print("gopass not installed")

def music(*args) -> None:
    """Music player (termusic)."""
    if test_command("termusic"):
        subprocess.run(["termusic"] + list(args))
    else:
        print("termusic not installed")

def dl(url: str, output: str = "") -> None:
    """Fast download (aria2c)."""
    if test_command("aria2c"):
        cmd = ["aria2c", "-x", "16", "-s", "16", "-k", "1M"]
        if output:
            cmd.extend(["-o", output])
        cmd.append(url)
        subprocess.run(cmd)
    else:
        print("aria2c not installed")

def neofetch() -> None:
    """System info (fastfetch)."""
    if test_command("fastfetch"):
        subprocess.run(["fastfetch"])
    elif test_command("neofetch"):
        subprocess.run(["neofetch"])
    else:
        sysinfo()

def psx(*args) -> None:
    """Process viewer (procs)."""
    if test_command("procs"):
        subprocess.run(["procs"] + list(args))
    else:
        subprocess.run(["ps", "aux"])

def difft(*args) -> None:
    """Diff tool (difftastic)."""
    if test_command("difft"):
        subprocess.run(["difft"] + list(args))
    else:
        subprocess.run(["diff"] + list(args))

def compress(*args) -> None:
    """Compress files (ouch)."""
    if test_command("ouch"):
        subprocess.run(["ouch", "compress"] + list(args))
    else:
        print("ouch not installed — use tar/gzip instead")

def decompress(archive: str, dest: str = ".") -> None:
    """Decompress archive (ouch)."""
    if test_command("ouch"):
        subprocess.run(["ouch", "decompress", archive, "--dir", dest])
    else:
        unzip_cmd(archive, dest)

def http(*args) -> None:
    """HTTP client (xh)."""
    if test_command("xh"):
        subprocess.run(["xh"] + list(args))
    elif test_command("httpie"):
        subprocess.run(["http"] + list(args))
    else:
        print("xh/httpie not installed")

def jsonq(*args) -> None:
    """Query JSON with jq."""
    if test_command("jq"):
        subprocess.run(["jq"] + list(args))
    else:
        print("jq not installed")

def yamql(*args) -> None:
    """Query YAML with yq."""
    if test_command("yq"):
        subprocess.run(["yq"] + list(args))
    else:
        print("yq not installed")

def dua_interactive(path: str = ".") -> None:
    """Interactive disk usage analyzer (dua)."""
    if test_command("dua"):
        subprocess.run(["dua", "i", path])
    else:
        print("dua not installed")

def sr(*args) -> None:
    """Fast search and replace (sd)."""
    if test_command("sd"):
        subprocess.run(["sd"] + list(args))
    else:
        print("sd not installed")

def watchx(*args) -> None:
    """Watch and execute commands (watchexec)."""
    if test_command("watchexec"):
        subprocess.run(["watchexec"] + list(args))
    else:
        print("watchexec not installed")

def taskr(*args) -> None:
    """Task runner (just)."""
    if test_command("just"):
        subprocess.run(["just"] + list(args))
    else:
        print("just not installed")

def trace(host: str = "8.8.8.8", *args) -> None:
    """Network traceroute (trippy)."""
    if test_command("trippy"):
        subprocess.run(["trippy", host] + list(args))
    elif test_command("traceroute"):
        subprocess.run(["traceroute", host] + list(args))
    else:
        print("trippy/traceroute not installed")


# ==============================================================================
# SECTION 5: THEME SWITCHING
# ==============================================================================

# Theme configuration maps
_THEME_CONFIG = {
    "catppuccin": {
        "bat": "Catppuccin Mocha",
        "fzf": (
            " --height=40% --layout=reverse --border=rounded --margin=1 --padding=1"
            " --info=inline --prompt='  ' --pointer='>' --marker='+"
            " --header-first --multi --cycle"
            " --color=bg+:#363a4f,bg:#1e1e2e,spinner:#f5e0dc,hl:#f38ba8"
            " --color=fg:#cdd6f4,header:#f38ba8,info:#cba6f7,pointer:#f5e0dc"
            " --color=marker:#b4befe,fg+:#cdd6f4,prompt:#cba6f7,hl+:#f38ba8"
            " --color=selected-bg:#45475a,border:#89b4fa,gutter:#1e1e2e"
        ),
        "omp": str(Path(_HOME) / "scoop" / "apps" / "oh-my-posh" / "current" / "themes" / "half-life.omp.json"),
    },
    "monokai": {
        "bat": "Monokai Extended",
        "fzf": (
            " --height=40% --layout=reverse --border=rounded --margin=1 --padding=1"
            " --info=inline --prompt='  ' --pointer='>' --marker='+"
            " --header-first --multi --cycle"
            " --color=bg+:#272822,bg:#1e1e1e,spinner:#f92672,hl:#a6e22e"
            " --color=fg:#f8f8f2,header:#a6e22e,info:#fd971f,pointer:#f92672"
            " --color=marker:#e6db74,fg+:#f8f8f2,prompt:#fd971f,hl+:#a6e22e"
            " --color=selected-bg:#3e3d32,border:#66d9ef,gutter:#1e1e1e"
        ),
        "omp": str(Path(_HOME) / "scoop" / "apps" / "oh-my-posh" / "current" / "themes" / "monokai.omp.json"),
    },
    "dark+": {
        "bat": "Visual Studio Dark+",
        "fzf": (
            " --height=40% --layout=reverse --border=rounded --margin=1 --padding=1"
            " --info=inline --prompt='  ' --pointer='>' --marker='+"
            " --header-first --multi --cycle"
            " --color=bg+:#1e1e1e,bg:#1e1e1e,spinner:#569cd6,hl:#569cd6"
            " --color=fg:#d4d4d4,header:#569cd6,info:#dcdcaa,pointer:#569cd6"
            " --color=marker:#dcdcaa,fg+:#d4d4d4,prompt:#dcdcaa,hl+:#569cd6"
            " --color=selected-bg:#264f78,border:#569cd6,gutter:#1e1e1e"
        ),
        "omp": str(Path(_HOME) / "scoop" / "apps" / "oh-my-posh" / "current" / "themes" / "dark-plus.omp.json"),
    },
    "atom": {
        "bat": "Atom One Dark",
        "fzf": (
            " --height=40% --layout=reverse --border=rounded --margin=1 --padding=1"
            " --info=inline --prompt='  ' --pointer='>' --marker='+"
            " --header-first --multi --cycle"
            " --color=bg+:#2c313a,bg:#1d1f21,spinner:#61afef,hl:#e06c75"
            " --color=fg:#abb2bf,header:#e06c75,info:#c678dd,pointer:#61afef"
            " --color=marker:#98c379,fg+:#abb2bf,prompt:#c678dd,hl+:#e06c75"
            " --color=selected-bg:#3e4451,border:#61afef,gutter:#1d1f21"
        ),
        "omp": str(Path(_HOME) / "scoop" / "apps" / "oh-my-posh" / "current" / "themes" / "atom.omp.json"),
    },
}

def theme(name: str = "catppuccin") -> None:
    """
    Switch shell theme between catppuccin, monokai, dark+, and atom.

    Affects: BAT_THEME, SHELL_THEME, FZF_DEFAULT_OPTS, POSH_THEME
    """
    name = name.lower()
    if name not in _THEME_CONFIG:
        print(f"Unknown theme: {name}")
        print(f"Available themes: {', '.join(_THEME_CONFIG.keys())}")
        return

    cfg = _THEME_CONFIG[name]

    # Set environment variables
    $BAT_THEME = cfg["bat"]
    $SHELL_THEME = name
    $FZF_DEFAULT_OPTS = cfg["fzf"]

    if Path(cfg["omp"]).exists():
        $POSH_THEME = cfg["omp"]

    print(f"Theme switched to: {name}")
    print(f"  BAT_THEME = {cfg['bat']}")
    print(f"  SHELL_THEME = {name}")
    print(f"  FZF colors updated")
    if Path(cfg["omp"]).exists():
        print(f"  POSH_THEME = {cfg['omp']}")
    else:
        print(f"  POSH_THEME = (oh-my-posh theme file not found)")
    print("\nRestart the shell or run 'invoke_profile()' for full effect.")


# ==============================================================================
# SECTION 6: XONSH-SPECIFIC SETTINGS (in try/except for safety)
# ==============================================================================

try:
    # --- Core Aliases: Modern replacements ---
    if test_command("bat"):
        aliases["cat"] = "bat"
    if test_command("eza"):
        aliases["ls"] = "eza"
    if test_command("fd"):
        aliases["find"] = "fd"
    if test_command("rg"):
        aliases["grep"] = "rg"
    if test_command("delta"):
        aliases["diff"] = "delta"
    if test_command("procs"):
        aliases["ps"] = "procs"
    if test_command("btm"):
        aliases["top"] = "btm"
    if test_command("dust"):
        aliases["du"] = "dust"
    if test_command("sd"):
        aliases["sed"] = "sd"
    if test_command("xh"):
        aliases["curl"] = "xh"
    if test_command("micro"):
        aliases["nano"] = "micro"
    aliases["vim"] = $EDITOR

    # --- Navigation aliases ---
    aliases[".."] = "cd .."
    aliases["..."] = "cd ../.."
    aliases["...."] = "cd ../../.."
    aliases["....."] = "cd ../../../.."

    # --- Developer TUI aliases ---
    if test_command("yazi"):
        aliases["fm"] = "yazi"
    elif test_command("lf"):
        aliases["fm"] = "lf"

    if test_command("lazygit"):
        aliases["glg"] = "lazygit"
    if test_command("gitui"):
        aliases["gd"] = "gitui"
    if test_command("dust"):
        aliases["disk"] = "dust"
    if test_command("bandwhich"):
        aliases["net"] = "bandwhich"
    if test_command("lazydocker"):
        aliases["dk"] = "lazydocker"
    if test_command("zk"):
        aliases["note"] = "zk"
    if test_command("kalker"):
        aliases["calc"] = "kalker"
    if test_command("navi"):
        aliases["cheat"] = "navi"
    if test_command("hyperfine"):
        aliases["bench"] = "hyperfine"
    if test_command("tokei"):
        aliases["cloc"] = "tokei"
    if test_command("glow"):
        aliases["md"] = "glow"
    if test_command("hexyl"):
        aliases["hexview"] = "hexyl"
    if test_command("gopass"):
        aliases["pass"] = "gopass"
    if test_command("termusic"):
        aliases["music"] = "termusic"
    if test_command("aria2c"):
        aliases["dl"] = "aria2c"
    if test_command("fastfetch"):
        aliases["neofetch"] = "fastfetch"
    if test_command("procs"):
        aliases["psx"] = "procs"
    if test_command("ouch"):
        aliases["compress"] = "ouch compress"
        aliases["decompress"] = "ouch decompress"
    if test_command("xh"):
        aliases["http"] = "xh"
    if test_command("jq"):
        aliases["jsonq"] = "jq"
    if test_command("yq"):
        aliases["yamql"] = "yq"
    if test_command("dua"):
        aliases["dua_interactive"] = "dua i"
    if test_command("sd"):
        aliases["sr"] = "sd"
    if test_command("watchexec"):
        aliases["watchx"] = "watchexec"
    if test_command("just"):
        aliases["taskr"] = "just"
    if test_command("trippy"):
        aliases["trace"] = "trippy"

    # --- Git aliases ---
    aliases["gs"] = "git status"
    aliases["ga"] = "git add"
    aliases["gc"] = "git commit -m"
    aliases["gpush"] = "git push"
    aliases["gpull"] = "git pull"
    aliases["gcl"] = "git clone"
    aliases["g"] = "git"
    aliases["gcom"] = "git commit -m"
    aliases["gp"] = "git push"
    aliases["gst"] = "git stash"
    aliases["gstp"] = "git stash pop"
    aliases["gl"] = "git log --oneline --graph --decorate -20"
    aliases["gla"] = "git log --oneline --graph --decorate --all -20"
    aliases["gdif"] = "git diff"
    aliases["gds"] = "git diff --staged"
    aliases["gb"] = "git branch"
    aliases["gco"] = "git checkout"
    aliases["gsw"] = "git switch"
    aliases["gr"] = "git rebase"
    aliases["grs"] = "git reset"
    aliases["gt"] = "git tag"
    aliases["gw"] = "git worktree"

    if test_command("lazygit"):
        aliases["lazyg"] = "lazygit"

    aliases["show_help"] = show_help
    aliases["show-help"] = show_help

    # --- Oh-My-Posh Prompt Init (with timeout and fallback) ---
    try:
        import threading
        _omp_result = [None]

        def _init_omp():
            try:
                omp_path = shutil.which("oh-my-posh")
                if omp_path:
                    theme_path = $POSH_THEME
                    theme_ok = Path(theme_path).exists()
                    # Cache the generated init script; regenerate when the theme
                    # file or the oh-my-posh executable changes.
                    _omp_cache_dir = Path(_HOME) / ".cache" / "oh-my-posh"
                    _omp_cache = _omp_cache_dir / "init-xonsh.py"
                    _fresh = False
                    if _omp_cache.exists():
                        _cache_mtime = _omp_cache.stat().st_mtime
                        _fresh = _cache_mtime >= Path(omp_path).stat().st_mtime and (not theme_ok or _cache_mtime >= Path(theme_path).stat().st_mtime)
                    if _fresh:
                        _omp_result[0] = _omp_cache.read_text(encoding="utf-8")
                    else:
                        if theme_ok:
                            _omp_result[0] = subprocess.run([omp_path, "init", "xonsh", "--config", theme_path], capture_output=True, text=True).stdout
                        else:
                            _omp_result[0] = subprocess.run([omp_path, "init", "xonsh"], capture_output=True, text=True).stdout
                        if _omp_result[0]:
                            _omp_cache_dir.mkdir(parents=True, exist_ok=True)
                            _omp_cache.write_text(_omp_result[0], encoding="utf-8")
            except Exception:
                _omp_result[0] = None

        _omp_thread = threading.Thread(target=_init_omp, daemon=True)
        _omp_thread.start()
        _omp_thread.join(timeout=3.0)

        if _omp_result[0]:
            execx(_omp_result[0])
        else:
            # Fallback: simple colored prompt
            $PROMPT = "{env_name}{BOLD_GREEN}{user}@{hostname}{BOLD_BLUE} {cwd}{RESET} \n{BOLD_RED}${RESET} "
    except Exception as e:
        # Ultimate fallback prompt
        $PROMPT = "{BOLD_GREEN}{user}@{hostname}{RESET}:{BOLD_BLUE}{cwd}{RESET}$ "

    # --- Carapace Completion Init ---
    # Follows the official setup: exec($(carapace _carapace))
    # We cache the generated init to avoid the ~0.5s carapace call on every
    # startup, but invalidate the cache when carapace is updated.
    # NOTE: carapace inserts its own bin dir into PATH; on this system the
    # carapace binary is on PATH via scoop shims, so that insertion is
    # harmless (it adds an empty/unused dir).
    try:
        if test_command("carapace"):
            _carapace_exe = Path(shutil.which("carapace"))
            _carapace_version = subprocess.run(
                [str(_carapace_exe), "--version"],
                capture_output=True, text=True, check=True
            ).stdout.strip()
            _carapace_cache_dir = Path(os.environ.get("LOCALAPPDATA", str(Path(_HOME) / "AppData" / "Local"))) / "carapace" / "init"
            _carapace_cache = _carapace_cache_dir / f"init-xonsh.{hash(_carapace_version)}.py"
            _carapace_fresh = False
            if _carapace_cache.exists():
                _carapace_fresh = _carapace_cache.stat().st_mtime >= _carapace_exe.stat().st_mtime
            if _carapace_fresh:
                _carapace_init = _carapace_cache.read_text(encoding="utf-8")
            else:
                _carapace_init = subprocess.run(
                    ["carapace", "_carapace", "xonsh"],
                    capture_output=True, text=True, check=True
                ).stdout
                _carapace_cache_dir.mkdir(parents=True, exist_ok=True)
                _carapace_cache.write_text(_carapace_init, encoding="utf-8")
            exec(compile(_carapace_init, "<carapace>", "exec"))
            _CARAPACE_INITIALIZED = True
    except Exception as _cara_err:
        pass

    # --- Zoxide Init ---
    # Mirror pwsh: run `zoxide init xonsh` once, cache the result, and exec it.
    # This provides the `z`/`zi` functions and the pwd hook.
    try:
        if test_command("zoxide"):
            _zoxide_exe = Path(shutil.which("zoxide"))
            _zoxide_cache_dir = Path(os.environ.get("LOCALAPPDATA", str(Path(_HOME) / "AppData" / "Local"))) / "zoxide" / "init"
            _zoxide_cache = _zoxide_cache_dir / f"init-xonsh.{hash(subprocess.run([str(_zoxide_exe), "--version"], capture_output=True, text=True, check=True).stdout.strip())}.py"
            _zoxide_fresh = False
            if _zoxide_cache.exists():
                _zoxide_fresh = _zoxide_cache.stat().st_mtime >= _zoxide_exe.stat().st_mtime
            if _zoxide_fresh:
                _zoxide_init = _zoxide_cache.read_text(encoding="utf-8")
            else:
                _zoxide_init = subprocess.run(
                    ["zoxide", "init", "xonsh"],
                    capture_output=True, text=True, check=True
                ).stdout
                _zoxide_cache_dir.mkdir(parents=True, exist_ok=True)
                _zoxide_cache.write_text(_zoxide_init, encoding="utf-8")
            exec(compile(_zoxide_init, "<zoxide>", "exec"))
            _ZOXIDE_INITIALIZED = True
    except Exception as _zox_err:
        pass

    # --- History Config ---
    $XONSH_HISTORY_SIZE = "100000 commands"
    $XONSH_HISTORY_FILE = str(Path(_HOME) / ".local" / "share" / "xonsh" / "xonsh_history.json")

    # Ensure history directory exists
    Path($XONSH_HISTORY_FILE).parent.mkdir(parents=True, exist_ok=True)

    # --- Xonsh Display Settings ---
    $XONSH_SHOW_TRACEBACK = False
    $XONSH_STDERR_PREFIX = ""
    $XONSH_STDERR_POSTFIX = ""
    $SUPPRESS_BRANCH_NAME_CHAR_LIMIT = "40"

    # Xontrib settings
    $XONSH_AUTOPUSHDIR = True  # Automatically push directories onto the stack
    $XONSH_DIRSTACK_SIZE = 20
    $XONSH_EXPAND_ENV_VARS = True
    $COMPLETIONS_MODE = "lazy"  # Lazy-load completions for performance

except Exception as _xonsh_err:
    # If any xonsh-specific setting fails, log it but don't crash
    import traceback
    print(f"[xonshrc warning] Error in xonsh settings: {_xonsh_err}")
    traceback.print_exc()


# ==============================================================================
# SECTION 7: COMPREHENSIVE HELP (defined above as show_help)
# ==============================================================================
# The show_help() function is defined in Section 3 above.
# It provides a complete reference of ALL custom functions, ALL xonsh builtins,
# ALL aliases, ALL keybindings, carapace info, and theme switching instructions.


# ==============================================================================
# SECTION 8: STARTUP
# ==============================================================================

# Run debug message
debug_message()

# Print help hint
print("  💡 Tip: Type 'show_help' for complete command reference")
print("  🎨 Tip: Type 'theme(\"monokai\")' to switch themes")
print()

# --- Fastfetch on first launch with random logo ---
if test_command("fastfetch"):
    import random
    from pathlib import Path
    _ff_logos = list((Path(_HOME) / ".config" / "fastfetch" / "logos").glob("*.txt"))
    if _ff_logos:
        _ff_logo = random.choice(_ff_logos)
        subprocess.run(["fastfetch", "--logo", str(_ff_logo)])
    else:
        subprocess.run(["fastfetch"])


