# yasb-chrome

Shared Win32 panel chrome, acrylic canvas, and window plumbing used across all Rangalipi custom widgets (`saka-popup`, `palette-picker`, and `keybind`).

## Rationale

Custom native widgets in this rice are built with raw Win32 + GDI rather than heavy GUI frameworks or embedded webviews. `yasb-chrome` consolidates the low-level rendering mathematics so that:
1. **Corner Radii & Glass Do Not Drift**: All popups share the exact same anti-aliasing curve math, padding ratios, outer hairline highlight, and drop shadow.
2. **Layered Window Alpha Trap Avoidance**: Standard GDI writes `alpha = 0` into 32-bit bitmap pixel memory. `yasb-chrome` provides pixel-level alpha post-processing and premultiplication routines required by Win32 `UpdateLayeredWindow`.
3. **Single-Instance Enforcement**: Named Win32 Mutex primitives and inter-process activation allow clicking a widget's icon repeatedly to toggle or refocus without multi-process leakage.

## Modules

| Module | Purpose |
|---|---|
| `canvas.rs` | 32-bit ARGB software canvas: rounded rectangle anti-aliasing, linear gradients, inner/outer hairlines, drop shadows, radial moon spheres |
| `gdi.rs` | Dual-pass backdrop blur, GDI device context double-buffering, blitting to screen or layered window |
| `scale.rs` | Apple HIG typographic hierarchy, vertical metrics, baseline alignment |
| `singleton.rs` | Win32 Mutex single-instance guard and window foreground promotion (`SetForegroundWindow`) |
| `typeface.rs` | ClearType font matching with fallback chains (preferring JetBrainsMono NF, FiraCode NF, and Telugu fonts) |

## Tests

```powershell
cargo test --manifest-path tools\chrome\yasb-chrome\Cargo.toml
```
37 unit tests covering alpha blending, corner geometry, blur impulse preservation, ClearType font substitution, and mutex locks.
