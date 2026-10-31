// ============================================================================
// Author -: HKDEVS
// Project -: Window-Rice
// Title -: WINDOW-RICE Installer Application State
// ============================================================================
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Mutex;
use std::sync::Arc;
use crate::catalog::{get_catalog, PackageItem};
use crate::detect::{detect_package_managers, inspect_package, ItemInspection, PackageManagerStatus};
use crate::worker::{self, PackageJob, TaskContext, WorkerMessage, WorkerTask};
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
    /// [0] Full-revoke confirmation modal open (y/Y confirms, n/N/Esc cancels).
    pub confirm_uninstall: bool,
    // === Async worker state (TUI mode; CLI/unit tests stay synchronous) ===
    pub async_worker: bool,
    pub is_busy: bool,
    pub active_task_name: Option<String>,
    pub progress_percentage: u16,
    pub progress_label: String,
    pub progress_detail: String,
    pub log_scroll_offset: usize,
    pub cancel_flag: Arc<AtomicBool>,
    worker_rx: Option<Receiver<WorkerMessage>>,
    worker_handle: Option<std::thread::JoinHandle<()>>,
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
            async_worker: false,
            confirm_uninstall: false,
            is_busy: false,
            active_task_name: None,
            progress_percentage: 0,
            progress_label: "Ready".to_string(),
            progress_detail: String::new(),
            log_scroll_offset: 0,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            worker_rx: None,
            worker_handle: None,
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
        // Full-revoke confirmation modal: absorbs everything until answered.
        if self.confirm_uninstall {
            match code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.confirm_uninstall = false;
                    self.uninstall_everything();
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.confirm_uninstall = false;
                    self.status_message =
                        "Full revoke cancelled. Nothing was modified.".to_string();
                }
                KeyCode::Esc => {
                    self.confirm_uninstall = false;
                    self.status_message =
                        "Full revoke cancelled. Nothing was modified.".to_string();
                }
                _ => {}
            }
            return self.should_quit;
        }

        if self.is_busy {
            // Busy mode: only navigation, selection, console scroll, cancel & quit.
            match code {
                KeyCode::Esc => {
                    self.cancel_current_task();
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    self.next_item();
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    self.prev_item();
                }
                KeyCode::PageDown => self.page_down(),
                KeyCode::PageUp => self.page_up(),
                KeyCode::Home => self.home(),
                KeyCode::End => self.end(),
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
                KeyCode::Char('[') => self.scroll_log_up(),
                KeyCode::Char(']') => self.scroll_log_down(),
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    self.cancel_current_task();
                    self.should_quit = true;
                }
                // Action hotkeys (i/l/u/b/r/c/v/w...) provide clear user feedback while busy.
                _ => {
                    let task_name = self.active_task_name.as_deref().unwrap_or("Task");
                    self.status_message = format!("{} in progress ({}%). Press [Esc] to cancel before starting a new action.", task_name, self.progress_percentage);
                }
            }
            return self.should_quit;
        }

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
            KeyCode::Char('[') => {
                self.scroll_log_up();
            }
            KeyCode::Char(']') => {
                self.scroll_log_down();
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                self.dispatch(WorkerTask::Link);
            }
            KeyCode::Char('u') | KeyCode::Char('U')
            | KeyCode::Char('x') | KeyCode::Char('X')
            | KeyCode::Delete => {
                self.dispatch(WorkerTask::Unlink);
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                self.dispatch(WorkerTask::InstallLink);
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                self.dispatch(WorkerTask::Verify);
            }
            KeyCode::Char('b') | KeyCode::Char('B') => {
                self.dispatch(WorkerTask::Bootstrap);
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.dispatch(WorkerTask::Reload);
            }
            KeyCode::Char('c') | KeyCode::Char('C')
            | KeyCode::Char('g') | KeyCode::Char('G') => {
                self.dispatch(WorkerTask::CheckUpdates);
            }
            KeyCode::Char('w') | KeyCode::Char('W') => {
                self.dispatch(WorkerTask::Wallpapers);
            }
            KeyCode::Char('0') => {
                // Full revoke: open the red confirmation modal first.
                self.confirm_uninstall = true;
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

    // ========================================================================
    // Async worker plumbing (TUI: worker thread; CLI/tests: synchronous pass)
    // ========================================================================

    /// Routes a worker-produced message into App state for the synchronous path.
    fn absorb_sync_message(&mut self, msg: WorkerMessage) {
        match msg {
            WorkerMessage::Log(text) => {
                self.logs.push(text.clone());
                log_to_file(&text);
            }
            WorkerMessage::Progress { percentage, label, detail } => {
                self.progress_percentage = percentage;
                self.progress_label = label;
                self.progress_detail = detail;
            }
            WorkerMessage::PmStatus(pm) => {
                self.pm_status = pm;
            }
            WorkerMessage::GitStatus(status) => {
                self.git_update_status = Some(status);
            }
            WorkerMessage::TaskFinished { .. } => {}
        }
    }

    /// Snapshot of everything a task needs, taken before the worker starts.
    fn build_task_context(&self, task: WorkerTask) -> Result<TaskContext, (String, String)> {
        let mut ctx = TaskContext::new(self.repo_root.clone(), self.cancel_flag.clone(), self.pm_status.clone());
        match task {
            WorkerTask::InstallLink => {
                let packages: Vec<PackageItem> = self
                    .items
                    .iter()
                    .filter(|i| i.selected)
                    .map(|i| i.package.clone())
                    .collect();
                if packages.is_empty() {
                    return Err((
                        "No packages selected for installation.".to_string(),
                        "No packages selected.".to_string(),
                    ));
                }
                ctx.packages = packages;
            }
            WorkerTask::Link => {
                ctx.jobs = self
                    .items
                    .iter()
                    .filter(|i| i.selected && !i.package.configs.is_empty())
                    .map(|i| PackageJob {
                        pkg_id: i.package.id.to_string(),
                        pkg_name: i.package.name.to_string(),
                        configs: i.package.configs.clone(),
                    })
                    .collect();
                ctx.also_install_wallpapers = self.items.iter().any(|i| i.selected && i.package.id == "wallpapers");
            }
            WorkerTask::Unlink => {
                ctx.jobs = self
                    .items
                    .iter()
                    .filter(|i| i.selected)
                    .map(|i| PackageJob {
                        pkg_id: i.package.id.to_string(),
                        pkg_name: i.package.name.to_string(),
                        configs: i.package.configs.clone(),
                    })
                    .collect();
            }
            WorkerTask::Bootstrap | WorkerTask::Verify | WorkerTask::Reload | WorkerTask::CheckUpdates => {}
            WorkerTask::Wallpapers => {
                ctx.custom_dest = None;
            }
            // Full revoke reads the entire catalog inside the flow and the
            // ledger for wallpaper entries; no extra context needed.
            WorkerTask::UninstallAll => {}
        }
        Ok(ctx)
    }

    /// Runs a task synchronously by routing flow messages into App state.
    fn run_task_flow(&mut self, task: WorkerTask, ctx: TaskContext) -> String {
        let result = match task {
            WorkerTask::InstallLink => worker::flow_install(&ctx, &mut |m| self.absorb_sync_message(m)),
            WorkerTask::Link => worker::flow_link(&ctx, &mut |m| self.absorb_sync_message(m)),
            WorkerTask::Unlink => worker::flow_unlink(&ctx, &mut |m| self.absorb_sync_message(m)),
            WorkerTask::Bootstrap => worker::flow_bootstrap(&ctx, &mut |m| self.absorb_sync_message(m)),
            WorkerTask::Verify => worker::flow_verify(&ctx, &mut |m| self.absorb_sync_message(m)),
            WorkerTask::Reload => worker::flow_reload(&ctx, &mut |m| self.absorb_sync_message(m)),
            WorkerTask::CheckUpdates => worker::flow_check_updates(&ctx, &mut |m| self.absorb_sync_message(m)),
            WorkerTask::Wallpapers => worker::flow_wallpapers(&ctx, false, &mut |m| self.absorb_sync_message(m)),
            WorkerTask::UninstallAll => worker::flow_uninstall_all(&ctx, &mut |m| self.absorb_sync_message(m)),
        };
        result.summary
    }

    /// Single dispatch point for every action hotkey / API entry.
    fn dispatch(&mut self, task: WorkerTask) {
        if self.async_worker {
            self.start_worker_task(task);
        } else {
            match self.build_task_context(task) {
                Ok(ctx) => {
                    let summary = self.run_task_flow(task, ctx);
                    self.refresh_detection();
                    self.status_message = summary;
                }
                Err((log, status)) => {
                    self.add_log(log);
                    self.status_message = status;
                }
            }
        }
    }

    /// Public revoke entry used by the TUI confirmation modal and the CLI
    /// `--uninstall-all` flag: removes every rice-owned config target,
    /// restores/backs up, preserves user-modified copies, reverts wallpaper
    /// entries, and never touches program binaries.
    pub fn uninstall_everything(&mut self) {
        self.dispatch(WorkerTask::UninstallAll);
    }

    /// TUI entry: hands a task to a dedicated worker thread and switches to busy mode.
    pub fn start_worker_task(&mut self, task: WorkerTask) {
        if self.is_busy {
            self.add_log("[WARN] A task is already running. Press Esc to cancel it before starting a new one.");
            return;
        }

        let ctx = match self.build_task_context(task) {
            Ok(ctx) => ctx,
            Err((log, status)) => {
                self.add_log(log);
                self.status_message = status;
                return;
            }
        };

        let (tx, rx) = mpsc::channel::<WorkerMessage>();
        self.worker_rx = Some(rx);
        self.is_busy = true;
        self.active_task_name = Some(task.display_name().to_string());
        self.progress_percentage = 0;
        self.progress_label = "Initializing...".to_string();
        self.progress_detail = String::new();
        self.log_scroll_offset = 0;
        self.status_message = format!("Task started: {}. Press Esc to cancel.", task.display_name());
        self.add_log(format!("[START] Dispatching task '{}' to worker thread.", task.display_name()));

        // Fresh cancellation token exclusively for this task.
        self.cancel_flag = Arc::new(AtomicBool::new(false));
        let ctx = TaskContext {
            cancel_flag: self.cancel_flag.clone(),
            ensure_pm: true,
            stream_install: true,
            ..ctx
        };

        let task_name = task.display_name().to_string();
        let thread_name = format!("rice-worker-{}", task_name.to_lowercase().replace([' '], "-"));
        let spawned = std::thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                let result = worker::run_flow(task, &ctx, &mut |m: WorkerMessage| {
                    let _ = tx.send(m);
                });
                let _ = tx.send(WorkerMessage::TaskFinished {
                    task_name,
                    success: result.success,
                    summary: result.summary,
                });
            });

        match spawned {
            Ok(handle) => self.worker_handle = Some(handle),
            Err(e) => {
                self.is_busy = false;
                self.active_task_name = None;
                self.worker_rx = None;
                self.status_message = "Failed to start worker thread.".to_string();
                self.add_log(format!("[ERROR] Failed to spawn worker thread: {}", e));
            }
        }
    }

    /// Drains worker messages called from the TUI event loop every frame.
    pub fn poll_worker_events(&mut self) {
        if let Some(rx) = self.worker_rx.take() {
            let mut task_done = false;
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    WorkerMessage::Log(text) => {
                        self.logs.push(text.clone());
                        log_to_file(&text);
                    }
                    WorkerMessage::Progress { percentage, label, detail } => {
                        self.progress_percentage = percentage;
                        self.progress_label = label;
                        self.progress_detail = detail;
                    }
                    WorkerMessage::PmStatus(pm) => {
                        self.pm_status = pm;
                    }
                    WorkerMessage::GitStatus(status) => {
                        self.git_update_status = Some(status);
                    }
                    WorkerMessage::TaskFinished { task_name, success, summary } => {
                        self.is_busy = false;
                        self.active_task_name = None;
                        self.worker_handle = None;
                        self.status_message = summary.clone();
                        self.progress_percentage = 0;
                        self.progress_label = "Ready".to_string();
                        self.progress_detail = String::new();
                        if !success {
                            self.add_log(format!("[ENDED] Task '{}' ended: {}", task_name, summary));
                        }
                        self.refresh_detection();
                        task_done = true;
                    }
                }
            }
            self.worker_rx = if task_done { None } else { Some(rx) };
        }

        // Safety net: worker thread ended without sending TaskFinished (panic etc.).
        if self.is_busy {
            let thread_ended = self.worker_handle.as_ref().map_or(false, |h| h.is_finished());
            if thread_ended {
                self.worker_handle = None;
                self.worker_rx = None;
                self.is_busy = false;
                self.active_task_name = None;
                self.progress_percentage = 0;
                self.progress_label = "Ready".to_string();
                self.progress_detail = String::new();
                self.add_log("[ERROR] Worker thread ended without a completion message.".to_string());
                self.status_message = "Worker thread ended unexpectedly.".to_string();
                self.refresh_detection();
            }
        }
    }

    /// Requests cooperative cancellation of the running task (Esc during busy).
    pub fn cancel_current_task(&mut self) {
        if !self.is_busy {
            return;
        }
        if !self.cancel_flag.swap(true, Ordering::SeqCst) {
            self.add_log("[CANCEL] Cancellation requested - signalling worker task...");
        }
        self.status_message = "Cancelling current task...".to_string();
    }

    /// Scrolls the activity console backwards (older entries).
    pub fn scroll_log_up(&mut self) {
        let max = self.logs.len();
        self.log_scroll_offset = (self.log_scroll_offset + 10).min(max);
    }

    /// Scrolls the activity console forwards / back to live tail.
    pub fn scroll_log_down(&mut self) {
        self.log_scroll_offset = self.log_scroll_offset.saturating_sub(10);
    }

    pub fn link_selected_configs(&mut self) {
        self.dispatch(WorkerTask::Link);
    }

    pub fn unlink_selected_configs(&mut self) {
        self.dispatch(WorkerTask::Unlink);
    }

    pub fn unlink_all_configs(&mut self) {
        self.select_all();
        self.unlink_selected_configs();
    }

    pub fn install_and_link_selected(&mut self) {
        self.dispatch(WorkerTask::InstallLink);
    }

    pub fn bootstrap_package_managers(&mut self) {
        self.dispatch(WorkerTask::Bootstrap);
    }

    pub fn reload_environment(&mut self) {
        self.dispatch(WorkerTask::Reload);
    }

    pub fn check_updates(&mut self) {
        self.dispatch(WorkerTask::CheckUpdates);
    }

    pub fn install_wallpapers(&mut self, custom_dest: Option<PathBuf>) {
        if self.async_worker && custom_dest.is_none() {
            self.dispatch(WorkerTask::Wallpapers);
            return;
        }
        let Ok(mut ctx) = self.build_task_context(WorkerTask::Wallpapers) else { return };
        ctx.custom_dest = custom_dest;
        let summary = self.run_task_flow(WorkerTask::Wallpapers, ctx);
        self.status_message = summary;
    }

    pub fn uninstall_wallpapers(&mut self, custom_dest: Option<PathBuf>) {
        let Ok(mut ctx) = self.build_task_context(WorkerTask::Wallpapers) else { return };
        ctx.custom_dest = custom_dest;
        let result = worker::flow_wallpapers(&ctx, true, &mut |m| self.absorb_sync_message(m));
        self.status_message = result.summary;
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
            async_worker: false,
            confirm_uninstall: false,
            is_busy: false,
            active_task_name: None,
            progress_percentage: 0,
            progress_label: String::new(),
            progress_detail: String::new(),
            log_scroll_offset: 0,
            cancel_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            worker_rx: None,
            worker_handle: None,
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
            async_worker: false,
            confirm_uninstall: false,
            is_busy: false,
            active_task_name: None,
            progress_percentage: 0,
            progress_label: String::new(),
            progress_detail: String::new(),
            log_scroll_offset: 0,
            cancel_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            worker_rx: None,
            worker_handle: None,
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
            async_worker: false,
            confirm_uninstall: false,
            is_busy: false,
            active_task_name: None,
            progress_percentage: 0,
            progress_label: String::new(),
            progress_detail: String::new(),
            log_scroll_offset: 0,
            cancel_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            worker_rx: None,
            worker_handle: None,
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

    #[test]
    fn test_sequential_install_and_link_pipeline() {
        let _env = TestEnv::new("seq_install_link");
        let repo_root = PathBuf::from(".");
        let mut app = App::new(repo_root);
        assert!(app.items.len() >= 2);

        // Select first 2 packages
        app.select_none();
        app.items[0].selected = true;
        app.items[1].selected = true;

        app.install_and_link_selected();

        // Verify sequential progress logs were generated
        let all_logs = app.logs.join("\n");
        assert!(all_logs.contains("Starting Sequential Package Installation, Linking & Verification"));
        assert!(all_logs.contains("=== [1/2] Processing:"));
        assert!(all_logs.contains("=== [2/2] Processing:"));
        assert!(all_logs.contains("STEP 3: VERIFY"));
        assert!(all_logs.contains("Advancing to next selected option"));
        assert!(all_logs.contains("Setup complete for 2 package(s)"));
    }

    #[test]
    fn test_log_scroll_keys_and_caps() {
        let _env = TestEnv::new("log_scroll");
        let mut app = App::new(PathBuf::from("."));
        assert_eq!(app.log_scroll_offset, 0);

        for _ in 0..100 {
            app.handle_key(KeyCode::Char('['));
        }
        assert_eq!(app.log_scroll_offset, app.logs.len(), "scroll-up must cap at total log entries");

        for _ in 0..100 {
            app.handle_key(KeyCode::Char(']'));
        }
        assert_eq!(app.log_scroll_offset, 0, "scroll-down must saturate at zero (live tail)");
    }

    #[test]
    fn test_async_worker_guards() {
        let _env = TestEnv::new("async_worker_guards");
        let mut app = App::new(PathBuf::from("."));
        app.async_worker = true;
        app.select_none();

        // Nothing selected -> install dispatch must refuse without going busy.
        app.start_worker_task(crate::worker::WorkerTask::InstallLink);
        assert!(!app.is_busy, "empty selection must not enter busy mode");
        assert!(app.status_message.contains("No packages selected"));
    }

    #[test]
    fn test_async_worker_state_machine() {
        let _env = TestEnv::new("async_worker_sm");
        let mut app = App::new(PathBuf::from("."));
        app.async_worker = true;
        app.select_none();

        assert!(!app.is_busy);
        app.start_worker_task(crate::worker::WorkerTask::CheckUpdates);
        assert!(app.is_busy, "is_busy must be true immediately after dispatch");
        assert_eq!(app.active_task_name.as_deref(), Some(crate::worker::WorkerTask::CheckUpdates.display_name()));

        // While busy: action hotkeys are ignored, navigation still works.
        let logs_before = app.logs.len();
        app.handle_key(KeyCode::Char('i'));
        assert!(app.is_busy, "action hotkeys must not dispatch while a task is running");
        assert_eq!(app.logs.len(), logs_before, "ignored action must not log");
        app.handle_key(KeyCode::Char('j'));
        assert_eq!(app.selected_index, 1, "navigation must stay available while busy");
        app.handle_key(KeyCode::Char('['));
        assert!(app.log_scroll_offset > 0, "console scroll must stay available while busy");

        // Cancel signal sets the cooperative flag; task still terminates itself.
        app.cancel_current_task();
        assert!(app.cancel_flag.load(std::sync::atomic::Ordering::SeqCst), "cancel flag must be set");
        assert!(app.status_message.contains("Cancelling"), "status must reflect cancellation request");

        // Poll until the worker finishes (bounded, ~15s max) and returns to idle.
        // The worker runs an EXTERNAL `git status` against the checkout, which
        // alone takes ~3s on slow/network filesystems and longer under full
        // suite parallelism - the bound only guards against a true hang, the
        // assertions below still verify completion and summary correctness.
        let mut attempts = 0;
        while app.is_busy && attempts < 1500 {
            app.poll_worker_events();
            std::thread::sleep(std::time::Duration::from_millis(10));
            attempts += 1;
        }
        assert!(!app.is_busy, "worker task must finish and exit busy mode");
        assert!(app.active_task_name.is_none());
        assert!(app.git_update_status.is_some(), "GitStatus message must update repository state");
        assert!(app.status_message.contains("up to date") || app.status_message.contains("publish")
            || app.status_message.contains("custom state") || app.status_message.contains("Could not check"),
            "summary must describe repository status, got: {}", app.status_message);
    }
}
