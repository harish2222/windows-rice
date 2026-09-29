# Shared window-manager configs

Snapshots of the live configs that run alongside this YASB bar, kept here
so the whole setup replicates from one folder.

| File | Live location (authoritative) |
|---|---|
| `glazewm/config.yaml` | `~\.glzr\glazewm\config.yaml` (hotkey layer only; Komorebi tiles) |
| `komorebi/komorebi.json` | `~\.config\komorebi\komorebi.json` (tiling engine; animations + Wine border colours) |
| `komorebi/komorebi.bar.json` | `~\.config\komorebi\komorebi.bar.json` (built-in bar config; the YASB bar is the active one) |
| `komorebi/applications.json` | `~\.config\komorebi\applications.json` (per-app rules) |
| `komorebi/komorebi-startup.ps1` | `~\.config\komorebi\komorebi-startup.ps1` (login boot chain) |

Refresh: run `tools\setup\yasb-setup.ps1` (snapshot step), or copy back to
restore. The bar itself is `config.yaml` + `styles.css` at this root;
companion tools live in `tools\` (`theme`, `picker`, `setup`).
