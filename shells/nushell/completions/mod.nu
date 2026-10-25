# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: shells\nushell\completions / mod
# ============================================================================
# ==============================================================================
# Nushell Completions Module — Custom completions for domain-specific tools
# ==============================================================================

# ── Git completions ──
def "nu-complete git branches" [] {
    ^git branch -a | lines | each { |l| $l | str replace '\*' '' | str trim | str replace -a 'remotes/' '' }
}

def "nu-complete git refs" [] {
    ^git log -20 --pretty=format:"%h %s" | lines | each { |l| { value: ($l | split row ' ' | first) description: ($l | skip 1 | str trim) } }
}

def "nu-complete git tags" [] {
    ^git tag -l | lines | each { |l| $l | str trim }
}

def "nu-complete git stashes" [] {
    ^git stash list | lines | each { |l| $l | str trim }
}

# ── Cargo completions ──
def "nu-complete cargo subcommands" [] {
    [
        "build", "run", "test", "bench", "doc", "publish", "install",
        "uninstall", "clean", "check", "clippy", "fmt", "audit",
        "tree", "update", "fetch", "git-checkout", "new", "init",
        "search", "generate-lockfile", "manifest-path"
    ]
}

def "nu-complete cargo features" [] {
    # Would need to parse Cargo.toml
    []
}

# ── PlatformIO completions ──
def "nu-complete platformio subcommands" [] {
    [
        { value: "run" description: "Build project" }
        { value: "device monitor" description: "Monitor serial port" }
        { value: "device list" description: "List connected devices" }
        { value: "init" description: "Initialize project" }
        { value: "lib install" description: "Install library" }
        { value: "lib update" description: "Update libraries" }
        { value: "lib list" description: "List installed libraries" }
        { value: "platform install" description: "Install platform" }
        { value: "platform update" description: "Update platforms" }
        { value: "platform list" description: "List installed platforms" }
        { value: "system info" description: "Show system info" }
        { value: "check" description: "Static analysis" }
        { value: "package pack" description: "Create package" }
    ]
}

def "nu-complete platformio boards" [] {
    [
        { value: "esp32dev" description: "ESP32 DevKit" }
        { value: "esp32-s2-saola-1" description: "ESP32-S2 Saola" }
        { value: "esp32-s3-devkitc-1" description: "ESP32-S3 DevKit" }
        { value: "esp8266" description: "ESP8266 NodeMCU" }
        { value: "nodemcuv2" description: "NodeMCU v2" }
        { value: "uno" description: "Arduino Uno" }
        { value: "mega2560" description: "Arduino Mega 2560" }
        { value: "nano33ble" description: "Arduino Nano 33 BLE" }
        { value: "nucleo_f103rb" description: "STM32 Nucleo F103RB" }
        { value: "nucleo_h743zi" description: "STM32 Nucleo H743ZI" }
        { value: "bluepill_f103c8" description: "Blue Pill STM32F103C8" }
        { value: "teensy41" description: "Teensy 4.1" }
        { value: "teensy40" description: "Teensy 4.0" }
        { value: "rpipico" description: "Raspberry Pi Pico" }
        { value: "pico-sdk" description: "Raspberry Pi Pico SDK" }
    ]
}

def "nu-complete platformio environments" [] {
    # Parse platformio.ini
    if ('platformio.ini' | path exists) {
        open platformio.ini
        | lines
        | each { |l| $l | str trim }
        | where { |l| $l =~ '^\[' }
        | each { |l| $l | str replace '\[' '' | str replace '\]' '' }
    } else {
        []
    }
}

# ── STM32/CMSIS completions ──
def "nu-complete stm32 chips" [] {
    [
        { value: "STM32F030" description: "Cortex-M0, 48MHz" }
        { value: "STM32F103C8" description: "Cortex-M3, 72MHz (Blue Pill)" }
        { value: "STM32F103RC" description: "Cortex-M3, 72MHz, 512KB" }
        { value: "STM32F407VG" description: "Cortex-M4, 168MHz (Discovery)" }
        { value: "STM32F407ZT" description: "Cortex-M4, 168MHz, 1MB" }
        { value: "STM32F446RE" description: "Cortex-M4, 180MHz (Nucleo)" }
        { value: "STM32F746ZG" description: "Cortex-M7, 216MHz (Nucleo)" }
        { value: "STM32H743ZI" description: "Cortex-M7, 480MHz (Nucleo)" }
        { value: "STM32H747XI" description: "Dual Core M7+M4 (Discovery)" }
        { value: "STM32L476RG" description: "Cortex-M4, 80MHz, Ultra-low Power" }
        { value: "STM32L031K6" description: "Cortex-M0+, 32MHz, Low Power" }
        { value: "STM32G431CB" description: "Cortex-M4, 170MHz (Nucleo)" }
        { value: "STM32U575ZI" description: "Cortex-M33, 160MHz, Ultra-low Power" }
    ]
}

def "nu-complete stm32 programmers" [] {
    [
        { value: "stlink" description: "ST-Link V2/V3" }
        { value: "jlink" description: "J-Link" }
        { value: "cmsis-dap" description: "CMSIS-DAP" }
        { value: "blackmagic" description: "Black Magic Probe" }
        { value: "stlink-v2-1" description: "ST-Link V2-1 (on Nucleo)" }
        { value: "stlink-v3" description: "ST-Link V3" }
    ]
}

def "nu-complete verilator flags" [] {
    [
        { value: "--lint-only" description: "Lint only, no compilation" }
        { value: "--trace" description: "Enable VCD tracing" }
        { value: "--cc" description: "Generate C++ code" }
        { value: "--exe" description: "Enable executable" }
        { value: "--build" description: "Build after compilation" }
        { value: "--top-module" description: "Specify top module" }
        { value: "-Wall" description: "Enable all warnings" }
        { value: "-Wno-fatal" description: "Don't treat warnings as errors" }
    ]
}

def "nu-complete iverilog flags" [] {
    [
        { value: "-o" description: "Output file" }
        { value: "-g2012" description: "SystemVerilog 2012" }
        { value: "-Wall" description: "Enable all warnings" }
        { value: "-t null" description: "Type of output" }
        { value: "-I" description: "Include directory" }
    ]
}

def "nu-complete cocotb tests" [] {
    [
        { value: "test_basic" description: "Basic test" }
        { value: "test_counter" description: "Counter test" }
        { value: "test_shift_reg" description: "Shift register test" }
        { value: "test_fsm" description: "FSM test" }
    ]
}

# ── Docker completions ──
def "nu-complete docker containers" [] {
    ^docker ps -a --format '{{.Names}}' | lines | each { |l| $l | str trim }
}

def "nu-complete docker images" [] {
    ^docker images --format '{{.Repository}}:{{.Tag}}' | lines | each { |l| $l | str trim }
}

def "nu-complete docker networks" [] {
    ^docker network ls --format '{{.Name}}' | lines | each { |l| $l | str trim }
}

def "nu-complete docker volumes" [] {
    ^docker volume ls --format '{{.Name}}' | lines | each { |l| $l | str trim }
}

# ── Scoop completions ──
def "nu-complete scoop buckets" [] {
    ^scoop bucket list | lines | each { |l| $l | split row ' ' | first }
}

def "nu-complete scoop packages" [] {
    ^scoop list | lines | each { |l| $l | split row ' ' | first }
}

# ── npm completions ──
def "nu-complete npm scripts" [] {
    if ('package.json' | path exists) {
        open package.json
        | get -o scripts | default {}
        | columns
    } else {
        []
    }
}

# ── CMake completions ──
def "nu-complete cmake generators" [] {
    [
        { value: "Ninja" description: "Ninja build system" }
        { value: "Visual Studio 17 2022" description: "Visual Studio 2022" }
        { value: "Unix Makefiles" description: "Unix Makefiles" }
        { value: "MinGW Makefiles" description: "MinGW Makefiles" }
        { value: "NMake Makefiles" description: "NMake Makefiles" }
        { value: "MSYS Makefiles" description: "MSYS Makefiles" }
    ]
}

def "nu-complete cmake presets" [] {
    if ('CMakePresets.json' | path exists) {
        open CMakePresets.json
        | get configurePresets
        | each { |p| { value: $p.name, description: ($p.description? | default $p.name) } }
    } else {
        []
    }
}

# ── Conan completions ──
def "nu-complete conan profiles" [] {
    ^conan profile list | lines | each { |l| $l | str trim | str replace ':' '' }
}

def "nu-complete conan packages" [] {
    # Would need to search conan cache
    []
}

# ── Git completions for aliases ──
def "nu-complete git remotes" [] {
    ^git remote | lines | each { |l| $l | str trim }
}


# ── Cargo project completions ──
def "nu-complete cargo binaries" [] {
    if ('Cargo.toml' | path exists) {
        open Cargo.toml
        | get package.name?
        | if ($in | is-not-empty) {
            [{ value: $"cargo run" description: "Run main binary" }]
        } else {
            [{ value: "." description: "Run default binary" }]
        }
    } else {
        []
    }
}

# ── PlatformIO project completions ──
def "nu-complete pio environments" [] {
    if ('platformio.ini' | path exists) {
        open platformio.ini
        | lines
        | each { |l| $l | str trim }
        | where { |l| $l =~ '^\[env:' }
        | each { |l| $l | str replace '\[env:' '' | str replace '\]' '' }
    } else {
        []
    }
}

def "nu-complete pio boards" [] {
    [
        { value: "esp32dev" description: "ESP32 DevKit" }
        { value: "esp32-s2-saola-1" description: "ESP32-S2 Saola" }
        { value: "esp32-s3-devkitc-1" description: "ESP32-S3 DevKit" }
        { value: "esp8266" description: "ESP8266 NodeMCU" }
        { value: "nodemcuv2" description: "NodeMCU v2" }
        { value: "uno" description: "Arduino Uno" }
        { value: "mega2560" description: "Arduino Mega 2560" }
        { value: "nano33ble" description: "Arduino Nano 33 BLE" }
        { value: "nucleo_f103rb" description: "STM32 Nucleo F103RB" }
        { value: "nucleo_h743zi" description: "STM32 Nucleo H743ZI" }
        { value: "bluepill_f103c8" description: "Blue Pill STM32F103C8" }
        { value: "teensy41" description: "Teensy 4.1" }
        { value: "teensy40" description: "Teensy 4.0" }
        { value: "rpipico" description: "Raspberry Pi Pico" }
        { value: "pico-sdk" description: "Raspberry Pi Pico SDK" }
    ]
}

# ── Nushell built-in completions (enhanced) ──
def "nu-complete command aliases" [] {
    $env.ALIASES? | columns | each { |l| { value: $l description: $"Alias for: ($env.ALIASES | get $l)" } }
}

def "nu-complete colors" [] {
    [
        "black", "dark_gray", "red", "dark_red", "green", "dark_green",
        "yellow", "dark_yellow", "blue", "dark_blue", "purple", "dark_purple",
        "cyan", "dark_cyan", "white", "light_gray"
    ]
}

# ── GitHub CLI completions ──
def "nu-complete gh repos" [] {
    ^gh repo list --limit 100 --json nameWithOwner --jq '.[].nameWithOwner' | lines | each { |l| $l | str trim }
}

def "nu-complete gh issues" [] {
    ^gh issue list --limit 100 --json number --jq '.[].number' | lines | each { |l| $l | str trim }
}

def "nu-complete gh prs" [] {
    ^gh pr list --limit 100 --json number --jq '.[].number' | lines | each { |l| $l | str trim }
}

def "nu-complete gh workflows" [] {
    ^gh workflow list --json name --jq '.[].name' | lines | each { |l| $l | str trim }
}
