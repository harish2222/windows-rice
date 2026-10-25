mod catalog;
mod detect;
mod linker;
mod bootstrap;
mod verifier;
mod app;
mod ui;

use std::env;
use std::io::stdout;
use std::path::PathBuf;
use std::time::Duration;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
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
WINDOWS RICE - Interactive Installer & Configurator
===================================================
Usage: rice-installer.exe [OPTIONS]

Options:
    --verify            Run non-interactive system health check and exit
    --link-all          Link all configuration files (symlink with copy fallback)
    --install-missing   Install missing packages and link corresponding configs
    --bootstrap         Bootstrap Scoop and Chocolatey if missing
    -h, --help          Show this help message

Interactive TUI Controls:
    [↑/↓] or [k/j]      Navigate packages
    [Space]             Toggle selection checkbox
    [a]                 Select all / Deselect all
    [m]                 Select missing / unlinked packages only
    [l]                 Link configs for selected packages
    [i]                 Install missing binaries & link configs for selected
    [v]                 Run comprehensive verification health check
    [b]                 Bootstrap Scoop / Chocolatey
    [q] or [Esc]        Quit
"#);
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let repo_root = find_repo_root();

    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return Ok(());
    }

    if args.iter().any(|a| a == "--verify") {
        let report = run_verification(&repo_root);
        for line in report.lines {
            println!("{}", line);
        }
        return Ok(());
    }

    if args.iter().any(|a| a == "--bootstrap") {
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

    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        if app.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            app.should_quit = true;
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.next_item();
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.prev_item();
                        }
                        KeyCode::Char(' ') => {
                            app.toggle_selected();
                        }
                        KeyCode::Char('a') => {
                            app.select_all();
                        }
                        KeyCode::Char('m') => {
                            app.select_missing_only();
                        }
                        KeyCode::Char('l') => {
                            app.link_selected_configs();
                        }
                        KeyCode::Char('i') => {
                            app.install_and_link_selected();
                        }
                        KeyCode::Char('v') => {
                            app.run_verification_report();
                        }
                        KeyCode::Char('b') => {
                            app.bootstrap_package_managers();
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    println!("\nRice installer closed cleanly.");
    Ok(())
}
