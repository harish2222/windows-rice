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
    let output = Command::new("powershell.exe")
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
