// ============================================================================
// Author -: HKDEVS
// Project -: Window-Rice
// Title -: WINDOW-RICE Installer Main Entrypoint
// ============================================================================
mod catalog;
mod detect;
mod ledger;
mod linker;
mod bootstrap;
mod verifier;
mod worker;
mod app;
mod ui;

use std::env;
use std::io::stdout;
use std::path::PathBuf;
use std::time::Duration;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::App;
use verifier::run_verification;

fn find_repo_root() -> PathBuf {
    // 1. Current executable's parent or ancestor
    if let Ok(exe_path) = env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            if parent.join("carapace").exists() || parent.join("glazewm").exists() {
                return parent.to_path_buf();
            }
            if let Some(grandparent) = parent.parent() {
                if grandparent.join("carapace").exists() || grandparent.join("glazewm").exists() {
                    return grandparent.to_path_buf();
                }
            }
        }
    }

    // 2. Current working directory
    if let Ok(cwd) = env::current_dir() {
        if cwd.join("carapace").exists() || cwd.join("glazewm").exists() {
            return cwd;
        }
        if cwd.join(".config").exists() {
            return cwd.join(".config");
        }
    }

    // 3. User profile .config
    if let Some(user_profile) = env::var_os("USERPROFILE") {
        let p = PathBuf::from(user_profile).join(".config");
        if p.exists() {
            return p;
        }
    }

    PathBuf::from(".")
}

fn print_help() {
    println!(r#"
WINDOW-RICE INSTALLER - Interactive Setup & Real-Time Rice Manager
=================================================================
Usage: rice-installer.exe [OPTIONS]

Options:
    --reload, -r             Reload Komorebi, GlazeWM, and YASB status bar
    --update, -c, -u         Check git repository upstream status for updates
    --wallpapers, -w [PATH]  Install 8K wallpaper collection (default: Pictures\Wallpapers)
    --uninstall-wallpapers   Remove 8K wallpapers from Pictures\Wallpapers
    --verify, -v             Run non-interactive system health check and exit
    --link-all               Link all configuration files (symlink with copy fallback)
    --unlink-all             Unlink all configs cleanly without trace & restore backups
    --uninstall-all          FULL REVOKE: remove every rice-installed config & wallpaper
                             entry (binaries stay, user-modified copies preserved)
    --unlink, -x             Unlink configs for selected/missing items & restore backups
    --install-missing        Install missing packages and link corresponding configs
    --bootstrap, -b          Bootstrap Scoop, Chocolatey, UV, and Vcpkg if missing
    -h, --help               Show this help message

Interactive TUI Controls:
    [↑/↓] or [k/j]           Navigate up/down
    [PgUp/PgDn]              Page scroll through package list
    [Home/End]               Jump to first / last package
    [Space] or [Enter]       Toggle selection checkbox
    [a/A]                    Select all / Deselect all
    [n/N]                    Deselect none
    [m/M]                    Select missing / unlinked packages only
    [l/L]                    Link configs for selected packages (and auto-reload)
    [u/U] or [x/X] or [Del]  Unlink selected configs cleanly & restore original backups
    [0]                      Full revoke - uninstall EVERYTHING the installer placed
                             (red confirmation modal; [Y] confirms, [N/Esc] cancels)
    [i/I]                    Install missing binaries & link configs for selected
    [r/R]                    Reload Komorebi, GlazeWM, and YASB
    [c/C] or [g/G]           Check git repository for updates
    [w/W]                    Install 8K wallpapers to Pictures\Wallpapers
    [v/V]                    Run comprehensive verification health check
    [b/B]                    Bootstrap Scoop / Chocolatey / UV / Vcpkg
    [[/]]                    Scroll the activity console ([ live tail ])
    [Esc] while task runs    Cancel/abort the running task (subprocess killed)
    [q/Q]                    Quit cleanly

All long actions (install/link/unlink/bootstrap/verify/reload/update/wallpapers)
run on a dedicated worker thread with live progress, streaming [OUT] console
output, and cooperative Esc cancellation."#);
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let repo_root = find_repo_root();

    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return Ok(());
    }

    if args.iter().any(|a| a == "--reload" || a == "-r") {
        let mut app = App::new(repo_root);
        app.reload_environment();
        for log in &app.logs {
            println!("{}", log);
        }
        return Ok(());
    }

    if args.iter().any(|a| a == "--update" || a == "--check-updates" || a == "-c") {
        let mut app = App::new(repo_root);
        app.check_updates();
        for log in &app.logs {
            println!("{}", log);
        }
        return Ok(());
    }

    if let Some(pos) = args.iter().position(|a| a == "--wallpapers" || a == "-w") {
        let custom_dest = args.get(pos + 1).map(PathBuf::from);
        let mut app = App::new(repo_root);
        app.install_wallpapers(custom_dest);
        for log in &app.logs {
            println!("{}", log);
        }
        return Ok(());
    }

    if args.iter().any(|a| a == "--uninstall-wallpapers") {
        println!("Uninstalling 8K Wallpapers...");
        let mut app = App::new(repo_root);
        app.uninstall_wallpapers(None);
        for log in &app.logs {
            println!("{}", log);
        }
        return Ok(());
    }

    if args.iter().any(|a| a == "--uninstall-all") {
        println!("FULL REVOKE: removing EVERY config installed by Window-Rice & its wallpaper entries...");
        println!("Binaries stay installed; user-modified copies are preserved & archived; repo-native targets are never touched.");
        let mut app = App::new(repo_root);
        app.uninstall_everything();
        for log in &app.logs {
            println!("{}", log);
        }
        return Ok(());
    }

    if args.iter().any(|a| a == "--verify" || a == "-v") {
        let report = run_verification(&repo_root);
        app::log_to_file("--- Running Health Check Report (CLI --verify) ---");
        for line in &report.lines {
            println!("{}", line);
            app::log_to_file(line);
        }
        let summary = format!(
            "Health check complete: {}/{} binaries, {}/{} configs.",
            report.installed_binaries, report.total_packages, report.linked_configs, report.total_packages
        );
        app::log_to_file(&summary);
        return Ok(());
    }

    if args.iter().any(|a| a == "--bootstrap" || a == "-b") {
        println!("Bootstrapping package managers...");
        let mut app = App::new(repo_root);
        app.bootstrap_package_managers();
        for log in &app.logs {
            println!("{}", log);
        }
        return Ok(());
    }

    if args.iter().any(|a| a == "--link-all") {
        println!("Linking all configuration files...");
        let mut app = App::new(repo_root);
        app.select_all();
        app.link_selected_configs();
        for log in &app.logs {
            println!("{}", log);
        }
        return Ok(());
    }

    if args.iter().any(|a| a == "--unlink-all") {
        println!("Unlinking all configuration files & restoring backups (binaries preserved)...");
        let mut app = App::new(repo_root);
        app.unlink_all_configs();
        for log in &app.logs {
            println!("{}", log);
        }
        return Ok(());
    }

    if args.iter().any(|a| a == "--unlink" || a == "--unlink-selected" || a == "-x") {
        println!("Unlinking selected configuration files & restoring backups (binaries preserved)...");
        let mut app = App::new(repo_root);
        let explicit_pkgs: Vec<String> = args
            .iter()
            .skip(1)
            .filter(|a| !a.starts_with('-'))
            .cloned()
            .collect();

        if explicit_pkgs.is_empty() {
            for item in &mut app.items {
                item.selected = matches!(
                    item.inspection.link_status,
                    detect::LinkStatus::LinkedSymlink(_) | detect::LinkStatus::LinkedCopy(_)
                );
            }
        } else {
            for item in &mut app.items {
                item.selected = explicit_pkgs
                    .iter()
                    .any(|p| p.eq_ignore_ascii_case(item.package.id) || p.eq_ignore_ascii_case(item.package.name));
            }
        }

        app.unlink_selected_configs();
        for log in &app.logs {
            println!("{}", log);
        }
        return Ok(());
    }

    if args.iter().any(|a| a == "--install-missing") {
        println!("Installing missing packages and linking configs...");
        let mut app = App::new(repo_root);
        app.select_missing_only();
        app.install_and_link_selected();
        for log in &app.logs {
            println!("{}", log);
        }
        return Ok(());
    }

    if args.iter().any(|a| a == "--install" || a == "--install-selected" || a == "-i") {
        println!("Installing and linking selected packages (with automated testing)...");
        let mut app = App::new(repo_root);
        let explicit_pkgs: Vec<String> = args
            .iter()
            .skip(1)
            .filter(|a| !a.starts_with('-'))
            .cloned()
            .collect();

        if explicit_pkgs.is_empty() {
            app.select_missing_only();
        } else {
            for item in &mut app.items {
                item.selected = explicit_pkgs
                    .iter()
                    .any(|p| p.eq_ignore_ascii_case(item.package.id) || p.eq_ignore_ascii_case(item.package.name));
            }
        }

        app.install_and_link_selected();
        for log in &app.logs {
            println!("{}", log);
        }
        return Ok(());
    }

    // Launch interactive TUI
    run_tui(repo_root)?;
    Ok(())
}

fn run_tui(repo_root: PathBuf) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(repo_root);
    // TUI mode: long actions run on a dedicated worker thread with live progress.
    app.async_worker = true;

    loop {
        // Drain worker messages (logs / progress / completion) before drawing.
        app.poll_worker_events();

        terminal.draw(|f| ui::render(f, &app))?;

        if app.should_quit {
            break;
        }

        let poll_ms = if app.is_busy { 50 } else { 100 };
        if event::poll(Duration::from_millis(poll_ms))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if app.handle_key(key.code) {
                        break;
                    }
                }
            }
        }
    }

    // If a worker task is still running on quit, signal cancellation so the
    // streaming runner kills its subprocess before the process exits.
    if app.is_busy {
        app.cancel_current_task();
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    println!("\nWINDOW-RICE installer closed cleanly.");
    Ok(())
}
