// ============================================================================
// Author -: HKDEVS
// Project -: Window-Rice
// Title -: WINDOW-RICE Installer Package Manager Bootstrapper
// ============================================================================
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use anyhow::{Context, Result};
use crate::detect::{detect_package_managers, find_binary_in_path, user_profile_file_exists, PackageManagerStatus};

pub fn bootstrap_scoop() -> Result<String> {
    let script = "Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser -Force; [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; Invoke-RestMethod -Uri https://get.scoop.sh | Invoke-Expression";
    run_powershell_command(script)
}

pub fn bootstrap_choco() -> Result<String> {
    let script = "Set-ExecutionPolicy Bypass -Scope Process -Force; [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))";
    run_powershell_command(script)
}

pub fn bootstrap_uv() -> Result<String> {
    let script = "Set-ExecutionPolicy RemoteSigned -Scope CurrentUser -Force; [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; Invoke-RestMethod -Uri https://astral.sh/uv/install.ps1 | Invoke-Expression";
    run_powershell_command(script)
}

pub fn bootstrap_vcpkg() -> Result<String> {
    let script = "if (Get-Command scoop -ErrorAction SilentlyContinue) { scoop install vcpkg } else { git clone https://github.com/microsoft/vcpkg.git $env:USERPROFILE/vcpkg; & $env:USERPROFILE/vcpkg/bootstrap-vcpkg.bat }";
    run_powershell_command(script)
}

/// Sanitizes package installation commands by injecting non-interactive flags
/// to prevent subprocesses from hanging on interactive prompts (e.g. winget license agreements).
pub fn prepare_install_command(cmd: &str) -> String {
    let trimmed = cmd.trim();
    if trimmed.starts_with("winget install ") {
        if !trimmed.contains("--accept-source-agreements") {
            format!("{} --exact --silent --accept-source-agreements --accept-package-agreements --disable-interactivity", trimmed)
        } else {
            trimmed.to_string()
        }
    } else {
        trimmed.to_string()
    }
}

/// Ensures the Scoop `extras` bucket is registered so window managers and utilities can be resolved.
pub fn ensure_scoop_buckets(cancel_token: &Arc<AtomicBool>, on_line: &mut dyn FnMut(String)) {
    let has_scoop = find_binary_in_path("scoop.cmd").is_some() || user_profile_file_exists("scoop/shims/scoop.cmd");
    if has_scoop {
        on_line("Checking Scoop buckets (extras, nerd-fonts)...".to_string());
        let script = r#"
            $buckets = scoop bucket list 2>$null
            if ($buckets -notmatch 'extras') {
                Write-Host 'Adding extras bucket...'
                scoop bucket add extras
            }
            if ($buckets -notmatch 'nerd-fonts') {
                Write-Host 'Adding nerd-fonts bucket...'
                scoop bucket add nerd-fonts
            }
        "#;
        let _ = run_streaming_command(script, cancel_token, |l| {
            on_line(format!("  [SCOOP BUCKET] {}", l));
        });
    }
}

/// Autonomous package manager resolver: ensures either Scoop or Winget is present and configured.
pub fn ensure_package_manager_ready(cancel_token: &Arc<AtomicBool>, on_line: &mut dyn FnMut(String)) -> PackageManagerStatus {
    let mut pm = detect_package_managers();
    if !pm.scoop && !pm.winget {
        on_line("  [PREREQ] Neither Scoop nor Winget found. Auto-bootstrapping Scoop...".to_string());
        match bootstrap_scoop() {
            Ok(_) => {
                on_line("  [PASS] Scoop package manager successfully installed.".to_string());
                pm = detect_package_managers();
            }
            Err(e) => {
                on_line(format!("  [FAIL] Error installing Scoop: {}", e));
            }
        }
    }

    if pm.scoop {
        ensure_scoop_buckets(cancel_token, on_line);
    }

    pm
}

/// Runs a package installation command synchronously, returning captured stdout.
pub fn run_package_install(install_cmd: &str) -> Result<String> {
    let sanitized = prepare_install_command(install_cmd);
    run_powershell_command(&sanitized)
}

/// Augments the given Command's PATH with user profile locations (Scoop shims, Cargo bin, Local bin).
fn augment_command_path(cmd: &mut Command) {
    if let Some(user_profile) = std::env::var_os("USERPROFILE") {
        let scoop_shims = PathBuf::from(&user_profile).join("scoop").join("shims");
        let scoop_git = PathBuf::from(&user_profile).join("scoop").join("apps").join("git").join("current").join("cmd");
        let cargo_bin = PathBuf::from(&user_profile).join(".cargo").join("bin");
        let local_bin = PathBuf::from(&user_profile).join(".local").join("bin");
        if let Some(current_path) = std::env::var_os("PATH") {
            let mut paths = vec![scoop_shims, scoop_git, cargo_bin, local_bin];
            paths.extend(std::env::split_paths(&current_path));
            if let Ok(new_path) = std::env::join_paths(paths) {
                cmd.env("PATH", new_path);
            }
        }
    }
}

/// Synchronous PowerShell execution helper.
fn run_powershell_command(command: &str) -> Result<String> {
    let mut cmd = Command::new("powershell.exe");
    augment_command_path(&mut cmd);
    cmd.env("CI", "1");
    cmd.env("POWERSHELL_TELEMETRY_OPTOUT", "1");

    let output = cmd
        .args(&["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", command])
        .output()
        .with_context(|| format!("Failed to execute powershell command: {}", command))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        anyhow::bail!("Command failed with code {:?}:\nStdout: {}\nStderr: {}", output.status.code(), stdout, stderr);
    }

    Ok(stdout)
}

/// Asynchronous streaming subprocess runner with real-time stdout/stderr line forwarding
/// and cooperative cancellation via cancel_token.
pub fn run_streaming_command<F>(command: &str, cancel_token: &Arc<AtomicBool>, mut on_line: F) -> Result<()>
where
    F: FnMut(String),
{
    let mut cmd = Command::new("powershell.exe");
    augment_command_path(&mut cmd);
    cmd.env("CI", "1");
    cmd.env("POWERSHELL_TELEMETRY_OPTOUT", "1");

    cmd.args(&["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", command]);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().with_context(|| format!("Failed to spawn powershell command: {}", command))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let (line_tx, line_rx) = std::sync::mpsc::channel();

    let tx1 = line_tx.clone();
    if let Some(out) = stdout {
        std::thread::spawn(move || {
            let reader = BufReader::new(out);
            for line in reader.lines().flatten() {
                let trimmed = line.trim_end();
                if !trimmed.is_empty() {
                    let _ = tx1.send(trimmed.to_string());
                }
            }
        });
    }

    let tx2 = line_tx.clone();
    if let Some(err) = stderr {
        std::thread::spawn(move || {
            let reader = BufReader::new(err);
            for line in reader.lines().flatten() {
                let trimmed = line.trim_end();
                if !trimmed.is_empty() {
                    let _ = tx2.send(trimmed.to_string());
                }
            }
        });
    }
    drop(line_tx);

    loop {
        // Drain any lines that arrived
        while let Ok(line) = line_rx.try_recv() {
            on_line(line);
        }

        // Cooperative cancellation check
        if cancel_token.load(Ordering::SeqCst) {
            let _ = child.kill();
            on_line("[CANCELLED] Subprocess killed by user.".to_string());
            anyhow::bail!("Operation cancelled by user.");
        }

        // Check if child process has exited
        match child.try_wait()? {
            Some(status) => {
                // Drain any final buffered lines
                while let Ok(line) = line_rx.recv_timeout(Duration::from_millis(50)) {
                    on_line(line);
                }
                if status.success() {
                    return Ok(());
                } else {
                    anyhow::bail!("Process exited with status {:?}", status.code());
                }
            }
            None => {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}
