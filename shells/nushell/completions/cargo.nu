# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: Nushell Cargo Build Tool Command Completions
# ============================================================================
# Cargo custom completions for Nushell

# ── Cargo subcommands ──
export def "nu-complete cargo subcommands" [] {
    [
        { value: "bench" description: "Compile and run benchmarks" }
        { value: "build" description: "Compile a Rust package" }
        { value: "clean" description: "Remove artifacts" }
        { value: "clone" description: "Create a new Cargo package" }
        { value: "config" description: "Inspect Cargo configuration" }
        { value: "coverage" description: "Code coverage" }
        { value: "create" description: "Create a new Cargo package" }
        { value: "csearch" description: "Search packages in the registry" }
        { value: "doc" description: "Build documentation" }
        { value: "fetch" description: "Fetch dependencies" }
        { value: "fix" description: "Automatically fix Rust code" }
        { value: "generate-lockfile" description: "Generate Cargo.lock file" }
        { value: "git-checkout" description: "Checkout a git repository" }
        { value: "help" description: "Displays help for a subcommand" }
        { value: "init" description: "Create a new Cargo package in current directory" }
        { value: "install" description: "Build and install a Rust binary" }
        { value: "locate-project" description: "Locate the project Cargo.toml" }
        { value: "login" description: "Save an API token from the registry" }
        { value: "logout" description: "Remove an API token from the registry" }
        { value: "metadata" description: "Output the resolved dependencies of a package" }
        { value: "new" description: "Create a new Cargo package" }
        { value: "owner" description: "Manage the owners of a crate on the registry" }
        { value: "package" description: "Assemble the local package into a distributable tarball" }
        { value: "pkgid" description: "Print a maximally qualified package name" }
        { value: "publish" description: "Upload a package to the registry" }
        { value: "read-manifest" description: "Print a JSON manifest of a package" }
        { value: "run" description: "Run a binary or example of the local package" }
        { value: "rustc" description: "Pass `--crate-type` to rustc" }
        { value: "rustup" description: "Ensure the rust toolchain is up to date" }
        { value: "search" description: "Search packages in the registry" }
        { value: "test" description: "Execute all unit and integration tests" }
        { value: "tree" description: "Display a dependency tree" }
        { value: "uninstall" description: "Remove a Rust binary" }
        { value: "update" description: "Update dependencies in Cargo.lock" }
        { value: "vendor" description: "Vendor all dependencies locally" }
        { value: "verify" description: "Verify the chosen package" }
        { value: "version" description: "Show Cargo version" }
        { value: "watch" description: "Watch files for changes and rebuild" }
        { value: "xbuild" description: "Compile a local package and all of its dependencies" }
    ]
}

# ── Target triples ──
export def "nu-complete cargo targets" [] {
    [
        { value: "x86_64-unknown-linux-gnu" description: "Linux x86_64" }
        { value: "x86_64-unknown-linux-musl" description: "Linux x86_64 (musl)" }
        { value: "aarch64-unknown-linux-gnu" description: "Linux ARM64" }
        { value: "x86_64-pc-windows-msvc" description: "Windows x86_64 MSVC" }
        { value: "x86_64-pc-windows-gnu" description: "Windows x86_64 GNU" }
        { value: "aarch64-pc-windows-msvc" description: "Windows ARM64" }
        { value: "x86_64-apple-darwin" description: "macOS x86_64" }
        { value: "aarch64-apple-darwin" description: "macOS ARM64" }
        { value: "wasm32-unknown-unknown" description: "WebAssembly" }
        { value: "wasm32-wasi" description: "WebAssembly (WASI)" }
        { value: "riscv64gc-unknown-linux-gnu" description: "Linux RISC-V 64" }
    ]
}

# ── Cargo features ──
export def "nu-complete cargo features" [] {
    ["default", "full", "std", "async", "tokio", "serde", "derive", "macros", "nightly"]
}

# ── Profile names ──
export def "nu-complete cargo profiles" [] {
    [
        { value: "dev" description: "Development profile" }
        { value: "release" description: "Release profile" }
        { value: "test" description: "Test profile" }
        { value: "bench" description: "Benchmark profile" }
        { value: "doc" description: "Documentation profile" }
    ]
}

# ── Extern definitions ──
export extern "cargo" [
    subcommand: string@"nu-complete cargo subcommands"
]

export extern "cargo build" [
    ...packages: string
    --target(-t): string@"nu-complete cargo targets"
    --release(-r): bool
    --debug(-d): bool
    --features(-f): string@"nu-complete cargo features"
    --all-features: bool
    --no-default-features: bool
    --workspace(-w): bool
    --jobs(-j): int
    --verbose(-v): bool
    --quiet(-q): bool
    --message-format: string
    --bin: string
    --example: string
    --test: string
    --bench: string
]

export extern "cargo run" [
    ...args: string
    --release(-r): bool
    --target(-t): string@"nu-complete cargo targets"
    --features(-f): string@"nu-complete cargo features"
    --bin: string
    --example: string
    --profile: string@"nu-complete cargo profiles"
]

export extern "cargo test" [
    ...tests: string
    --release(-r): bool
    --target(-t): string@"nu-complete cargo targets"
    --features(-f): string@"nu-complete cargo features"
    --all-features: bool
    --no-default-features: bool
    --workspace(-w): bool
    --lib: bool
    --bin: string
    --test: string
    --bench: string
    --doc: bool
    --no-run: bool
    --nocapture: bool
    --test-threads: int
    --verbose(-v): bool
]

export extern "cargo doc" [
    ...packages: string
    --release(-r): bool
    --no-deps: bool
    --open: bool
    --target(-t): string@"nu-complete cargo targets"
    --features(-f): string@"nu-complete cargo features"
    --document-private-items: bool
]

export extern "cargo clippy" [
    ...args: string
    --target(-t): string@"nu-complete cargo targets"
    --release(-r): bool
    --features(-f): string@"nu-complete cargo features"
    --all-features: bool
    --no-default-features: bool
    --workspace(-w): bool
    --all-targets: bool
    --fix: bool
    --allow-dirty: bool
]

export extern "cargo fmt" [
    ...args: string
    --check: bool
    --all: bool
    --package(-p): string
]

export extern "cargo clean" [
    --target(-t): string@"nu-complete cargo targets"
    --release(-r): bool
    --doc: bool
    --out-dir: string
]

export extern "cargo install" [
    ...crates: string
    --version(-V): string
    --git: string
    --branch(-b): string
    --tag(-t): string
    --path: string
    --features(-f): string@"nu-complete cargo features"
    --target(-t): string@"nu-complete cargo targets"
    --force(-f): bool
    --locked: bool
]

export extern "cargo update" [
    ...packages: string
    --aggressive(-p): bool
    --precise: string
]

export extern "cargo search" [
    query: string
    --limit(-l): int
]

export extern "cargo publish" [
    --allow-dirty: bool
    --dry-run: bool
    --no-verify: bool
    --token: string
    --index: string
]

export extern "cargo tree" [
    ...packages: string
    --target(-t): string@"nu-complete cargo targets"
    --features(-f): string@"nu-complete cargo features"
    --no-dedupe: bool
    --prefix(depth): string
    --format(-f): string
    --invert(-i): bool
    --edges(-e): string
]

export extern "cargo bench" [
    ...benchmarks: string
    --release(-r): bool
    --target(-t): string@"nu-complete cargo targets"
    --features(-f): string@"nu-complete cargo features"
    --no-run: bool
]

export extern "cargo metadata" [
    --format-version(-f): int
    --no-deps: bool
    --features(-f): string@"nu-complete cargo features"
]

export extern "cargo fix" [
    --allow-dirty: bool
    --allow-staged: bool
    --edition: bool
    --target(-t): string@"nu-complete cargo targets"
    --features(-f): string@"nu-complete cargo features"
]

export extern "cargo vendor" [
    ...args: string
    --versioned-dirs: bool
    --relative-path: bool
]

export extern "cargo version" []

export extern "cargo watch" [
    --watch(-w): string
    --exec(-x): string
    --clear(-c): bool
    --no-git: bool
    --why: bool
]
