# hotkeys-overlay — Alt+/ cheatsheet for the rice

An **Omarchy-style hotkey reference overlay**: press `Alt+/` anywhere and
a translucent cheat-sheet pops up listing every binding in the rice
(focus/move hjkl, resize mode, fullscreen, terminal, browser, editor,
komorebic passthroughs, …). Press it again — or hit Esc, Enter, any
letter, or click — and it's gone.

## Design

- **Zero external crates**: raw Win32 via hand-rolled `extern "system"`
  declarations (`RegisterClass`, `CreateWindowEx`, `WM_PAINT`,
  `SetLayeredWindowAttributes`, ...). The overlay is a borderless,
  always-on-top, layered window; no WebView, no Qt, no runtime deps.
- **Toggle semantics**: first launch shows the window; a second launch
  finds the existing window by class/title and closes it — so the same
  hotkey is both show and hide, and no resident daemon is needed.
- **Single file**: all 645 lines live in `src/main.rs`; `Cargo.toml`
  has an empty `[dependencies]` section, so builds are instant and
  reproducible offline.

## Bind & build

GlazeWM's config (`configs/glazewm/config.yaml`) binds `alt+slash` to
shell-exec this exe. Rebuild after edits:

```sh
cd ~/.config/hotkeys-overlay && cargo build --release
```

The bar's `komorebi` layer and this overlay never conflict: the overlay
ignores window-management rules entirely (it's a plain popup-class
window, and komorebi's float rules treat it as unmanaged).
