# Shared window-manager configs

Snapshots of the live configs that run alongside this YASB bar, kept here
so the whole setup replicates from one folder.

| File | Live location (authoritative) |
|---|---|
| `glazewm/config.yaml` | `~\.glzr\glazewm\config.yaml` (hotkey layer only; Komorebi tiles) |
| `komorebi/komorebi.json` | `~\komorebi.json` (tiling engine) |
| `komorebi/komorebi-startup.ps1` | `~\.config\komorebi\komorebi-startup.ps1` (login boot chain) |

Refresh: run `tools\setup\yasb-setup.ps1` (snapshot step), or copy back to
restore. The bar itself is `config.yaml` + `styles.css` at this root;
companion tools live in `tools\` (`theme`, `picker`, `setup`).
