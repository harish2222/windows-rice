# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: Xonsh Runtime Configuration
# ============================================================================
"""Xonsh rc — HKDEVS unified edition (single source of truth).

This file is the ONLY xonsh startup file. The previous
`~/.config/xonsh/rc.xsh` and `~/.config/xonsh/init.xsh` were consolidated here
to remove duplicated helper definitions and the per-launch oh-my-posh
subprocess that `init.xsh` spawned.

Design rules
------------
* No subprocess is spawned on the startup path (OMP + zoxide init scripts are
  cached on disk; they are regenerated only when their binary/theme changes).
* No banner, no fastfetch, no carapace import — those cost seconds.
* Everything expensive beyond prompt/env setup is an on-demand function.
"""
import base64
import os
import shutil
import subprocess
from pathlib import Path

# ── 1. Environment ───────────────────────────────────────────────────────────
_HOME = Path.home()
_USER = os.environ.get("USERNAME") or os.environ.get("USER", "user")
_IS_WINDOWS = (os.name == "nt")

# Fast-path Scoop shims first.  Every startup dependency is installed there,
# so this avoids directory enumeration across the complete Windows PATH.
_WHICH_EXTS = [e for e in os.environ.get("PATHEXT", ".COM;.EXE;.BAT;.CMD").split(";") if e]
_SHIM_DIR = _HOME / "scoop" / "shims"

def _which(name):
    """Resolve Scoop shims cheaply, then preserve normal PATH semantics."""
    if not name or os.path.isabs(name):
        return shutil.which(name)
    for suffix in ("", *_WHICH_EXTS):
        candidate = _SHIM_DIR / f"{name}{suffix}"
        if candidate.is_file():
            return str(candidate)
    return shutil.which(name)

# Build PATH in one pass; xonsh env mutations are cheap but each one costs.
_extra = [
    _HOME / "scoop" / "shims",
    _HOME / ".local" / "bin",
    _HOME / ".cargo" / "bin",
    _HOME / "go" / "bin",
]
_existing = os.environ.get("PATH", "").split(os.pathsep)
_seen, _paths = set(), []
for _p in [str(p) for p in _extra] + _existing:
    if _p and _p not in _seen:
        _seen.add(_p)
        _paths.append(_p)
$PATH = os.pathsep.join(_paths)

$XDG_CONFIG_HOME = str(_HOME / ".config")
$XDG_CACHE_HOME = str(_HOME / ".cache")
$XDG_DATA_HOME = str(_HOME / ".local" / "share")
$XDG_STATE_HOME = str(_HOME / ".local" / "state")

$EDITOR = _which("code") or _which("notepad") or "notepad"
$VISUAL = $EDITOR
$PAGER = "less"
$LESS = "-RF -i -j.3 -K -M"
$LESSHISTFILE = "-"
$PYTHONIOENCODING = "utf-8"
$PYTHONDONTWRITEBYTECODE = "0"
$BAT_THEME = "Catppuccin Mocha"
$BAT_STYLE = "full"
$BAT_PAGER = "less -RF"
$EZA_ICONS_AUTO = "1"
$ZOXIDE_NO_MSG = "1"
$_ZO_ECHO = "1"
$_ZO_RESOLVE_SYMLINKS = "1"
$_ZO_FZF_OPTS = "--height 15 --min-height 10 --reverse --preview-window=right:40% --border"
$CARAPACE_BRIDGES = "zsh,fish,bash,inshellisense"
if _which("bat"):
    $MANPAGER = "bat -l man -p"
    $MANWIDTH = "999"

$FZF_DEFAULT_OPTS = (
    " --height=15 --min-height=10 --layout=reverse --border=rounded --info=inline"
    " --color=bg+:#363a4f,bg:#1e1e2e,spinner:#f5e0dc,hl:#f38ba8"
    " --color=fg:#cdd6f4,header:#f38ba8,info:#cba6f7,pointer:#f5e0dc"
    " --color=marker:#b4befe,fg+:#cdd6f4,prompt:#cba6f7,hl+:#f38ba8"
    " --color=selected-bg:#45475a,border:#89b4fa"
)
if _which("fd"):
    $FZF_DEFAULT_COMMAND = "fd --type f --hidden --follow --exclude .git --exclude node_modules --exclude .venv"

# ── 2. Prompt — cached Oh My Posh ────────────────────────────────────────────
_POSH_THEME = _HOME / ".config" / "oh-my-posh" / "half-life.omp.json"
_OMP_CACHE = _HOME / ".cache" / "oh-my-posh" / "init-xonsh-half-life.xsh"
$POSH_THEME = str(_POSH_THEME)

def _omp_is_stale():
    if not _OMP_CACHE.exists():
        return True
    cache_mtime = _OMP_CACHE.stat().st_mtime
    refs = [_which("oh-my-posh")]
    if _POSH_THEME.exists():
        refs.append(str(_POSH_THEME))
    for ref in refs:
        try:
            if ref and Path(ref).stat().st_mtime > cache_mtime:
                return True
        except OSError:
            pass
    return False

if _which("oh-my-posh"):
    try:
        if _omp_is_stale():
            _OMP_CACHE.parent.mkdir(parents=True, exist_ok=True)
            _out = subprocess.run(
                ["oh-my-posh", "init", "xonsh", "--config", str(_POSH_THEME)],
                capture_output=True, text=True, check=True,
            ).stdout
            _OMP_CACHE.write_text(_out, encoding="utf-8")
        # Generated xonsh integrations must run in Xonsh's persistent context,
        # not the transient local context that a bare `execx` creates.
        execx(
            _OMP_CACHE.read_text(encoding="utf-8"),
            "exec",
            __xonsh__.ctx,
            filename=str(_OMP_CACHE),
        )
    except Exception:
        $PROMPT = "{BOLD_GREEN}{user}@{hostname}{RESET} {BOLD_BLUE}{cwd}{RESET}\n{BOLD_MAGENTA}❯{RESET} "
else:
    $PROMPT = "{BOLD_GREEN}{user}@{hostname}{RESET} {BOLD_BLUE}{cwd}{RESET}\n{BOLD_MAGENTA}❯{RESET} "

# ── 3. zoxide (cached init script for instant startup & on_chdir hook) ───────
_ZOXIDE_CACHE = _HOME / ".cache" / "zoxide" / "init-xonsh.xsh"
_zoxide_exe = _which("zoxide")

if _zoxide_exe:
    try:
        if (not _ZOXIDE_CACHE.exists()) or (Path(_zoxide_exe).stat().st_mtime > _ZOXIDE_CACHE.stat().st_mtime):
            _ZOXIDE_CACHE.parent.mkdir(parents=True, exist_ok=True)
            _out = subprocess.run(["zoxide", "init", "xonsh", "--cmd", "cd"], capture_output=True, text=True, check=True).stdout
            _extra = "\nbuiltins.aliases['z'] = __zoxide_z\nbuiltins.aliases['zi'] = __zoxide_zi\n"
            _ZOXIDE_CACHE.write_text(_out + _extra, encoding="utf-8")
        execx(_ZOXIDE_CACHE.read_text(encoding="utf-8"), "exec", __xonsh__.ctx, filename=str(_ZOXIDE_CACHE))
    except Exception:
        pass

# ── 4. History ───────────────────────────────────────────────────────────────
$XONSH_HISTORY_SIZE = "100000 commands"
$XONSH_HISTORY_FILE = str(_HOME / ".local" / "share" / "xonsh" / "xonsh_history.json")
$XONSH_SHOW_TRACEBACK = False
$XONSH_STDERR_PREFIX = ""
$XONSH_STDERR_POSTFIX = ""
Path($XONSH_HISTORY_FILE).parent.mkdir(parents=True, exist_ok=True)

# ── 5. Aliases ───────────────────────────────────────────────────────────────
# ls family (eza when available)
if _which("eza"):
    aliases["ls"] = "eza --icons=auto --group-directories-first"
    aliases["la"] = "eza --icons=auto --group-directories-first"
    aliases["ll"] = "eza --all --long --header --git --icons=auto --group-directories-first"
    aliases["lt"] = "eza --tree --level=2 --icons=auto"
else:
    aliases["ll"] = "ls -la"

if _which("bat"):
    aliases["cat"] = "bat --paging=never"
if _which("rg"):
    aliases["grep"] = "rg"
if _which("fd"):
    aliases["find"] = "fd"
if _which("lazygit"):
    aliases["lg"] = "lazygit"
if _which("lazydocker"):
    aliases["ldk"] = "lazydocker"
if _which("btop"):
    aliases["top"] = "btop"
if _which("fastfetch"):
    aliases["fetch"] = "fastfetch"
    aliases["ffetch"] = "fastfetch"
    aliases["neofetch"] = "fastfetch"

aliases["vim"] = $EDITOR

# git
for _name, _cmd in {
    "gs": "git status", "gc": "git commit -m",
    "gp": "git push", "gpl": "git pull", "gcl": "git clone",
    "gl": "git log --oneline --graph --decorate -20",
    "gd": "git diff", "gds": "git diff --staged",
    "gb": "git branch", "gco": "git checkout", "gsw": "git switch",
    "gr": "git rebase", "gst": "git stash", "gstp": "git stash pop",
}.items():
    aliases[_name] = _cmd

def ga():
    """Stage the current directory, matching the PowerShell `ga` helper."""
    return subprocess.run(["git", "add", "."])

aliases["ga"] = ga
aliases["gpush"] = "git push"
aliases["gpull"] = "git pull"

# ── 6. Functions (on-demand only) ────────────────────────────────────────────
def _has(cmd):
    return _which(cmd) is not None

def mkcd(path):
    Path(path).mkdir(parents=True, exist_ok=True)
    os.chdir(path)

def ff(pattern=".", *args):
    if _has("fd"):
        subprocess.run(["fd", pattern, *args])
    else:
        for p in Path(".").rglob(pattern):
            print(p)

def cpy(*text):
    subprocess.run(["clip.exe"], input=" ".join(text), text=True)

def pst():
    out = subprocess.run(["powershell", "-NoProfile", "-Command", "Get-Clipboard"], capture_output=True, text=True)
    return out.stdout.strip()

def pubip():
    out = subprocess.run(["curl", "-s", "https://ifconfig.me"], capture_output=True, text=True)
    print(out.stdout.strip())

def uptime():
    subprocess.run(["powershell", "-NoProfile", "-Command", "(Get-Date) - (Get-CimInstance Win32_OperatingSystem).LastBootUpTime"])

def df():
    subprocess.run(["powershell", "-NoProfile", "-Command", "Get-Volume | Format-Table DriveLetter,FileSystemLabel,SizeRemaining,Size"])

def flushdns():
    subprocess.run(["ipconfig", "/flushdns"])

def _lockmyscreen():
    subprocess.run(["rundll32.exe", "user32.dll,LockWorkStation"])

def _delay_seconds(time):
    """Shared duration parser: plain seconds or a 20s/10m/2h duration."""
    value = str(time).strip().lower()
    units = {"s": 1, "m": 60, "h": 3600}
    try:
        if value.isdecimal():
            return int(value)
        return int(value[:-1]) * units[value[-1]]
    except (KeyError, ValueError):
        raise ValueError("invalid time: use plain seconds, 20s, 10m, or 2h")


def _start_delayed_action(action, seconds):
    """Lock, sleep and sign-out have no native delayed form, so a detached
    hidden powershell.exe performs the wait. Detached so closing xonsh does
    not cancel the pending action, which matches shutdown.exe's own
    behaviour. The trailing #hkdevs-delayed-<action> token is the plaintext
    tag the cancel helpers hunt for. Plain tokens only (none has a space):
    powershell.exe treats everything after -Command as one script text, so
    the list joins to  Start-Sleep -Seconds N ; <action> ;
    #hkdevs-delayed-<action>."""
    inner = {
        "lock": "'rundll32.exe','user32.dll,LockWorkStation'",
        "sleep": "'rundll32.exe','powrprof.dll,SetSuspendState','0,1,0'",
        "signout": "'logoff.exe'",
    }[action]
    script = (
        "Start-Process powershell -WindowStyle Hidden -ArgumentList "
        "'-NoProfile','-Command','Start-Sleep','-Seconds'," + str(int(seconds)) +
        ",';'," + inner + ",'" + "#hkdevs-delayed-" + action + "'"
    )
    kw = {"creationflags": subprocess.CREATE_NO_WINDOW} if _IS_WINDOWS else {}
    subprocess.Popen(["powershell.exe", "-NoProfile", "-Command", script], **kw)


def _stop_delayed_workers(action):
    """Kill hidden powershell.exe wait workers tagged with
    #hkdevs-delayed-<action>; returns how many were killed (0 = none
    pending). The probe runs through -EncodedCommand, so its own command
    line carries the tag only inside base64 and can never self-match."""
    script = (
        "$c = 0; Get-CimInstance Win32_Process -Filter \"Name = 'powershell.exe' "
        "AND CommandLine LIKE '%#hkdevs-delayed-" + action +
        "%'\" -ErrorAction SilentlyContinue | "
        "ForEach-Object { Stop-Process -Id $_.ProcessId -Force; $c++ }; exit $c"
    )
    encoded = base64.b64encode(script.encode("utf-16-le")).decode("ascii")
    kw = {"creationflags": subprocess.CREATE_NO_WINDOW} if _IS_WINDOWS else {}
    proc = subprocess.run(
        ["powershell.exe", "-NoProfile", "-EncodedCommand", encoded], **kw
    )
    return proc.returncode if proc.returncode > 0 else 0


def _shutdown(time=None):
    """Schedule a forced shutdown; `shutdown cancel` aborts a pending one
    (shutdown.exe /a covers both shutdown and restart schedules)."""
    if time is not None and str(time).strip().lower() == "cancel":
        proc = subprocess.run(["shutdown.exe", "/a"], capture_output=True, text=True)
        print("Pending shutdown cancelled." if proc.returncode == 0
              else "No pending shutdown to cancel.")
        return
    if time is None:
        print("usage: shutdown [seconds | 20s|10m|2h | cancel]")
        return
    subprocess.run(["shutdown.exe", "/s", "/f", "/t", str(_delay_seconds(time))])


def _reboot(time=None):
    """Schedule a restart; `reboot cancel` aborts a pending schedule."""
    if time is not None and str(time).strip().lower() == "cancel":
        proc = subprocess.run(["shutdown.exe", "/a"], capture_output=True, text=True)
        print("Pending restart cancelled." if proc.returncode == 0
              else "No pending restart to cancel.")
        return
    if time is None:
        print("usage: reboot [seconds | 20s|10m|2h | cancel]")
        return
    subprocess.run(["shutdown.exe", "/r", "/f", "/t", str(_delay_seconds(time))])


def _lock(time=None):
    """Lock after a delay; `lock cancel` aborts; a bare `lock` locks now."""
    try:
        if time is not None and str(time).strip().lower() == "cancel":
            killed = _stop_delayed_workers("lock")
            if killed:
                print(f"Cancelled {killed} delayed lock job(s).")
            else:
                print("No delayed lock pending.")
            return
        if time is None:
            _lockmyscreen()
            return
        seconds = _delay_seconds(time)
    except ValueError as exc:
        print(exc)
        return
    _start_delayed_action("lock", seconds)


def _sleep(time=None):
    """Suspend after a delay; `sleep cancel` aborts the pending one."""
    try:
        if time is not None and str(time).strip().lower() == "cancel":
            killed = _stop_delayed_workers("sleep")
            if killed:
                print(f"Cancelled {killed} delayed sleep job(s).")
            else:
                print("No delayed sleep pending.")
            return
        if time is None:
            print("usage: sleep [seconds | 20s|10m|2h | cancel]")
            return
        seconds = _delay_seconds(time)
    except ValueError as exc:
        print(exc)
        return
    _start_delayed_action("sleep", seconds)


def _signout(time=None):
    """Sign out; `signout <time>` delays, a bare `signout` signs off now and
    `signout cancel` aborts the pending delayed sign-out."""
    try:
        if time is not None and str(time).strip().lower() == "cancel":
            killed = _stop_delayed_workers("signout")
            if killed:
                print(f"Cancelled {killed} delayed sign-out job(s).")
            else:
                print("No delayed sign-out pending.")
            return
        if time is None:
            subprocess.run(["logoff.exe"])
            return
        seconds = _delay_seconds(time)
    except ValueError as exc:
        print(exc)
        return
    _start_delayed_action("signout", seconds)

def _logout(time=None):
    """Alias of signout: `logout <time>` delays, a bare `logout` signs off
    now, and `logout cancel` aborts — identical to signout in every form."""
    _signout(time)

def _restart(time=None):
    """Alias of reboot: `restart <time>` schedules and `restart cancel`
    aborts — identical to reboot in every form."""
    _reboot(time)

def winutil():
    subprocess.run(["powershell", "-NoProfile", "-Command", "irm https://christitus.com/win | iex"])

def winutildev():
    subprocess.run(["powershell", "-NoProfile", "-Command", "irm https://christitus.com/windev | iex"])

def trash(path):
    target = Path(path).resolve()
    subprocess.run([
        "powershell", "-NoProfile", "-Command",
        f"Add-Type -AssemblyName Microsoft.VisualBasic; [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteFile('{target}', 'OnlyErrorDialogs', 'SendToRecycleBin')",
    ])

def admin(*args):
    if args:
        subprocess.run(["powershell", "-Command", f"Start-Process {' '.join(args)} -Verb RunAs"])
    else:
        subprocess.run(["powershell", "-Command", "Start-Process pwsh -Verb RunAs"])

def su():
    """Open this Xonsh runtime elevated, preserving the current directory."""
    import ctypes
    import sys
    arguments = subprocess.list2cmdline(["-m", "xonsh"])
    result = ctypes.windll.shell32.ShellExecuteW(
        None, "runas", sys.executable, arguments, os.getcwd(), 1
    )
    if result <= 32:
        print(f"unable to start elevated xonsh (ShellExecuteW error {result})")

def _show_help():
    print("""
  kew / cliamp     music TUIs            music      mpd + ncmpc
  fetch            fastfetch
  gs ga gc gp      git shortcuts         ll/la/lt   eza listings
  mkcd DIR         make + enter          ff PAT     fd search
  cpy TEXT         clipboard copy        pst        clipboard paste
  z DIR / zi       zoxide jump           admin CMD  elevated shell
  pubip            public IP             df         volumes
  signout TIME     delayed sign-off; bare logs off now
  logout/restart   aliases of signout/reboot (cancels too)
  <cmd> cancel     abort pending shutdown/reboot/lock/sleep/signout
""")

aliases["su"] = su

# Expose the delayed-verb functions as REAL shell commands (`signout cancel`,
# `logout 600`, bare `lock`, ...). The defs above are underscore-prefixed on
# purpose: xonsh only falls back to subprocess mode on NameError, so a bare
# one-token command whose name exists in ctx would evaluate as a Python
# expression (echoing a repr) instead of running — the public command names
# must live only in the aliases mapping. Xonsh invokes alias callables as
# fn(args, stdin), so the adapter unpacks the argument list into the
# functions' scalar `time` parameter and turns a stray ValueError/TypeError
# into the friendly invalid-time/usage message. The underlying executables
# stay reachable in full form, e.g. `shutdown.exe /s`.
def _verb_alias(vfn):
    def _shell(args, stdin=None):
        try:
            vfn(*args)
        except (ValueError, TypeError) as exc:
            print(exc)
    return _shell

for _pub, _f in {
    "shutdown": _shutdown,
    "reboot": _reboot,
    "lock": _lock,
    "sleep": _sleep,
    "signout": _signout,
    "logout": _logout,
    "restart": _restart,
    "lockmyscreen": _lockmyscreen,
    "show_help": _show_help,
    "help": _show_help,
}.items():
    aliases[_pub] = _verb_alias(_f)
del _pub, _f, _verb_alias
aliases[".."] = "cd .."
aliases["..."] = "cd ../.."

# ── 7. Uniform surface with pwsh / nu ─────────────────────────────────────────
def which(name):
    return shutil.which(name)

def export(name, value):
    """Set an environment variable for this Xonsh session."""
    __xonsh__.env[name] = value

def update_powershell():
    """Upgrade PowerShell through winget, matching the profile helper."""
    return subprocess.run([
        "winget", "upgrade", "--id", "Microsoft.PowerShell", "--exact",
        "--accept-source-agreements", "--accept-package-agreements",
    ])

def clear_cache():
    """Remove the same Windows cache contents as the PowerShell helper."""
    roots = [
        Path(os.environ.get("SystemRoot", r"C:\Windows")) / "Prefetch",
        Path(os.environ.get("SystemRoot", r"C:\Windows")) / "Temp",
        Path(os.environ.get("TEMP", str(_HOME / "AppData" / "Local" / "Temp"))),
        Path(os.environ.get("LOCALAPPDATA", str(_HOME / "AppData" / "Local"))) / "Microsoft" / "Windows" / "INetCache",
    ]
    for root in roots:
        try:
            for child in root.iterdir():
                try:
                    if child.is_symlink() or child.is_file():
                        child.unlink()
                    else:
                        shutil.rmtree(child)
                except OSError:
                    pass
        except OSError:
            pass

def invoke_profile():
    """Reload the Xonsh rc in its persistent interactive context."""
    rc = Path(os.environ.get("XONSHRC", str(_HOME / ".xonshrc")))
    execx(rc.read_text(encoding="utf-8"), "exec", __xonsh__.ctx, filename=str(rc))

aliases["Update-PowerShell"] = update_powershell
aliases["Clear-Cache"] = clear_cache
aliases["Invoke-Profile"] = invoke_profile

def sed(file, find, replace):
    target = Path(file)
    target.write_text(target.read_text(encoding="utf-8").replace(find, replace), encoding="utf-8")

def head(file, n=10):
    with open(file, encoding='utf-8', errors='replace') as fh:
        for i, line in enumerate(fh):
            if i >= n:
                break
            print(line, end='')

def tail(file, n=10):
    with open(file, encoding='utf-8', errors='replace') as fh:
        for line in fh.readlines()[-n:]:
            print(line, end='')

def nf(name):
    Path(name).touch()

def unzip(file, dest='.'):
    subprocess.run(['tar', '-xf', file, '-C', dest])

def sysinfo():
    subprocess.run(['powershell', '-NoProfile', '-Command', 'Get-ComputerInfo'])

def pgrep(name):
    subprocess.run(['powershell', '-NoProfile', '-Command', f"Get-Process -Name '*{name}*' -ErrorAction SilentlyContinue"])

def pkill(name):
    subprocess.run(['powershell', '-NoProfile', '-Command', f"Get-Process -Name '*{name}*' -ErrorAction SilentlyContinue | Stop-Process -Force"])

def k9(name):
    pkill(name)

def docs():
    os.chdir(_HOME / 'Documents')

def dtop():
    os.chdir(_HOME / 'Desktop')

def profile():
    os.chdir(_HOME / '.config' / 'xonsh')

def ep():
    subprocess.run([os.environ.get('EDITOR', 'notepad'), str(_HOME / '.xonshrc')])

def g(*args):
    exe = _which('zoxide')
    if exe:
        hit = subprocess.run([exe, 'query', *(args or ('github',))], capture_output=True, text=True)
        target = hit.stdout.strip()
        if target:
            os.chdir(target)
            return
    fallback = _HOME / 'github'
    if fallback.exists():
        os.chdir(fallback)

def gcom(*message):
    msg = ' '.join(message)
    subprocess.run(['git', 'add', '.'])
    subprocess.run(['git', 'commit', '-m', msg])

def lazyg(*message):
    gcom(*message)
    subprocess.run(['git', 'push'])

# ── 8. Music — mpd under ~/Music + the ncmpc TUI (same `music` as pwsh/nu) ───
import time as _time

def _mpd_up():
    import socket
    try:
        _s = socket.create_connection(("127.0.0.1", 6600), 0.4)
        _s.close()
        return True
    except OSError:
        return False

def music(*args):
    """Start mpd under ~/Music (if needed) and open the ncmpc TUI."""
    music_dir = _HOME / "Music"
    if not _mpd_up():
        _kw = {"cwd": str(music_dir), "stdout": subprocess.DEVNULL, "stderr": subprocess.DEVNULL}
        if _IS_WINDOWS:
            _kw["creationflags"] = subprocess.CREATE_NO_WINDOW
        subprocess.Popen(["mpd"], **_kw)
        for _ in range(15):
            _time.sleep(0.3)
            if _mpd_up():
                break
    if not _mpd_up():
        print("mpd is not responding on 127.0.0.1:6600")
        return
    prev = os.getcwd()
    os.chdir(str(music_dir))
    try:
        subprocess.run(["ncmpc", *args])
    finally:
        os.chdir(prev)

aliases["music"] = music


# ── 9. FZF — Interactive Autocompletion, Auto-Suggestions & Keybindings ──────
$XONSH_PROMPT_AUTO_SUGGEST = True
$AUTO_SUGGEST_IN_COMPLETIONS = True
$FUZZY_PATH_COMPLETION = True
$SUBSEQUENCE_PATH_COMPLETION = True
$COMPLETIONS_DISPLAY = "multi"
$COMPLETION_MODE = "menu-complete"
$COMPLETIONS_MENU_ROWS = 10

def _fzf_pick(items, *extra_args):
    """Run fzf with items on stdin and return the selected line."""
    if not items:
        return ""
    input_text = "\n".join(items) if isinstance(items, (list, tuple, set)) else str(items)
    cmd = ["fzf", "--height=15", "--min-height=10", "--layout=reverse", "--border=rounded", *extra_args]
    try:
        proc = subprocess.run(cmd, input=input_text, text=True, capture_output=True, check=False)
        return proc.stdout.strip()
    except Exception:
        return ""

def _fzf_history_runner():
    """Fuzzy history search."""
    history_file = Path(os.environ.get("XONSH_HISTORY_FILE", _HOME / ".local" / "share" / "xonsh" / "xonsh_history.json"))
    items = []
    if hasattr(__xonsh__, "history") and __xonsh__.history:
        try:
            items = [str(x) for x in __xonsh__.history.items()]
        except Exception:
            pass
    if not items and history_file.exists():
        try:
            import json
            data = json.loads(history_file.read_text(encoding="utf-8"))
            if isinstance(data, list):
                items = [x.get("inp", "").strip() for x in data if isinstance(x, dict) and x.get("inp")]
            elif isinstance(data, dict) and "cmds" in data:
                items = [x.get("inp", "").strip() for x in data["cmds"] if isinstance(x, dict) and x.get("inp")]
        except Exception:
            pass
    seen = set()
    dedup = []
    for item in reversed(items):
        item = item.strip()
        if item and item not in seen:
            seen.add(item)
            dedup.append(item)
    return _fzf_pick(dedup, "--prompt=History❯ ", "--scheme=history")

# Prompt Toolkit Keybindings for interactive shell
try:
    from xonsh.shells.ptk_shell import events as _ptk_events
    from prompt_toolkit.application.run_in_terminal import run_in_terminal
    from prompt_toolkit.completion import CompleteEvent

    @_ptk_events.on_ptk_create
    def _register_fzf_ptk_bindings(bindings, **kwargs):
        @bindings.add("c-r")
        def _fzf_history_key(event):
            def _run():
                h_strings = []
                try:
                    if hasattr(event.current_buffer, "history"):
                        h_strings = list(event.current_buffer.history.get_strings())
                except Exception:
                    pass
                if not h_strings:
                    raw_h = _fzf_history_runner()
                    if raw_h:
                        h_strings = raw_h.splitlines()
                seen = set()
                dedup = []
                for item in reversed(h_strings):
                    item = item.strip()
                    if item and item not in seen:
                        seen.add(item)
                        dedup.append(item)
                curr_text = event.current_buffer.text.strip()
                query_args = [f"--query={curr_text}"] if curr_text else []
                selected = _fzf_pick(dedup, "--prompt=History❯ ", "--scheme=history", *query_args)
                if selected:
                    event.current_buffer.text = selected
                    event.current_buffer.cursor_position = len(selected)
            run_in_terminal(_run)

        @bindings.add("c-t")
        def _fzf_file_key(event):
            def _run():
                f_cmd = ["fd", "--type", "f", "--hidden", "--follow", "--exclude", ".git"] if _which("fd") else ["dir", "/b", "/s"]
                try:
                    p = subprocess.run(f_cmd, capture_output=True, text=True, check=False)
                    files = [f.strip() for f in p.stdout.splitlines() if f.strip()]
                except Exception:
                    files = []
                selected = _fzf_pick(files, "--prompt=File❯ ")
                if selected:
                    quoted = f'"{selected}"' if " " in selected else selected
                    event.current_buffer.insert_text(f"{quoted} ")
            run_in_terminal(_run)

        @bindings.add("escape", "c")
        def _fzf_cd_key(event):
            def _run():
                d_cmd = ["fd", "--type", "d", "--hidden", "--follow", "--exclude", ".git"] if _which("fd") else ["dir", "/b", "/s", "/ad"]
                try:
                    p = subprocess.run(d_cmd, capture_output=True, text=True, check=False)
                    dirs = [d.strip() for d in p.stdout.splitlines() if d.strip()]
                except Exception:
                    dirs = []
                preview = "--preview=eza --tree --level=2 --color=always {}" if _which("eza") else "--preview=dir {}"
                selected = _fzf_pick(dirs, "--prompt=Cd❯ ", preview)
                if selected and os.path.isdir(selected):
                    os.chdir(selected)
            run_in_terminal(_run)

        @bindings.add("c-space")
        def _fzf_complete_key(event):
            def _run():
                buf = event.current_buffer
                completer = buf.completer
                if completer is None:
                    return
                completions = list(completer.get_completions(buf.document, CompleteEvent(completion_requested=True)))
                if not completions:
                    f_cmd = ["fd", "--hidden", "--follow", "--exclude", ".git"] if _which("fd") else ["dir", "/b"]
                    try:
                        p = subprocess.run(f_cmd, capture_output=True, text=True, check=False)
                        items = [f.strip() for f in p.stdout.splitlines() if f.strip()]
                    except Exception:
                        items = []
                    selected = _fzf_pick(items, "--prompt=Complete❯ ")
                    if selected:
                        quoted = f'"{selected}"' if " " in selected else selected
                        buf.insert_text(f"{quoted} ")
                    return

                if len(completions) == 1:
                    buf.apply_completion(completions[0])
                    return

                items_map = {}
                display_lines = []
                for c in completions:
                    meta = f"  # {c.display_meta_text}" if getattr(c, "display_meta_text", None) else ""
                    key = f"{c.text}{meta}"
                    items_map[key] = c
                    display_lines.append(key)

                selected = _fzf_pick(display_lines, "--prompt=Complete❯ ")
                if selected and selected in items_map:
                    buf.apply_completion(items_map[selected])
            run_in_terminal(_run)
except Exception:
    pass

# ── 10. Carapace — Universal Multi-Shell Completer ─────────────────────────
if _which("carapace"):
    try:
        from xonsh.completers.completer import add_one_completer
        from xonsh.completers.tools import contextual_command_completer, RichCompletion
        import json

        # Bridge callbacks used by carapace internally
        def _carapace_getenv(args):
            key = args[0] if args else ""
            if key in __xonsh__.env:
                print(__xonsh__.env[key])
        aliases['get-env'] = _carapace_getenv

        def _carapace_setenv(args):
            if len(args) >= 2:
                __xonsh__.env[args[0]] = args[1]
                os.environ[args[0]] = args[1]
        aliases['set-env'] = _carapace_setenv

        def _carapace_unsetenv(args):
            if args:
                __xonsh__.env.pop(args[0], None)
                os.environ.pop(args[0], None)
        aliases['unset-env'] = _carapace_unsetenv

        @contextual_command_completer
        def _carapace_completer(context):
            """Universal Carapace Completer with alias expansion and native fallback."""
            if not context or context.arg_index <= 0 or not context.command:
                return None

            clean_cmd = context.command
            if clean_cmd.lower().endswith(".exe"):
                clean_cmd = clean_cmd[:-4]

            # Alias expansion
            alias_val = aliases.get(clean_cmd)
            extra_prefix_args = []
            if isinstance(alias_val, (list, tuple)):
                parts = [str(x) for x in alias_val]
                if parts:
                    clean_cmd = parts[0]
                    if clean_cmd.lower().endswith(".exe"):
                        clean_cmd = clean_cmd[:-4]
                    extra_prefix_args = parts[1:]
            elif isinstance(alias_val, str):
                parts = alias_val.split()
                if parts:
                    clean_cmd = parts[0]
                    if clean_cmd.lower().endswith(".exe"):
                        clean_cmd = clean_cmd[:-4]
                    extra_prefix_args = parts[1:]

            # Collect arguments before cursor
            words = [a.value for a in context.args[1:context.arg_index]]
            current_prefix = context.prefix.strip("'\"")

            cmd = ["carapace", clean_cmd, "xonsh", *extra_prefix_args, *words, current_prefix]
            try:
                proc = subprocess.run(cmd, capture_output=True, text=True, check=False)
                if proc.returncode != 0 or not proc.stdout.strip():
                    return None
                data = json.loads(proc.stdout)
                if not data:
                    return None
                results = set()
                raw_len = len(context.raw_prefix)
                for item in data:
                    val = item.get("Value", "")
                    disp = item.get("Display", val)
                    desc = item.get("Description", "")
                    style = item.get("Style", "")
                    results.add(RichCompletion(val, display=disp, description=desc, prefix_len=raw_len, append_closing_quote=False, style=style))
                return results if results else None
            except Exception:
                return None

        add_one_completer('carapace', _carapace_completer, 'start')
    except Exception:
        pass

# --- Standalone CLI Commands for Xonsh ---
def fcd(*args):
    """Fuzzy directory jumper with preview."""
    target = args[0] if args else "."
    d_cmd = ["fd", "--type", "d", "--hidden", "--follow", "--exclude", ".git", ".", str(target)] if _which("fd") else ["dir", "/b", "/s", "/ad", str(target)]
    try:
        p = subprocess.run(d_cmd, capture_output=True, text=True, check=False)
        dirs = [d.strip() for d in p.stdout.splitlines() if d.strip()]
    except Exception:
        dirs = []
    preview = "--preview=eza --tree --level=2 --color=always {}" if _which("eza") else "--preview=dir {}"
    selected = _fzf_pick(dirs, "--prompt=Cd❯ ", preview)
    if selected and os.path.isdir(selected):
        os.chdir(selected)

def fe(*args):
    """Fuzzy file finder & editor."""
    f_cmd = ["fd", "--type", "f", "--hidden", "--follow", "--exclude", ".git", *args] if _which("fd") else ["dir", "/b", "/s"]
    try:
        p = subprocess.run(f_cmd, capture_output=True, text=True, check=False)
        files = [f.strip() for f in p.stdout.splitlines() if f.strip()]
    except Exception:
        files = []
    preview = "--preview=bat --color=always --line-range :100 {}" if _which("bat") else "--preview=type {}"
    selected = _fzf_pick(files, "-m", "--prompt=Edit❯ ", preview)
    if selected:
        editor = os.environ.get("EDITOR", "notepad")
        subprocess.run([editor, *selected.splitlines()])

def fkill():
    """Fuzzy process killer."""
    p = subprocess.run(["powershell", "-NoProfile", "-Command", "Get-Process | Select-Object Id, ProcessName, CPU, WorkingSet64 | ForEach-Object { \"$($_.Id)`t$($_.ProcessName)`t$($_.CPU)`t$([math]::Round($_.WorkingSet64 / 1MB, 2))MB\" }"], capture_output=True, text=True, check=False)
    lines = [x.strip() for x in p.stdout.splitlines() if x.strip()]
    selected = _fzf_pick(lines, "-m", "--prompt=Kill❯ ", "--header=PID\tNAME\tCPU\tMEM")
    if selected:
        for line in selected.splitlines():
            pid = line.split("\t")[0].strip()
            if pid.isdigit():
                subprocess.run(["powershell", "-NoProfile", "-Command", f"Stop-Process -Id {pid} -Force"])
                print(f"Killed PID {pid}")

def fhistory():
    """Interactive command history runner."""
    selected = _fzf_history_runner()
    if selected:
        print(f"Executing: {selected}")
        execx(selected, "exec", __xonsh__.ctx)

def fgit():
    """Fuzzy git branch switcher."""
    p = subprocess.run(["git", "branch", "-a", "--color=always"], capture_output=True, text=True, check=False)
    branches = [b.strip() for b in p.stdout.splitlines() if b.strip()]
    selected = _fzf_pick(branches, "--ansi", "--prompt=Branch❯ ")
    if selected:
        clean = selected.lstrip("* ").strip()
        if clean.startswith("remotes/"):
            clean = "/".join(clean.split("/")[2:])
        subprocess.run(["git", "switch", clean])

def fgit_log():
    """Fuzzy git commit browser with preview."""
    p = subprocess.run(["git", "log", "--oneline", "--color=always"], capture_output=True, text=True, check=False)
    lines = [l.strip() for l in p.stdout.splitlines() if l.strip()]
    selected = _fzf_pick(lines, "--ansi", "--prompt=Commit❯ ", "--preview=git show --color=always {1}")
    if selected:
        commit_hash = selected.split()[0]
        subprocess.run(["git", "show", commit_hash])

def fenv():
    """Fuzzy environment variable viewer."""
    lines = [f"{k}={v}" for k, v in __xonsh__.env.items()]
    _fzf_pick(lines, "--prompt=Env❯ ")

aliases["fcd"] = fcd
aliases["fe"] = fe
aliases["fkill"] = fkill
aliases["fhistory"] = fhistory
aliases["fh"] = fhistory
aliases["fgit"] = fgit
aliases["fgl"] = fgit_log
aliases["fenv"] = fenv

def window_rice(args):
    """Windows Rice TUI installer & real-time manager."""
    target_exe = _HOME / ".config" / "installer" / "target" / "release" / "rice-installer.exe"
    cargo_toml = _HOME / ".config" / "installer" / "Cargo.toml"
    install_ps1 = _HOME / ".config" / "install.ps1"
    ps_bin = "pwsh" if _which("pwsh") else "powershell"

    if any(a in ("--reload", "-r") for a in args):
        print("Reloading Windows Rice Environment...")
        if _which("komorebic"):
            subprocess.run(["komorebic", "replace-configuration", str(_HOME / ".config" / "komorebi" / "komorebi.json")])
        if _which("glazewm"):
            subprocess.run(["glazewm", "command", "wm-reload-config"])
        yasb_setup = _HOME / ".config" / "yasb" / "tools" / "setup" / "yasb-setup.ps1"
        if yasb_setup.exists():
            subprocess.run([ps_bin, "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(yasb_setup)])
        print("Windows Rice environment reloaded!")
        return

    if not target_exe.exists() and cargo_toml.exists() and _which("cargo"):
        print("Building Windows Rice TUI...")
        subprocess.run(["cargo", "build", "--release", "--manifest-path", str(cargo_toml), "--quiet"])

    if target_exe.exists():
        subprocess.run([str(target_exe), *args])
    elif install_ps1.exists():
        subprocess.run([ps_bin, "-NoProfile", "-File", str(install_ps1), *args])
    else:
        print("rice-installer not found.")

aliases["window-rice"] = window_rice
aliases["rice"] = window_rice
