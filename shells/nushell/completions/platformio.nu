# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: Nushell PlatformIO Embedded Systems CLI Completions
# ============================================================================
# PlatformIO custom completions for Nushell

# ── PlatformIO subcommands ──
export def "nu-complete pio subcommands" [] {
    [
        { value: "init" description: "Initialize a new PlatformIO project" }
        { value: "run" description: "Process project environments" }
        { value: "upload" description: "Upload firmware to the target board" }
        { value: "test" description: "Run unit tests" }
        { value: "device" description: "Monitor/list/configure serial devices" }
        { value: "lib" description: "Library Manager" }
        { value: "platform" description: "Platform Manager" }
        { value: "pkg" description: "Package Manager" }
        { value: "boards" description: "List supported boards" }
        { value: "scons" description: "Advanced SCons scripting" }
        { value: "check" description: "Run static code analysis" }
        { value: "debug" description: "Start a debugging session" }
        { value: "remote" description: "Remote development" }
        { value: "home" description: "Open PlatformIO Home in a web browser" }
        { value: "settings" description: "Manage PlatformIO settings" }
        { value: "account" description: "PlatformIO Account" }
        { value: "org" description: "PlatformIO Organization" }
        { value: "piocore" description: "PlatformIO Core version and settings" }
        { value: "report" description: "Generate a report" }
        { value: "asset" description: "Asset Management" }
    ]
}

# ── Board names ──
export def "nu-complete pio boards" [] {
    let boards = [
        { value: "esp32dev" description: "ESP32 DevKit" }
        { value: "esp32s2" description: "ESP32-S2" }
        { value: "esp32s3" description: "ESP32-S3" }
        { value: "esp32c3" description: "ESP32-C3" }
        { value: "nodemcuv2" description: "NodeMCU V2 (ESP8266)" }
        { value: "d1_mini" description: "Wemos D1 Mini (ESP8266)" }
        { value: "uno" description: "Arduino Uno" }
        { value: "mega2560" description: "Arduino Mega 2560" }
        { value: "nanoatmega328" description: "Arduino Nano" }
        { value: "STM32F103C8" description: "Blue Pill (STM32F103C8)" }
        { value: "STM32F407VG" description: "Discovery (STM32F407VG)" }
        { value: "STM32H743ZI" description: "Nucleo-H743ZI" }
        { value: "STM32L476RG" description: "Nucleo-L476RG" }
        { value: "STM32F746ZG" description: "Nucleo-F746ZG" }
        { value: "native_posix" description: "Native POSIX" }
        { value: "native_posix_64" description: "Native POSIX 64-bit" }
        { value: "samd21_mini" description: "SAMD21 Mini" }
        { value: "teensy40" description: "Teensy 4.0" }
        { value: "rpipico" description: "Raspberry Pi Pico" }
        { value: "rpipicow" description: "Raspberry Pi Pico W" }
    ]
    $boards | each { |b| { value: $b.value description: $b.description } }
}

# ── Platform names ──
export def "nu-complete pio platforms" [] {
    [
        { value: "espressif32" description: "Espressif ESP32" }
        { value: "espressif8266" description: "Espressif ESP8266" }
        { value: "atmelavr" description: "Atmel AVR" }
        { value: "atmelsam" description: "Atmel SAM" }
        { value: "ststm32" description: "STMicroelectronics STM32" }
        { value: "nxp32" description: "NXP 32-bit" }
        { value: "raspberrypi" description: "Raspberry Pi" }
        { value: "intel_arc32" description: "Intel ARC32" }
        { value: "mbed" description: "ARM Mbed" }
        { value: "native" description: "Native host platform" }
        { value: "linux_arm" description: "Linux ARM" }
        { value: "linux_arm64" description: "Linux ARM64" }
        { value: "linux_x86_64" description: "Linux x86_64" }
    ]
}

# ── Library names (PIO Registry) ──
export def "nu-complete pio libs" [] {
    [
        { value: "WiFi" description: "WiFi library" }
        { value: "Wire" description: "I2C library" }
        { value: "SPI" description: "SPI library" }
        { value: "Servo" description: "Servo library" }
        { value: "Adafruit NeoPixel" description: "NeoPixel LED library" }
        { value: "FastLED" description: "FastLED library" }
        { value: "PubSubClient" description: "MQTT client" }
        { value: "ArduinoJson" description: "JSON library" }
        { value: "ESPAsyncWebServer" description: "Async Web Server" }
        { value: "TaskScheduler" description: "Cooperative multitasking" }
        { value: "OneWire" description: "OneWire protocol" }
        { value: "DallasTemperature" description: "Dallas Temperature" }
        { value: "DHT sensor library" description: "DHT temperature/humidity sensor" }
        { value: "Adafruit Unified Sensor" description: "Adafruit sensor abstraction" }
        { value: "Stepper" description: "Stepper motor library" }
    ]
}

# ── Framework names ──
export def "nu-complete pio frameworks" [] {
    [
        { value: "arduino" description: "Arduino framework" }
        { value: "espidf" description: "ESP-IDF framework" }
        { value: "mbed" description: "ARM Mbed framework" }
        { value: "stm32cube" description: "STM32Cube HAL" }
        { value: "cmsis" description: "CMSIS framework" }
        { value: "freertos" description: "FreeRTOS" }
        { value: "zephyr" description: "Zephyr RTOS" }
        { value: "native" description: "Native platform" }
        { value: "spl" description: "Standard Peripheral Library" }
        { value: "hal" description: "Hardware Abstraction Layer" }
    ]
}

# ── Upload protocols ──
export def "nu-complete pio upload protocols" [] {
    [
        { value: "esptool" description: "ESP tool (ESP32/ESP8266)" }
        { value: "stlink" description: "ST-Link debugger" }
        { value: "jlink" description: "J-Link debugger" }
        { value: "cmsis-dap" description: "CMSIS-DAP debugger" }
        { value: "blackmagic" description: "Black Magic Probe" }
        { value: "dfu" description: "DFU bootloader" }
        { value: "serial" description: "Serial/UART upload" }
        { value: "avrisp" description: "AVR ISP" }
        { value: "arduino" description: "Arduino bootloader" }
        { value: "usbboot" description: "USB Boot (RPi Pico)" }
        { value: "picotool" description: "PicoTool (RPi Pico)" }
    ]
}

# ── Extern definitions ──
export extern "platformio" [
    subcommand: string@"nu-complete pio subcommands"
    ...args: string
]

export extern "platformio init" [
    --board(-b): string@"nu-complete pio boards"
    --ide: string
    --project-option(-O): string
    --framework(-f): string@"nu-complete pio frameworks"
    --platform(-p): string@"nu-complete pio platforms"
]

export extern "platformio run" [
    ...targets: string
    --environment(-e): string
    --target(-t): string
    --jobs(-j): int
    --verbose(-v): bool
    --clean: bool
    --force: bool
    --silent: bool
]

export extern "platformio upload" [
    --environment(-e): string
    --target(-t): string
    --upload-port: string
    --upload-protocol: string@"nu-complete pio upload protocols"
    --verbose(-v): bool
]

export extern "platformio device" [
    subcommand?: string
    --list: bool
    --monitor: bool
    --port(-p): string
    --baud(-b): int
]

export extern "platformio lib" [
    subcommand?: string
    ...args: string
    --global(-g): bool
    --storage(-d): string
]

export extern "platformio platform" [
    subcommand?: string
    ...args: string
    --with-package: string
    --without-package: string
]

export extern "platformio check" [
    ...targets: string
    --environment(-e): string
    --severity: string
    --InlineSuppressions: bool
    --std: string
]

export extern "platformio debug" [
    ...targets: string
    --environment(-e): string
    --interface(-i): string
    --adapter: string
    --programmer: string
]

export extern "platformio remote" [
    subcommand?: string
    ...args: string
]

export extern "platformio boards" [
    platform?: string@"nu-complete pio platforms"
    --json: bool
    --filter(-f): string
]

export extern "platformio test" [
    ...targets: string
    --environment(-e): string
    --filter(-f): string
    --ignore: string
    --verbose(-v): bool
]

export extern "platformio home" [
    --host: string
    --port(-p): int
    --no-open: bool
]

export extern "platformio settings" [
    subcommand?: string
    ...args: string
    --list: bool
]

export extern "platformio account" [
    subcommand?: string
    ...args: string
]

export extern "platformio org" [
    subcommand?: string
    ...args: string
]

export extern "platformio remote" [
    subcommand?: string
    ...args: string
    --list: bool
]

export extern "platformio report" [
    subcommand?: string
    ...args: string
]

export extern "platformio asset" [
    subcommand?: string
    ...args: string
]

export extern "platformio piocore" []
