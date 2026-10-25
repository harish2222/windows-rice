# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: Nushell Fuzzy Finder Interactive Search Integration
# ============================================================================
# ==============================================================================
# FZF.NU — Interactive Fuzzy Completion, Auto-Suggestions & Helpers for Nushell
# ==============================================================================

# --- Interactive REPL Widgets (Invoked via Keybindings) ---

# Shows history matching prompt text with at least 10 entries shown
def --env fzf-history-widget [] {
    let current = (commandline)
    let cmd = (history | get command | uniq | reverse | str join (char nl) | fzf --height 15 --min-height 10 --layout reverse --border rounded --prompt 'History❯ ' --scheme history --query $current | str trim)
    if not ($cmd | is-empty) {
        commandline edit -r $cmd
    }
}

def --env fzf-file-widget [] {
    let file = (try { ^fd --type f --hidden --follow --exclude .git } catch { ^dir /b /s } | fzf --prompt 'File❯ ' --height 15 --min-height 10 --layout reverse --border rounded | str trim)
    if not ($file | is-empty) {
        let quoted = if ($file | str contains ' ') { $"\"($file)\"" } else { $file }
        commandline edit -i $"($quoted) "
    }
}

def --env fzf-cd-widget [] {
    let dir = (try { ^fd --type d --hidden --follow --exclude .git } catch { ^dir /b /s /ad } | fzf --prompt 'Cd❯ ' --height 15 --min-height 10 --layout reverse --border rounded | str trim)
    if not ($dir | is-empty) {
        cd $dir
    }
}

def --env fzf-complete-widget [] {
    let raw_cmd = (commandline)
    let comp = (commandline complete --detailed)
    if ($comp | is-empty) {
        # Fallback to file search if no active argument completions
        let file = (try { ^fd --hidden --follow --exclude .git } catch { ^dir /b /s } | fzf --prompt 'File❯ ' --height 15 --min-height 10 --layout reverse --border rounded | str trim)
        if not ($file | is-empty) {
            let quoted = if ($file | str contains ' ') { $"\"($file)\"" } else { $file }
            commandline edit -i $"($quoted) "
        }
        return
    }

    # If only 1 completion, apply immediately
    if ($comp | length) == 1 {
        let first = ($comp | first)
        let val = $first.value
        let start = $first.span.start
        let end = $first.span.end
        let prefix = if $start > 0 { $raw_cmd | str substring 0..($start - 1) } else { "" }
        let suffix = if $end < ($raw_cmd | str length) { $raw_cmd | str substring $end.. } else { "" }
        commandline edit -r $"($prefix)($val) ($suffix)"
        return
    }

    # Multiple completions: pipe to fzf with description
    let fzf_input = ($comp | each {|it|
        let desc = if ($it | columns | any {|c| $c == "description"}) and ($it.description != null) and not ($it.description | is-empty) {
            $"  # ($it.description)"
        } else { "" }
        $"($it.value)($desc)"
    } | str join (char nl))

    let selected = ($fzf_input | fzf --prompt 'Complete❯ ' --height 15 --min-height 10 --layout reverse --border rounded | split row "  #" | first | str trim)
    if not ($selected | is-empty) {
        let match = ($comp | where value == $selected | first)
        let start = $match.span.start
        let end = $match.span.end
        let prefix = if $start > 0 { $raw_cmd | str substring 0..($start - 1) } else { "" }
        let suffix = if $end < ($raw_cmd | str length) { $raw_cmd | str substring $end.. } else { "" }
        commandline edit -r $"($prefix)($selected) ($suffix)"
    }
}

# --- Standalone CLI Commands ---

# Fuzzy Directory Jumper with eza tree preview
def --env fcd [base_dir: string = "."] {
    let dir = (try {
        ^fd --type d --hidden --follow --exclude .git . $base_dir
    } catch {
        ^dir /b /s /ad $base_dir
    } | fzf --prompt 'Cd❯ ' --height 15 --min-height 10 --layout reverse --border rounded --preview 'eza --tree --level=2 --color=always {}' | str trim)
    if not ($dir | is-empty) {
        cd $dir
    }
}

# Fuzzy File Finder & Editor
def fe [...args: string] {
    let files = (try {
        ^fd --type f --hidden --follow --exclude .git ...$args
    } catch {
        ^dir /b /s
    } | fzf -m --prompt 'Edit❯ ' --height 15 --min-height 10 --layout reverse --border rounded --preview 'bat --color=always --line-range :100 {}' | lines)
    if not ($files | is-empty) {
        let editor = ($env.EDITOR? | default "nvim")
        ^$editor ...$files
    }
}

# Fuzzy Process Killer
def fkill [] {
    let procs = (ps | each {|p| $"($p.pid)\t($p.name)\t($p.cpu)%\t($p.mem)" } | str join (char nl))
    let selected = ($procs | fzf -m --prompt 'Kill❯ ' --header 'PID	NAME	CPU	MEM' --height 15 --min-height 10 --layout reverse --border rounded | lines)
    for line in $selected {
        let pid = ($line | split row "\t" | first | str trim | into int)
        try {
            kill $pid
            print $"Killed PID ($pid)"
        } catch {
            print $"Failed to kill PID ($pid)"
        }
    }
}

# Fuzzy History Explorer & Runner
def fhistory [] {
    let cmd = (history | get command | uniq | reverse | str join (char nl) | fzf --prompt 'History❯ ' --height 15 --min-height 10 --layout reverse --border rounded --scheme history | str trim)
    if not ($cmd | is-empty) {
        print $"Executing: ($cmd)"
        nu -c $cmd
    }
}

# Fuzzy Git Branch Switcher
def fgit-branch [] {
    let branch = (^git branch -a --color=always | fzf --ansi --prompt 'Branch❯ ' --height 15 --min-height 10 --layout reverse --border rounded | str trim | str replace -r '^[\*\s]+' '' | str replace -r '^remotes/[^/]+/' '')
    if not ($branch | is-empty) {
        ^git switch $branch
    }
}

# Fuzzy Git Commit Explorer
def fgit-log [] {
    let hash = (^git log --oneline --color=always | fzf --ansi --prompt 'Commit❯ ' --height 15 --min-height 10 --layout reverse --border rounded --preview 'git show --color=always {1}' | split row " " | first | str trim)
    if not ($hash | is-empty) {
        ^git show $hash
    }
}

# Fuzzy Git Status File Picker
def fgit-status [] {
    let file = (^git status -s | fzf -m --prompt 'GitFile❯ ' --height 15 --min-height 10 --layout reverse --border rounded | each {|it| $it | str substring 3.. | str trim })
    if not ($file | is-empty) {
        let editor = ($env.EDITOR? | default "nvim")
        ^$editor ...$file
    }
}

# Fuzzy Environment Variable Inspector
def fenv [] {
    let env_str = ($env | transpose key value | each {|it| $"($it.key)=($it.value)" } | str join (char nl))
    $env_str | fzf --prompt 'Env❯ ' --height 15 --min-height 10 --layout reverse --border rounded
}
