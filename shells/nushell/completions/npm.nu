# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: Nushell Node Package Manager Command Completions
# ============================================================================
# NPM custom completions for Nushell

# ── NPM subcommands ──
export def "nu-complete npm subcommands" [] {
    [
        { value: "access" description: "Set access level on published packages" }
        { value: "adduser" description: "Log in to the npm registry" }
        { value: "audit" description: "Run a security audit" }
        { value: "bin" description: "Display npm bin folder" }
        { value: "bugs" description: "Report bugs for a package" }
        { value: "cache" description: "Manage the npm cache" }
        { value: "ci" description: "Clean install with locked dependencies" }
        { value: "completion" description: "Tab completion for npm commands" }
        { value: "config" description: "Manage the npm configuration files" }
        { value: "dedupe" description: "Reduce duplication in the tree" }
        { value: "deprecate" description: "Deprecate a version of a package" }
        { value: "diff" description: "See what changes have been made" }
        { value: "dist-tag" description: "Tag a published version" }
        { value: "docs" description: "Open documentation for a package" }
        { value: "doctor" description: "Check the health of the npm environment" }
        { value: "edit" description: "Edit an installed package" }
        { value: "exec" description: "Run a command from an npm package" }
        { value: "explain" description: "Explain the installed items" }
        { value: "explore" description: "Browse an installed package" }
        { value: "fund" description: "Find funding for a package" }
        { value: "help" description: "Get help on npm commands" }
        { value: "hook" description: "Manage registry hooks" }
        { value: "init" description: "Create a package.json file" }
        { value: "install" description: "Install a package" }
        { value: "install-ci-test" description: "Install and run tests" }
        { value: "install-test" description: "Install and test" }
        { value: "link" description: "Symlink a package folder" }
        { value: "ll" description: "List installed packages" }
        { value: "login" description: "Log in to the npm registry" }
        { value: "ls" description: "List installed packages" }
        { value: "org" description: "Manage organization users and teams" }
        { value: "outdated" description: "Check for outdated packages" }
        { value: "owner" description: "Manage package owners" }
        { value: "pack" description: "Create a tarball from a package" }
        { value: "ping" description: "Ping the npm registry" }
        { value: "pkg" description: "Manage package.json" }
        { value: "prefix" description: "Display path prefix" }
        { value: "profile" description: "Change details on the npm registry" }
        { value: "prune" description: "Remove extraneous packages" }
        { value: "publish" description: "Publish a package" }
        { value: "query" description: "Query the registry using selectors" }
        { value: "rebuild" description: "Rebuild a package" }
        { value: "repo" description: "Open repository for a package" }
        { value: "restart" description: "Restart a package" }
        { value: "root" description: "Display npm root directory" }
        { value: "run-script" description: "Run arbitrary package scripts" }
        { value: "search" description: "Search for packages" }
        { value: "set-script" description: "Set lifecycle scripts" }
        { value: "shrinkwrap" description: "Create a shrinkwrap file" }
        { value: "star" description: "Star a package" }
        { value: "stars" description: "View starred packages" }
        { value: "start" description: "Start a package" }
        { value: "stop" description: "Stop a package" }
        { value: "team" description: "Manage organization teams" }
        { value: "test" description: "Test a package" }
        { value: "token" description: "Manage authentication tokens" }
        { value: "uninstall" description: "Remove a package" }
        { value: "unpublish" description: "Remove a package from the registry" }
        { value: "unstar" description: "Remove a star from a package" }
        { value: "update" description: "Update packages" }
        { value: "version" description: "Bump a package version" }
        { value: "view" description: "View registry info for a package" }
        { value: "whoami" description: "Display npm username" }
    ]
}

# ── Package names from node_modules ──
export def "nu-complete npm packages local" [] {
    if ("node_modules" | path exists) {
        ls node_modules | where type == dir | get name | each { |p| { value: $p description: "local package" } }
    } else { [] }
}

# ── Scripts from package.json ──
export def "nu-complete npm scripts" [] {
    if ("package.json" | path exists) {
        let pkg = (open package.json)
        if ($pkg.scripts? | is-not-empty) {
            $pkg.scripts | transpose key value | each { |s| { value: $s.key description: $s.value } }
        } else { [] }
    } else { [] }
}

# ── Extern definitions ──
export extern "npm" [
    subcommand: string@"nu-complete npm subcommands"
]

export extern "npm install" [
    ...packages: string
    --save(-S): bool
    --save-dev(-D): bool
    --save-optional(-O): bool
    --save-exact(-E): bool
    --global(-g): bool
    --legacy-peer-deps: bool
    --force(-f): bool
    --dry-run: bool
    --no-package-lock: bool
    --workspaces: bool
]

export extern "npm run" [
    script: string@"nu-complete npm scripts"
    ...args: string
    --if-present: bool
    --silent: bool
]

export extern "npm start" [
    ...args: string
]

export extern "npm test" [
    ...args: string
    --if-present: bool
]

export extern "npm uninstall" [
    ...packages: string
    --save(-S): bool
    --save-dev(-D): bool
    --save-optional(-O): bool
    --global(-g): bool
]

export extern "npm update" [
    ...packages: string
    --global(-g): bool
    --save: bool
]

export extern "npm publish" [
    tarball?: string
    --access: string
    --tag(-t): string
    --otp: string
    --dry-run: bool
    --provenance: bool
]

export extern "npm outdated" [
    ...packages: string
    --global(-g): bool
    --long(-l): bool
    --json: bool
]

export extern "npm ls" [
    ...packages: string
    --global(-g): bool
    --depth(-d): int
    --all(-a): bool
    --json: bool
    --long: bool
    --parseable: bool
    --all: bool
]

export extern "npm init" [
    --yes(-y): bool
    --scope: string
    --init-author-name: string
    --init-author-email: string
    --init-license: string
    --init-version: string
]

export extern "npm exec" [
    package: string
    ...args: string
    --package(-p): string
    --call(-c): string
]

export extern "npm ci" [
    --production: bool
    --no-optional: bool
]

export extern "npm cache" [
    subcommand: string
    --force(-f): bool
    --global(-g): bool
]

export extern "npm config" [
    subcommand: string
    ...args: string
    --global(-g): bool
    --json(-j): bool
]

export extern "npm audit" [
    subcommand?: string
    --omit(-O): string
    --json: bool
    --package-lock-only: bool
    --production: bool
]

export extern "npm search" [
    ...terms: string
    --long(-l): bool
    --json: bool
    --parseable: bool
]

export extern "npm pack" [
    ...packages: string
    --dry-run: bool
    --json: bool
]

export extern "npm version" [
    version: string
    --no-git-tag-version: bool
    --allow-same-version: bool
    --message(-m): string
]

export extern "npm view" [
    package: string
    --json: bool
    --long(-l): bool
    --workspace: string
]

export extern "npm link" [
    package?: string
    --global(-g): bool
]

export extern "npm token" [
    subcommand?: string
    ...args: string
    --list: bool
    --create: bool
]

export extern "npm login" [
    --scope: string
    --auth-type: string
    --registry: string
]

export extern "npm logout" [
    --scope: string
    --registry: string
]

export extern "npm whoami" [
    --registry: string
]

export extern "npm owner" [
    subcommand: string
    package: string
    --registry: string
]

export extern "npm team" [
    subcommand: string
    ...args: string
]

export extern "npm org" [
    subcommand: string
    ...args: string
]

export extern "npm repo" [
    package?: string
    --browser: string
]

export extern "npm bugs" [
    package?: string
    --browser: string
]

export extern "npm docs" [
    package?: string
    --browser: string
]

export extern "npm star" [
    ...packages: string
]

export extern "npm unstar" [
    ...packages: string
]

export extern "npm stars" [
    username?: string
]

export extern "npm dedupe" [
    ...packages: string
    --dry-run: bool
]

export extern "npm prune" [
    --production: bool
    --dry-run: bool
]

export extern "npm rebuild" [
    ...packages: string
]

export extern "npm fund" [
    package?: string
    --json: bool
]

export extern "npm diff" [
    --diff-arg(-d): string
    --diff-name-only: bool
    ...specs: string
]

export extern "npm explain" [
    ...paths: string
    --json: bool
]

export extern "npm query" [
    selector: string
]

export extern "npm pkg" [
    subcommand?: string
    ...args: string
    --json: bool
]

export extern "npm set-script" [
    name: string
    command: string
]

export extern "npm hook" [
    subcommand?: string
    ...args: string
]
