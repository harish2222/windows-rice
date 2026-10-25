<!--
Author -: HKDEVS
Project -: Window-Rice
Title -: YASB Komorebi Workspace State Synchronizer Documentation
-->
# komorebi-sync

Lightweight background synchronization helper that updates YASB bar workspace labels and titles based on Komorebi's live window state.

## Features

- Subscribes to Komorebi socket events or queries `komorebic state`.
- Formats workspace buttons dynamically (e.g. showing application icons or numbering).
- Filters out ignored floating or background processes.

## Tests

```powershell
cargo test --manifest-path tools\komorebi-sync\Cargo.toml
```
