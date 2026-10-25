# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: Nushell Git Version Control Command Completions
# ============================================================================
# Git custom completions for Nushell
# Rich completions with branch names, stash names, remote names

# ── Branch names ──
export def "nu-complete git branches" [] {
    let local = (^git branch --format='%(refname:short)' 2>/dev/null | lines)
    let remote = (^git branch -r --format='%(refname:short)' 2>/dev/null | lines | each { |r| $r | str replace 'origin/' '' })
    ($local | each { |b| { value: $b description: "local" } }) | merge ($remote | each { |r| { value: $r description: "remote" } })
}

# ── Local branch names only ──
export def "nu-complete git local branches" [] {
    ^git branch --format='%(refname:short)' 2>/dev/null | lines | each { |b| { value: $b description: "local branch" } }
}

# ── Remote branch names ──
export def "nu-complete git remote branches" [] {
    ^git branch -r --format='%(refname:short)' 2>/dev/null | lines | each { |r| { value: $r description: "remote branch" } }
}

# ── Stash names ──
export def "nu-complete git stash" [] {
    ^git stash list 2>/dev/null | lines | each { |s| 
        let desc = ($s | str replace 'stash@{[0-9]+}: ' '')
        { value: ($s | parse 'stash@{{{index}}}: {msg}' | get 0.index | first) description: $desc }
    }
}

# ── Remotes ──
export def "nu-complete git remotes" [] {
    ^git remote 2>/dev/null | lines | each { |r| { value: $r description: "remote" } }
}

# ── Tags ──
export def "nu-complete git tags" [] {
    ^git tag -l 2>/dev/null | lines | each { |t| { value: $t description: "tag" } }
}

# ── Commit messages from recent log ──
export def "nu-complete git log messages" [] {
    ^git log --oneline -20 2>/dev/null | lines | each { |l| 
        let parts = ($l | split row ' ')
        { value: ($parts | skip 1 | str join ' ') description: ($parts | first) }
    }
}

# ── Git subcommands ──
export def "nu-complete git subcommands" [] {
    [
        { value: "add" description: "Add file contents to the index" }
        { value: "am" description: "Apply a series of patches from a mailbox" }
        { value: "archive" description: "Create archive of files from a tree" }
        { value: "bisect" description: "Find the change that introduced a bug" }
        { value: "blame" description: "Show what revision and author last modified each line" }
        { value: "branch" description: "List, create, or delete branches" }
        { value: "browse" description: "Open git repository in a web browser" }
        { value: "bundle" description: "Pack objects and archives together" }
        { value: "cache" description: "Temporarily stage modified files" }
        { value: "checkout" description: "Switch branches or restore working tree files" }
        { value: "cherry-pick" description: "Apply the change introduced by an existing commit" }
        { value: "clean" description: "Remove untracked files from the working tree" }
        { value: "clone" description: "Clone a repository into a new directory" }
        { value: "commit" description: "Record changes to the repository" }
        { value: "config" description: "Get and set repository or global options" }
        { value: "describe" description: "Describe a commit using the most recent tag" }
        { value: "diff" description: "Show changes between commits, working tree, etc" }
        { value: "fetch" description: "Download objects and refs from another repository" }
        { value: "filter-branch" description: "Rewrite branches" }
        { value: "for-each-ref" description: "Output information on each ref" }
        { value: "format-patch" description: "Generate patch-ready commits from history" }
        { value: "gc" description: "Cleanup unnecessary files and optimize the repository" }
        { value: "grep" description: "Print lines matching a pattern" }
        { value: "gui" description: "Open git-gui" }
        { value: "hash-object" description: "Compute object ID and optionally create a blob" }
        { value: "help" description: "Display help for a git command" }
        { value: "init" description: "Create an empty Git repository" }
        { value: "instaweb" description: "Instantly browse the working directory in gitweb" }
        { value: "log" description: "Show commit logs" }
        { value: "ls-files" description: "Show information about files in the index" }
        { value: "ls-tree" description: "List the contents of a tree object" }
        { value: "merge" description: "Join two or more development histories together" }
        { value: "merge-base" description: "Find as good a common ancestor as possible" }
        { value: "mv" description: "Move or rename a file, directory, or symlink" }
        { value: "notes" description: "Add or inspect object notes" }
        { value: "pull" description: "Fetch from and integrate with another repository" }
        { value: "push" description: "Update remote refs along with associated objects" }
        { value: "rebase" description: "Reapply commits on top of another base tip" }
        { value: "reflog" description: "Show reflog" }
        { value: "relink" description: "Hardlink common objects between repos" }
        { value: "remote" description: "Manage set of tracked repositories" }
        { value: "repack" description: "Pack unpacked objects in a repository" }
        { value: "replace" description: "Create, list, delete refs to replace objects" }
        { value: "request-pull" description: "Show a summary of changes for integration" }
        { value: "reset" description: "Reset current HEAD to the specified state" }
        { value: "revert" description: "Revert an existing commit" }
        { value: "rm" description: "Remove files from the working tree and index" }
        { value: "send-email" description: "Send a collection of patches as emails" }
        { value: "shortlog" description: "Summarize git log output" }
        { value: "show" description: "Show various types of objects" }
        { value: "stash" description: "Stash the changes in a dirty working directory" }
        { value: "status" description: "Show the working tree status" }
        { value: "submodule" description: "Initialize, update, or inspect submodules" }
        { value: "switch" description: "Create a new branch or switch to an existing one" }
        { value: "tag" description: "Create, list, delete, or verify a tag object" }
        { value: "tig" description: "Text-mode interface for git" }
        { value: "worktree" description: "Manage multiple working trees" }
    ]
}

# ── Git log format ──
export def "nu-complete git log formats" [] {
    [
        { value: "--oneline" description: "Compact single-line format" }
        { value: "--graph" description: "ASCII graph" }
        { value: "--stat" description: "Show diffstat" }
        { value: "--name-status" description: "Show names and statuses" }
        { value: "--pretty=format:" description: "Custom format string" }
    ]
}

# ── Git diff targets ──
export def "nu-complete git diff targets" [] {
    [
        { value: "--cached" description: "Show staged changes" }
        { value: "--staged" description: "Alias for --cached" }
        { value: "--stat" description: "Show diffstat only" }
        { value: "--name-only" description: "Show changed filenames only" }
        { value: "--name-status" description: "Show changed filenames with status" }
    ]
}

# ── Extern definitions with completers ──
export extern "git" [
    subcommand: string@"nu-complete git subcommands"
]

export extern "git add" [
    ...files: string
    --interactive(-i): bool
    --patch(-p): bool
    --all(-A): bool
    --dry-run(-n): bool
]

export extern "git branch" [
    ...branches: string@"nu-complete git branches"
    --list(-l): bool
    --all(-a): bool
    --remote(-r): bool
    --delete(-d): bool
    --move(-m): bool
    --copy(-c): bool
    --force(-f): bool
    --verbose(-v): bool
]

export extern "git checkout" [
    ...branches: string@"nu-complete git branches"
    --create(-b): bool
    --force(-f): bool
    --detach(-d): bool
    --track(-t): bool
]

export extern "git switch" [
    ...branches: string@"nu-complete git local branches"
    --create(-c): bool
    --force(-f): bool
    --detach(-d): bool
    --track(-t): bool
]

export extern "git commit" [
    ...message: string
    --all(-a): bool
    --amend(-m): bool
    --no-edit: bool
    --allow-empty: bool
    --signoff(-s): bool
    --no-verify: bool
]

export extern "git push" [
    remote?: string@"nu-complete git remotes"
    ...branch: string@"nu-complete git local branches"
    --force(-f): bool
    --set-upstream(-u): bool
    --tags(-t): bool
    --delete(-d): bool
    --dry-run(-n): bool
]

export extern "git pull" [
    remote?: string@"nu-complete git remotes"
    ...branch: string@"nu-complete git local branches"
    --rebase(-r): bool
    --no-rebase: bool
    --ff-only: bool
]

export extern "git merge" [
    ...branches: string@"nu-complete git branches"
    --no-ff: bool
    --squash(-s): bool
    --abort: bool
]

export extern "git rebase" [
    ...branch: string@"nu-complete git branches"
    --interactive(-i): bool
    --abort: bool
    --continue: bool
    --skip: bool
    --onto: bool
]

export extern "git stash" [
    subcommand?: string
    --message(-m): string
    --include-untracked(-u): bool
    --keep-index(-k): bool
    --patch(-p): bool
    ...args: string@"nu-complete git stash"
]

export extern "git log" [
    ...args: string@"nu-complete git log formats"
    --oneline(-l): bool
    --graph(-g): bool
    --all(-a): bool
    --decorate(-d): bool
    --author: string
    --since: string
    --until: string
    --grep: string
    --num(-n): int
]

export extern "git diff" [
    ...args: string@"nu-complete git diff targets"
    --stat: bool
    --name-only: bool
    --cached: bool
    --staged: bool
    --color: bool
]

export extern "git clone" [
    url: string
    dir?: string
    --recursive(-r): bool
    --depth(-d): int
    --branch(-b): string@"nu-complete git remote branches"
    --shallow-submodules: bool
]

export extern "git tag" [
    ...tags: string@"nu-complete git tags"
    --list(-l): bool
    --delete(-d): bool
    --annotate(-a): bool
    --message(-m): string
]

export extern "git remote" [
    subcommand?: string
    ...remotes: string@"nu-complete git remotes"
    --verbose(-v): bool
]

export extern "git worktree" [
    subcommand?: string
    ...args: string
    --list: bool
]

export extern "git reset" [
    ...commit: string
    --hard(-H): bool
    --soft(-S): bool
    --mixed(-M): bool
]

export extern "git restore" [
    ...files: string
    --staged(-S): bool
    --worktree(-W): bool
    --source(-s): string
]

export extern "git clean" [
    --force(-f): bool
    --dry-run(-n): bool
    --interactive(-i): bool
    --recursive(-r): bool
    --ignored(-x): bool
]

export extern "git blame" [
    ...files: string
    --line-porcelain: bool
    --incremental: bool
]

export extern "git show" [
    ...args: string
    --stat: bool
    --name-only: bool
    --format: string
]

export extern "git describe" [
    ...args: string
    --tags(-t): bool
    --always(-a): bool
    --long(-l): bool
]

export extern "git config" [
    key?: string
    value?: string
    --list(-l): bool
    --global(-g): bool
    --local: bool
    --edit(-e): bool
    --get: bool
    --unset: bool
]

export extern "git submodule" [
    subcommand?: string
    ...args: string
    --init(-i): bool
    --update(-u): bool
    --recursive(-r): bool
]

export extern "git cherry-pick" [
    ...commits: string
    --continue: bool
    --abort: bool
    --skip: bool
    --no-commit: bool
]

export extern "git bisect" [
    subcommand?: string
    ...args: string
    --start: bool
    --reset: bool
    --good: bool
    --bad: bool
]

export extern "git revert" [
    ...commits: string
    --no-edit: bool
    --continue: bool
    --abort: bool
    --skip: bool
]

export extern "git reflog" [
    subcommand?: string
    ...args: string
    --all: bool
    --date: string
]

export extern "git shortlog" [
    ...args: string
    --summary(-s): bool
    --numbered(-n): bool
    --email(-e): bool
]
