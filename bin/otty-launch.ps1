# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: otty Launcher (maximized + focused)
# ============================================================================
# GlazeWM alt+Return binds here instead of plain `otty` because otty
# 1.6.0-beta has no on-launch maximize/focus keys of its own (verified against
# `config get` candidates and the 1.6.x docs). Behavior:
#   - otty already running  -> maximize + focus the existing window (single
#                             instance aware, never spawns a duplicate)
#   - otty not running      -> start it, wait for its window, maximize + focus
# ============================================================================

$signature = @'
using System;
using System.Runtime.InteropServices;

public static class WinRice
{
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr hWnd);
}
'@
if (-not ('WinRice' -as [type])) { Add-Type -TypeDefinition $signature }

function Maximize-Focus {
    param([IntPtr]$Hwnd)
    [WinRice]::ShowWindow($Hwnd, 3)  | Out-Null   # SW_MAXIMIZE
    [WinRice]::SetForegroundWindow($Hwnd) | Out-Null
    # Belt and braces: if something minimized it in the race, maximize again.
    if (-not [WinRice]::IsZoomed($Hwnd)) {
        [WinRice]::ShowWindow($Hwnd, 3) | Out-Null
    }
}

# --- Single instance aware: focus + maximize what is already there ----------
$running = Get-Process -Name otty -ErrorAction SilentlyContinue |
    Where-Object { $_.MainWindowHandle -ne [IntPtr]::Zero } |
    Select-Object -First 1
if ($running) {
    Maximize-Focus $running.MainWindowHandle
    exit 0
}

# --- Fresh launch ------------------------------------------------------------
$exe = (Get-Command otty -ErrorAction SilentlyContinue).Source
if (-not $exe) { $exe = 'C:\Program Files\otty\bin\otty.exe' }
if (-not (Test-Path -LiteralPath $exe)) { exit 1 }

Start-Process -FilePath $exe

# Wait up to 15s for the main window to be real, then maximize + focus it.
for ($i = 0; $i -lt 75; $i++) {
    Start-Sleep -Milliseconds 200
    $p = Get-Process -Name otty -ErrorAction SilentlyContinue |
        Where-Object { $_.MainWindowHandle -ne [IntPtr]::Zero } |
        Select-Object -First 1
    if ($p) {
        Start-Sleep -Milliseconds 400   # settle: let the window finish realizing
        Maximize-Focus $p.MainWindowHandle
        exit 0
    }
}

# Window never appeared (e.g. delayed startup); otty itself was launched.
exit 0
