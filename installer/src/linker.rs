// ============================================================================
// Author -: HKDEVS
// Project -: Window-Rice
// Title -: WINDOW-RICE Installer Symlink, Backup, and Restoration Engine
// ============================================================================
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use crate::catalog::{ConfigMapping, resolve_target_path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkOutcome {
    SymlinkCreated(PathBuf),
    CopiedFallback(PathBuf),
    AlreadyValid(PathBuf),
    BackedUpAndLinked { target: PathBuf, backup: PathBuf },
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnlinkOutcome {
    UnlinkedAndRestored { target: PathBuf, backup_source: PathBuf },
    UnlinkedCleanly(PathBuf),
    /// Copy-fallback config where the user made their own edits: archived and
    /// LEFT on disk — the rice never deletes user-modified configuration.
    PreservedUserModified { target: PathBuf, archive: PathBuf },
    /// Source and target are the same repo file (repo lives natively at the
    /// target). Removing it would delete the repository itself - never touch.
    RepoNativeNotTouched(PathBuf),
    NotLinked(PathBuf),
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRecord {
    pub package_id: String,
    pub original_target: PathBuf,
    pub backup_path: PathBuf,
    pub timestamp: u64,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BackupManifest {
    pub records: Vec<BackupRecord>,
}

pub fn get_backup_root() -> PathBuf {
    if let Some(override_dir) = std::env::var_os("RICE_BACKUP_DIR").filter(|d| !d.is_empty()) {
        return PathBuf::from(override_dir);
    }
    if let Some(user_profile) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        PathBuf::from(user_profile).join(".rice-backups")
    } else {
        PathBuf::from(".rice-backups")
    }
}

pub fn get_manifest_path() -> PathBuf {
    get_backup_root().join("manifest.json")
}

pub fn load_manifest() -> BackupManifest {
    let path = get_manifest_path();
    if path.exists() {
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(manifest) = serde_json::from_str(&data) {
                return manifest;
            }
        }
    }
    BackupManifest::default()
}

pub fn save_manifest(manifest: &BackupManifest) -> Result<()> {
    let root = get_backup_root();
    fs::create_dir_all(&root)?;
    let path = get_manifest_path();
    let data = serde_json::to_string_pretty(manifest)?;
    fs::write(path, data)?;
    Ok(())
}

pub fn backup_existing_target(package_id: &str, target: &Path, is_dir: bool) -> Result<Option<PathBuf>> {
    if !target.exists() {
        return Ok(None);
    }

    // If target is already a symlink, check if it points to repo
    if let Ok(meta) = target.symlink_metadata() {
        if meta.file_type().is_symlink() {
            return Ok(None);
        }
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let backup_dir = get_backup_root().join(package_id).join(timestamp.to_string());
    fs::create_dir_all(&backup_dir)?;

    let file_name = target.file_name().unwrap_or_else(|| std::ffi::OsStr::new("backup"));
    let backup_dest = backup_dir.join(file_name);

    if is_dir {
        copy_recursive(target, &backup_dest, true)?;
        let _ = fs::remove_dir_all(target);
    } else {
        fs::copy(target, &backup_dest)?;
        let _ = fs::remove_file(target);
    }

    // Record in manifest
    let mut manifest = load_manifest();
    manifest.records.push(BackupRecord {
        package_id: package_id.to_string(),
        original_target: target.to_path_buf(),
        backup_path: backup_dest.clone(),
        timestamp,
        is_dir,
    });
    let _ = save_manifest(&manifest);

    Ok(Some(backup_dest))
}

pub fn restore_target_backup(package_id: &str, target: &Path) -> Result<Option<PathBuf>> {
    let mut manifest = load_manifest();
    let record_idx = manifest.records.iter().rposition(|r| {
        r.package_id == package_id
            && (r.original_target == target
                || (r.original_target.canonicalize().is_ok()
                    && r.original_target.canonicalize().ok() == target.canonicalize().ok()))
    });

    if let Some(idx) = record_idx {
        let record = manifest.records.remove(idx);
        let _ = save_manifest(&manifest);

        if record.backup_path.exists() {
            if let Some(parent) = target.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if target.exists() || target.symlink_metadata().is_ok() {
                if record.is_dir {
                    let _ = fs::remove_dir_all(target);
                } else {
                    let _ = fs::remove_file(target);
                }
            }

            if record.is_dir {
                copy_recursive(&record.backup_path, target, true)?;
                let _ = fs::remove_dir_all(&record.backup_path);
            } else {
                let _ = fs::copy(&record.backup_path, target);
                let _ = fs::remove_file(&record.backup_path);
            }

            // Prune empty parent directory (timestamp folder) and grandparent (package folder)
            if let Some(parent) = record.backup_path.parent() {
                let _ = fs::remove_dir(parent);
                if let Some(grandparent) = parent.parent() {
                    if grandparent != get_backup_root() {
                        let _ = fs::remove_dir(grandparent);
                    }
                }
            }

            return Ok(Some(record.backup_path));
        }
    }

    Ok(None)
}

pub fn link_config_mapping(repo_root: &Path, mapping: &ConfigMapping, package_id: &str) -> LinkOutcome {
    let source = repo_root.join(mapping.source_rel);
    let target = resolve_target_path(mapping.target_pattern);

    if !source.exists() {
        return LinkOutcome::Failed(format!("Source does not exist: {}", source.display()));
    }

    // 1. If source and target are the exact same canonical path, it's already in-place
    if source == target || source.canonicalize().ok() == target.canonicalize().ok() {
        // Repo-native layout (this machine's ~/.config IS the repo): nothing
        // is "installed" by us here, so no ledger record must ever exist —
        // revoke guards refuse to touch repo-native targets.
        return LinkOutcome::AlreadyValid(target);
    }

    // 1b. Repo protection: NEVER write, replace, back up or delete anything
    //     inside the repository tree (.config). This project's configs ARE
    //     the repo, so a target resolving into it (but != the source above)
    //     would destroy tracked files. Refuse loudly instead.
    if is_repo_internal(&target, repo_root) {
        return LinkOutcome::Failed(format!(
            "Refusing to modify repo-internal path (repository files are read-only to the installer): {}",
            target.display()
        ));
    }

    // 2. Ensure parent directory exists
    if let Some(parent) = target.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return LinkOutcome::Failed(format!("Failed to create parent dir {}: {}", parent.display(), e));
        }
        // Re-check AFTER materializing the parent: a previously dangling (or
        // otherwise uncanonicalizable) parent symlink into the repo only
        // resolves once the directory exists - re-evaluate so the guard can
        // never be evaded through an unresolved parent.
        if is_repo_internal(&target, repo_root) {
            return LinkOutcome::Failed(format!(
                "Refusing to modify repo-internal path (repository files are read-only to the installer): {}",
                target.display()
            ));
        }
    }

    // 3. Check if target is already pointing to source via symlink
    if let Ok(meta) = target.symlink_metadata() {
        if meta.file_type().is_symlink() {
            if let Ok(dest) = fs::read_link(&target) {
                if dest == source || dest.canonicalize().ok() == source.canonicalize().ok() {
                    crate::ledger::ensure_symlink_record(package_id, &source, &target, mapping.is_dir);
                    return LinkOutcome::AlreadyValid(target);
                }
            }
            // Remove old or broken symlink
            let _ = if mapping.is_dir {
                fs::remove_dir(&target)
            } else {
                fs::remove_file(&target)
            };
        }
    }

    // 4. If target exists as regular user file/dir, BACK IT UP!
    let mut backup_created = None;
    if target.exists() {
        match backup_existing_target(package_id, &target, mapping.is_dir) {
            Ok(Some(bp)) => backup_created = Some(bp),
            Ok(None) => {},
            Err(e) => {
                return LinkOutcome::Failed(format!("Failed to backup existing target {}: {}", target.display(), e));
            }
        }
    }

    // 5. Try creating symbolic link
    let symlink_res = if mapping.is_dir {
        #[cfg(windows)]
        {
            if target.exists() {
                let _ = fs::remove_dir_all(&target);
            }
            std::os::windows::fs::symlink_dir(&source, &target)
        }
        #[cfg(not(windows))]
        {
            if target.exists() {
                let _ = fs::remove_dir_all(&target);
            }
            std::os::unix::fs::symlink(&source, &target)
        }
    } else {
        #[cfg(windows)]
        {
            if target.exists() {
                let _ = fs::remove_file(&target);
            }
            std::os::windows::fs::symlink_file(&source, &target)
        }
        #[cfg(not(windows))]
        {
            if target.exists() {
                let _ = fs::remove_file(&target);
            }
            std::os::unix::fs::symlink(&source, &target)
        }
    };

    match symlink_res {
        Ok(_) => {
            crate::ledger::record_installed(
                package_id,
                &source,
                &target,
                crate::ledger::LinkMode::Symlink,
                mapping.is_dir,
            );
            if let Some(bp) = backup_created {
                LinkOutcome::BackedUpAndLinked { target, backup: bp }
            } else {
                LinkOutcome::SymlinkCreated(target)
            }
        }
        Err(e) => {
            // Symlink failed fallback to recursive copy
            if let Err(copy_err) = copy_recursive(&source, &target, mapping.is_dir) {
                LinkOutcome::Failed(format!("Symlink failed ({}) and copy fallback failed: {}", e, copy_err))
            } else {
                crate::ledger::record_installed(
                    package_id,
                    &source,
                    &target,
                    crate::ledger::LinkMode::Copy,
                    mapping.is_dir,
                );
                if let Some(bp) = backup_created {
                    LinkOutcome::BackedUpAndLinked { target, backup: bp }
                } else {
                    LinkOutcome::CopiedFallback(target)
                }
            }
        }
    }
}

/// Byte-for-byte equivalence check between a repo source and an installed
/// copy. For directories: same relative file set (no extras added by the
/// user) and identical contents for every file. Used to decide whether a
/// copy-fallback install is still pure repo data (safe to remove) or has
/// diverged (user's own - preserve).
pub fn paths_equivalent(src: &Path, dst: &Path, is_dir: bool) -> bool {
    if !src.exists() || !dst.exists() {
        return false;
    }
    if !is_dir {
        return file_contents_equal(src, dst);
    }
    if !src.is_dir() || !dst.is_dir() {
        return false;
    }
    fn walk(src: &Path, dst: &Path) -> bool {
        let src_entries: std::collections::BTreeSet<String> = match fs::read_dir(src) {
            Ok(rd) => rd
                .flatten()
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect(),
            Err(_) => return false,
        };
        let dst_entries: std::collections::BTreeSet<String> = match fs::read_dir(dst) {
            Ok(rd) => rd
                .flatten()
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect(),
            Err(_) => return false,
        };
        // Any file the user added (or any file the rice never had) means the
        // directory is no longer pure repo content.
        if src_entries != dst_entries {
            return false;
        }
        for name in &src_entries {
            let s = src.join(name);
            let d = dst.join(name);
            if s.is_dir() {
                if !d.is_dir() || !walk(&s, &d) {
                    return false;
                }
            } else if !file_contents_equal(&s, &d) {
                return false;
            }
        }
        true
    }
    walk(src, dst)
}

fn file_contents_equal(a: &Path, b: &Path) -> bool {
    let Ok(am) = fs::metadata(a) else { return false };
    let Ok(bm) = fs::metadata(b) else { return false };
    if am.len() != bm.len() {
        return false;
    }
    use std::io::Read;
    let (Ok(mut fa), Ok(mut fb)) = (fs::File::open(a), fs::File::open(b)) else {
        return false;
    };
    const CHUNK: usize = 64 * 1024;
    let mut ba = vec![0u8; CHUNK];
    let mut bb = vec![0u8; CHUNK];
    loop {
        let ra = fa.read(&mut ba).unwrap_or(usize::MAX);
        let rb = fb.read(&mut bb).unwrap_or(usize::MAX);
        if ra != rb {
            return false;
        }
        if ra == 0 {
            return true;
        }
        if ba[..ra] != bb[..rb] {
            return false;
        }
    }
}

/// Ownership-aware removal of one config mapping.
///
/// Decision tree per target:
///   1. Repo-native (source == target canonical path)  -> RepoNativeNotTouched.
///      The repository IS the config here; deleting it would delete the repo.
///   2. Symlink pointing elsewhere                     -> remove link, restore
///      user backup when the manifest has one.
///   3. Real file/dir WITH a manifest backup           -> backup is the user's
///      original; current copy is repo data -> restore backup.
///   4. Real file/dir WITHOUT backup, still identical
///      to the repo source                             -> pure rice data;
///      safe to remove (UnlinkedCleanly).
///   5. Real file/dir WITHOUT backup, user modified it -> Preserve: archive a
///      copy into .rice-backups, leave the file untouched, report.
pub fn unlink_config_mapping(repo_root: &Path, mapping: &ConfigMapping, package_id: &str) -> UnlinkOutcome {
    let source = repo_root.join(mapping.source_rel);
    let target = resolve_target_path(mapping.target_pattern);

    let symlink_meta = target.symlink_metadata().ok();
    let is_symlink = symlink_meta.as_ref().map_or(false, |m| m.file_type().is_symlink());

    // --- 1. Repo-native guard: source and target are the same file ------ //
    // The canonical comparison must only collapse REAL files/dirs (the layout
    // where ~ IS the repo). A symlink target canonicalizes INTO the repo, but
    // that link is a rice-installed ownership artifact: the canonical trap
    // previously swallowed it as "repo-native", so symlink-mode installs were
    // never unlinked and backup restores never fired. File-level reparse
    // targets take the symlink branch below (remove link -> restore backup).
    // Dir-level junction/symlink layouts (AppData junction into the repo)
    // stay repo-native: removing the mount would disturb the repo mapping.
    let same_canon = if is_symlink {
        mapping.is_dir
    } else {
        source == target
            || (source.canonicalize().is_ok()
                && source.canonicalize().ok() == target.canonicalize().ok())
    };
    if same_canon {
        return UnlinkOutcome::RepoNativeNotTouched(target);
    }

    // Repo protection: never remove, restore over, or archive anything that
    // lives inside the repository tree itself (normalized WITHOUT following
    // the final component, so an outside symlink pointing INTO the repo is
    // still handled by the symlink branch below).
    if is_repo_internal(&target, repo_root) {
        return UnlinkOutcome::RepoNativeNotTouched(target);
    }

    let had_target = target.exists() || symlink_meta.is_some();

    if had_target && !is_symlink {
        // Real user file/dir or a copy the installer materialized. If the
        // backup manifest owns a record, the backup IS the user's original
        // content: restore it (this both removes the rice copy and returns
        // the user's file).
        match restore_target_backup(package_id, &target) {
            Ok(Some(bp)) => {
                crate::ledger::remove_record(package_id, &target);
                return UnlinkOutcome::UnlinkedAndRestored {
                    target,
                    backup_source: bp,
                };
            }
            Ok(None) => {}
            Err(e) => {
                return UnlinkOutcome::Failed(format!(
                    "Failed to restore backup for {}: {}",
                    target.display(),
                    e
                ));
            }
        }
        // No manifest record: is this still pure repo content, or the user's
        // own (modified) configuration?
        if paths_equivalent(&source, &target, mapping.is_dir) {
            let remove_res = if mapping.is_dir {
                fs::remove_dir_all(&target)
            } else {
                fs::remove_file(&target)
            };
            return match remove_res {
                Ok(_) => {
                    crate::ledger::remove_record(package_id, &target);
                    UnlinkOutcome::UnlinkedCleanly(target)
                }
                Err(e) => UnlinkOutcome::Failed(format!("Failed to remove {}: {}", target.display(), e)),
            };
        }
        // User-modified copy-fallback config: archive + preserve on disk.
        return match archive_preserved_copy(package_id, &target, mapping.is_dir) {
            Ok(archive) => UnlinkOutcome::PreservedUserModified { target, archive },
            Err(e) => UnlinkOutcome::Failed(format!(
                "Failed to archive preserved config {}: {}",
                target.display(),
                e
            )),
        };
    }

    if had_target && is_symlink {
        // Symlink: always repo-owned. Remove the link itself (never follow).
        let remove_res = if mapping.is_dir {
            fs::remove_dir(&target)
        } else {
            fs::remove_file(&target)
        };
        if let Err(e) = remove_res {
            return UnlinkOutcome::Failed(format!("Failed to remove symlink {}: {}", target.display(), e));
        }
        crate::ledger::remove_record(package_id, &target);
        return match restore_target_backup(package_id, &target) {
            Ok(Some(bp)) => UnlinkOutcome::UnlinkedAndRestored {
                target,
                backup_source: bp,
            },
            Ok(None) => UnlinkOutcome::UnlinkedCleanly(target),
            Err(e) => UnlinkOutcome::Failed(format!("Unlinked {} but failed to restore backup: {}", target.display(), e)),
        };
    }

    // No target at all; still surface any orphan backup for restoration.
    match restore_target_backup(package_id, &target) {
        Ok(Some(bp)) => UnlinkOutcome::UnlinkedAndRestored {
            target,
            backup_source: bp,
        },
        Ok(None) => UnlinkOutcome::NotLinked(target),
        Err(e) => UnlinkOutcome::Failed(format!("Failed to restore orphan backup: {}", e)),
    }
}

/// Archives a user-modified config copy into the backup root without touching
/// the live file. Returns the archive path.
fn archive_preserved_copy(package_id: &str, target: &Path, is_dir: bool) -> Result<PathBuf> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let archive_dir = get_backup_root().join(package_id).join(format!("preserved-{}", timestamp));
    fs::create_dir_all(&archive_dir)?;
    let file_name = target.file_name().unwrap_or_else(|| std::ffi::OsStr::new("preserved"));
    let archive_dest = archive_dir.join(file_name);
    copy_recursive(target, &archive_dest, is_dir)?;
    Ok(archive_dest)
}

/// True when `path` resolves to a location inside `repo_root`.
/// Normalization canonicalizes the DEEPEST EXISTING ancestor (following any
/// symlinks/junctions along the way) and re-appends the remaining
/// non-existent components, so an unresolved parent path can never smuggle a
/// write into the repo. Comparison is case-insensitive (Windows filesystems)
/// and tolerates the `\?\` verbatim prefix `Path::canonicalize` adds.
pub(crate) fn is_repo_internal(path: &Path, repo_root: &Path) -> bool {
    fn normalize(p: &Path) -> PathBuf {
        // The FINAL component is never followed (it may be the symlink
        // itself: a link living outside the repo whose destination is inside
        // it must still be handled by the caller's symlink branch, not by
        // this guard). Only the parent chain is resolved.
        let (parent, name) = match (p.parent(), p.file_name()) {
            (Some(parent), Some(name)) => (parent, name),
            _ => {
                let whole = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
                return strip_verbatim(whole);
            }
        };

        // Deepest-existing-ancestor resolution of the parent: walk up until
        // canonicalize succeeds (following parent symlinks/junctions), then
        // re-append the components that did not exist yet.
        let mut unresolved: Vec<std::ffi::OsString> = Vec::new();
        let mut cursor = parent.to_path_buf();
        let resolved = loop {
            if let Ok(c) = cursor.canonicalize() {
                break Some(c);
            }
            match (cursor.file_name(), cursor.parent()) {
                (Some(part), Some(up)) => {
                    unresolved.push(part.to_os_string());
                    cursor = up.to_path_buf();
                }
                _ => break None,
            }
        };
        let mut norm = match resolved {
            Some(c) => c,
            None => parent.to_path_buf(),
        };
        for comp in unresolved.iter().rev() {
            norm.push(comp);
        }
        norm.push(name);
        strip_verbatim(norm)
    }

    fn strip_verbatim(p: PathBuf) -> PathBuf {
        if let Some(s) = p.to_str() {
            if let Some(stripped) = s.strip_prefix(r"\\?\") {
                return PathBuf::from(stripped);
            }
        }
        p
    }

    fn components(p: &Path) -> Vec<String> {
        p.components()
            .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
            .collect()
    }

    let path_c = components(&normalize(path));
    let root_c = components(&normalize(repo_root));
    !root_c.is_empty()
        && path_c.len() >= root_c.len()
        && path_c[..root_c.len()] == root_c[..]
}

pub fn copy_recursive(src: &Path, dst: &Path, is_dir: bool) -> Result<()> {
    if src == dst || src.canonicalize().ok() == dst.canonicalize().ok() {
        return Ok(());
    }

    if !is_dir && src.is_file() {
        if dst.exists() {
            let _ = fs::remove_file(dst);
        }
        fs::copy(src, dst).with_context(|| format!("Failed to copy {} to {}", src.display(), dst.display()))?;
        return Ok(());
    }

    if src.is_dir() {
        if !dst.exists() {
            fs::create_dir_all(dst).with_context(|| format!("Failed to create directory {}", dst.display()))?;
        }

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let entry_path = entry.path();
            let dest_path = dst.join(entry.file_name());

            if entry_path == dest_path || entry_path.canonicalize().ok() == dest_path.canonicalize().ok() {
                continue;
            }

            if entry_path.is_dir() {
                copy_recursive(&entry_path, &dest_path, true)?;
            } else {
                if dest_path.exists() {
                    let _ = fs::remove_file(&dest_path);
                }
                fs::copy(&entry_path, &dest_path)?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::fs;

    pub(crate) static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_copy_recursive_file() {
        let temp_dir = std::env::temp_dir().join("rice_test_copy_file");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let src = temp_dir.join("source.txt");
        let dst = temp_dir.join("dest.txt");
        fs::write(&src, "hello world").unwrap();

        copy_recursive(&src, &dst, false).unwrap();
        assert!(dst.exists());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "hello world");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_copy_recursive_dir() {
        let temp_dir = std::env::temp_dir().join("rice_test_copy_dir");
        let _ = fs::remove_dir_all(&temp_dir);
        let src_dir = temp_dir.join("src");
        let dst_dir = temp_dir.join("dst");
        fs::create_dir_all(src_dir.join("sub")).unwrap();
        fs::write(src_dir.join("a.txt"), "A").unwrap();
        fs::write(src_dir.join("sub").join("b.txt"), "B").unwrap();

        copy_recursive(&src_dir, &dst_dir, true).unwrap();
        assert!(dst_dir.join("a.txt").exists());
        assert!(dst_dir.join("sub").join("b.txt").exists());
        assert_eq!(fs::read_to_string(dst_dir.join("sub").join("b.txt")).unwrap(), "B");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_backup_and_restore_cycle() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let temp_dir = std::env::temp_dir().join("rice_test_backup_restore");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let backup_root = temp_dir.join(".rice-backups");
        unsafe { std::env::set_var("RICE_BACKUP_DIR", &backup_root); }

        let dummy_target = temp_dir.join("original_user_config.yaml");
        fs::write(&dummy_target, "custom user setting: 12345").unwrap();

        // 1. Run backup
        let backup_result = backup_existing_target("test_pkg", &dummy_target, false).unwrap();
        assert!(backup_result.is_some());
        let backup_path = backup_result.unwrap();
        assert!(backup_path.exists());
        assert_eq!(fs::read_to_string(&backup_path).unwrap(), "custom user setting: 12345");
        // Target should be moved/removed
        assert!(!dummy_target.exists());

        // 2. Simulate new rice installation in target
        fs::write(&dummy_target, "rice themed setting: catppuccin").unwrap();
        assert_eq!(fs::read_to_string(&dummy_target).unwrap(), "rice themed setting: catppuccin");

        // 3. Restore backup
        let restored = restore_target_backup("test_pkg", &dummy_target).unwrap();
        assert!(restored.is_some());
        assert!(dummy_target.exists());
        assert_eq!(fs::read_to_string(&dummy_target).unwrap(), "custom user setting: 12345");

        // Verify orphan directories pruned
        let pkg_backup_dir = backup_root.join("test_pkg");
        assert!(!pkg_backup_dir.exists(), "Empty package directory should be pruned on restore");

        // Cleanup
        unsafe { std::env::remove_var("RICE_BACKUP_DIR"); }
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_backup_and_restore_directory_cycle() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let temp_dir = std::env::temp_dir().join("rice_test_dir_backup_restore");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let backup_root = temp_dir.join(".rice-backups");
        unsafe { std::env::set_var("RICE_BACKUP_DIR", &backup_root); }

        let dummy_target_dir = temp_dir.join("user_config_dir");
        fs::create_dir_all(dummy_target_dir.join("nested")).unwrap();
        fs::write(dummy_target_dir.join("settings.json"), r#"{"theme": "user_custom"}"#).unwrap();
        fs::write(dummy_target_dir.join("nested").join("profile.xml"), "<user>data</user>").unwrap();

        // 1. Run recursive directory backup
        let backup_result = backup_existing_target("test_dir_pkg", &dummy_target_dir, true).unwrap();
        assert!(backup_result.is_some());
        let backup_path = backup_result.unwrap();
        assert!(backup_path.exists());
        assert!(backup_path.join("settings.json").exists());
        assert!(backup_path.join("nested").join("profile.xml").exists());
        // Original target dir should be removed/moved
        assert!(!dummy_target_dir.exists());

        // Manifest should catalog the directory backup
        let manifest = load_manifest();
        assert!(manifest.records.iter().any(|r| r.package_id == "test_dir_pkg" && r.is_dir));

        // 2. Simulate rice installation replacing the directory
        fs::create_dir_all(&dummy_target_dir).unwrap();
        fs::write(dummy_target_dir.join("settings.json"), r#"{"theme": "rice_catppuccin"}"#).unwrap();

        // 3. Restore backup
        let restored = restore_target_backup("test_dir_pkg", &dummy_target_dir).unwrap();
        assert!(restored.is_some());
        assert!(dummy_target_dir.exists());
        assert_eq!(
            fs::read_to_string(dummy_target_dir.join("settings.json")).unwrap(),
            r#"{"theme": "user_custom"}"#
        );
        assert_eq!(
            fs::read_to_string(dummy_target_dir.join("nested").join("profile.xml")).unwrap(),
            "<user>data</user>"
        );

        // Verify empty timestamp and package folders are pruned
        let pkg_backup_dir = backup_root.join("test_dir_pkg");
        assert!(!pkg_backup_dir.exists(), "Empty package directory should be pruned after directory restore");

        // Cleanup
        unsafe { std::env::remove_var("RICE_BACKUP_DIR"); }
        let _ = fs::remove_dir_all(&temp_dir);
    }

    fn leak_path(p: &Path) -> &'static str {
        Box::leak(p.to_string_lossy().to_string().into_boxed_str())
    }

    #[test]
    fn test_paths_equivalent_file_and_dir() {
        let temp_dir = std::env::temp_dir().join("rice_test_equiv");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Files: identical vs changed
        let f1 = temp_dir.join("a.toml");
        let f2 = temp_dir.join("b.toml");
        fs::write(&f1, "theme = mocha").unwrap();
        fs::write(&f2, "theme = mocha").unwrap();
        assert!(super::file_contents_equal(&f1, &f2));
        assert!(paths_equivalent(&f1, &f2, false));
        fs::write(&f2, "theme = custom").unwrap();
        assert!(!paths_equivalent(&f1, &f2, false));
        assert!(!paths_equivalent(&f1, &temp_dir.join("missing.toml"), false));

        // Directories: same set + same contents vs user-added extra file
        let sdir = temp_dir.join("srcdir");
        let ddir = temp_dir.join("dstdir");
        fs::create_dir_all(sdir.join("sub")).unwrap();
        fs::create_dir_all(ddir.join("sub")).unwrap();
        fs::write(sdir.join("k.yaml"), "K: 1").unwrap();
        fs::write(sdir.join("sub").join("n.txt"), "N").unwrap();
        fs::write(ddir.join("k.yaml"), "K: 1").unwrap();
        fs::write(ddir.join("sub").join("n.txt"), "N").unwrap();
        assert!(paths_equivalent(&sdir, &ddir, true));
        fs::write(ddir.join("my-own-file.txt"), "user added this").unwrap();
        assert!(
            !paths_equivalent(&sdir, &ddir, true),
            "user-added files inside an installed dir make it NOT repo-owned"
        );
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_unlink_identical_copy_removed() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let temp_dir = std::env::temp_dir().join("rice_test_unlink_copy");
        let _ = fs::remove_dir_all(&temp_dir);
        let backup_root = temp_dir.join(".rice-backups");
        fs::create_dir_all(&backup_root).unwrap();
        unsafe { std::env::set_var("RICE_BACKUP_DIR", &backup_root) }

        let repo = temp_dir.join("repo").join("somepkg");
        fs::create_dir_all(&repo).unwrap();
        fs::write(repo.join("cfg.toml"), "v = 1").unwrap();
        let mpz_target = temp_dir.join("targets").join("somepkg");
        fs::create_dir_all(&mpz_target).unwrap();
        // Copy-fallback install: identical bytes.
        fs::write(mpz_target.join("cfg.toml"), "v = 1").unwrap();

        let pattern = leak_path(&mpz_target);
        let mapping = crate::catalog::ConfigMapping {
            source_rel: leak_path(&repo),
            target_pattern: pattern,
            is_dir: true,
        };
        let outcome =
            super::unlink_config_mapping(&temp_dir.join("repo"), &mapping, "somepkg");
        assert!(
            matches!(outcome, super::UnlinkOutcome::UnlinkedCleanly(_)),
            "identical pure-rice copy must be removed, got {:?}",
            outcome
        );
        assert!(!mpz_target.exists(), "target must be removed");
        // Ledger record for this target must be gone.
        let ledger = crate::ledger::load_ledger();
        assert!(!ledger.records.iter().any(|r| r.pkg_id == "somepkg"));

        unsafe { std::env::remove_var("RICE_BACKUP_DIR") };
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_unlink_user_modified_copy_preserved() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let temp_dir = std::env::temp_dir().join("rice_test_unlink_modified");
        let _ = fs::remove_dir_all(&temp_dir);
        let backup_root = temp_dir.join(".rice-backups");
        fs::create_dir_all(&backup_root).unwrap();
        unsafe { std::env::set_var("RICE_BACKUP_DIR", &backup_root) }

        let repo = temp_dir.join("repo").join("pkg2");
        fs::create_dir_all(&repo).unwrap();
        fs::write(repo.join("cfg.toml"), "v = 1").unwrap();
        let target_file = temp_dir.join("targets").join("pkg2.toml");
        fs::create_dir_all(target_file.parent().unwrap()).unwrap();
        // Installer copied, USER edited afterwards.
        fs::write(&target_file, "v = 2 # my personal tweak").unwrap();

        let mapping = crate::catalog::ConfigMapping {
            source_rel: leak_path(&repo),
            target_pattern: leak_path(&target_file),
            is_dir: false,
        };
        let outcome =
            super::unlink_config_mapping(&temp_dir.join("repo"), &mapping, "pkg2");
        assert!(
            matches!(outcome, super::UnlinkOutcome::PreservedUserModified { .. }),
            "user-modified copy must be preserved, got {:?}",
            outcome
        );
        // The live file stays; the archive copy exists with identical bytes.
        assert!(target_file.exists(), "modified config must remain on disk");
        assert_eq!(
            fs::read_to_string(&target_file).unwrap(),
            "v = 2 # my personal tweak"
        );
        // A preserved archive directory must exist under .rice-backups/pkg2.
        let archives_dir = backup_root.join("pkg2");
        let archived = fs::read_dir(&archives_dir)
            .map(|entries| entries.count())
            .unwrap_or(0);
        assert!(
            archived > 0,
            "a preserved archive must exist under {}",
            archives_dir.display()
        );
        unsafe { std::env::remove_var("RICE_BACKUP_DIR") };
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_unlink_repo_native_never_touched() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let temp_dir = std::env::temp_dir().join("rice_test_unlink_native");
        let _ = fs::remove_dir_all(&temp_dir);
        let backup_root = temp_dir.join(".rice-backups");
        fs::create_dir_all(&backup_root).unwrap();
        unsafe { std::env::set_var("RICE_BACKUP_DIR", &backup_root) }

        // Mapping where the "repo directory" IS the target (repo lives at
        // ~/.config with ~/.config as both repo_root and install target).
        let cfg = temp_dir.join("pkg3");
        fs::create_dir_all(&cfg).unwrap();
        fs::write(cfg.join("x.yaml"), "live config").unwrap();
        let mapping = crate::catalog::ConfigMapping {
            source_rel: leak_path(&cfg),
            target_pattern: leak_path(&cfg),
            is_dir: true,
        };
        let outcome = super::unlink_config_mapping(&temp_dir, &mapping, "pkg3");
        assert!(
            matches!(outcome, super::UnlinkOutcome::RepoNativeNotTouched(_)),
            "repo-native target must be skipped, got {:?}",
            outcome
        );
        assert!(cfg.join("x.yaml").exists(), "repo-native files must NEVER be deleted");
        unsafe { std::env::remove_var("RICE_BACKUP_DIR") };
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_unlink_restores_user_backup_over_rice_copy() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let temp_dir = std::env::temp_dir().join("rice_test_unlink_restore");
        let _ = fs::remove_dir_all(&temp_dir);
        let backup_root = temp_dir.join(".rice-backups");
        fs::create_dir_all(&backup_root).unwrap();
        unsafe { std::env::set_var("RICE_BACKUP_DIR", &backup_root) }

        let target_file = temp_dir.join("eam.json");
        fs::write(&target_file, r#"{"user":"original"}"#).unwrap();
        // Installer found the user file and backed it up
        let _ = super::backup_existing_target("pkg4", &target_file, false).unwrap();
        // ...then installed its own copy, which is NOT what unlink should keep
        fs::write(&target_file, r#"{"theme":"rice"}"#).unwrap();

        let repo = temp_dir.join("repo").join("pkg4");
        fs::create_dir_all(&repo).unwrap();
        fs::write(repo.join("eam.json"), r#"{"theme":"rice"}"#).unwrap();
        let mapping = crate::catalog::ConfigMapping {
            source_rel: leak_path(&repo),
            target_pattern: leak_path(&target_file),
            is_dir: false,
        };
        let outcome = super::unlink_config_mapping(&temp_dir.join("repo"), &mapping, "pkg4");
        assert!(
            matches!(outcome, super::UnlinkOutcome::UnlinkedAndRestored { .. }),
            "manifest backup wins over equivalence: user original returns, got {:?}",
            outcome
        );
        assert_eq!(
            fs::read_to_string(&target_file).unwrap(),
            r#"{"user":"original"}"#,
            "the user's own pre-install content must be back in place"
        );
        unsafe { std::env::remove_var("RICE_BACKUP_DIR") };
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_unlink_symlink_to_source_restores_user_backup() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let result = (|| -> Result<(), String> {
            let temp_dir = std::env::temp_dir().join("rice_test_unlink_symlink_restore");
            let _ = fs::remove_dir_all(&temp_dir);
            fs::create_dir_all(&temp_dir).unwrap();

            let backup_root = temp_dir.join(".rice-backups");
            fs::create_dir_all(&backup_root).unwrap();
            unsafe { std::env::set_var("RICE_BACKUP_DIR", &backup_root) }

            // Repo-owned source file
            let repo = temp_dir.join("repo");
            fs::create_dir_all(&repo).unwrap();
            let source = repo.join("wezterm.lua");
            fs::write(&source, "rice theme").unwrap();

            // 1. The user's original file exists; the installer backs it up
            //    before displacing it with a symlink-mode link.
            let target_file = temp_dir.join(".wezterm.lua");
            fs::write(&target_file, "user original").unwrap();
            let backup = super::backup_existing_target("wezterm", &target_file, false).unwrap();
            assert!(backup.is_some(), "displacement must archive the user file");

            // 2. The installer links: target is a symlink pointing at the
            //    source. Its canonical form resolves INTO the repo, which the
            //    old guard collapsed into "repo-native" (never unlink, never
            //    restore) - the exact CI-lifecycle regression this test locks.
            let link_created = {
                // cfg! (a runtime bool) unifies both branches at typecheck -
                // `#[cfg]` attributes keep the platform-specific paths
                // compiled per-target only.
                #[cfg(windows)]
                { std::os::windows::fs::symlink_file(&source, &target_file) }
                #[cfg(not(windows))]
                { std::os::unix::fs::symlink(&source, &target_file) }
            };
            if let Err(e) = link_created {
                println!("symlink creation unavailable on this host ({}); skipping", e);
                unsafe { std::env::remove_var("RICE_BACKUP_DIR") };
                let _ = fs::remove_dir_all(&temp_dir);
                return Ok(());
            }
            assert!(
                target_file.symlink_metadata().unwrap().file_type().is_symlink(),
                "target must be a symlink before the revoke"
            );

            // 3. Revoke must classify the symlink as rice-owned: remove the
            //    link and restore the user's original content byte-for-byte.
            let mapping = crate::catalog::ConfigMapping {
                source_rel: "wezterm.lua",
                target_pattern: leak_path(&target_file),
                is_dir: false,
            };
            let outcome = super::unlink_config_mapping(&repo, &mapping, "wezterm");
            assert!(
                matches!(outcome, super::UnlinkOutcome::UnlinkedAndRestored { .. }),
                "symlink-to-source targets must unlink & restore, got {:?}",
                outcome
            );
            assert_eq!(
                fs::read_to_string(&target_file).unwrap(),
                "user original",
                "the displaced user file must be restored by full revoke"
            );

            unsafe { std::env::remove_var("RICE_BACKUP_DIR") };
            let _ = fs::remove_dir_all(&temp_dir);
            Ok(())
        })();
        if let Err(e) = result {
            panic!("{}", e);
        }
    }

    #[test]
    fn test_backup_ignores_existing_symlinks() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let temp_dir = std::env::temp_dir().join("rice_test_symlink_backup");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let backup_root = temp_dir.join(".rice-backups");
        unsafe { std::env::set_var("RICE_BACKUP_DIR", &backup_root); }

        let original_file = temp_dir.join("real_config.json");
        fs::write(&original_file, r#"{"original": true}"#).unwrap();

        let symlink_target = temp_dir.join("symlink_config.json");

        #[cfg(windows)]
        let symlink_res = std::os::windows::fs::symlink_file(&original_file, &symlink_target);
        #[cfg(not(windows))]
        let symlink_res = std::os::unix::fs::symlink(&original_file, &symlink_target);

        if symlink_res.is_ok() {
            // Target is a symlink: backup_existing_target must return Ok(None)
            let result = backup_existing_target("test_symlink_pkg", &symlink_target, false).unwrap();
            assert!(result.is_none(), "Symlink targets must return Ok(None) without backing up");

            // Verify symlink still exists and points to real file
            assert!(symlink_target.symlink_metadata().is_ok());

            // Verify nothing was recorded in manifest
            let manifest = load_manifest();
            assert!(!manifest.records.iter().any(|r| r.package_id == "test_symlink_pkg"));
        }

        // Cleanup
        unsafe { std::env::remove_var("RICE_BACKUP_DIR"); }
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_link_refuses_repo_internal_target() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let temp_dir = std::env::temp_dir().join("rice_test_repo_guard_link");
        let _ = fs::remove_dir_all(&temp_dir);
        let repo = temp_dir.join("repo");
        fs::create_dir_all(repo.join("nested")).unwrap();
        fs::write(repo.join("src.yaml"), "rice source").unwrap();
        // A live repo file that is NOT the mapping's source: linking must
        // refuse instead of backing it up and replacing it with a symlink.
        let repo_file = repo.join("nested").join("live.yaml");
        fs::write(&repo_file, "tracked repo content").unwrap();

        let backup_root = temp_dir.join(".rice-backups");
        fs::create_dir_all(&backup_root).unwrap();
        unsafe { std::env::set_var("RICE_BACKUP_DIR", &backup_root) }

        let mapping = crate::catalog::ConfigMapping {
            source_rel: leak_path(&repo.join("src.yaml")),
            target_pattern: leak_path(&repo_file),
            is_dir: false,
        };
        let outcome = super::link_config_mapping(&repo, &mapping, "guardpkg");
        assert!(
            matches!(&outcome, super::LinkOutcome::Failed(msg) if msg.contains("repo-internal")),
            "link into the repo tree must be refused, got {:?}",
            outcome
        );
        // The tracked file is byte-for-byte untouched, and no ledger/backup
        // record was created for it.
        assert_eq!(
            fs::read_to_string(&repo_file).unwrap(),
            "tracked repo content",
            "repo file must never be modified by the linker"
        );
        // No backup was staged for the refused target either.
        let staged: Vec<_> = fs::read_dir(&backup_root)
            .map(|it| it.flatten().collect::<Vec<_>>())
            .unwrap_or_default();
        assert!(staged.is_empty(), "refused link must not stage backups, found {:?}", staged);
        let ledger = crate::ledger::load_ledger();
        assert!(!ledger.records.iter().any(|r| r.pkg_id == "guardpkg"));
        let manifest = load_manifest();
        assert!(!manifest.records.iter().any(|r| r.package_id == "guardpkg"));

        unsafe { std::env::remove_var("RICE_BACKUP_DIR") };
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_link_guard_closes_dangling_parent_symlink_evasion() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let temp_dir = std::env::temp_dir().join("rice_test_repo_guard_dangling");
        let _ = fs::remove_dir_all(&temp_dir);
        let repo = temp_dir.join("repo");
        fs::create_dir_all(&repo).unwrap();
        fs::write(repo.join("src.yaml"), "rice source").unwrap();

        // Parent path component is a DANGLING symlink pointing INTO the repo:
        // canonicalize() fails while the link is dangling, which used to let a
        // write slip through the repo-write guard. The guard must refuse
        // either during the pre-check or (after the parent materializes) on
        // the post-create_dir_all re-check.
        let ghost = repo.join("ghost");
        let parent_link = temp_dir.join("parent-link");
        #[cfg(windows)]
        let link_res = std::os::windows::fs::symlink_dir(&ghost, &parent_link);
        #[cfg(not(windows))]
        let link_res = std::os::unix::fs::symlink(&ghost, &parent_link);
        if link_res.is_err() {
            println!("symlink creation unavailable on this host; skipping");
            let _ = fs::remove_dir_all(&temp_dir);
            return;
        }

        let backup_root = temp_dir.join(".rice-backups");
        fs::create_dir_all(&backup_root).unwrap();
        unsafe { std::env::set_var("RICE_BACKUP_DIR", &backup_root) }

        let target = parent_link.join("config.yaml");
        let mapping = crate::catalog::ConfigMapping {
            source_rel: leak_path(&repo.join("src.yaml")),
            target_pattern: leak_path(&target),
            is_dir: false,
        };
        let outcome = super::link_config_mapping(&repo, &mapping, "guard-dangling");

        assert!(
            matches!(&outcome, super::LinkOutcome::Failed(_)),
            "write through a dangling parent link into the repo must be refused, got {:?}",
            outcome
        );
        assert!(
            !target.exists(),
            "no file may be written through the dangling link"
        );
        assert_eq!(
            fs::read_to_string(repo.join("src.yaml")).unwrap(),
            "rice source",
            "repo source must be untouched"
        );

        unsafe { std::env::remove_var("RICE_BACKUP_DIR") };
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_unlink_refuses_repo_internal_target() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let temp_dir = std::env::temp_dir().join("rice_test_repo_guard_unlink");
        let _ = fs::remove_dir_all(&temp_dir);
        let repo = temp_dir.join("repo");
        fs::create_dir_all(&repo).unwrap();
        fs::write(repo.join("src.yaml"), "rice source").unwrap();
        let repo_file = repo.join("live.yaml");
        fs::write(&repo_file, "tracked repo content").unwrap();

        let backup_root = temp_dir.join(".rice-backups");
        fs::create_dir_all(&backup_root).unwrap();
        unsafe { std::env::set_var("RICE_BACKUP_DIR", &backup_root) }

        let mapping = crate::catalog::ConfigMapping {
            source_rel: leak_path(&repo.join("src.yaml")),
            target_pattern: leak_path(&repo_file),
            is_dir: false,
        };
        let outcome = super::unlink_config_mapping(&repo, &mapping, "guardpkg");
        assert!(
            matches!(outcome, super::UnlinkOutcome::RepoNativeNotTouched(_)),
            "unlink inside the repo tree must be refused, got {:?}",
            outcome
        );
        assert_eq!(
            fs::read_to_string(&repo_file).unwrap(),
            "tracked repo content",
            "repo file must never be deleted by the unlinker"
        );

        unsafe { std::env::remove_var("RICE_BACKUP_DIR") };
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
