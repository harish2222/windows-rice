// ============================================================================
// Author -: HKDEVS
// Project -: Window-Rice
// Title -: WINDOW-RICE Installer Environment Detection Engine
// ============================================================================
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
    pub uv: bool,
    pub vcpkg: bool,
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
    /// Repo-native in-place config: the repository IS the config home
    /// (repo root == target directory), so nothing was copied or linked.
    InPlace(PathBuf),
    NotLinked,
    NotApplicable,
}

#[derive(Debug, Clone)]
pub struct ItemInspection {
    pub binary_status: BinaryStatus,
    pub link_status: LinkStatus,
    pub is_missing_any: bool,
}

/// Classification of a single source -> target config mapping.
///
/// Verification semantics: a config only counts as deployed when it is
/// genuinely in its final state - in-place (repo-native), a symlink that
/// resolves to the repo source, or a real file/dir at the target. A broken
/// symlink or a symlink pointing somewhere else is NOT deployed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigState {
    /// Source and target are the same location (repo IS the config home).
    InPlace,
    /// Target is a symlink/junction that resolves to the repo source.
    Symlinked,
    /// Target is a real file/dir (copy-fallback install or user's own copy).
    Present,
    /// Target missing, broken link, or a link pointing somewhere else.
    Missing,
}

pub fn classify_config(source: &Path, target: &Path) -> ConfigState {
    // 1. Symlinked target FIRST: `canonicalize` follows the link, so a link
    //    pointing at the source collapses onto the source path - checked after
    //    the in-place test it would be misreported as "in place". Only a link
    //    that resolves exactly to the source counts; broken links and links to
    //    foreign destinations are NOT deployed configs.
    let target_is_symlink = target
        .symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);
    if target_is_symlink {
        let src_canon = source.canonicalize().ok();
        return match (target.canonicalize(), src_canon) {
            (Ok(resolved), Some(src)) if resolved == src => ConfigState::Symlinked,
            _ => ConfigState::Missing,
        };
    }

    // 2. Repo-native layout: source and target are the same (real) path. Only
    //    treat canonical equality as in-place when the source actually exists -
    //    two `None` canonicalizations (both paths missing) must NOT be equal
    //    (the old check reported missing configs as deployed).
    let src_canon = source.canonicalize().ok();
    let tgt_canon = target.canonicalize().ok();
    let in_place = source == target || (src_canon.is_some() && src_canon == tgt_canon);
    if in_place {
        return if target.exists() {
            ConfigState::InPlace
        } else {
            ConfigState::Missing
        };
    }

    // 3. Plain existing file/dir (copy-fallback install or user's own copy).
    if target.exists() {
        ConfigState::Present
    } else {
        ConfigState::Missing
    }
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

    status.uv = find_binary_in_path("uv.exe").is_some()
        || user_profile_file_exists(".local/bin/uv.exe")
        || user_profile_file_exists(".cargo/bin/uv.exe")
        || user_profile_file_exists("scoop/shims/uv.exe");

    status.vcpkg = find_binary_in_path("vcpkg.exe").is_some()
        || user_profile_file_exists("scoop/shims/vcpkg.exe")
        || std::env::var_os("VCPKG_ROOT").map(|p| PathBuf::from(p).join("vcpkg.exe").exists()).unwrap_or(false)
        || Path::new("C:\\vcpkg\\vcpkg.exe").exists()
        || user_profile_file_exists("vcpkg/vcpkg.exe");

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
        let local_bin = PathBuf::from(&user_profile).join(".local/bin").join(binary_name);
        if local_bin.exists() {
            return Some(local_bin);
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
    let mut all_in_place = true;
    let mut first_target = None;

    for config in &item.configs {
        let source = repo_root.join(config.source_rel);
        let target = resolve_target_path(config.target_pattern);
        if first_target.is_none() {
            first_target = Some(target.clone());
        }

        match classify_config(&source, &target) {
            ConfigState::InPlace => {}
            ConfigState::Symlinked => {
                any_symlink = true;
                all_in_place = false;
            }
            ConfigState::Present => {
                all_in_place = false;
            }
            ConfigState::Missing => {
                all_linked = false;
                break;
            }
        }
    }

    let link_status = if !all_linked {
        LinkStatus::NotLinked
    } else if any_symlink {
        LinkStatus::LinkedSymlink(first_target.unwrap_or_default())
    } else if all_in_place {
        LinkStatus::InPlace(first_target.unwrap_or_default())
    } else {
        LinkStatus::LinkedCopy(first_target.unwrap_or_default())
    };

    let is_missing_any = matches!(binary_status, BinaryStatus::Missing)
        || matches!(link_status, LinkStatus::NotLinked);

    ItemInspection {
        binary_status,
        link_status,
        is_missing_any,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{Category, ConfigMapping, PackageItem};
    use std::fs;
    use std::path::PathBuf;

    fn leak(p: &Path) -> &'static str {
        Box::leak(p.to_string_lossy().to_string().into_boxed_str())
    }

    fn temp_dir(name: &str) -> PathBuf {
        // PID-suffixed: fixed names would collide if two test processes ever
        // run the suite concurrently (cargo test + e2e both invoke it).
        let d = std::env::temp_dir().join(format!("{}_{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn make_symlink(src: &Path, link: &Path) -> std::io::Result<()> {
        #[cfg(windows)]
        {
            if src.is_dir() {
                std::os::windows::fs::symlink_dir(src, link)
            } else {
                std::os::windows::fs::symlink_file(src, link)
            }
        }
        #[cfg(not(windows))]
        {
            std::os::unix::fs::symlink(src, link)
        }
    }

    #[test]
    fn test_classify_config_repo_native_in_place() {
        let dir = temp_dir("rice_detect_inplace");
        let cfg = dir.join("komorebi.json");
        fs::write(&cfg, "{}").unwrap();
        assert_eq!(classify_config(&cfg, &cfg), ConfigState::InPlace);

        // Regression: the same nonexistent path must NOT be "in place". Two
        // failed canonicalizations are both None and must never compare equal
        // (the old `source.canonicalize().ok() == target.canonicalize().ok()`
        // check reported missing configs as deployed).
        let ghost = dir.join("ghost.json");
        assert_eq!(classify_config(&ghost, &ghost), ConfigState::Missing);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_classify_config_missing_present_and_ghost_source() {
        let dir = temp_dir("rice_detect_states");
        let src = dir.join("src.yaml");
        fs::write(&src, "a: 1").unwrap();

        let absent = dir.join("absent.yaml");
        assert_eq!(classify_config(&src, &absent), ConfigState::Missing);

        let present = dir.join("present.yaml");
        fs::write(&present, "user copy").unwrap();
        assert_eq!(classify_config(&src, &present), ConfigState::Present);

        // Source AND target both missing (different paths): None==None must
        // not classify anything as linked.
        let ghost_src = dir.join("ghost-src.yaml");
        assert_eq!(classify_config(&ghost_src, &absent), ConfigState::Missing);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_classify_config_symlink_states() {
        let dir = temp_dir("rice_detect_symlink");
        let src = dir.join("wezterm.lua");
        fs::write(&src, "rice").unwrap();

        // 1. Valid symlink -> source: Symlinked.
        let good = dir.join("good.lua");
        if make_symlink(&src, &good).is_ok() {
            assert_eq!(classify_config(&src, &good), ConfigState::Symlinked);
        } else {
            println!("symlink creation unavailable on this host; skipping valid-link case");
        }

        // 2. Broken symlink (destination gone): Missing - a dangling link is
        //    not a deployed config and used to PASS verification.
        let broken = dir.join("broken.lua");
        let ghost_src = dir.join("does-not-exist.lua");
        if make_symlink(&ghost_src, &broken).is_ok() {
            assert_eq!(classify_config(&src, &broken), ConfigState::Missing);
        }

        // 3. Symlink pointing at a FOREIGN destination: Missing - not the
        //    rice link, used to PASS verification as LinkedSymlink.
        let foreign_src = dir.join("other.lua");
        fs::write(&foreign_src, "other").unwrap();
        let foreign = dir.join("foreign.lua");
        if make_symlink(&foreign_src, &foreign).is_ok() {
            assert_eq!(classify_config(&src, &foreign), ConfigState::Missing);
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_inspect_package_aggregate_link_state() {
        let dir = temp_dir("rice_detect_aggregate");
        let repo = dir.join("repo");
        fs::create_dir_all(&repo).unwrap();
        let cfg_a = repo.join("a.yaml");
        fs::write(&cfg_a, "a").unwrap();
        let cfg_b = repo.join("b.yaml"); // intentionally never created

        let targets = dir.join("targets");
        fs::create_dir_all(&targets).unwrap();
        let target_a = targets.join("a.yaml");
        fs::write(&target_a, "a").unwrap();
        let target_b = targets.join("b.yaml"); // missing on disk

        let map_a = ConfigMapping {
            source_rel: leak(&cfg_a),
            target_pattern: leak(&target_a),
            is_dir: false,
        };
        let map_b = ConfigMapping {
            source_rel: leak(&cfg_b),
            target_pattern: leak(&target_b),
            is_dir: false,
        };

        // One of two configs missing -> whole package NotLinked.
        let item_partial = PackageItem {
            id: "detect-aggregate",
            name: "Detect Aggregate",
            category: Category::Terminals,
            description: "aggregate test",
            binaries: &[],
            scoop_cmd: None,
            winget_cmd: None,
            choco_cmd: None,
            cargo_cmd: None,
            configs: vec![map_a.clone(), map_b],
        };
        let inspection = inspect_package(&item_partial, &repo);
        assert_eq!(
            inspection.link_status,
            LinkStatus::NotLinked,
            "a missing config target must fail the whole package"
        );
        assert!(inspection.is_missing_any);

        // All configs deployed as plain copies -> LinkedCopy.
        let item_ok = PackageItem {
            id: "detect-aggregate",
            name: "Detect Aggregate",
            category: Category::Terminals,
            description: "aggregate test",
            binaries: &[],
            scoop_cmd: None,
            winget_cmd: None,
            choco_cmd: None,
            cargo_cmd: None,
            configs: vec![map_a],
        };
        let inspection = inspect_package(&item_ok, &repo);
        assert_eq!(inspection.link_status, LinkStatus::LinkedCopy(target_a.clone()));
        assert!(!inspection.is_missing_any);

        // Repo-native layout: source == target -> InPlace (never a "copy").
        let item_native = PackageItem {
            id: "detect-aggregate",
            name: "Detect Aggregate",
            category: Category::Terminals,
            description: "aggregate test",
            binaries: &[],
            scoop_cmd: None,
            winget_cmd: None,
            choco_cmd: None,
            cargo_cmd: None,
            configs: vec![ConfigMapping {
                source_rel: leak(&cfg_a),
                target_pattern: leak(&cfg_a),
                is_dir: false,
            }],
        };
        let inspection = inspect_package(&item_native, &repo);
        assert_eq!(inspection.link_status, LinkStatus::InPlace(cfg_a.clone()));
        assert!(!inspection.is_missing_any);

        let _ = fs::remove_dir_all(&dir);
    }
}
