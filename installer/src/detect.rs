use std::path::{Path, PathBuf};
use crate::catalog::{PackageItem, resolve_target_path};

#[derive(Debug, Clone, Default)]
pub struct PackageManagerStatus {
    pub scoop: bool,
    pub winget: bool,
    pub choco: bool,
    pub cargo: bool,
    pub python: bool,
    pub npm: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryStatus {
    Installed(PathBuf),
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkStatus {
    LinkedSymlink(PathBuf),
    LinkedCopy(PathBuf),
    NotLinked,
    NotApplicable,
}

#[derive(Debug, Clone)]
pub struct ItemInspection {
    pub binary_status: BinaryStatus,
    pub link_status: LinkStatus,
    pub is_missing_any: bool,
}

pub fn detect_package_managers() -> PackageManagerStatus {
    let mut status = PackageManagerStatus::default();

    status.scoop = find_binary_in_path("scoop.cmd").is_some()
        || find_binary_in_path("scoop.ps1").is_some()
        || user_profile_file_exists("scoop/shims/scoop.cmd");

    status.winget = find_binary_in_path("winget.exe").is_some();
    status.choco = find_binary_in_path("choco.exe").is_some()
        || Path::new("C:\\ProgramData\\chocolatey\\bin\\choco.exe").exists();

    status.cargo = find_binary_in_path("cargo.exe").is_some()
        || user_profile_file_exists(".cargo/bin/cargo.exe");

    status.python = find_binary_in_path("python.exe").is_some()
        || find_binary_in_path("py.exe").is_some();

    status.npm = find_binary_in_path("npm.cmd").is_some()
        || find_binary_in_path("npm.exe").is_some();

    status
}

pub fn user_profile_file_exists(rel: &str) -> bool {
    if let Some(user_profile) = std::env::var_os("USERPROFILE") {
        PathBuf::from(user_profile).join(rel).exists()
    } else {
        false
    }
}

pub fn find_binary_in_path(binary_name: &str) -> Option<PathBuf> {
    // 1. Direct path check if absolute
    let direct = Path::new(binary_name);
    if direct.is_absolute() && direct.exists() {
        return Some(direct.to_path_buf());
    }

    // 2. Extra standard locations on Windows
    if let Some(user_profile) = std::env::var_os("USERPROFILE") {
        let scoop_shim = PathBuf::from(&user_profile).join("scoop/shims").join(binary_name);
        if scoop_shim.exists() {
            return Some(scoop_shim);
        }
        let cargo_bin = PathBuf::from(&user_profile).join(".cargo/bin").join(binary_name);
        if cargo_bin.exists() {
            return Some(cargo_bin);
        }
    }

    // 3. Search PATH
    if let Some(paths) = std::env::var_os("PATH") {
        for path in std::env::split_paths(&paths) {
            let candidate = path.join(binary_name);
            if candidate.exists() {
                return Some(candidate);
            }
            if !binary_name.ends_with(".exe") && !binary_name.ends_with(".cmd") && !binary_name.ends_with(".bat") {
                for ext in &[".exe", ".cmd", ".bat", ".ps1"] {
                    let with_ext = path.join(format!("{}{}", binary_name, ext));
                    if with_ext.exists() {
                        return Some(with_ext);
                    }
                }
            }
        }
    }

    None
}

pub fn inspect_package(item: &PackageItem, repo_root: &Path) -> ItemInspection {
    let mut found_bin = None;
    for &bin in item.binaries {
        if let Some(p) = find_binary_in_path(bin) {
            found_bin = Some(p);
            break;
        }
    }

    let binary_status = if item.binaries.is_empty() {
        BinaryStatus::Installed(PathBuf::from("standalone-asset"))
    } else {
        match found_bin {
            Some(p) => BinaryStatus::Installed(p),
            None => BinaryStatus::Missing,
        }
    };

    if item.configs.is_empty() {
        let is_missing = matches!(binary_status, BinaryStatus::Missing);
        return ItemInspection {
            binary_status,
            link_status: LinkStatus::NotApplicable,
            is_missing_any: is_missing,
        };
    }

    let mut all_linked = true;
    let mut any_symlink = false;
    let mut first_target = None;

    for config in &item.configs {
        let source = repo_root.join(config.source_rel);
        let target = resolve_target_path(config.target_pattern);
        if first_target.is_none() {
            first_target = Some(target.clone());
        }

        // If source and target are the same path, it's already in-place in the repo
        if source == target || source.canonicalize().ok() == target.canonicalize().ok() {
            continue;
        }

        if !target.exists() {
            // Check if it's a broken symlink
            if target.symlink_metadata().is_ok() {
                any_symlink = true;
            } else {
                all_linked = false;
                break;
            }
        } else {
            // Check if symlink
            if let Ok(meta) = target.symlink_metadata() {
                if meta.file_type().is_symlink() {
                    any_symlink = true;
                }
            }
        }
    }

    let link_status = if all_linked {
        if any_symlink {
            LinkStatus::LinkedSymlink(first_target.unwrap_or_default())
        } else {
            LinkStatus::LinkedCopy(first_target.unwrap_or_default())
        }
    } else {
        LinkStatus::NotLinked
    };

    let is_missing_any = matches!(binary_status, BinaryStatus::Missing)
        || matches!(link_status, LinkStatus::NotLinked);

    ItemInspection {
        binary_status,
        link_status,
        is_missing_any,
    }
}
