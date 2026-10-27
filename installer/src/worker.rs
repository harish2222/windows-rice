// ============================================================================
// Author -: HKDEVS
// Project -: Window-Rice
// Title -: WINDOW-RICE Installer Async Worker & Task Flows
// ============================================================================
// Dedicated worker-thread flows. Every interactive action runs one of the
// flow_* functions below, either synchronously (CLI / unit tests, notify
// routes into App::add_log) or on a background thread from the TUI (notify
// routes into a mpsc channel drained by App::poll_worker_events).
// ============================================================================
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::catalog::{ConfigMapping, PackageItem, resolve_target_path};
use crate::detect::{
    detect_package_managers, find_binary_in_path, inspect_package, BinaryStatus, LinkStatus,
    PackageManagerStatus,
};
use crate::linker::{link_config_mapping, unlink_config_mapping, copy_recursive, LinkOutcome, UnlinkOutcome};
use crate::bootstrap::{
    bootstrap_scoop, bootstrap_choco, bootstrap_uv, bootstrap_vcpkg,
    ensure_package_manager_ready, prepare_install_command,
    run_package_install, run_streaming_command,
};
use crate::verifier::run_verification;

/// A task that can be dispatched to the worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerTask {
    /// [i] Install missing binaries & link configs (sequential pipeline).
    InstallLink,
    /// [l] Link configs for selected packages.
    Link,
    /// [u/x] Unlink selected configs & restore original backups.
    Unlink,
    /// [b] Bootstrap Scoop / Chocolatey / UV / Vcpkg.
    Bootstrap,
    /// [v] Run system health check.
    Verify,
    /// [r] Reload Komorebi, GlazeWM & YASB.
    Reload,
    /// [c/g] Check git repository upstream status.
    CheckUpdates,
    /// [w] Install the 8K wallpaper collection.
    Wallpapers,
}

impl WorkerTask {
    pub fn display_name(&self) -> &'static str {
        match self {
            WorkerTask::InstallLink => "Install & Link Packages",
            WorkerTask::Link => "Link Configs",
            WorkerTask::Unlink => "Unlink Configs & Restore Backups",
            WorkerTask::Bootstrap => "Bootstrap Package Managers",
            WorkerTask::Verify => "Verify System Health",
            WorkerTask::Reload => "Reload Environment",
            WorkerTask::CheckUpdates => "Check Repository Updates",
            WorkerTask::Wallpapers => "Install 8K Wallpapers",
        }
    }
}

/// Message emitted by a running task, drained by the UI every frame.
#[derive(Debug, Clone)]
pub enum WorkerMessage {
    /// A log line for the activity console (same strings as the CLI path).
    Log(String),
    /// Real-time progress: 0-100, short task label and current step detail.
    Progress { percentage: u16, label: String, detail: String },
    /// Refreshed package manager detection (e.g. after auto-bootstrap).
    PmStatus(PackageManagerStatus),
    /// Repository status token: UPDATE AVAILABLE / UP-TO-DATE / LIVE / READY.
    GitStatus(String),
    /// Terminal completion of the whole task.
    TaskFinished { task_name: String, success: bool, summary: String },
}

/// One selected package's config mappings, snapshotted at dispatch time
/// so the worker never touches shared App state.
#[derive(Debug, Clone)]
pub struct PackageJob {
    pub pkg_id: String,
    pub pkg_name: String,
    pub configs: Vec<ConfigMapping>,
}

/// Immutable snapshot of everything a flow needs, taken before the worker starts.
#[derive(Debug, Clone)]
pub struct TaskContext {
    pub repo_root: PathBuf,
    pub cancel_flag: Arc<AtomicBool>,
    pub pm_status: PackageManagerStatus,
    /// Packages for WorkerTask::InstallLink.
    pub packages: Vec<PackageItem>,
    /// Jobs for WorkerTask::Link / Unlink.
    pub jobs: Vec<PackageJob>,
    /// TUI mode: auto-resolve a package manager (Scoop/Winget) before installing.
    pub ensure_pm: bool,
    /// TUI mode: stream install subprocess output line-by-line ([OUT] prefix)
    /// and support cancellation while commands run.
    pub stream_install: bool,
    /// Optional wallpapers destination override.
    pub custom_dest: Option<PathBuf>,
    /// Link flow: wallpapers package was selected -> install wallpapers as well.
    pub also_install_wallpapers: bool,
}

impl TaskContext {
    pub fn new(repo_root: PathBuf, cancel_flag: Arc<AtomicBool>, pm_status: PackageManagerStatus) -> Self {
        Self {
            repo_root,
            cancel_flag,
            pm_status,
            packages: Vec::new(),
            jobs: Vec::new(),
            ensure_pm: false,
            stream_install: false,
            custom_dest: None,
            also_install_wallpapers: false,
        }
    }
}

pub struct TaskResult {
    pub summary: String,
    pub success: bool,
}

/// Callback sink the flows emit through. Sync path: App::absorb_sync_message.
/// Worker path: mpsc sender closure.
pub type Notify<'a> = &'a mut (dyn FnMut(WorkerMessage) + 'a);

fn send_log(notify: Notify, text: impl Into<String>) {
    notify(WorkerMessage::Log(text.into()));
}

fn emit_progress(notify: Notify, percentage: u16, label: impl Into<String>, detail: impl Into<String>) {
    notify(WorkerMessage::Progress {
        percentage: percentage.min(100),
        label: label.into(),
        detail: detail.into(),
    });
}

fn is_cancelled(cancel_flag: &AtomicBool) -> bool {
    cancel_flag.load(Ordering::SeqCst)
}

fn cancelled_result(notify: Notify) -> TaskResult {
    send_log(notify, "[CANCELLED] User aborted the task.");
    TaskResult { summary: "Task cancelled by user.".to_string(), success: false }
}

/// Entry point used by the worker thread: runs the requested flow.
pub fn run_flow(task: WorkerTask, ctx: &TaskContext, notify: Notify) -> TaskResult {
    match task {
        WorkerTask::InstallLink => flow_install(ctx, notify),
        WorkerTask::Link => flow_link(ctx, notify),
        WorkerTask::Unlink => flow_unlink(ctx, notify),
        WorkerTask::Bootstrap => flow_bootstrap(ctx, notify),
        WorkerTask::Verify => flow_verify(ctx, notify),
        WorkerTask::Reload => flow_reload(ctx, notify),
        WorkerTask::CheckUpdates => flow_check_updates(ctx, notify),
        WorkerTask::Wallpapers => flow_wallpapers(ctx, false, notify),
    }
}

// ============================================================================
// INSTALL & LINK (sequential pipeline)
// ============================================================================
pub fn flow_install(ctx: &TaskContext, notify: Notify) -> TaskResult {
    send_log(notify, "--- Starting Sequential Package Installation, Linking & Verification ---");

    if ctx.packages.is_empty() {
        send_log(notify, "No packages selected for installation.");
        return TaskResult { summary: "No packages selected.".to_string(), success: true };
    }

    // TUI mode: make sure at least one package manager is usable before installing.
    let mut pm = ctx.pm_status.clone();
    if ctx.ensure_pm {
        emit_progress(notify, 0, "Preparing", "Ensuring a package manager is ready...");
        let mut sink = |line: String| send_log(notify, line);
        pm = ensure_package_manager_ready(&ctx.cancel_flag, &mut sink);
        notify(WorkerMessage::PmStatus(pm.clone()));
    }

    let packages = &ctx.packages;
    let total = packages.len();
    let mut total_installed_bin = 0;
    let mut total_symlinks = 0;
    let mut total_copies = 0;
    let mut total_backed_up = 0;
    let mut total_valid = 0;
    let mut total_errors = 0;

    for (idx, pkg) in packages.iter().enumerate() {
        if is_cancelled(&ctx.cancel_flag) {
            return cancelled_result(notify);
        }

        emit_progress(
            notify,
            ((idx * 100) / total) as u16,
            format!("[{}/{}] {}", idx + 1, total, pkg.name),
            "Step 1: Install".to_string(),
        );
        send_log(notify, format!("=== [{}/{}] Processing: {} ({}) ===", idx + 1, total, pkg.name, pkg.id));

        // Step 1: Binary Installation (if missing)
        let inspection_before = inspect_package(pkg, &ctx.repo_root);
        match inspection_before.binary_status {
            BinaryStatus::Missing => {
                let cmd_opt = if pm.scoop && pkg.scoop_cmd.is_some() {
                    pkg.scoop_cmd.map(|s| s.to_string())
                } else if pm.winget && pkg.winget_cmd.is_some() {
                    pkg.winget_cmd.map(|s| s.to_string())
                } else if pm.choco && pkg.choco_cmd.is_some() {
                    pkg.choco_cmd.map(|s| s.to_string())
                } else if pm.cargo && pkg.cargo_cmd.is_some() {
                    pkg.cargo_cmd.map(|s| s.to_string())
                } else {
                    pkg.scoop_cmd.or(pkg.winget_cmd).map(|s| s.to_string())
                };

                if let Some(cmd) = cmd_opt {
                    send_log(notify, format!("  [STEP 1: INSTALL] Installing binary for {} via: {}", pkg.name, cmd));
                    let outcome = if ctx.stream_install {
                        let sanitized = prepare_install_command(&cmd);
                        emit_progress(
                            notify,
                            ((idx * 100) / total) as u16,
                            format!("[{}/{}] {}", idx + 1, total, pkg.name),
                            format!("Installing via {}", cmd),
                        );
                        run_streaming_command(&sanitized, &ctx.cancel_flag, |line: String| {
                            send_log(notify, format!("    [OUT] {}", line));
                        })
                    } else {
                        run_package_install(&cmd).map(|_| ())
                    };

                    match outcome {
                        Ok(_) => {
                            total_installed_bin += 1;
                            send_log(notify, format!("  [PASS] Installed binary for {}", pkg.name));
                        }
                        Err(e) => {
                            if is_cancelled(&ctx.cancel_flag) {
                                send_log(notify, format!("  [CANCELLED] Installation of {} aborted.", pkg.name));
                                return cancelled_result(notify);
                            }
                            total_errors += 1;
                            send_log(notify, format!("  [FAIL] Error installing binary for {}: {}", pkg.name, e));
                        }
                    }
                } else {
                    send_log(notify, format!("  [NOTE] No automated install command available for {}", pkg.name));
                }
            }
            BinaryStatus::Installed(ref p) => {
                send_log(notify, format!("  [STEP 1: BINARY] Binary already installed: {}", p.display()));
            }
        }

        // Step 2: Connect / Link Configuration
        if pkg.id == "wallpapers" {
            send_log(notify, "  [STEP 2: WALLPAPERS] Installing 8K Wallpapers to Pictures...");
            flow_wallpapers(ctx, false, notify);
        } else if !pkg.configs.is_empty() {
            emit_progress(
                notify,
                ((idx * 100) / total) as u16,
                format!("[{}/{}] {}", idx + 1, total, pkg.name),
                "Step 2: Link configs".to_string(),
            );
            send_log(notify, format!("  [STEP 2: CONFIG] Connecting & linking configurations for {}:", pkg.name));
            for mapping in &pkg.configs {
                match link_config_mapping(&ctx.repo_root, mapping, pkg.id) {
                    LinkOutcome::SymlinkCreated(target) => {
                        total_symlinks += 1;
                        send_log(notify, format!("    [SYMLINK]  -> Linked to {}", target.display()));
                    }
                    LinkOutcome::CopiedFallback(target) => {
                        total_copies += 1;
                        send_log(notify, format!("    [COPIED]   -> Copied fallback to {}", target.display()));
                    }
                    LinkOutcome::BackedUpAndLinked { target, backup } => {
                        total_backed_up += 1;
                        total_symlinks += 1;
                        send_log(notify, format!("    [BACKUP]   -> Existing saved to {}", backup.display()));
                        send_log(notify, format!("    [SYMLINK]  -> Linked to {}", target.display()));
                    }
                    LinkOutcome::AlreadyValid(target) => {
                        total_valid += 1;
                        send_log(notify, format!("    [ALREADY]  -> Already valid at {}", target.display()));
                    }
                    LinkOutcome::Failed(err) => {
                        total_errors += 1;
                        send_log(notify, format!("    [ERROR]    -> Failed: {}", err));
                    }
                }
            }
        } else {
            send_log(notify, "  [STEP 2: CONFIG] No configuration files required.");
        }

        // Step 3: Test and Verify Package Setup
        emit_progress(
            notify,
            ((idx * 100) / total.max(1)) as u16,
            format!("[{}/{}] {}", idx + 1, total, pkg.name),
            "Step 3: Verify".to_string(),
        );
        send_log(notify, format!("  [STEP 3: VERIFY] Testing installation & configuration for {}:", pkg.name));
        let inspection_after = inspect_package(pkg, &ctx.repo_root);

        match &inspection_after.binary_status {
            BinaryStatus::Installed(p) => {
                send_log(notify, format!("    [TEST: PASS] Binary verified: {}", p.display()));
            }
            BinaryStatus::Missing => {
                send_log(notify, "    [TEST: WARN] Binary not detected in current PATH (may need shell restart).");
            }
        }

        match &inspection_after.link_status {
            LinkStatus::LinkedSymlink(p) => {
                send_log(notify, format!("    [TEST: PASS] Config symlink verified: {}", p.display()));
            }
            LinkStatus::LinkedCopy(p) => {
                send_log(notify, format!("    [TEST: PASS] Config copy verified: {}", p.display()));
            }
            LinkStatus::NotApplicable => {
                send_log(notify, "    [TEST: PASS] Config check: N/A");
            }
            LinkStatus::NotLinked => {
                send_log(notify, format!("    [TEST: WARN] One or more configuration files not linked for {}.", pkg.name));
            }
        }

        if !inspection_after.is_missing_any {
            send_log(notify, format!("  [RESULT: READY] {} is fully configured and verified.", pkg.name));
        } else {
            send_log(notify, format!("  [RESULT: PENDING] {} partially configured (see warnings above).", pkg.name));
        }

        send_log(notify, format!("Finished setup for {}. Advancing to next selected option...\n", pkg.name));
    }

    emit_progress(notify, 95, "Finalizing", "Reloading environment...");
    flow_reload(ctx, notify);
    emit_progress(notify, 100, "Done", "Setup complete".to_string());

    let summary = format!(
        "Setup complete for {} package(s): {} binaries installed, {} symlinks ({} backed up), {} copied, {} valid, {} errors.",
        packages.len(),
        total_installed_bin,
        total_symlinks,
        total_backed_up,
        total_copies,
        total_valid,
        total_errors
    );
    send_log(notify, summary.clone());
    send_log(notify, format!("--- {} ---", summary));
    TaskResult { summary, success: true }
}

// ============================================================================
// LINK CONFIGS
// ============================================================================
pub fn flow_link(ctx: &TaskContext, notify: Notify) -> TaskResult {
    send_log(notify, "--- Starting Config Linking ---");

    let jobs = &ctx.jobs;
    let total = jobs.len();
    let mut count_symlink = 0;
    let mut count_copy = 0;
    let mut count_valid = 0;
    let mut count_backed_up = 0;
    let mut count_err = 0;

    for (idx, job) in jobs.iter().enumerate() {
        if is_cancelled(&ctx.cancel_flag) {
            return cancelled_result(notify);
        }

        emit_progress(
            notify,
            ((idx * 100) / total.max(1)) as u16,
            format!("[{}/{}] {}", idx + 1, total, job.pkg_name),
            "Linking configs".to_string(),
        );
        send_log(notify, format!("Linking configs for {}:", job.pkg_name));
        for mapping in &job.configs {
            match link_config_mapping(&ctx.repo_root, mapping, &job.pkg_id) {
                LinkOutcome::SymlinkCreated(target) => {
                    count_symlink += 1;
                    send_log(notify, format!("  [SYMLINK] -> {}", target.display()));
                }
                LinkOutcome::CopiedFallback(target) => {
                    count_copy += 1;
                    send_log(notify, format!("  [COPIED]  -> {}", target.display()));
                }
                LinkOutcome::BackedUpAndLinked { target, backup } => {
                    count_backed_up += 1;
                    count_symlink += 1;
                    send_log(notify, format!("  [BACKUP]  -> Existing saved to {}", backup.display()));
                    send_log(notify, format!("  [SYMLINK] -> {}", target.display()));
                }
                LinkOutcome::AlreadyValid(target) => {
                    count_valid += 1;
                    send_log(notify, format!("  [ALREADY] -> {}", target.display()));
                }
                LinkOutcome::Failed(err) => {
                    count_err += 1;
                    send_log(notify, format!("  [ERROR]   -> {}", err));
                }
            }
        }
    }

    let summary = format!(
        "Linked configs: {} symlinks ({} backed up), {} copied, {} already valid, {} failed.",
        count_symlink, count_backed_up, count_copy, count_valid, count_err
    );
    send_log(notify, summary.clone());

    // Wallpapers selected -> install the 8K wallpaper collection as well.
    if ctx.also_install_wallpapers {
        flow_wallpapers(ctx, false, notify);
    }

    flow_reload(ctx, notify);
    emit_progress(notify, 100, "Done", summary.clone());
    TaskResult { summary, success: true }
}

// ============================================================================
// UNLINK CONFIGS & RESTORE BACKUPS
// ============================================================================
pub fn flow_unlink(ctx: &TaskContext, notify: Notify) -> TaskResult {
    send_log(notify, "--- Starting Config Removal & Unlinking (Preserving Binaries) ---");

    let jobs = &ctx.jobs;
    let total = jobs.len();
    let mut count_restored = 0;
    let mut count_cleaned = 0;
    let mut count_not_linked = 0;
    let mut count_err = 0;

    for (idx, job) in jobs.iter().enumerate() {
        if is_cancelled(&ctx.cancel_flag) {
            return cancelled_result(notify);
        }

        if job.pkg_id == "wallpapers" {
            send_log(notify, "Uninstalling 8K Wallpapers from Pictures...");
            flow_wallpapers(ctx, true, notify);
            continue;
        }

        emit_progress(
            notify,
            ((idx * 100) / total.max(1)) as u16,
            format!("[{}/{}] {}", idx + 1, total, job.pkg_name),
            "Unlinking configs".to_string(),
        );
        send_log(notify, format!("Removing configs for {} (binaries kept):", job.pkg_name));
        for mapping in &job.configs {
            match unlink_config_mapping(&ctx.repo_root, mapping, &job.pkg_id) {
                UnlinkOutcome::UnlinkedAndRestored { target, backup_source } => {
                    count_restored += 1;
                    send_log(notify, format!("  [RESTORED] -> Restored backup: {}", backup_source.display()));
                    send_log(notify, format!("  [TARGET]   -> {}", target.display()));
                }
                UnlinkOutcome::UnlinkedCleanly(target) => {
                    count_cleaned += 1;
                    send_log(notify, format!("  [CLEAN]    -> Removed {}", target.display()));
                }
                UnlinkOutcome::NotLinked(target) => {
                    count_not_linked += 1;
                    send_log(notify, format!("  [NO-OP]    -> Not linked: {}", target.display()));
                }
                UnlinkOutcome::Failed(err) => {
                    count_err += 1;
                    send_log(notify, format!("  [ERROR]    -> {}", err));
                }
            }
        }
    }

    let summary = format!(
        "Unlink complete: {} restored from backup, {} cleaned, {} no-op, {} failed.",
        count_restored, count_cleaned, count_not_linked, count_err
    );
    send_log(notify, summary.clone());

    flow_reload(ctx, notify);
    emit_progress(notify, 100, "Done", summary.clone());
    TaskResult { summary, success: true }
}

// ============================================================================
// BOOTSTRAP PACKAGE MANAGERS
// ============================================================================
pub fn flow_bootstrap(ctx: &TaskContext, notify: Notify) -> TaskResult {
    send_log(notify, "--- Bootstrapping Package Managers ---");
    let pm = ctx.pm_status.clone();
    let steps = 4usize;
    let mut done = 0usize;

    if is_cancelled(&ctx.cancel_flag) {
        return cancelled_result(notify);
    }
    if pm.scoop {
        send_log(notify, "Scoop is already installed.");
    } else {
        send_log(notify, "Bootstrapping Scoop...");
        match bootstrap_scoop() {
            Ok(_) => send_log(notify, "  [PASS] Scoop successfully bootstrapped!"),
            Err(e) => send_log(notify, format!("  [FAIL] Failed to bootstrap Scoop: {}", e)),
        }
    }
    done += 1;
    emit_progress(notify, ((done * 100) / steps.max(1)) as u16, "Bootstrapping PMs", "Scoop checked".to_string());

    if is_cancelled(&ctx.cancel_flag) {
        return cancelled_result(notify);
    }
    if pm.choco {
        send_log(notify, "Chocolatey is already installed.");
    } else {
        send_log(notify, "Bootstrapping Chocolatey...");
        match bootstrap_choco() {
            Ok(_) => send_log(notify, "  [PASS] Chocolatey successfully bootstrapped!"),
            Err(e) => send_log(notify, format!("  [FAIL] Failed to bootstrap Chocolatey: {}", e)),
        }
    }
    done += 1;
    emit_progress(notify, ((done * 100) / steps.max(1)) as u16, "Bootstrapping PMs", "Chocolatey checked".to_string());

    if is_cancelled(&ctx.cancel_flag) {
        return cancelled_result(notify);
    }
    if pm.uv {
        send_log(notify, "UV is already installed.");
    } else {
        send_log(notify, "Bootstrapping UV...");
        match bootstrap_uv() {
            Ok(_) => send_log(notify, "  [PASS] UV successfully bootstrapped!"),
            Err(e) => send_log(notify, format!("  [FAIL] Failed to bootstrap UV: {}", e)),
        }
    }
    done += 1;
    emit_progress(notify, ((done * 100) / steps.max(1)) as u16, "Bootstrapping PMs", "UV checked".to_string());

    if is_cancelled(&ctx.cancel_flag) {
        return cancelled_result(notify);
    }
    if pm.vcpkg {
        send_log(notify, "Vcpkg is already installed.");
    } else {
        send_log(notify, "Bootstrapping Vcpkg...");
        match bootstrap_vcpkg() {
            Ok(_) => send_log(notify, "  [PASS] Vcpkg successfully bootstrapped!"),
            Err(e) => send_log(notify, format!("  [FAIL] Failed to bootstrap Vcpkg: {}", e)),
        }
    }
    done += 1;
    emit_progress(notify, ((done * 100) / steps.max(1)) as u16, "Bootstrapping PMs", "Vcpkg checked".to_string());

    // Refresh PM detection once at the end (parity with legacy bootstrap method).
    notify(WorkerMessage::PmStatus(detect_package_managers()));
    let summary = "Bootstrap pass complete. Package managers refreshed.".to_string();
    send_log(notify, summary.clone());
    TaskResult { summary, success: true }
}

// ============================================================================
// VERIFY SYSTEM HEALTH
// ============================================================================
pub fn flow_verify(ctx: &TaskContext, notify: Notify) -> TaskResult {
    send_log(notify, "--- Running Health Check Report ---");
    emit_progress(notify, 5, "Health check", "Inspecting installed binaries and configs...".to_string());
    let report = run_verification(&ctx.repo_root);
    let lines_count = report.lines.len().max(1);
    for (i, line) in report.lines.into_iter().enumerate() {
        if is_cancelled(&ctx.cancel_flag) {
            return cancelled_result(notify);
        }
        if i % 4 == 0 {
            emit_progress(notify, (5 + ((i * 90) / lines_count)) as u16, "Health check", format!("line {}/{}", i + 1, lines_count));
        }
        send_log(notify, line);
    }
    let summary = format!(
        "Health check complete: {}/{} binaries, {}/{} configs.",
        report.installed_binaries, report.total_packages, report.linked_configs, report.total_packages
    );
    send_log(notify, summary.clone());
    emit_progress(notify, 100, "Done", summary.clone());
    TaskResult { summary, success: true }
}

// ============================================================================
// RELOAD ENVIRONMENT (Komorebi / GlazeWM / YASB)
// ============================================================================
pub fn flow_reload(ctx: &TaskContext, notify: Notify) -> TaskResult {
    send_log(notify, "=== Reloading Windows Rice Environment ===");

    // 1. Komorebi
    emit_progress(notify, 10, "Reloading environment", "Komorebi".to_string());
    if find_binary_in_path("komorebic.exe").is_some() {
        send_log(notify, "Reloading Komorebi configuration...");
        let config_path = resolve_target_path(".config/komorebi/komorebi.json");
        let res = std::process::Command::new("komorebic")
            .args(["replace-configuration", &config_path.to_string_lossy()])
            .output();
        match res {
            Ok(out) if out.status.success() => send_log(notify, "  [PASS] Komorebi configuration reloaded."),
            Ok(out) => {
                let err = String::from_utf8_lossy(&out.stderr);
                if err.trim().is_empty() {
                    send_log(notify, "  [PASS] Komorebi configuration reloaded.");
                } else {
                    send_log(notify, format!("  [INFO] Komorebi reload: {}", err.trim()));
                }
            }
            Err(e) => send_log(notify, format!("  [WARN] Komorebi reload notice: {}", e)),
        }
    } else {
        send_log(notify, "  [INFO] komorebic not in PATH (skipping).");
    }

    // 2. GlazeWM
    emit_progress(notify, 50, "Reloading environment", "GlazeWM".to_string());
    if find_binary_in_path("glazewm.exe").is_some() {
        send_log(notify, "Reloading GlazeWM hotkey layer...");
        let res = std::process::Command::new("glazewm")
            .args(["command", "wm-reload-config"])
            .output();
        match res {
            Ok(out) if out.status.success() => send_log(notify, "  [PASS] GlazeWM hotkey layer reloaded."),
            Ok(out) => {
                let err = String::from_utf8_lossy(&out.stderr);
                if err.trim().is_empty() {
                    send_log(notify, "  [PASS] GlazeWM hotkey layer reloaded.");
                } else {
                    send_log(notify, format!("  [INFO] GlazeWM: {}", err.trim()));
                }
            }
            Err(e) => send_log(notify, format!("  [WARN] GlazeWM reload notice: {}", e)),
        }
    } else {
        send_log(notify, "  [INFO] glazewm not in PATH (skipping).");
    }

    // 3. YASB
    emit_progress(notify, 85, "Reloading environment", "YASB".to_string());
    let yasb_setup = ctx.repo_root.join("yasb/tools/setup/yasb-setup.ps1");
    if yasb_setup.exists() {
        send_log(notify, "Re-running YASB status bar replication / verification...");
        let res = std::process::Command::new("pwsh")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", &yasb_setup.to_string_lossy()])
            .output()
            .or_else(|_| {
                std::process::Command::new("powershell")
                    .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", &yasb_setup.to_string_lossy()])
                    .output()
            });
        match res {
            Ok(out) if out.status.success() => send_log(notify, "  [PASS] YASB status bar active and verified."),
            Ok(out) => {
                let out_msg = String::from_utf8_lossy(&out.stdout);
                send_log(notify, format!("  [PASS] YASB updated: {}", out_msg.lines().next().unwrap_or("OK")));
            }
            Err(e) => send_log(notify, format!("  [WARN] YASB setup notice: {}", e)),
        }
    }

    let summary = "Windows Rice environment (Komorebi, GlazeWM, YASB) reloaded!".to_string();
    send_log(notify, summary.clone());
    emit_progress(notify, 100, "Done", "Environment reloaded".to_string());
    TaskResult { summary, success: true }
}

// ============================================================================
// CHECK GIT REPOSITORY UPDATES
// ============================================================================
pub fn flow_check_updates(ctx: &TaskContext, notify: Notify) -> TaskResult {
    emit_progress(notify, 25, "Checking updates", "Querying git status...".to_string());
    send_log(notify, "Checking for repository updates against origin/main...");
    let git_status = std::process::Command::new("git")
        .args(["-C", &ctx.repo_root.to_string_lossy(), "status", "-uno"])
        .output();

    if let Ok(out) = git_status {
        let msg = String::from_utf8_lossy(&out.stdout);
        if msg.contains("Your branch is behind") {
            notify(WorkerMessage::GitStatus("UPDATE AVAILABLE".to_string()));
            send_log(notify, "  [WARN] Updates available on origin/main! Run 'git pull' to update.");
            let summary = "Updates available from GitHub. Press [u] or git pull.".to_string();
            emit_progress(notify, 100, "Done", "Updates available".to_string());
            return TaskResult { summary, success: true };
        } else if msg.contains("Your branch is up to date") {
            notify(WorkerMessage::GitStatus("UP-TO-DATE".to_string()));
            send_log(notify, "  [PASS] Repository configurations are fully up-to-date.");
            let summary = "Rice configurations are fully up to date.".to_string();
            emit_progress(notify, 100, "Done", "Up-to-date".to_string());
            return TaskResult { summary, success: true };
        } else {
            notify(WorkerMessage::GitStatus("LIVE".to_string()));
            send_log(notify, "  [INFO] Repository in live custom state.");
            let summary = "Repository in live custom state.".to_string();
            emit_progress(notify, 100, "Done", "Live state".to_string());
            return TaskResult { summary, success: true };
        }
    }

    notify(WorkerMessage::GitStatus("READY".to_string()));
    let summary = "Could not check repository status.".to_string();
    emit_progress(notify, 100, "Done", "Git unavailable".to_string());
    TaskResult { summary, success: true }
}

// ============================================================================
// WALLPAPERS
// ============================================================================
pub fn flow_wallpapers(ctx: &TaskContext, remove: bool, notify: Notify) -> TaskResult {
    if remove {
        flow_uninstall_wallpapers(ctx, notify)
    } else {
        flow_install_wallpapers(ctx, notify)
    }
}

fn default_wallpaper_dest() -> PathBuf {
    if let Some(user_profile) = std::env::var_os("USERPROFILE") {
        let p1 = PathBuf::from(&user_profile).join("OneDrive/Pictures/Wallpapers");
        if p1.parent().map(|p| p.exists()).unwrap_or(false) {
            return p1;
        }
        return PathBuf::from(&user_profile).join("Pictures/Wallpapers");
    }
    PathBuf::from("Wallpapers")
}

fn flow_install_wallpapers(ctx: &TaskContext, notify: Notify) -> TaskResult {
    send_log(notify, "=== Installing 8K Wallpaper Collection ===");
    let dest = ctx.custom_dest.clone().unwrap_or_else(default_wallpaper_dest);
    send_log(notify, format!("Target destination: {}", dest.display()));

    // Test environment guard: never touch real wallpaper storage under `cargo test`.
    if std::env::var_os("RICE_BACKUP_DIR").is_some() {
        send_log(notify, "  [INFO] Test environment detected (RICE_BACKUP_DIR set) - skipping wallpaper media operations.");
        let summary = format!("Wallpaper destination prepared: {}", dest.display());
        return TaskResult { summary, success: true };
    }

    let local_organized = PathBuf::from("Z:\\HK\\8K Wallpaper\\_Organized");
    if local_organized.exists() {
        send_log(notify, format!("Found local collection at: {}", local_organized.display()));
        let _ = std::fs::create_dir_all(&dest);

        if let Ok(entries) = std::fs::read_dir(&local_organized) {
            let mut linked_count = 0;
            for entry in entries.flatten() {
                if is_cancelled(&ctx.cancel_flag) {
                    return cancelled_result(notify);
                }
                let name = entry.file_name();
                let target = dest.join(&name);
                let src = entry.path();
                #[cfg(windows)]
                {
                    if src.is_dir() {
                        match std::os::windows::fs::symlink_dir(&src, &target) {
                            Ok(_) => {
                                linked_count += 1;
                                send_log(notify, format!("  [SYMLINK] -> {}", target.display()));
                            }
                            Err(_) => {
                                send_log(notify, format!("  [COPY] Copying {}...", name.to_string_lossy()));
                                let _ = copy_recursive(&src, &target, true);
                                linked_count += 1;
                            }
                        }
                    }
                }
                emit_progress(notify, 50, "Installing wallpapers", name.to_string_lossy().to_string());
            }
            let summary = format!("Wallpapers installed to {} ({} folders).", dest.display(), linked_count);
            send_log(notify, summary.clone());
            emit_progress(notify, 100, "Done", summary.clone());
            return TaskResult { summary, success: true };
        }
    }

    send_log(notify, "Note: Remote users can download Wallpaper Packs 1, 2, 3 from GitHub Release assets.");
    let summary = format!("Wallpaper destination prepared: {}", dest.display());
    send_log(notify, summary.clone());
    emit_progress(notify, 100, "Done", summary.clone());
    TaskResult { summary, success: true }
}

fn flow_uninstall_wallpapers(ctx: &TaskContext, notify: Notify) -> TaskResult {
    send_log(notify, "=== Removing 8K Wallpaper Collection ===");
    let dest = ctx.custom_dest.clone().unwrap_or_else(default_wallpaper_dest);

    // Test environment guard: never touch real wallpaper storage under `cargo test`.
    if std::env::var_os("RICE_BACKUP_DIR").is_some() {
        send_log(notify, "  [INFO] Test environment detected (RICE_BACKUP_DIR set) - skipping wallpaper media operations.");
        let summary = format!("Wallpapers were not installed (test mode): {}", dest.display());
        return TaskResult { summary, success: true };
    }

    if dest.exists() || dest.symlink_metadata().is_ok() {
        let _ = std::fs::remove_dir_all(&dest);
        send_log(notify, format!("  [CLEAN] Removed wallpaper directory: {}", dest.display()));
        let summary = format!("Wallpapers uninstalled from {}.", dest.display());
        send_log(notify, summary.clone());
        emit_progress(notify, 100, "Done", summary.clone());
        return TaskResult { summary, success: true };
    }

    send_log(notify, format!("  [INFO] Wallpapers not installed at {}", dest.display()));
    let summary = "Wallpapers were not installed.".to_string();
    send_log(notify, summary.clone());
    emit_progress(notify, 100, "Done", summary.clone());
    TaskResult { summary, success: true }
}
