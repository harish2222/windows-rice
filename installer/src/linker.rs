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
        return LinkOutcome::AlreadyValid(target);
    }

    // 2. Ensure parent directory exists
    if let Some(parent) = target.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return LinkOutcome::Failed(format!("Failed to create parent dir {}: {}", parent.display(), e));
        }
    }

    // 3. Check if target is already pointing to source via symlink
    if let Ok(meta) = target.symlink_metadata() {
        if meta.file_type().is_symlink() {
            if let Ok(dest) = fs::read_link(&target) {
                if dest == source || dest.canonicalize().ok() == source.canonicalize().ok() {
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
            } else if let Some(bp) = backup_created {
                LinkOutcome::BackedUpAndLinked { target, backup: bp }
            } else {
                LinkOutcome::CopiedFallback(target)
            }
        }
    }
}

pub fn unlink_config_mapping(_repo_root: &Path, mapping: &ConfigMapping, package_id: &str) -> UnlinkOutcome {
    let target = resolve_target_path(mapping.target_pattern);
    let had_target = target.exists() || target.symlink_metadata().is_ok();

    if had_target {
        let remove_res = if mapping.is_dir {
            if let Ok(meta) = target.symlink_metadata() {
                if meta.file_type().is_symlink() {
                    fs::remove_dir(&target)
                } else {
                    fs::remove_dir_all(&target)
                }
            } else {
                fs::remove_dir_all(&target)
            }
        } else {
            fs::remove_file(&target)
        };

        if let Err(e) = remove_res {
            return UnlinkOutcome::Failed(format!("Failed to remove {}: {}", target.display(), e));
        }
    }

    match restore_target_backup(package_id, &target) {
        Ok(Some(bp)) => UnlinkOutcome::UnlinkedAndRestored { target, backup_source: bp },
        Ok(None) => {
            if had_target {
                UnlinkOutcome::UnlinkedCleanly(target)
            } else {
                UnlinkOutcome::NotLinked(target)
            }
        }
        Err(e) => UnlinkOutcome::Failed(format!("Unlinked {} but failed to restore backup: {}", target.display(), e)),
    }
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
}
