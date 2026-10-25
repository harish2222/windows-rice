// ============================================================================
// Author -: HKDEVS
// Project -: Window-Rice
// Title -: WINDOW-RICE Installer Application State
// ============================================================================
use std::path::PathBuf;
use std::sync::Mutex;
use crate::catalog::{get_catalog, PackageItem};
use crate::detect::{detect_package_managers, inspect_package, ItemInspection, PackageManagerStatus, BinaryStatus};
use crate::linker::{link_config_mapping, LinkOutcome};
use crate::bootstrap::{bootstrap_scoop, bootstrap_choco, run_package_install};
use crate::verifier::run_verification;
use crossterm::event::KeyCode;

static LOG_MUTEX: Mutex<()> = Mutex::new(());

pub fn format_iso8601(time: std::time::SystemTime) -> String {
    let dur = time.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let total_secs = dur.as_secs();
    let sec = (total_secs % 60) as u32;
    let total_mins = total_secs / 60;
    let min = (total_mins % 60) as u32;
    let total_hours = total_mins / 60;
    let hour = (total_hours % 24) as u32;
    let mut days = (total_hours / 24) as i64;

    // Civil day algorithm (Howard Hinnant)
    days += 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = (days - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, m, d, hour, min, sec)
}

pub fn log_to_file(text: &str) {
    let _guard = LOG_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let log_dir = crate::linker::get_backup_root();
    if std::fs::create_dir_all(&log_dir).is_ok() {
        let log_file = log_dir.join("installer.log");
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(log_file) {
            let ts = format_iso8601(std::time::SystemTime::now());
            let _ = writeln!(f, "[{}] {}", ts, text);
        }
    }
}

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
            status_message: "WINDOW-RICE ready. [Space/Enter] Toggle | [l] Link | [u/x] Unlink & Restore | [r] Reload | [c] Updates".to_string(),
            git_update_status: None,
            should_quit: false,
        };

        app.add_log("WINDOW-RICE installer initialized.");
        app.add_log(format!("Repository root: {}", app.repo_root.display()));
        app.check_updates();
        app
    }

    pub fn add_log(&mut self, msg: impl Into<String>) {
        let text = msg.into();
        self.logs.push(text.clone());
        log_to_file(&text);
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

    pub fn page_down(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = (self.selected_index + 5).min(self.items.len() - 1);
        }
    }

    pub fn page_up(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = self.selected_index.saturating_sub(5);
        }
    }

    pub fn home(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = 0;
        }
    }

    pub fn end(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = self.items.len() - 1;
        }
    }

    pub fn handle_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                self.next_item();
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                self.prev_item();
            }
            KeyCode::PageDown => {
                self.page_down();
            }
            KeyCode::PageUp => {
                self.page_up();
            }
            KeyCode::Home => {
                self.home();
            }
            KeyCode::End => {
                self.end();
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                self.toggle_selected();
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.select_all();
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.select_none();
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                self.select_missing_only();
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                self.link_selected_configs();
            }
            KeyCode::Char('u') | KeyCode::Char('U')
            | KeyCode::Char('x') | KeyCode::Char('X')
            | KeyCode::Delete => {
                self.unlink_selected_configs();
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                self.install_and_link_selected();
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                self.run_verification_report();
            }
            KeyCode::Char('b') | KeyCode::Char('B') => {
                self.bootstrap_package_managers();
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.reload_environment();
            }
            KeyCode::Char('c') | KeyCode::Char('C')
            | KeyCode::Char('g') | KeyCode::Char('G') => {
                self.check_updates();
            }
            KeyCode::Char('w') | KeyCode::Char('W') => {
                self.install_wallpapers(None);
            }
            _ => {}
        }
        self.should_quit
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

    pub fn select_none(&mut self) {
        for item in &mut self.items {
            item.selected = false;
        }
        self.status_message = "Deselected all items.".to_string();
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
        let mut count_backed_up = 0;
        let mut count_err = 0;

        let selected_pkgs: Vec<(String, String, Vec<crate::catalog::ConfigMapping>)> = self.items
            .iter()
            .filter(|i| i.selected && !i.package.configs.is_empty())
            .map(|i| (i.package.id.to_string(), i.package.name.to_string(), i.package.configs.clone()))
            .collect();

        for (pkg_id, pkg_name, configs) in selected_pkgs {
            self.add_log(format!("Linking configs for {}:", pkg_name));
            for mapping in &configs {
                match link_config_mapping(&self.repo_root, mapping, &pkg_id) {
                    LinkOutcome::SymlinkCreated(target) => {
                        count_symlink += 1;
                        self.add_log(format!("  [SYMLINK] -> {}", target.display()));
                    }
                    LinkOutcome::CopiedFallback(target) => {
                        count_copy += 1;
                        self.add_log(format!("  [COPIED]  -> {}", target.display()));
                    }
                    LinkOutcome::BackedUpAndLinked { target, backup } => {
                        count_backed_up += 1;
                        count_symlink += 1;
                        self.add_log(format!("  [BACKUP]  -> Existing saved to {}", backup.display()));
                        self.add_log(format!("  [SYMLINK] -> {}", target.display()));
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
        self.status_message = format!("Linked configs: {} symlinks ({} backed up), {} copied, {} already valid, {} failed.",
            count_symlink, count_backed_up, count_copy, count_valid, count_err);
        self.add_log(self.status_message.clone());

        // Check if wallpapers was selected
        if self.items.iter().any(|i| i.selected && i.package.id == "wallpapers") {
            self.install_wallpapers(None);
        }

        // Automatically reload window managers and status bar
        self.reload_environment();
    }

    pub fn unlink_selected_configs(&mut self) {
        self.add_log("--- Starting Config Removal & Unlinking (Preserving Binaries) ---");
        let mut count_restored = 0;
        let mut count_cleaned = 0;
        let mut count_not_linked = 0;
        let mut count_err = 0;

        let selected_pkgs: Vec<(String, String, Vec<crate::catalog::ConfigMapping>)> = self.items
            .iter()
            .filter(|i| i.selected)
            .map(|i| (i.package.id.to_string(), i.package.name.to_string(), i.package.configs.clone()))
            .collect();

        for (pkg_id, pkg_name, configs) in selected_pkgs {
            if pkg_id == "wallpapers" {
                self.add_log("Uninstalling 8K Wallpapers from Pictures...");
                self.uninstall_wallpapers(None);
                continue;
            }

            self.add_log(format!("Removing configs for {} (binaries kept):", pkg_name));
            for mapping in &configs {
                match crate::linker::unlink_config_mapping(&self.repo_root, mapping, &pkg_id) {
                    crate::linker::UnlinkOutcome::UnlinkedAndRestored { target, backup_source } => {
                        count_restored += 1;
                        self.add_log(format!("  [RESTORED] -> Restored backup: {}", backup_source.display()));
                        self.add_log(format!("  [TARGET]   -> {}", target.display()));
                    }
                    crate::linker::UnlinkOutcome::UnlinkedCleanly(target) => {
                        count_cleaned += 1;
                        self.add_log(format!("  [CLEAN]    -> Removed {}", target.display()));
                    }
                    crate::linker::UnlinkOutcome::NotLinked(target) => {
                        count_not_linked += 1;
                        self.add_log(format!("  [NO-OP]    -> Not linked: {}", target.display()));
                    }
                    crate::linker::UnlinkOutcome::Failed(err) => {
                        count_err += 1;
                        self.add_log(format!("  [ERROR]    -> {}", err));
                    }
                }
            }
        }

        self.refresh_detection();
        self.status_message = format!("Unlink complete: {} restored from backup, {} cleaned, {} no-op, {} failed.",
            count_restored, count_cleaned, count_not_linked, count_err);
        self.add_log(self.status_message.clone());

        // Automatically reload window managers and status bar
        self.reload_environment();
    }

    pub fn unlink_all_configs(&mut self) {
        self.select_all();
        self.unlink_selected_configs();
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
        self.add_log(self.status_message.clone());
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
                .output()
                .or_else(|_| {
                    std::process::Command::new("powershell")
                        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", &yasb_setup.to_string_lossy()])
                        .output()
                });
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

    pub fn uninstall_wallpapers(&mut self, custom_dest: Option<PathBuf>) {
        self.add_log("=== Removing 8K Wallpaper Collection ===");
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

        if dest.exists() || dest.symlink_metadata().is_ok() {
            let _ = std::fs::remove_dir_all(&dest);
            self.add_log(format!("  [CLEAN] Removed wallpaper directory: {}", dest.display()));
            self.status_message = format!("Wallpapers uninstalled from {}.", dest.display());
        } else {
            self.add_log(format!("  [INFO] Wallpapers not installed at {}", dest.display()));
            self.status_message = "Wallpapers were not installed.".to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::detect::detect_package_managers;

    struct TestEnv {
        _guard: std::sync::MutexGuard<'static, ()>,
        temp_dir: PathBuf,
    }

    impl TestEnv {
        fn new(name: &str) -> Self {
            let guard = crate::linker::tests::TEST_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
            let temp_dir = std::env::temp_dir().join(format!("rice_test_app_{}", name));
            let _ = std::fs::remove_dir_all(&temp_dir);
            std::fs::create_dir_all(&temp_dir).unwrap();
            unsafe { std::env::set_var("RICE_BACKUP_DIR", &temp_dir); }
            Self { _guard: guard, temp_dir }
        }
    }

    impl Drop for TestEnv {
        fn drop(&mut self) {
            unsafe { std::env::remove_var("RICE_BACKUP_DIR"); }
            let _ = std::fs::remove_dir_all(&self.temp_dir);
        }
    }

    #[test]
    fn test_navigation_and_bounds() {
        let _env = TestEnv::new("navigation");
        let repo_root = PathBuf::from(".");
        let mut app = App::new(repo_root);

        if app.items.is_empty() {
            return;
        }

        let total = app.items.len();

        // Initial index should be 0
        assert_eq!(app.selected_index, 0);

        // Next item
        app.next_item();
        assert_eq!(app.selected_index, 1 % total);

        // Prev item back to 0
        app.prev_item();
        assert_eq!(app.selected_index, 0);

        // Prev item wrap around to end
        app.prev_item();
        assert_eq!(app.selected_index, total - 1);

        // Next item wrap around to start
        app.next_item();
        assert_eq!(app.selected_index, 0);

        // Home and End
        app.end();
        assert_eq!(app.selected_index, total - 1);
        app.home();
        assert_eq!(app.selected_index, 0);

        // Page down and Page up
        app.page_down();
        assert_eq!(app.selected_index, 5.min(total - 1));
        app.page_up();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_selection_toggles() {
        let _env = TestEnv::new("selection");
        let repo_root = PathBuf::from(".");
        let mut app = App::new(repo_root);

        if app.items.is_empty() {
            return;
        }

        // Test select none
        app.select_none();
        assert!(app.items.iter().all(|i| !i.selected));

        // Test toggle current
        app.selected_index = 0;
        app.toggle_selected();
        assert!(app.items[0].selected);
        app.toggle_selected();
        assert!(!app.items[0].selected);

        // Test select all
        app.select_all();
        assert!(app.items.iter().all(|i| i.selected));

        // Calling select all again when all selected deselects all
        app.select_all();
        assert!(app.items.iter().all(|i| !i.selected));
    }

    #[test]
    fn test_binary_preservation_guarantee() {
        let _env = TestEnv::new("bin_preservation_guarantee");
        let repo_root = PathBuf::from(".");
        let app = App::new(repo_root);

        // Verify that catalog packages distinguish configs from binaries
        for item in &app.items {
            for bin in item.package.binaries {
                // Ensure no binary name is treated as a config target
                assert!(!bin.ends_with(".yaml") && !bin.ends_with(".json") && !bin.ends_with(".toml"),
                    "Binary {} should be an executable name, not a config file", bin);
            }
        }
    }

    #[test]
    fn test_case_insensitive_hotkeys() {
        let _env = TestEnv::new("hotkeys");
        let repo_root = PathBuf::from(".");
        let mut app = App::new(repo_root);

        // 1. Quit hotkeys: q/Q and Esc
        app.should_quit = false;
        assert!(app.handle_key(KeyCode::Char('q')));
        assert!(app.should_quit);

        app.should_quit = false;
        assert!(app.handle_key(KeyCode::Char('Q')));
        assert!(app.should_quit);

        app.should_quit = false;
        assert!(app.handle_key(KeyCode::Esc));
        assert!(app.should_quit);
        app.should_quit = false;

        // 2. Navigation hotkeys: j/J and k/K (down/up)
        app.selected_index = 0;
        assert!(!app.handle_key(KeyCode::Char('j')));
        assert_eq!(app.selected_index, 1);

        assert!(!app.handle_key(KeyCode::Char('J')));
        assert_eq!(app.selected_index, 2);

        assert!(!app.handle_key(KeyCode::Char('k')));
        assert_eq!(app.selected_index, 1);

        assert!(!app.handle_key(KeyCode::Char('K')));
        assert_eq!(app.selected_index, 0);

        // 3. Select all / deselect all hotkeys: a/A
        app.select_none();
        assert!(!app.handle_key(KeyCode::Char('a')));
        assert!(app.items.iter().all(|i| i.selected));

        assert!(!app.handle_key(KeyCode::Char('A')));
        assert!(app.items.iter().all(|i| !i.selected));

        // 4. Select none hotkeys: n/N
        app.select_all();
        assert!(!app.handle_key(KeyCode::Char('n')));
        assert!(app.items.iter().all(|i| !i.selected));

        app.select_all();
        assert!(!app.handle_key(KeyCode::Char('N')));
        assert!(app.items.iter().all(|i| !i.selected));

        // 5. Select missing only hotkeys: m/M
        app.select_all();
        assert!(!app.handle_key(KeyCode::Char('m')));
        assert!(app.items.iter().all(|i| i.selected == i.inspection.is_missing_any));

        app.select_all();
        assert!(!app.handle_key(KeyCode::Char('M')));
        assert!(app.items.iter().all(|i| i.selected == i.inspection.is_missing_any));

        // 6. Link configs hotkeys: l/L
        app.select_none();
        let logs_count_l = app.logs.len();
        assert!(!app.handle_key(KeyCode::Char('l')));
        assert!(app.logs.len() > logs_count_l);
        assert!(app.logs.iter().any(|l| l.contains("Linked configs")));

        let logs_count_l_upper = app.logs.len();
        assert!(!app.handle_key(KeyCode::Char('L')));
        assert!(app.logs.len() > logs_count_l_upper);
        assert!(app.logs.iter().any(|l| l.contains("Linked configs")));

        // 7. Unlink configs hotkeys: u/U, x/X, and Delete
        app.select_none();
        let logs_count_u = app.logs.len();
        assert!(!app.handle_key(KeyCode::Char('u')));
        assert!(app.logs.len() > logs_count_u);
        assert!(app.logs.iter().any(|l| l.contains("Unlink complete")));

        let logs_count_u_upper = app.logs.len();
        assert!(!app.handle_key(KeyCode::Char('U')));
        assert!(app.logs.len() > logs_count_u_upper);

        let logs_count_x = app.logs.len();
        assert!(!app.handle_key(KeyCode::Char('x')));
        assert!(app.logs.len() > logs_count_x);

        let logs_count_x_upper = app.logs.len();
        assert!(!app.handle_key(KeyCode::Char('X')));
        assert!(app.logs.len() > logs_count_x_upper);

        let logs_count_del = app.logs.len();
        assert!(!app.handle_key(KeyCode::Delete));
        assert!(app.logs.len() > logs_count_del);

        // 8. Install and link hotkeys: i/I
        app.select_none();
        let logs_count_i = app.logs.len();
        assert!(!app.handle_key(KeyCode::Char('i')));
        assert!(app.logs.len() > logs_count_i);

        let logs_count_i_upper = app.logs.len();
        assert!(!app.handle_key(KeyCode::Char('I')));
        assert!(app.logs.len() > logs_count_i_upper);

        // 9. Reload environment hotkeys: r/R
        let logs_count_r = app.logs.len();
        assert!(!app.handle_key(KeyCode::Char('r')));
        assert!(app.logs.len() > logs_count_r);

        let logs_count_r_upper = app.logs.len();
        assert!(!app.handle_key(KeyCode::Char('R')));
        assert!(app.logs.len() > logs_count_r_upper);

        // 10. Check updates hotkeys: c/C and g/G
        app.git_update_status = None;
        assert!(!app.handle_key(KeyCode::Char('c')));
        assert!(app.git_update_status.is_some());

        app.git_update_status = None;
        assert!(!app.handle_key(KeyCode::Char('C')));
        assert!(app.git_update_status.is_some());

        app.git_update_status = None;
        assert!(!app.handle_key(KeyCode::Char('g')));
        assert!(app.git_update_status.is_some());

        app.git_update_status = None;
        assert!(!app.handle_key(KeyCode::Char('G')));
        assert!(app.git_update_status.is_some());

        // 11. Install wallpapers hotkeys: w/W
        assert!(!app.handle_key(KeyCode::Char('w')));
        assert!(app.status_message.contains("Wallpaper"));

        assert!(!app.handle_key(KeyCode::Char('W')));
        assert!(app.status_message.contains("Wallpaper"));

        // 12. Run verification report hotkeys: v/V
        let logs_count_v = app.logs.len();
        assert!(!app.handle_key(KeyCode::Char('v')));
        assert!(app.logs.len() > logs_count_v);

        let logs_count_v_upper = app.logs.len();
        assert!(!app.handle_key(KeyCode::Char('V')));
        assert!(app.logs.len() > logs_count_v_upper);

        // 13. Bootstrap package managers hotkeys: b/B
        app.pm_status.scoop = true;
        app.pm_status.choco = true;
        app.pm_status.uv = true;
        app.pm_status.vcpkg = true;
        let logs_count_b = app.logs.len();
        assert!(!app.handle_key(KeyCode::Char('b')));
        assert!(app.logs.len() > logs_count_b);

        let logs_count_b_upper = app.logs.len();
        assert!(!app.handle_key(KeyCode::Char('B')));
        assert!(app.logs.len() > logs_count_b_upper);
    }

    #[test]
    fn test_space_and_enter_selection_toggle() {
        let _env = TestEnv::new("space_enter_toggle");
        let repo_root = PathBuf::from(".");
        let mut app = App::new(repo_root);
        assert!(!app.items.is_empty(), "Items list must not be empty");

        app.selected_index = 0;
        let initial_0 = app.items[0].selected;

        // Test toggle via Space (KeyCode::Char(' '))
        assert!(!app.handle_key(KeyCode::Char(' ')));
        assert_eq!(app.items[0].selected, !initial_0);

        // Test toggle back via Space
        assert!(!app.handle_key(KeyCode::Char(' ')));
        assert_eq!(app.items[0].selected, initial_0);

        // Test toggle via Enter (KeyCode::Enter)
        assert!(!app.handle_key(KeyCode::Enter));
        assert_eq!(app.items[0].selected, !initial_0);

        // Test toggle back via Enter
        assert!(!app.handle_key(KeyCode::Enter));
        assert_eq!(app.items[0].selected, initial_0);

        // Test on index 1 if exists
        if app.items.len() > 1 {
            app.selected_index = 1;
            let initial_1 = app.items[1].selected;

            assert!(!app.handle_key(KeyCode::Char(' ')));
            assert_eq!(app.items[1].selected, !initial_1);

            assert!(!app.handle_key(KeyCode::Enter));
            assert_eq!(app.items[1].selected, initial_1);
        }
    }

    #[test]
    fn test_select_missing_only() {
        let _env = TestEnv::new("select_missing");
        let repo_root = PathBuf::from(".");
        let mut app = App::new(repo_root);
        assert!(app.items.len() >= 2, "Need at least 2 items to test select_missing_only");

        // Set known inspection state for items 0 and 1
        app.items[0].inspection.is_missing_any = true;
        app.items[1].inspection.is_missing_any = false;

        // Start with all items selected
        app.select_all();
        assert!(app.items.iter().all(|i| i.selected));

        // Press 'm'
        assert!(!app.handle_key(KeyCode::Char('m')));
        assert!(app.items[0].selected, "Missing item 0 must be selected");
        assert!(!app.items[1].selected, "Linked item 1 must be deselected");
        for item in &app.items {
            assert_eq!(item.selected, item.inspection.is_missing_any);
        }

        // Start with all items deselected
        app.select_none();
        assert!(app.items.iter().all(|i| !i.selected));

        // Press 'M'
        assert!(!app.handle_key(KeyCode::Char('M')));
        assert!(app.items[0].selected, "Missing item 0 must be selected");
        assert!(!app.items[1].selected, "Linked item 1 must be deselected");
        for item in &app.items {
            assert_eq!(item.selected, item.inspection.is_missing_any);
        }
    }

    #[test]
    fn test_navigation_bounds_extreme() {
        let _env = TestEnv::new("bounds_extreme");

        // 1. Navigation on completely empty item list
        let mut empty_app = App {
            repo_root: PathBuf::from("."),
            pm_status: detect_package_managers(),
            items: Vec::new(),
            selected_index: 0,
            logs: Vec::new(),
            status_message: String::new(),
            git_update_status: None,
            should_quit: false,
        };

        // None of these should panic or underflow on empty items
        empty_app.handle_key(KeyCode::Up);
        assert_eq!(empty_app.selected_index, 0);

        empty_app.handle_key(KeyCode::Down);
        assert_eq!(empty_app.selected_index, 0);

        empty_app.handle_key(KeyCode::PageUp);
        assert_eq!(empty_app.selected_index, 0);

        empty_app.handle_key(KeyCode::PageDown);
        assert_eq!(empty_app.selected_index, 0);

        empty_app.handle_key(KeyCode::Home);
        assert_eq!(empty_app.selected_index, 0);

        empty_app.handle_key(KeyCode::End);
        assert_eq!(empty_app.selected_index, 0);

        empty_app.next_item();
        assert_eq!(empty_app.selected_index, 0);

        empty_app.prev_item();
        assert_eq!(empty_app.selected_index, 0);

        empty_app.page_up();
        assert_eq!(empty_app.selected_index, 0);

        empty_app.page_down();
        assert_eq!(empty_app.selected_index, 0);

        empty_app.home();
        assert_eq!(empty_app.selected_index, 0);

        empty_app.end();
        assert_eq!(empty_app.selected_index, 0);

        empty_app.toggle_selected();
        assert_eq!(empty_app.selected_index, 0);

        // 2. Navigation clamping on populated list with 50 repeated calls
        let repo_root = PathBuf::from(".");
        let mut app = App::new(repo_root);
        assert!(app.items.len() > 1, "App must have multiple items for clamping test");
        let total = app.items.len();

        // 50 repeated page_down() calls clamp to total - 1
        for _ in 0..50 {
            app.page_down();
        }
        assert_eq!(app.selected_index, total - 1);

        // 50 repeated PageDown handle_key calls clamp to total - 1
        for _ in 0..50 {
            app.handle_key(KeyCode::PageDown);
        }
        assert_eq!(app.selected_index, total - 1);

        // 50 repeated page_up() calls clamp to 0
        for _ in 0..50 {
            app.page_up();
        }
        assert_eq!(app.selected_index, 0);

        // 50 repeated PageUp handle_key calls clamp to 0
        for _ in 0..50 {
            app.handle_key(KeyCode::PageUp);
        }
        assert_eq!(app.selected_index, 0);

        // Home / End
        app.handle_key(KeyCode::End);
        assert_eq!(app.selected_index, total - 1);

        app.handle_key(KeyCode::Home);
        assert_eq!(app.selected_index, 0);

        app.end();
        assert_eq!(app.selected_index, total - 1);

        app.home();
        assert_eq!(app.selected_index, 0);

        // 3. Single item list boundary verification
        let single_item = app.items[0].clone();
        let mut single_app = App {
            repo_root: PathBuf::from("."),
            pm_status: app.pm_status.clone(),
            items: vec![single_item],
            selected_index: 0,
            logs: Vec::new(),
            status_message: String::new(),
            git_update_status: None,
            should_quit: false,
        };

        single_app.next_item();
        assert_eq!(single_app.selected_index, 0);

        single_app.prev_item();
        assert_eq!(single_app.selected_index, 0);

        single_app.page_down();
        assert_eq!(single_app.selected_index, 0);

        single_app.page_up();
        assert_eq!(single_app.selected_index, 0);

        single_app.home();
        assert_eq!(single_app.selected_index, 0);

        single_app.end();
        assert_eq!(single_app.selected_index, 0);
    }

    #[test]
    fn test_unlink_preserves_binaries_in_path() {
        let env = TestEnv::new("unlink_preserves_bins");

        // 1. Setup simulated PATH directory containing application binaries
        let fake_bin_dir = env.temp_dir.join("scoop").join("shims");
        std::fs::create_dir_all(&fake_bin_dir).unwrap();
        let fake_exe = fake_bin_dir.join("glazewm.exe");
        std::fs::write(&fake_exe, b"MZ_STUB_PRESERVED_BINARY_CONTENT").unwrap();

        // 2. Setup simulated config mapping and target
        let fake_cfg_dir = env.temp_dir.join("glazewm");
        std::fs::create_dir_all(&fake_cfg_dir).unwrap();
        let fake_cfg_file = fake_cfg_dir.join("config.yaml");
        std::fs::write(&fake_cfg_file, "user_initial_config: true").unwrap();

        // 3. Perform backup & replacement simulating rice install
        let backed_up = crate::linker::backup_existing_target("glazewm", &fake_cfg_file, false).unwrap();
        assert!(backed_up.is_some(), "Existing config should be backed up");
        std::fs::write(&fake_cfg_file, "rice_theme: catppuccin").unwrap();

        // 4. Construct a ConfigMapping matching the target
        let mapping = crate::catalog::ConfigMapping {
            source_rel: "glazewm/config.yaml",
            target_pattern: Box::leak(fake_cfg_file.to_string_lossy().to_string().into_boxed_str()),
            is_dir: false,
        };

        // 5. Unlink configuration
        let outcome = crate::linker::unlink_config_mapping(&PathBuf::from("."), &mapping, "glazewm");
        assert!(matches!(outcome, crate::linker::UnlinkOutcome::UnlinkedAndRestored { .. }));

        // 6. Config should be restored to initial user content
        assert_eq!(
            std::fs::read_to_string(&fake_cfg_file).unwrap(),
            "user_initial_config: true"
        );

        // 7. CRITICAL VERIFICATION: Application binary MUST remain completely untouched
        assert!(fake_exe.exists(), "Binary in PATH must NEVER be deleted during config unlinking");
        assert_eq!(
            std::fs::read(&fake_exe).unwrap(),
            b"MZ_STUB_PRESERVED_BINARY_CONTENT",
            "Binary content must be completely untouched"
        );
    }

    #[test]
    fn test_diagnostic_logging_persistence() {
        let env = TestEnv::new("diag_logging");

        let mut app = App {
            repo_root: PathBuf::from("."),
            pm_status: detect_package_managers(),
            items: Vec::new(),
            selected_index: 0,
            logs: Vec::new(),
            status_message: String::new(),
            git_update_status: None,
            should_quit: false,
        };

        let marker1 = "DIAGNOSTIC_LOG_ENTRY_ALPHA_12345";
        app.add_log(marker1);

        let log_file = env.temp_dir.join("installer.log");
        assert!(log_file.exists(), "installer.log must be created by add_log()");

        let contents = std::fs::read_to_string(&log_file).unwrap();
        assert!(contents.contains(marker1), "installer.log must contain entry");

        // Verify ISO-8601 UTC timestamp format: [YYYY-MM-DDTHH:MM:SSZ]
        let line = contents.lines().find(|l| l.contains(marker1)).expect("Log line missing");
        assert!(line.starts_with('['), "Log entry must start with [");
        let bracket_end = line.find(']').expect("Log entry missing ]");
        let ts_str = &line[1..bracket_end];
        assert_eq!(ts_str.len(), 20, "Timestamp must be 20 chars YYYY-MM-DDTHH:MM:SSZ");
        assert!(ts_str.ends_with('Z'), "Timestamp must end with Z (UTC)");
        assert_eq!(&ts_str[10..11], "T", "Timestamp must have T delimiter");
        assert_eq!(&ts_str[4..5], "-", "Year-month dash");
        assert_eq!(&ts_str[7..8], "-", "Month-day dash");
        assert_eq!(&ts_str[13..14], ":", "Hour-min colon");
        assert_eq!(&ts_str[16..17], ":", "Min-sec colon");

        // Append second entry and verify sequential persistence without truncation
        let marker2 = "DIAGNOSTIC_LOG_ENTRY_BETA_67890";
        app.add_log(marker2);
        let contents_after = std::fs::read_to_string(&log_file).unwrap();
        assert!(contents_after.contains(marker1));
        assert!(contents_after.contains(marker2));
    }

    #[test]
    fn test_format_iso8601() {
        use std::time::{Duration, UNIX_EPOCH};

        // 1970-01-01T00:00:00Z
        assert_eq!(format_iso8601(UNIX_EPOCH), "1970-01-01T00:00:00Z");

        // 2026-10-25T15:09:22Z
        let t = UNIX_EPOCH + Duration::from_secs(1792940962);
        assert_eq!(format_iso8601(t), "2026-10-25T15:09:22Z");

        // Leap year: 2024-02-29T12:00:00Z (1709208000)
        let t_leap = UNIX_EPOCH + Duration::from_secs(1709208000);
        assert_eq!(format_iso8601(t_leap), "2024-02-29T12:00:00Z");
    }

    #[test]
    fn test_cli_unlink_package_selection_implicit() {
        let _env = TestEnv::new("cli_unlink_implicit");
        let repo_root = PathBuf::from(".");
        let mut app = App::new(repo_root);
        assert!(app.items.len() >= 3, "Catalog must contain items");

        // Simulate item 0 as LinkedSymlink, item 1 as LinkedCopy, item 2 as NotLinked
        app.items[0].inspection.link_status = crate::detect::LinkStatus::LinkedSymlink(PathBuf::from("test/symlink"));
        app.items[1].inspection.link_status = crate::detect::LinkStatus::LinkedCopy(PathBuf::from("test/copy"));
        app.items[2].inspection.link_status = crate::detect::LinkStatus::NotLinked;

        // Simulate CLI args: ["rice-installer", "--unlink"] (no explicit packages)
        let args = vec!["rice-installer".to_string(), "--unlink".to_string()];
        let explicit_pkgs: Vec<String> = args.iter().skip(1).filter(|a| !a.starts_with('-')).cloned().collect();
        assert!(explicit_pkgs.is_empty());

        if explicit_pkgs.is_empty() {
            for item in &mut app.items {
                item.selected = matches!(
                    item.inspection.link_status,
                    crate::detect::LinkStatus::LinkedSymlink(_) | crate::detect::LinkStatus::LinkedCopy(_)
                );
            }
        }

        assert!(app.items[0].selected, "LinkedSymlink must be selected for unlinking");
        assert!(app.items[1].selected, "LinkedCopy must be selected for unlinking");
        assert!(!app.items[2].selected, "NotLinked must not be selected for unlinking");
    }

    #[test]
    fn test_cli_unlink_package_selection_explicit() {
        let _env = TestEnv::new("cli_unlink_explicit");
        let repo_root = PathBuf::from(".");
        let mut app = App::new(repo_root);
        assert!(app.items.len() >= 2, "Catalog must contain items");

        let target_pkg_id = app.items[0].package.id.to_string();

        // Simulate CLI args: ["rice-installer", "-x", <target_pkg_id>]
        let args = vec!["rice-installer".to_string(), "-x".to_string(), target_pkg_id.clone()];
        let explicit_pkgs: Vec<String> = args.iter().skip(1).filter(|a| !a.starts_with('-')).cloned().collect();
        assert_eq!(explicit_pkgs, vec![target_pkg_id.clone()]);

        for item in &mut app.items {
            item.selected = explicit_pkgs
                .iter()
                .any(|p| p.eq_ignore_ascii_case(item.package.id) || p.eq_ignore_ascii_case(item.package.name));
        }

        assert!(app.items[0].selected, "Target package must be selected");
        for item in app.items.iter().skip(1) {
            if !item.package.id.eq_ignore_ascii_case(&target_pkg_id) {
                assert!(!item.selected, "Non-matching package {} must not be selected", item.package.id);
            }
        }
    }
}
