# ==============================================================================
# CONFIG-ADDITIONS.NU — Extra integrations, sourced from config.nu §30
# RULES (Nu 0.114 semantics):
#   • `def`/`alias` inside `if`/`match` blocks are BLOCK-SCOPED — they vanish when
#     the block exits (nushell#8974). Every definition here is therefore top-level.
#   • Tool availability is checked INSIDE each body via `has`, so missing tools
#     print a hint instead of a raw "command not found".
#   • No defs that already exist in config.nu (winutil, lg, lsg, ping, dl, ytdl,
#     go-mod, nr, ni, nrd, nrt are all defined there) — duplicates would raise
#     `duplicate_command_def`.
# ==============================================================================

# ── Completions module (nu-complete functions) ──
source "C:/Users/haris/.config/nushell/completions/mod.nu"

# ── Clipboard ──
def clip-copy [text: string] {
    $text | ^Set-Clipboard
    print "✅ Copied"
}
def clip-paste [] { ^Get-Clipboard }

# ── uv / Python ──
def uv-sync [] {
    if not (has "uv") { print "❌ uv not found: scoop install uv"; return }
    ^uv sync
}
def uv-add [...packages: string] {
    if not (has "uv") { print "❌ uv not found: scoop install uv"; return }
    ^uv add ...$packages
}
def uv-run [...args: string] {
    if not (has "uv") { print "❌ uv not found: scoop install uv"; return }
    ^uv run ...$args
}

# ── mise — tool version manager ──
def mise-list [] {
    if not (has "mise") { print "❌ mise not found"; return }
    ^mise ls
}
def mise-use [tool: string, version: string] {
    if not (has "mise") { print "❌ mise not found"; return }
    ^mise use $"($tool)@($version)"
}

# ── pnpm ──
def pn [...args: string] {
    if not (has "pnpm") { print "❌ pnpm not found: scoop install pnpm"; return }
    ^pnpm ...$args
}
def pnr [...args: string] {
    if not (has "pnpm") { print "❌ pnpm not found: scoop install pnpm"; return }
    ^pnpm run ...$args
}
def pni [...args: string] {
    if not (has "pnpm") { print "❌ pnpm not found: scoop install pnpm"; return }
    ^pnpm install ...$args
}

# ── bun ──
def brun [file: string, ...args: string] {
    if not (has "bun") { print "❌ bun not found: scoop install bun"; return }
    ^bun run $file ...$args
}
def btest [...args: string] {
    if not (has "bun") { print "❌ bun not found: scoop install bun"; return }
    ^bun test ...$args
}

# ── deno (run/fmt/lint) ──
def drun [file: string, ...args: string] {
    if not (has "deno") { print "❌ deno not found: scoop install deno"; return }
    ^deno run --allow-all $file ...$args
}
def deno-fmt [...args: string] {
    if not (has "deno") { print "❌ deno not found: scoop install deno"; return }
    ^deno fmt ...$args
}
def deno-lint [...args: string] {
    if not (has "deno") { print "❌ deno not found: scoop install deno"; return }
    ^deno lint ...$args
}

# ── cargo shortcuts (c-prefixed; r-prefixed cargo defs live in config.nu) ──
def crun [...args: string] {
    if not (has "cargo") { print "❌ cargo not found"; return }
    ^cargo run ...$args
}
def ctest [...args: string] {
    if not (has "cargo") { print "❌ cargo not found"; return }
    ^cargo test ...$args
}
def cbuild [...args: string] {
    if not (has "cargo") { print "❌ cargo not found"; return }
    ^cargo build ...$args
}
def crelease [] {
    if not (has "cargo") { print "❌ cargo not found"; return }
    ^cargo build --release
}
def cdoc [] {
    if not (has "cargo") { print "❌ cargo not found"; return }
    ^cargo doc --open
}
def cfmt [] {
    if not (has "cargo") { print "❌ cargo not found"; return }
    ^cargo fmt
}
def cclippy [...args: string] {
    if not (has "cargo") { print "❌ cargo not found"; return }
    ^cargo clippy ...$args
}

# ── npm build (nr/ni/nrd/nrt are in config.nu) ──
def nbuild [] {
    if not (has "npm") { print "❌ npm not found"; return }
    ^npm run build
}

# ── lazydocker wrapper (lazygit lg is in config.nu) ──
def --env ldk [] {
    if not (has "lazydocker") { print "❌ lazydocker not found: scoop install lazydocker"; return }
    ^lazydocker
}

# ── Yazi enhanced — fzf-powered cd ──
def --env fcd [...args: string] {
    if not (has "fd") or not (has "fzf") { print "❌ fd + fzf required"; return }
    let dir = (^fd --type d --hidden --follow --exclude .git ...$args | fzf --prompt 'Cd❯ ' --height 40% --layout reverse --preview 'eza --tree --level=2 --color=always {}' | str trim)
    if ($dir | is-not-empty) { cd $dir }
}

# ── Disk usage ──
def diskusage [] {
    if not (has "dua") { print "❌ dua not found: scoop install dua-cli"; return }
    ^dua interactive
}

# ── Nix ──
def nix-shell [...packages: string] {
    if not (has "nix") { print "❌ nix not found"; return }
    ^nix-shell -p ...$packages
}
def nix-flake [...args: string] {
    if not (has "nix") { print "❌ nix not found"; return }
    ^nix flake ...$args
}

# ── Bookmark manager ──
def bookmark [...args: string] {
    if not (has "buku") { print "❌ buku not found: scoop install buku"; return }
    ^buku ...$args
}

# ── Markdown / bat ──
alias glowp = glow --pager
def mdp [file: string] {
    if not (has "glow") { print "❌ glow not found: scoop install glow"; return }
    ^glow $file
}
def batp [file: string] {
    if not (has "bat") { print "❌ bat not found: scoop install bat"; return }
    ^bat --paging=always $file
}
def batn [file: string] {
    if not (has "bat") { print "❌ bat not found: scoop install bat"; return }
    ^bat --number --paging=never $file
}

# ── eza extras (la/ll/lt/lsg live in config.nu) ──
def lg-tree [] {
    if not (has "eza") { print "❌ eza not found: scoop install eza"; return }
    ^eza --long --tree --level=2 --git --icons
}

# ── jq / yq ──
def jsonview [file: string] {
    if not (has "jq") { print "❌ jq not found: scoop install jq"; return }
    open $file | to json -r | ^jq
}

# ── Benchmark ──
def benchmark [...args: string] {
    if not (has "hyperfine") { print "❌ hyperfine not found: scoop install hyperfine"; return }
    ^hyperfine ...$args
}

# ── procs ──
def procs-top [n: int = 10] {
    if not (has "procs") { print "❌ procs not found: scoop install procs"; return }
    ^procs --sort cpu
}

# ── tokei ──
def code-stats [...args: string] {
    if not (has "tokei") { print "❌ tokei not found: scoop install tokei"; return }
    ^tokei ...$args
}

# ── tldr (tealdeer) ──
def tldr [cmd: string] {
    if not (has "tldr") { print "❌ tldr not found: scoop install tldr"; return }
    ^tldr $cmd
}

# ── ffmpeg info ──
def ffmpeg-info [file: string] {
    if not (has "ffprobe") { print "❌ ffprobe not found: scoop install ffmpeg"; return }
    ^ffprobe -hide_banner $file
}

# ── Watchexec ──
def watchx [...args: string] {
    if not (has "watchexec") { print "❌ watchexec not found: scoop install watchexec"; return }
    ^watchexec ...$args
}

# ── just ──
alias j = just

# ── zellij ──
def zj [...args: string] {
    if not (has "zellij") { print "❌ zellij not found: scoop install zellij"; return }
    ^zellij ...$args
}

# ==============================================================================
# UNIFORM COMMAND SURFACE — mirrors the PowerShell profile (native Nu first)
# Same names/behaviour as pwsh and xonsh: git, listings, nav, clipboard,
# process, file and system helpers. No helper scripts are involved.
# ==============================================================================

# ── Listings (eza) ──
alias la = eza --icons=auto --group-directories-first
alias ll = eza --all --long --header --git --icons=auto --group-directories-first
alias lt = eza --tree --level=2 --icons=auto
alias fetch = fastfetch
alias ffetch = fastfetch
alias neofetch = fastfetch
alias grep = rg

# ── Git (same names as pwsh) ──
alias gs = git status
alias gp = git push
alias gpl = git pull
alias gpush = git push
alias gpull = git pull
alias gcl = git clone
alias gl = git log --oneline --graph --decorate -20
alias gd = git diff
alias gds = git diff --staged
alias gb = git branch
alias gco = git checkout
alias gsw = git switch
alias gr = git rebase
alias gst = git stash
alias gstp = git stash pop
alias lg = lazygit

def gc [...message: string] { ^git commit -m ($message | str join ' ') }
def ga [] { ^git add . }

def gcom [...message: string] {
    ^git add .
    ^git commit -m ($message | str join ' ')
}

def lazyg [...message: string] {
    ^git add .
    ^git commit -m ($message | str join ' ')
    ^git push
}

def --env g [] {
    let hit = (^zoxide query github | str trim)
    if ($hit | is-not-empty) { cd $hit } else if (("~/github" | path expand) | path exists) { cd ~/github }
}

# ── Navigation / files ──
def --env mkcd [path: path] {
    mkdir $path
    cd $path
}

def --env docs [] { cd ("~/Documents" | path expand) }
def --env dtop [] { cd ("~/Desktop" | path expand) }
def --env profile [] { cd ("~/.config/nushell" | path expand) }
def ep [] { ^nvim $nu.config-path }

def ff [pattern: string] {
    if (which fd | is-not-empty) { ^fd $pattern } else { ls **/*$pattern* }
}

def head [file: path, n: int = 10] { open $file | lines | first $n }
def tail [file: path, n: int = 10] { open $file | lines | last $n }
def unzip [file: path, dest: path = "."] { ^tar -xf $file -C $dest }

def sed [file: path, find: string, replace: string] {
    open --raw $file | str replace --all $find $replace | save --force $file
}

def trash [target: path] {
    let full = ($target | path expand)
    let head = "Add-Type -AssemblyName Microsoft.VisualBasic; [Microsoft.VisualBasic.FileIO.FileSystem]::"
    let tail = "('" + $full + "', 'OnlyErrorDialogs', 'SendToRecycleBin')"
    if (($target | path type) == "dir") {
        ^powershell -NoProfile -Command ($head + "DeleteDirectory" + $tail)
    } else {
        ^powershell -NoProfile -Command ($head + "DeleteFile" + $tail)
    }
}

alias nf = touch

# ── Clipboard ──
def cpy [text: string] { $text | ^clip.exe }
def pst [] { ^powershell -NoProfile -Command Get-Clipboard }

# ── Processes ──
def pgrep [name: string] { ps | where name =~ $name }
def pkill [name: string] {
    let matches = (ps | where name =~ $name)
    if ($matches | is-empty) { print $"no process matching ($name)" } else { $matches | each { |p| kill --force $p.pid } }
}
alias k9 = pkill

# ── System info ──
def pubip [] { (http get https://ifconfig.me).body | str trim }
def uptime [] { sys host | get uptime }
def df [] { sys disks }
def sysinfo [] { sys }
def flushdns [] { ^ipconfig /flushdns }
def lockmyscreen [] { ^rundll32.exe user32.dll,LockWorkStation }

# The PowerShell verb `export` is a Nushell parser keyword; `set-env` provides
# the same environment-variable operation under a valid native Nu command.
def --env set-env [name: string, value: string] { load-env {($name): $value} }

def update-powershell [] {
    ^winget upgrade --id Microsoft.PowerShell --exact --accept-source-agreements --accept-package-agreements
}

def clear-cache [] {
    let roots = [
        ($env.SystemRoot | path join 'Prefetch')
        ($env.SystemRoot | path join 'Temp')
        $env.TEMP
        ($env.LOCALAPPDATA | path join 'Microsoft' 'Windows' 'INetCache')
    ]
    for root in $roots {
        if ($root | path exists) {
            for item in (glob ($root | path join '*')) {
                try { rm --force --recursive $item } catch { }
            }
        }
    }
}

# ── Delayed system actions (shutdown / reboot / lock / sleep / signout) ──
# Same duration syntax as the PowerShell profile: plain seconds, 20s, 10m, 2h.
def delay-seconds [time: string] {
    let parts = ($time | parse -r '^(?<value>\d+)(?<unit>[smh])$')
    if (($parts | length) == 1) {
        let part = ($parts | first)
        match $part.unit {
            's' => ($part.value | into int),
            'm' => (($part.value | into int) * 60),
            'h' => (($part.value | into int) * 3600)
        }
    } else if ($time =~ '^\d+$') {
        $time | into int
    } else {
        error make {msg: 'invalid time: use plain seconds, 20s, 10m, or 2h'}
    }
}

def shutdown [time: string] {
    ^shutdown.exe /s /f /t (delay-seconds $time)
}

def reboot [time: string] {
    ^shutdown.exe /r /f /t (delay-seconds $time)
}

# Shared cancel printer: shutdown.exe /a aborts any pending shutdown OR
# restart schedule — exit 0 = aborted, 1116 = nothing was pending.
def cancel-pending-shutdown [kind: string] {
    let r = (do { ^shutdown.exe /a } | complete)
    if $kind == 'restart' {
        if ($r.exit_code == 0) {
            print 'Pending restart cancelled.'
        } else {
            print 'No pending restart to cancel.'
        }
    } else {
        if ($r.exit_code == 0) {
            print 'Pending shutdown cancelled.'
        } else {
            print 'No pending shutdown to cancel.'
        }
    }
}

# `shutdown cancel` / `reboot cancel` abort any pending shutdown OR restart
# schedule (they share shutdown.exe /a). Officially defined via Nushell's
# quoted-subcommand form.
def "shutdown cancel" [] {
    cancel-pending-shutdown 'shutdown'
}

def "reboot cancel" [] {
    cancel-pending-shutdown 'restart'
}

# Lock, sleep and sign-out have no native delayed form. A detached hidden
# powershell.exe performs the wait so closing nu does not cancel the pending
# action (an in-process `job spawn` thread would die with this shell). Nu
# quoting stays quote-free: powershell.exe treats everything after -Command
# as the script, and every Start-Process token below contains no spaces. The
# trailing #hkdevs-delayed-* token is the plaintext tag the "… cancel"
# subcommands hunt for.
def start-delayed-system-action [seconds: int, action: string] {
    let worker = (match $action {
        'lock' => "'rundll32.exe','user32.dll,LockWorkStation'"
        'sleep' => "'rundll32.exe','powrprof.dll,SetSuspendState','0,1,0'"
        'signout' => "'logoff.exe'"
    })
    let marker = (match $action {
        'lock' => '#hkdevs-delayed-lock'
        'sleep' => '#hkdevs-delayed-sleep'
        'signout' => '#hkdevs-delayed-signout'
    })
    ^powershell -NoProfile -Command $"Start-Process powershell -WindowStyle Hidden -ArgumentList '-NoProfile','-Command','Start-Sleep','-Seconds',($seconds),';',($worker),';','($marker)'"
}

# Kills the hidden wait workers carrying the #hkdevs-delayed-<action> tag;
# returns how many were cancelled (0 = nothing pending). The WQL filter does
# the whole match with no $ variables, so the text is nu-quote-safe, and the
# probe token keeps plain `powershell -Command` probes from self-matching.
def cancel-delayed [action: string] {
    let filter = ("Name = 'powershell.exe' AND CommandLine LIKE '%hkdevs-delayed-" + $action + "%' AND NOT (CommandLine LIKE '%hk-delayed-probe9f7%')")
    let script = ("Get-CimInstance Win32_Process -Filter \"" + $filter + "\" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty ProcessId")
    let lines = (^powershell -NoProfile -Command $script | lines | where {|l| ($l | str trim | is-not-empty)})
    let ids = ($lines | each {|l| ($l | str trim | into int)})
    for id in $ids {
        ^taskkill /F /PID $id
    }
    ($ids | length)
}

def lock [time: string] {
    start-delayed-system-action (delay-seconds $time) 'lock'
}

def "lock cancel" [] {
    let n = (cancel-delayed 'lock')
    if $n == 0 {
        print 'No delayed lock pending.'
    } else {
        print $"Cancelled ($n) delayed lock jobs."
    }
}

# Overrides nu's builtin `sleep` (which waits, not suspends): this `sleep`
# suspends the system after the delay; use `sleep 5sec` builtin via `nu -c`
# or `run-external` if an in-session wait is ever needed.
def sleep [time: string] {
    start-delayed-system-action (delay-seconds $time) 'sleep'
}

def "sleep cancel" [] {
    let n = (cancel-delayed 'sleep')
    if $n == 0 {
        print 'No delayed sleep pending.'
    } else {
        print $"Cancelled ($n) delayed sleep jobs."
    }
}

# Bare `signout` signs off immediately (logoff.exe); `signout <time>` delays.
def signout [time?: string] {
    if $time == null {
        ^logoff.exe
    } else {
        start-delayed-system-action (delay-seconds $time) 'signout'
    }
}

# Shared impl for `signout cancel` / `logout cancel`.
def cancel-signout [] {
    let n = (cancel-delayed 'signout')
    if $n == 0 {
        print 'No delayed sign-out pending.'
    } else {
        print $"Cancelled ($n) delayed sign-out jobs."
    }
}

def "signout cancel" [] {
    cancel-signout
}

# Memory aliases for signout / reboot. Real wrappers rather than `alias`:
# `logout cancel` / `restart cancel` must dispatch into matching quoted
# subcommands, and aliases cannot carry those extra definitions — while
# runtime arguments inside these wrappers always stay on the normal path.
def logout [time?: string] {
    signout $time
}

def "logout cancel" [] {
    cancel-signout
}

def restart [time: string] {
    reboot $time
}

def "restart cancel" [] {
    cancel-pending-shutdown 'restart'
}

# These mirror the opt-in PowerShell commands.  They do no work until called.
def winutil [] { ^powershell -NoProfile -Command "irm https://christitus.com/win | iex" }
def winutildev [] { ^powershell -NoProfile -Command "irm https://christitus.com/windev | iex" }

# ── Elevated shells ──
def admin [...args: string] {
    if ($args | is-empty) {
        ^powershell -NoProfile -Command "Start-Process pwsh -Verb RunAs"
    } else {
        ^powershell -NoProfile -Command $"Start-Process ($args | str join ' ') -Verb RunAs"
    }
}

def su [] {
    # Start the exact Nu binary that launched this session, in this directory.
    # This is deliberately separate from `admin`, which retains its PowerShell
    # compatibility behaviour.
    ^powershell -NoProfile -Command $"Start-Process -FilePath '($nu.current-exe)' -WorkingDirectory '(pwd)' -Verb RunAs"
}

# ── Help ──
def show-help [] {
    print "  kew / cliamp     music TUIs            fetch      fastfetch"
    print "  gs ga gc gp      git shortcuts         ll/la/lt   eza listings"
    print "  mkcd DIR         make + enter          ff PAT     fd search"
    print "  cpy TEXT         clipboard copy        pst        clipboard paste"
    print "  z DIR / zi       zoxide jump           su         elevated same shell"
    print "  pubip            public IP             df         volumes"
    print "  lockmyscreen     lock now (lock TIME delays; signout TIME signs off)"
    print "  logout/restart   aliases of signout/reboot (cancels too)"
    print "  <cmd> cancel     abort pending shutdown/reboot/lock/sleep/signout"
}
