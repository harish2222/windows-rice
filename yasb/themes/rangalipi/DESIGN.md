# Apple Widget Design System Specification (DESIGN.md)

This document establishes the Apple Human Interface Guidelines (HIG) and WidgetKit-inspired design principles governing the **native Win32/Rust custom widgets** (`keybind`, `palette-picker`, and `saka-popup`), while preserving the authentic **Madhubani / Rangalipi rice design** for native YASB Qt bar widgets and popups.

---

## 1. System Division of Design Responsibility

- **Native YASB Bar & Native Qt Popups**:
  - Maintain the authentic Rangalipi / Madhubani rice aesthetic: classic double frame borders (`border: 3px double var(--border)`), standard radius hierarchy (`--border-radius: 12px`), CSS-only adaptive island frosted glass, and strictly icon-only active window buttons.
- **Custom Native Widgets (`keybind`, `palette-picker`, `saka-popup`)**:
  - Adhere strictly to the Apple Widget System (HIG / WidgetKit): 18px squircles, translucent acrylic materials, 1px hairline rim strokes, clear typography hierarchy, and instant glanceability.

Apple's widget architecture is built on three central tenets:
- **Glanceability**: Widgets and flyouts are not dense desktop applications. They communicate their primary state in under two seconds.
- **Deference**: The chrome exists solely to elevate the content. Panels use liquid glass, subtle rim lighting, and translucent materials that blend into desktop wallpapers rather than fighting them with heavy borders or opaque slabs.
- **Depth**: Spatial hierarchy is expressed through layered translucency, soft ambient shadows, and continuous curvature (squircles).

---

## 2. Materials & Visual Language

### A. Surface Materials & Liquid Glass
1. **Elevated Panels (Popups & Dialogs)**:
   - Primary flyout canvases paint `--glassmenu` / `--acrylic` at **50% opacity** across light themes and translucent tint in dark themes.
   - Backgrounds let the desktop wallpaper bleed through softly with high-radius Gaussian backdrop blur.
2. **Inner Cards & Tiles**:
   - Secondary content cards (e.g. volume sliders, theme swatch cells, panchang bands, keybinding rows) sit on `--tile` (`rgba(255, 255, 255, 0.05)` in dark, `rgba(0, 0, 0, 0.04)` in light).
3. **Hairline Rim Stroke**:
   - Replaced all legacy 3px double/solid borders with a singular, crisp **1px hairline stroke** (`var(--hairline)`). In dark mode: `rgba(255, 255, 255, 0.14)`; in light mode: `rgba(0, 0, 0, 0.08)`.
   - Simulates Apple's frosted edge specular highlight.

### B. Geometry & Squircles
Apple interfaces avoid sharp corners or circular pill extremes for rectangular cards, favoring continuous curvature:
- **Flyout Windows / Main Shells**: `border-radius: 18px` (`--border-radius`)
- **Inner Interactive Tiles & Cards**: `border-radius: 12px` (`--border-radius2`)
- **Pills, Badges & Toggles**: `border-radius: 9999px` (fully rounded capsule for keys, tags, and segmented controls)

---

## 3. Typography & Information Hierarchy

- **Font Family**: San Francisco / FiraCode Nerd Font / JetBrainsMono Nerd Font.
- **Weight Hierarchy**:
  - **Heading / Title**: 600-700 (Semibold / Bold) with clean letter-spacing.
  - **Body / Primary Labels**: 500 (Medium).
  - **Secondary Hints / Subtext**: 400 (Regular) tinted with `--subtext` at ~60% opacity.
  - **Tertiary / Inactive Metadata**: `--text-faint` at ~35-40% opacity.
- **Accents**:
  - Color is applied with surgical restraint: only active states, selection indicators, or critical alerts receive `--accent`.

---

## 4. Custom Widget Implementations

### A. Keybindings Cheatsheet (`tools/keybind`)
- **Anatomy**:
  - **Window**: 680×440px floating card with `RADIUS = 18px` and 1px hairline border.
  - **Search Bar**: Capsule pill (`SEARCH_R = 12px`) with subtle inner background and live debounce.
  - **Key Badges**: Soft pill badges (`badge_bg` + `badge_border`) matching macOS keyboard shortcut displays.
  - **Scrollbar**: Elegant Apple-style auto-fading thumb that tracks mouse wheel and drag gestures without cluttering the view.
  - **Rendering**: DirectWrite / ClearType subpixel rendering with high contrast against the translucent acrylic pane.

### B. Theme Palette Switcher (`tools/picker/palette-picker`)
- **Anatomy**:
  - **Window**: 1020px wide panel with `RADIUS = 18px`, framed by an ambient edge highlight.
  - **Segmented Control**: Two-way pill toggle for **Frosted Glass** (`ON / OFF`), sliding smoothly with high-contrast active state.
  - **Cards**: 3-column grid of 12px rounded cards (`CELL_R = 12px`) featuring 4-swatch palettes and an antialiased dot indicating the current live theme.
  - **Frosted Glass Integration**: Drives CSS-only island translucency without touching OS DWM whole-bar blur.

### C. Saka Panchang Popup (`tools/saka-popup`)
- **Anatomy**:
  - **Header**: Weekday pill card and moon phase glyph with soft backdrop gradient.
  - **Panchang Cards**: Metric rows (Tithi, Nakshatra, Yoga, Karana, Sun times) rendered with dual-script typography (Latin + Telugu) and a progress bar for lunar phase completion.
  - **Materials**: 18px squircle window with layered translucency.

### D. Active Window & Workspaces (`YASB Bar`)
- **Icon-Only Window Representation**:
  - In `komorebi_stack` and `active_window`, window titles/text are completely hidden (`.label { display: none; }`).
  - Open windows in the current workspace appear strictly as high-resolution application icons.
  - The focused window is highlighted with `.window.active`.
- **Dynamic Workspaces**:
  - Empty unactive workspaces are automatically hidden (`hide_empty_workspaces: true`).
  - Numbers are clean and unadorned without bottom underline boxes.
