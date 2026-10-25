# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: shells\nushell\modules / conda
# ============================================================================
# =============================================================================
#  conda.nu — ZERO-LAG CONDA FOR NUSHELL (KV-snapshot approach)
#  First `conda activate <env>` spawns conda once and caches the resulting
#  environment as JSON; every later activation restores the snapshot
#  instantly, without spawning conda at all.
# =============================================================================

# --- Cache location (XDG) ---
def --env conda-cache-dir [] {
    $env.XDG_CACHE_HOME?
    | default ($env.HOME? | default $env.USERPROFILE? | path join ".cache")
    | path join "nushell" "conda"
}

# --- Parse `conda shell.posix activate` output into a record ---
def parse-conda-posix [out: string] {
    $out
    | lines
    | each {|line|
        if ($line | str starts-with "export ") {
            let body = ($line | str substring 7.. | str trim --right --char ';')
            let eq = ($body | str index-of '=')
            if $eq != -1 {
                let key = ($body | str substring 0..($eq - 1))
                let val = ($body
                    | str substring ($eq + 1)..
                    | str trim --char "'"
                    | str replace --all "'\\''" "'")
                {key: $key, val: $val}
            } else { null }
        } else { null }
    }
    | compact
    | reduce --fold {} {|it, acc| $acc | upsert $it.key $it.val}
}

# --- Fast activation: cached snapshot or one-time conda spawn ---
def --env conda-activate [name: string] {
    let cache_file = (conda-cache-dir | path join $"($name).json")

    let env_vars = if ($cache_file | path exists) {
        try { open $cache_file } catch { {} }
    } else {
        # Windows: cmd.exe formatter keeps the Windows PATH intact (the posix
        # formatter would emit Unix-style PATH and break every .exe lookup).
        let res = (try {
            if ($nu.os-info.name == "windows") {
                ^conda shell.cmd.exe activate $name | complete
            } else {
                ^conda shell.posix activate $name | complete
            }
        } catch { null })
        if $res == null or $res.exit_code != 0 {
            print $"conda: activation of '($name)' failed (does the env exist?)."
            return
        }
        let parsed = (if ($nu.os-info.name == "windows") {
            # Conda 25+ prints a temp .env file path; older versions print @SET lines.
            let env_file = ($res.stdout | str trim)
            if (($env_file | str lowercase | str ends-with ".env") and ($env_file | path exists)) {
                let content = (open $env_file | into string)
                rm $env_file
                parse-conda-cmd $content
            } else {
                parse-conda-cmd $res.stdout
            }
        } else {
            parse-conda-posix $res.stdout
        })
        if ($parsed | is-empty) {
            print $"conda: could not parse activation output for '($name)'."
            return
        }
        mkdir (conda-cache-dir)
        $parsed | to json | save -f $cache_file
        $parsed
    }

    if ($env_vars | is-empty) { return }

    let applied = ($env_vars | columns)

    # Snapshot every key the new env is about to touch, so deactivate can restore.
    let saved = ($applied
    | reduce --fold {} {|key, acc|
        let cur = ($env | get -o $key)
        if $cur == null { $acc } else { $acc | insert $key $cur }
    })

    $env.CONDA_STACK = (($env.CONDA_STACK? | default []) | append {saved: $saved, applied: $applied})

    load-env $env_vars
}

# --- Deactivation: restore the snapshot taken at activation ---
def --env conda-deactivate [] {
    let stack = ($env.CONDA_STACK? | default [])

    if ($stack | is-empty) {
        # Nothing tracked (e.g. base auto-activated elsewhere) — hand it to conda.
        try { ^conda deactivate } catch { print "conda: nothing to deactivate." }
        return
    }

    let frame = ($stack | last)
    $env.CONDA_STACK = ($stack | skip 1)

    let saved_keys = ($frame.saved | columns)
    for key in ($frame.applied | where {|k| $k not-in $saved_keys}) {
        try { hide-env $key } catch { }
    }
    load-env $frame.saved
}

# --- Parse conda activation output (Windows) ---
# Conda 25+ writes `KEY=VALUE` lines to a temp .env file (printed as its path);
# older versions print `@SET "KEY=VALUE"` lines. Handles both.
def parse-conda-cmd [text: string] {
    $text
    | lines
    | each {|line|
        let t = ($line | str trim)
        let body = if (($t | str lowercase) | str starts-with "@set \"") {
            ($t | str substring 6.. | str trim --right --char '"')
        } else { $t }
        let eq = ($body | str index-of '=')
        if $eq > 0 {
            {key: ($body | str substring 0..($eq - 1)), val: ($body | str substring ($eq + 1)..)}
        } else { null }
    }
    | compact
    | reduce --fold {} {|it, acc| $acc | upsert $it.key $it.val}
}

# --- Wrapper: fast path for activate/deactivate, passthrough for everything ---
def --env conda [...args: string] {
    let sub = ($args | get -o 0 | default "")

    match $sub {
        "activate" => {
            if ($args | length) > 1 { conda-activate ($args | get 1) } else { ^conda activate }
        }
        "deactivate" => { conda-deactivate }
        _ => { ^conda ...$args }
    }
}
