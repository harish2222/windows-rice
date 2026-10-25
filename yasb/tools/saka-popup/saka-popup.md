<!--
Author -: HKDEVS
Project -: Window-Rice
Title -: Saka Indian Calendar Moon Phase Popup Guide
-->
# saka-popup — The Vedic Panchangam & Lunar Portal Panel

A frameless, always-on-top, Apple HIG-inspired Win32 panel displaying the complete Vedic Panchangam with dual hero moon discs, realistic telescopic selenography, solar timeline, and responsive layouts.

## Visual Design & Hero Moons

The header features dual hero moon discs ($116\text{px}$ diameter in Portrait/Landscape, $108\text{px}$ in Compact):
1. **3D Rotating Moon (Left)**: Renders a 3D rotating planetary sphere with physical lighting and spherical surface coordinates, demonstrating rotation over time.
2. **Telescopic Night Sky View (Right)**: An authentic Earth-facing astronomical telescope view. Unlike generic procedural spheres, it renders actual near-side lunar geography:
   - Major lunar maria: *Mare Imbrium*, *Oceanus Procellarum*, *Mare Serenitatis*, *Mare Tranquillitatis*, *Mare Crisium*, *Mare Nubium*.
   - Bright crater ray systems: *Tycho* (radiating south across the highlands), *Copernicus*, and *Kepler*.
   - Celestial Star Portal: Deep cosmic void background (`#050812` to `#0a1224`), thin optical lens rim highlight, forward-scattering atmospheric corona, physical earthshine, and a deterministic 24-star celestial starfield.

## Three Layout Modes

Press **M** to cycle layout modes. The panel persists your chosen mode across launches in `saka-state.json`:

| Mode | Dimensions (EN / TE) | Geometry & Purpose |
|---|---|---|
| **Portrait** | $760\text{px} \times 920\text{px}$ / $820\text{px} \times 976\text{px}$ | Vertical command center: dual hero moons, dedicated Udaya Tithi and Day Duration clearance bands, solar timeline, and 11 full-width panchang rows with progress bars and captions. |
| **Landscape** | $1140\text{px} \times 622\text{px}$ / $1180\text{px} \times 630\text{px}$ | Wide dual-column view: Left card anchors hero moons, solar arc, and day metrics; Right card presents a spacious 2-column grid of panchangam attributes. |
| **Compact** | $680\text{px} \times 502\text{px}$ / $720\text{px} \times 518\text{px}$ | Responsive desktop glance widget: Card 1 features Hero Tithi progress bar + 2×2 auto-fitting chip matrix (Nakshatra, Yoga, Karana, Vara). Card 2 features Sunrise/Sunset with elevated Day Duration pill + solar progress track + 2×2 cycle chip matrix (Phase, Amanta, Purnimanta, Ends). |

## Interactive Controls

| Input | Action |
|---|---|
| **Tab** | Toggle Telugu / English script |
| **M** | Cycle layout mode (Portrait → Landscape → Compact) |
| **Escape** | Close panel |
| **Click Outside / Focus Loss** | Automatically dismiss |

## Panchangam Data Attributes

1. **Tithi**: Current lunar day with real-time percentage progress bar and exact start/end timestamps.
2. **Udaya Tithi & Day Duration**: Clear non-overlapping display of sunrise tithi and total diurnal duration.
3. **Nakshatra**: Active lunar mansion, ruling planet/deity, and remaining duration.
4. **Yoga**: Auspicious/inauspicious solar-lunar angular combination.
5. **Karana**: Half-tithi active period.
6. **Vara**: Solar weekday with planetary lord.
7. **Western & Vedic Moon Phase**: Phase name, exact illumination percentage, and transition countdown.
8. **Next Major Phase**: Date and time of next Purnima (Full) or Amavasya (New) moon.
9. **Amanta & Purnimanta Months**: Current month names in both lunar reckoning traditions.
10. **Solar Timeline**: Visual sunrise/sunset track with real-time solar pip progress.
11. **Saka & Vikram Samvat Eras**: Traditional astronomical calendar years.

## Theming & Apple HIG Aesthetics

- **Dynamic CSS Integration**: Inherits active palette tokens (`--background`, `--text`, `--accent`, `--surface0`, `--border`, `--hairline`) from `styles.css` via `yasb-theme`.
- **Derived Contrast Floor**: Automatically calculates required background opacity to satisfy WCAG AA contrast (>= 4.5:1) against any wallpaper behind the acrylic window.
- **Bilingual Typographic Hierarchy**: ClearType font resolution using Segoe UI Variable for Latin and Nirmala UI / Gautami for Telugu.

## Dev & Headless Verification

```powershell
# Run 50 unit tests
cargo test --manifest-path tools\saka-popup\Cargo.toml

# Smoke test (renders all rows and metrics headlessly)
saka-popup.exe --smoke

# Headless snapshot rendering for visual QA
saka-popup.exe --snapshot preview.png --mode compact --te
```
