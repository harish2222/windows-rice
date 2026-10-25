use std::path::PathBuf;
use crate::catalog::{get_catalog, PackageItem};
use crate::detect::{detect_package_managers, inspect_package, ItemInspection, PackageManagerStatus, BinaryStatus};
use crate::linker::{link_config_mapping, LinkOutcome};
use crate::bootstrap::{bootstrap_scoop, bootstrap_choco, run_package_install};
use crate::verifier::run_verification;

#[derive(Debug, Clone)]
pub struct UiItem {
    pub package: PackageItem,
    pub inspection: ItemInspection,
    pub selected: bool,
}

pub struct App {
    pub repo_root: PathBuf,
    pub pm_status: PackageManagerStatus,
    pub items: Vec<UiItem>,
    pub selected_index: usize,
    pub logs: Vec<String>,
    pub status_message: String,
    pub should_quit: bool,
}

impl App {
    pub fn new(repo_root: PathBuf) -> Self {
        let pm_status = detect_package_managers();
        let catalog = get_catalog();

        let mut items = Vec::new();
        for pkg in catalog {
            let inspection = inspect_package(&pkg, &repo_root);
            let selected = inspection.is_missing_any;
            items.push(UiItem {
                package: pkg,
                inspection,
                selected,
            });
        }

        let mut app = Self {
            repo_root,
            pm_status,
            items,
            selected_index: 0,
            logs: Vec::new(),
            status_message: "Ready. [Space] Toggle | [a] All | [m] Missing | [l] Link | [i] Install | [v] Verify".to_string(),
            should_quit: false,
        };

        app.add_log("Windows Rice Installer initialized.");
        app.add_log(format!("Repo root: {}", app.repo_root.display()));
        app
    }

    pub fn add_log(&mut self, msg: impl Into<String>) {
        self.logs.push(msg.into());
    }

    pub fn refresh_detection(&mut self) {
        self.pm_status = detect_package_managers();
        for item in &mut self.items {
            item.inspection = inspect_package(&item.package, &self.repo_root);
        }
    }

    pub fn next_item(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.items.len();
        }
    }

    pub fn prev_item(&mut self) {
        if !self.items.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.items.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    pub fn toggle_selected(&mut self) {
        if let Some(item) = self.items.get_mut(self.selected_index) {
            item.selected = !item.selected;
        }
    }

    pub fn select_all(&mut self) {
        let any_unselected = self.items.iter().any(|i| !i.selected);
        for item in &mut self.items {
            item.selected = any_unselected;
        }
        self.status_message = if any_unselected { "Selected all items." } else { "Deselected all items." }.to_string();
    }

    pub fn select_missing_only(&mut self) {
        for item in &mut self.items {
            item.selected = item.inspection.is_missing_any;
        }
        self.status_message = "Selected all missing/unlinked items.".to_string();
    }

    pub fn link_selected_configs(&mut self) {
        self.add_log("--- Starting Config Linking ---");
        let mut count_symlink = 0;
        let mut count_copy = 0;
        let mut count_valid = 0;
        let mut count_err = 0;

        let selected_pkgs: Vec<(String, Vec<crate::catalog::ConfigMapping>)> = self.items
            .iter()
            .filter(|i| i.selected && !i.package.configs.is_empty())
            .map(|i| (i.package.name.to_string(), i.package.configs.clone()))
            .collect();

        for (pkg_name, configs) in selected_pkgs {
            self.add_log(format!("Linking configs for {}:", pkg_name));
            for mapping in &configs {
                match link_config_mapping(&self.repo_root, mapping) {
                    LinkOutcome::SymlinkCreated(target) => {
                        count_symlink += 1;
                        self.add_log(format!("  [SYMLINK] -> {}", target.display()));
                    }
                    LinkOutcome::CopiedFallback(target) => {
                        count_copy += 1;
                        self.add_log(format!("  [COPIED]  -> {}", target.display()));
                    }
                    LinkOutcome::AlreadyValid(target) => {
                        count_valid += 1;
                        self.add_log(format!("  [ALREADY] -> {}", target.display()));
                    }
                    LinkOutcome::Failed(err) => {
                        count_err += 1;
                        self.add_log(format!("  [ERROR]   -> {}", err));
                    }
                }
            }
        }

        self.refresh_detection();
        self.status_message = format!("Linked configs: {} symlinks, {} copied, {} already valid, {} failed.",
            count_symlink, count_copy, count_valid, count_err);
        self.add_log(self.status_message.clone());
    }

    pub fn install_and_link_selected(&mut self) {
        self.add_log("--- Starting Installation & Linking ---");

        let items_to_install: Vec<(String, Option<String>)> = self.items
            .iter()
            .filter(|i| i.selected && matches!(i.inspection.binary_status, BinaryStatus::Missing))
            .map(|i| {
                let cmd_opt = if self.pm_status.scoop && i.package.scoop_cmd.is_some() {
                    i.package.scoop_cmd.map(|s| s.to_string())
                } else if self.pm_status.winget && i.package.winget_cmd.is_some() {
                    i.package.winget_cmd.map(|s| s.to_string())
                } else if self.pm_status.choco && i.package.choco_cmd.is_some() {
                    i.package.choco_cmd.map(|s| s.to_string())
                } else if self.pm_status.cargo && i.package.cargo_cmd.is_some() {
                    i.package.cargo_cmd.map(|s| s.to_string())
                } else {
                    i.package.scoop_cmd.or(i.package.winget_cmd).map(|s| s.to_string())
                };
                (i.package.name.to_string(), cmd_opt)
            })
            .collect();

        for (pkg_name, cmd_opt) in items_to_install {
            if let Some(cmd) = cmd_opt {
                self.add_log(format!("Installing {} via: {}", pkg_name, cmd));
                match run_package_install(&cmd) {
                    Ok(_) => self.add_log(format!("  [PASS] Installed {}", pkg_name)),
                    Err(e) => self.add_log(format!("  [FAIL] Error installing {}: {}", pkg_name, e)),
                }
            } else {
                self.add_log(format!("  [SKIP] No automated install command available for {}", pkg_name));
            }
        }

        // Link configs
        self.link_selected_configs();
    }

    pub fn run_verification_report(&mut self) {
        self.add_log("--- Running Health Check Report ---");
        let report = run_verification(&self.repo_root);
        for line in report.lines {
            self.add_log(line);
        }
        self.refresh_detection();
        self.status_message = format!("Health check complete: {}/{} binaries, {}/{} configs.",
            report.installed_binaries, report.total_packages, report.linked_configs, report.total_packages);
    }

    pub fn bootstrap_package_managers(&mut self) {
        self.add_log("--- Bootstrapping Package Managers ---");
        if !self.pm_status.scoop {
            self.add_log("Bootstrapping Scoop...");
            match bootstrap_scoop() {
                Ok(_) => self.add_log("  [PASS] Scoop successfully bootstrapped!"),
                Err(e) => self.add_log(format!("  [FAIL] Failed to bootstrap Scoop: {}", e)),
            }
        } else {
            self.add_log("Scoop is already installed.");
        }

        if !self.pm_status.choco {
            self.add_log("Bootstrapping Chocolatey...");
            match bootstrap_choco() {
                Ok(_) => self.add_log("  [PASS] Chocolatey successfully bootstrapped!"),
                Err(e) => self.add_log(format!("  [FAIL] Failed to bootstrap Chocolatey: {}", e)),
            }
        } else {
            self.add_log("Chocolatey is already installed.");
        }

        if !self.pm_status.uv {
            self.add_log("Bootstrapping UV...");
            match crate::bootstrap::bootstrap_uv() {
                Ok(_) => self.add_log("  [PASS] UV successfully bootstrapped!"),
                Err(e) => self.add_log(format!("  [FAIL] Failed to bootstrap UV: {}", e)),
            }
        } else {
            self.add_log("UV is already installed.");
        }

        if !self.pm_status.vcpkg {
            self.add_log("Bootstrapping Vcpkg...");
            match crate::bootstrap::bootstrap_vcpkg() {
                Ok(_) => self.add_log("  [PASS] Vcpkg successfully bootstrapped!"),
                Err(e) => self.add_log(format!("  [FAIL] Failed to bootstrap Vcpkg: {}", e)),
            }
        } else {
            self.add_log("Vcpkg is already installed.");
        }

        self.refresh_detection();
    }
}
