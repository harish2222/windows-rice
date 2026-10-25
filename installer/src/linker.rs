use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use crate::catalog::{ConfigMapping, resolve_target_path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkOutcome {
    SymlinkCreated(PathBuf),
    CopiedFallback(PathBuf),
    AlreadyValid(PathBuf),
    Failed(String),
}

pub fn link_config_mapping(repo_root: &Path, mapping: &ConfigMapping) -> LinkOutcome {
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
            // Remove old/broken symlink
            let _ = if mapping.is_dir {
                fs::remove_dir(&target)
            } else {
                fs::remove_file(&target)
            };
        }
    }

    // 4. Try creating symbolic link
    let symlink_res = if mapping.is_dir {
        #[cfg(windows)]
        {
            // If target directory already exists as regular directory, remove or rename it to allow symlink
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
        Ok(_) => LinkOutcome::SymlinkCreated(target),
        Err(e) => {
            // Symlink failed (e.g. Error 1314: A required privilege is not held by the client)
            // Fallback to recursive copy
            if let Err(copy_err) = copy_recursive(&source, &target, mapping.is_dir) {
                LinkOutcome::Failed(format!("Symlink failed ({}) and copy fallback failed: {}", e, copy_err))
            } else {
                LinkOutcome::CopiedFallback(target)
            }
        }
    }
}

fn copy_recursive(src: &Path, dst: &Path, is_dir: bool) -> Result<()> {
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
