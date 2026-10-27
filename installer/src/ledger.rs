// ============================================================================
// Author -: HKDEVS
// Project -: Window-Rice
// Title -: WINDOW-RICE Installed Targets & Wallpaper Entries Ledger
// ============================================================================
// Durable record of every config target the installer set up itself, so the
// revoke path can answer the critical question: "was this file placed by the
// Window-Rice repo, or is it the user's own config?" The backup manifest
// (manifest.json) records USER files we displaced; this ledger records
// REPO-owned links/copies we created, plus the individual wallpaper entries
// the wallpaper flow materialized (entries-only removal, never a blind
// whole-directory nuke).
//
// Storage: <backup-root>\installed.json (RICE_BACKUP_DIR override honored in
// tests so the real profile is never touched).
// ============================================================================
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::Result;
use serde::{Deserialize, Serialize};

static LEDGER_MUTEX: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LinkMode {
    Symlink,
    Copy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledRecord {
    pub pkg_id: String,
    /// Absolute resolved target path that now points into the repo.
    pub target: PathBuf,
    /// Absolute repo-source path the target points to (or was copied from).
    pub source: PathBuf,
    pub mode: LinkMode,
    pub is_dir: bool,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallpaperEntry {
    /// Destination root (e.g. Pictures\Wallpapers) the entry was created in.
    pub dest: PathBuf,
    /// Name of the folder/file created below `dest`.
    pub entry_name: String,
    pub is_symlink: bool,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstalledLedger {
    pub records: Vec<InstalledRecord>,
    pub wallpaper_entries: Vec<WallpaperEntry>,
}

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn get_ledger_path() -> PathBuf {
    crate::linker::get_backup_root().join("installed.json")
}

pub fn load_ledger() -> InstalledLedger {
    let path = get_ledger_path();
    if path.exists() {
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(ledger) = serde_json::from_str::<InstalledLedger>(&data) {
                return ledger;
            }
        }
    }
    InstalledLedger::default()
}

pub fn save_ledger(ledger: &InstalledLedger) -> Result<()> {
    let root = crate::linker::get_backup_root();
    fs::create_dir_all(&root)?;
    fs::write(get_ledger_path(), serde_json::to_string_pretty(ledger)?)?;
    Ok(())
}

/// Record (idempotently) that the installer owns `target` via the given mode.
/// A same (pkg_id, target) pair is replaced, never duplicated.
pub fn record_installed(pkg_id: &str, source: &PathBuf, target: &PathBuf, mode: LinkMode, is_dir: bool) {
    let _guard = LEDGER_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    let mut ledger = load_ledger();
    ledger
        .records
        .retain(|r| !(r.pkg_id == pkg_id && r.target == *target));
    ledger.records.push(InstalledRecord {
        pkg_id: pkg_id.to_string(),
        target: target.clone(),
        source: source.clone(),
        mode,
        is_dir,
        timestamp: now_ts(),
    });
    let _ = save_ledger(&ledger);
}

/// Ensure a symlink record exists for (pkg, target) — used when a link is
/// already valid so revoke still knows the target is repo-owned.
pub fn ensure_symlink_record(pkg_id: &str, source: &PathBuf, target: &PathBuf, is_dir: bool) {
    let _guard = LEDGER_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    let mut ledger = load_ledger();
    if ledger
        .records
        .iter()
        .any(|r| r.pkg_id == pkg_id && r.target == *target && r.mode == LinkMode::Symlink)
    {
        return;
    }
    ledger
        .records
        .retain(|r| !(r.pkg_id == pkg_id && r.target == *target));
    ledger.records.push(InstalledRecord {
        pkg_id: pkg_id.to_string(),
        target: target.clone(),
        source: source.clone(),
        mode: LinkMode::Symlink,
        is_dir,
        timestamp: now_ts(),
    });
    let _ = save_ledger(&ledger);
}

/// Drop the (pkg, target) record after a successful removal.
pub fn remove_record(pkg_id: &str, target: &PathBuf) {
    let _guard = LEDGER_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    let mut ledger = load_ledger();
    ledger
        .records
        .retain(|r| !(r.pkg_id == pkg_id && r.target == *target));
    let _ = save_ledger(&ledger);
}

/// Drop every installed record and wallpaper entry (used after a completed
/// full revoke to leave no stale tracking files behind).
pub fn clear_all() {
    let _guard = LEDGER_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    let _ = save_ledger(&InstalledLedger::default());
}

/// Record one wallpaper entry the installer just created below `dest`.
pub fn record_wallpaper_entry(dest: &PathBuf, entry_name: &str, is_symlink: bool) {
    let _guard = LEDGER_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    let mut ledger = load_ledger();
    ledger
        .wallpaper_entries
        .retain(|e| !(e.dest == *dest && e.entry_name == entry_name));
    ledger.wallpaper_entries.push(WallpaperEntry {
        dest: dest.clone(),
        entry_name: entry_name.to_string(),
        is_symlink,
        timestamp: now_ts(),
    });
    let _ = save_ledger(&ledger);
}

/// All ledger wallpaper entries for a destination root.
pub fn wallpaper_entries_for(dest: &PathBuf) -> Vec<WallpaperEntry> {
    load_ledger()
        .wallpaper_entries
        .into_iter()
        .filter(|e| e.dest == *dest)
        .collect()
}

/// Remove one wallpaper entry record.
pub fn remove_wallpaper_entry(dest: &PathBuf, entry_name: &str) {
    let _guard = LEDGER_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    let mut ledger = load_ledger();
    ledger
        .wallpaper_entries
        .retain(|e| !(e.dest == *dest && e.entry_name == entry_name));
    let _ = save_ledger(&ledger);
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn temp_ledger_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rice_test_ledger_{}", name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("RICE_BACKUP_DIR", &dir) };
        dir
    }

    #[test]
    fn test_record_and_dedup() {
        let dir = temp_ledger_dir("dedup");
        let src = dir.join("repo-src");
        let tgt = dir.join("tgt");
        record_installed("pkg1", &src, &tgt, LinkMode::Copy, false);
        record_installed("pkg1", &src, &tgt, LinkMode::Symlink, false);
        let ledger = load_ledger();
        assert_eq!(
            ledger.records.iter().filter(|r| r.pkg_id == "pkg1").count(),
            1,
            "same (pkg,target) pair must never duplicate"
        );
        assert_eq!(ledger.records[0].mode, LinkMode::Symlink);
        unsafe { std::env::remove_var("RICE_BACKUP_DIR") };
    }

    #[test]
    fn test_remove_record_and_clear() {
        let dir = temp_ledger_dir("remove");
        let src = dir.join("repo-src");
        record_installed("p1", &src, &dir.join("a"), LinkMode::Copy, true);
        record_installed("p1", &src, &dir.join("b"), LinkMode::Symlink, true);
        record_installed("p2", &src, &dir.join("c"), LinkMode::Copy, false);
        record_wallpaper_entry(&dir.join("pics"), "Dir1", true);
        record_wallpaper_entry(&dir.join("pics"), "Dir2", false);

        remove_record("p1", &dir.join("a"));
        let l1 = load_ledger();
        assert_eq!(l1.records.len(), 2, "only target 'a' record removed");
        assert!(!l1.records.iter().any(|r| r.target == dir.join("a")));

        remove_wallpaper_entry(&dir.join("pics"), "Dir1");
        let l2 = load_ledger();
        assert_eq!(l2.wallpaper_entries.len(), 1);
        assert_eq!(l2.wallpaper_entries[0].entry_name, "Dir2");

        clear_all();
        let l3 = load_ledger();
        assert!(l3.records.is_empty() && l3.wallpaper_entries.is_empty());
        unsafe { std::env::remove_var("RICE_BACKUP_DIR") };
    }

    #[test]
    fn test_wallpaper_entries_for_filters_by_dest() {
        let dir = temp_ledger_dir("filter");
        let d1 = dir.join("pictures");
        let d2 = dir.join("other");
        record_wallpaper_entry(&d1, "OnlyD1", true);
        record_wallpaper_entry(&d2, "OnlyD2", true);
        let got = wallpaper_entries_for(&d1);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].dest, d1);
        unsafe { std::env::remove_var("RICE_BACKUP_DIR") };
    }
}
