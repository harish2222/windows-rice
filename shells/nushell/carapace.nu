# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: shells\nushell / carapace
# ============================================================================
# ==============================================================================
# CARAPACE.NU — External Completer Bridge for Nushell
# Comprehensive multi-shell autocompletion (built-in specs + bridges)
# ==============================================================================

def --env carapace--shell-builtins [] {
    (help commands | where category != "" | get name | each { split row " " | first } | uniq | str join "\n")
}

def --env carapace--shell-functions [] {
    (help commands | where category == "" | get name | each { split row " " | first } | uniq | str join "\n")
}

# Universal Carapace Completer
# Handles current, famous, and future commands with alias expansion & native fallback
def --env carapace-complete [cmd: string, ...args: string] {
    let clean_cmd = ($cmd | str replace --regex '\.exe$' '')
    let expanded = (try { scope aliases | where name == $clean_cmd | get -o 0.expansion } catch { null })
    
    let target_cmd = if $expanded != null {
        ($expanded | split row " " | first | str replace --regex '\.exe$' '')
    } else {
        $clean_cmd
    }

    let full_args = if $expanded != null {
        let alias_parts = ($expanded | split row " ")
        (($alias_parts | skip 1) ++ $args)
    } else {
        $args
    }

    try {
        let result = (carapace $target_cmd nushell ...$full_args | from json)
        if ($result | is-empty) { null } else { $result }
    } catch {
        null
    }
}
