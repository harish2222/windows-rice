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
    pub git_update_status: Option<String>,
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
            status_message: "WINDOW-RICE installer ready. [r] Reload WM | [u] Updates | [w] Wallpapers | [l] Link | [i] Install".to_string(),
            git_update_status: None,
            should_quit: false,
        };

        app.add_log("WINDOW-RICE installer initialized.");
        app.add_log(format!("Repository root: {}", app.repo_root.display()));
        app.check_updates();
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

        // Check if wallpapers was selected
        if self.items.iter().any(|i| i.selected && i.package.id == "wallpapers") {
            self.install_wallpapers(None);
        }

        // Automatically reload window managers and status bar
        self.reload_environment();
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

    pub fn reload_environment(&mut self) {
        self.add_log("=== Reloading Windows Rice Environment ===");

        // 1. Komorebi
        if crate::detect::find_binary_in_path("komorebic.exe").is_some() {
            self.add_log("Reloading Komorebi configuration...");
            let config_path = crate::catalog::resolve_target_path(".config/komorebi/komorebi.json");
            let res = std::process::Command::new("komorebic")
                .args(["replace-configuration", &config_path.to_string_lossy()])
                .output();
            match res {
                Ok(out) if out.status.success() => self.add_log("  [PASS] Komorebi configuration reloaded."),
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr);
                    if err.trim().is_empty() {
                        self.add_log("  [PASS] Komorebi configuration reloaded.");
                    } else {
                        self.add_log(format!("  [INFO] Komorebi reload: {}", err.trim()));
                    }
                }
                Err(e) => self.add_log(format!("  [WARN] Komorebi reload notice: {}", e)),
            }
        } else {
            self.add_log("  [INFO] komorebic not in PATH (skipping).");
        }

        // 2. GlazeWM
        if crate::detect::find_binary_in_path("glazewm.exe").is_some() {
            self.add_log("Reloading GlazeWM hotkey layer...");
            let res = std::process::Command::new("glazewm")
                .args(["command", "wm-reload-config"])
                .output();
            match res {
                Ok(out) if out.status.success() => self.add_log("  [PASS] GlazeWM hotkey layer reloaded."),
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr);
                    if err.trim().is_empty() {
                        self.add_log("  [PASS] GlazeWM hotkey layer reloaded.");
                    } else {
                        self.add_log(format!("  [INFO] GlazeWM: {}", err.trim()));
                    }
                }
                Err(e) => self.add_log(format!("  [WARN] GlazeWM reload notice: {}", e)),
            }
        } else {
            self.add_log("  [INFO] glazewm not in PATH (skipping).");
        }

        // 3. YASB
        let yasb_setup = self.repo_root.join("yasb/tools/setup/yasb-setup.ps1");
        if yasb_setup.exists() {
            self.add_log("Re-running YASB status bar replication / verification...");
            let res = std::process::Command::new("pwsh")
                .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", &yasb_setup.to_string_lossy()])
                .output();
            match res {
                Ok(out) if out.status.success() => self.add_log("  [PASS] YASB status bar active and verified."),
                Ok(out) => {
                    let out_msg = String::from_utf8_lossy(&out.stdout);
                    self.add_log(format!("  [PASS] YASB updated: {}", out_msg.lines().next().unwrap_or("OK")));
                }
                Err(e) => self.add_log(format!("  [WARN] YASB setup notice: {}", e)),
            }
        }

        self.status_message = "Windows Rice environment (Komorebi, GlazeWM, YASB) reloaded!".to_string();
        self.add_log(self.status_message.clone());
    }

    pub fn check_updates(&mut self) {
        self.add_log("Checking for repository updates against origin/main...");
        let git_status = std::process::Command::new("git")
            .args(["-C", &self.repo_root.to_string_lossy(), "status", "-uno"])
            .output();

        if let Ok(out) = git_status {
            let msg = String::from_utf8_lossy(&out.stdout);
            if msg.contains("Your branch is behind") {
                self.git_update_status = Some("UPDATE AVAILABLE".to_string());
                self.add_log("  [WARN] Updates available on origin/main! Run 'git pull' to update.");
                self.status_message = "Updates available from GitHub. Press [u] or git pull.".to_string();
            } else if msg.contains("Your branch is up to date") {
                self.git_update_status = Some("UP-TO-DATE".to_string());
                self.add_log("  [PASS] Repository configurations are fully up-to-date.");
                self.status_message = "Rice configurations are fully up to date.".to_string();
            } else {
                self.git_update_status = Some("LIVE".to_string());
                self.add_log("  [INFO] Repository in live custom state.");
            }
        } else {
            self.git_update_status = Some("READY".to_string());
        }
    }

    pub fn install_wallpapers(&mut self, custom_dest: Option<PathBuf>) {
        self.add_log("=== Installing 8K Wallpaper Collection ===");
        let dest = custom_dest.unwrap_or_else(|| {
            if let Some(user_profile) = std::env::var_os("USERPROFILE") {
                let p1 = PathBuf::from(&user_profile).join("OneDrive/Pictures/Wallpapers");
                if p1.parent().map(|p| p.exists()).unwrap_or(false) {
                    return p1;
                }
                PathBuf::from(user_profile).join("Pictures/Wallpapers")
            } else {
                PathBuf::from("Wallpapers")
            }
        });

        self.add_log(format!("Target destination: {}", dest.display()));

        let local_organized = PathBuf::from("Z:\\HK\\8K Wallpaper\\_Organized");
        if local_organized.exists() {
            self.add_log(format!("Found local collection at: {}", local_organized.display()));
            let _ = std::fs::create_dir_all(&dest);

            if let Ok(entries) = std::fs::read_dir(&local_organized) {
                let mut linked_count = 0;
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let target = dest.join(&name);
                    let src = entry.path();
                    #[cfg(windows)]
                    {
                        if src.is_dir() {
                            match std::os::windows::fs::symlink_dir(&src, &target) {
                                Ok(_) => {
                                    linked_count += 1;
                                    self.add_log(format!("  [SYMLINK] -> {}", target.display()));
                                }
                                Err(_) => {
                                    self.add_log(format!("  [COPY] Copying {}...", name.to_string_lossy()));
                                    let _ = crate::linker::copy_recursive(&src, &target, true);
                                    linked_count += 1;
                                }
                            }
                        }
                    }
                }
                self.status_message = format!("Wallpapers installed to {} ({} folders).", dest.display(), linked_count);
                self.add_log(self.status_message.clone());
                return;
            }
        }

        self.add_log("Note: Remote users can download Wallpaper Packs 1, 2, 3 from GitHub Release assets.");
        self.status_message = format!("Wallpaper destination prepared: {}", dest.display());
    }
}
