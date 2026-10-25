<!--
Author -: HKDEVS
Project -: Window-Rice
Title -: Windows Rice Project Documentation & Architecture
-->
# screen-shot

Fast, layered-window capable screen capture CLI written in Rust for Windows.

## The Layered Window Capture Problem

Windows rendered using `WS_EX_LAYERED` with `UpdateLayeredWindow` (such as `saka-popup`, `palette-picker`, and `keybind`) are invisible to standard GDI `PrintWindow(..., PW_RENDERFULLCONTENT)` and many Python PIL `ImageGrab` captures. They come back completely blank or transparent.

`screen-shot` solves this by acquiring the screen DC directly and issuing `BitBlt` with the `CAPTUREBLT` flag (`0x40000000`), accurately capturing layered acrylic windows, drop shadows, and ClearType text.

## Usage

```powershell
# Capture entire screen
cargo run --release --manifest-path tools\screen-shot\Cargo.toml -- output.png

# Capture specific window by title/class or rect
cargo run --release --manifest-path tools\screen-shot\Cargo.toml -- --x 100 --y 100 --w 800 --h 600 crop.png
```
