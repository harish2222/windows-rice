// ============================================================================
// Author -: HKDEVS
// Project -: Window-Rice
// Title -: WINDOW-RICE Installer System Health Verifier
// ============================================================================
use std::path::Path;
use crate::catalog::get_catalog;
use crate::detect::{detect_package_managers, inspect_package, BinaryStatus, LinkStatus};

pub struct VerificationReport {
    pub total_packages: usize,
    /// Packages that actually map configuration targets (README contract:
    /// 20 config targets). The config denominator uses this - NOT the full
    /// catalog - so the summary can legitimately reach 20/20.
    pub total_configs: usize,
    pub installed_binaries: usize,
    pub linked_configs: usize,
    pub lines: Vec<String>,
}

pub fn run_verification(repo_root: &Path) -> VerificationReport {
    let mut lines = Vec::new();
    lines.push("================================================================================".to_string());
    lines.push("                   WINDOWS RICE SYSTEM HEALTH CHECK REPORT                      ".to_string());
    lines.push("================================================================================".to_string());

    // 1. Package Managers
    let pm = detect_package_managers();
    lines.push("\n[Package Managers]".to_string());
    lines.push(format!("  - Scoop:       {}", if pm.scoop { "[PASS] Installed" } else { "[WARN] Not found" }));
    lines.push(format!("  - Winget:      {}", if pm.winget { "[PASS] Installed" } else { "[WARN] Not found" }));
    lines.push(format!("  - Chocolatey:  {}", if pm.choco { "[PASS] Installed" } else { "[INFO] Not found" }));
    lines.push(format!("  - Cargo:       {}", if pm.cargo { "[PASS] Installed" } else { "[INFO] Not found" }));
    lines.push(format!("  - UV (Python): {}", if pm.uv { "[PASS] Installed" } else { "[INFO] Not found" }));
    lines.push(format!("  - Vcpkg (C++): {}", if pm.vcpkg { "[PASS] Installed" } else { "[INFO] Not found" }));
    lines.push(format!("  - Python:      {}", if pm.python { "[PASS] Installed" } else { "[WARN] Not found" }));
    lines.push(format!("  - NPM:         {}", if pm.npm { "[PASS] Installed" } else { "[INFO] Not found" }));

    // 2. Packages inspection
    let catalog = get_catalog();
    let total_packages = catalog.len();
    let total_configs = catalog.iter().filter(|i| !i.configs.is_empty()).count();
    let mut installed_binaries = 0;
    let mut linked_configs = 0;

    lines.push("\n[Components & Configs]".to_string());
    lines.push(format!("{:<25} {:<14} {:<18} {:<24}", "Package", "Binary", "Config Status", "Target Location"));
    lines.push("-".repeat(80));

    for item in &catalog {
        let inspection = inspect_package(item, repo_root);

        let bin_str = match &inspection.binary_status {
            BinaryStatus::Installed(_) => {
                installed_binaries += 1;
                "[PASS] Found"
            }
            BinaryStatus::Missing => "[FAIL] Missing",
        };

        let link_str = match &inspection.link_status {
            LinkStatus::LinkedSymlink(_) => {
                linked_configs += 1;
                "[PASS] Symlink"
            }
            LinkStatus::LinkedCopy(_) => {
                linked_configs += 1;
                "[PASS] Copied"
            }
            LinkStatus::InPlace(_) => {
                linked_configs += 1;
                "[PASS] In-Place"
            }
            LinkStatus::NotLinked => "[WARN] Not Linked",
            LinkStatus::NotApplicable => "[INFO] N/A",
        };

        let target_str = item.configs.first()
            .map(|c| c.target_pattern)
            .unwrap_or("PATH only");

        lines.push(format!("{:<25} {:<14} {:<18} {:<24}", item.name, bin_str, link_str, target_str));
    }

    // 3. Carapace specs count
    let specs_dir = repo_root.join("carapace").join("specs");
    if specs_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&specs_dir) {
            let count = entries.filter_map(Result::ok).count();
            lines.push(format!("\n[Carapace Autocompletions]: {} custom YAML specs loaded in {}", count, specs_dir.display()));
        }
    }

    lines.push("-".repeat(80));
    lines.push(format!("Summary: {}/{} binaries detected, {}/{} configs linked.",
        installed_binaries, total_packages, linked_configs, total_configs));
    lines.push("================================================================================".to_string());

    VerificationReport {
        total_packages,
        total_configs,
        installed_binaries,
        linked_configs,
        lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_summary_denominators() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("repo root")
            .to_path_buf();
        let report = run_verification(&root);

        // README contract: 23 catalog packages, 20 configuration targets.
        assert_eq!(report.total_packages, 23, "catalog must expose 23 packages");
        assert_eq!(
            report.total_configs, 20,
            "config denominator must count only config-bearing packages (20), matching README"
        );

        // The rendered summary must use the config-bearing denominator so a
        // fully-linked host can actually reach 20/20.
        let summary = report
            .lines
            .iter()
            .find(|l| l.starts_with("Summary:"))
            .expect("summary line must exist");
        assert!(
            summary.contains(&format!("{}/{} configs linked", report.linked_configs, 20)),
            "summary must be `X/20 configs linked`, got: {}",
            summary
        );
        assert!(
            summary.contains(&format!("{}/{} binaries detected", report.installed_binaries, 23)),
            "summary must keep the `X/23 binaries detected` contract, got: {}",
            summary
        );

        // Report structure required by e2e (T1-R5-03).
        let all = report.lines.join("\n");
        assert!(all.contains("WINDOWS RICE SYSTEM HEALTH CHECK REPORT"));
        assert!(all.contains("[Package Managers]"));
        assert!(all.contains("[Components & Configs]"));
        assert!(all.contains("[Carapace Autocompletions]"));
    }
}
