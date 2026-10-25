// ============================================================================
// Author -: HKDEVS
// Project -: Window-Rice
// Title -: WINDOW-RICE Installer Package Manager Bootstrapper
// ============================================================================
use std::process::Command;
use anyhow::{Context, Result};

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

pub fn run_package_install(install_cmd: &str) -> Result<String> {
    run_powershell_command(install_cmd)
}

fn run_powershell_command(command: &str) -> Result<String> {
    let mut cmd = Command::new("powershell.exe");

    // Augment PATH with Scoop shims, Cargo bin, and Local bin if present
    if let Some(user_profile) = std::env::var_os("USERPROFILE") {
        let scoop_shims = std::path::PathBuf::from(&user_profile).join("scoop").join("shims");
        let cargo_bin = std::path::PathBuf::from(&user_profile).join(".cargo").join("bin");
        let local_bin = std::path::PathBuf::from(&user_profile).join(".local").join("bin");
        if let Some(current_path) = std::env::var_os("PATH") {
            let mut paths = vec![scoop_shims, cargo_bin, local_bin];
            paths.extend(std::env::split_paths(&current_path));
            if let Ok(new_path) = std::env::join_paths(paths) {
                cmd.env("PATH", new_path);
            }
        }
    }

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
