# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: Alacritty Launcher (maximized + focused, otty-style)
# ============================================================================
# GlazeWM win+shift+return binds here. Komorebi owns all tiling (GlazeWM
# ignores every window), so a plain `shell-exec alacritty` gets tiled away and
# startup_mode="Maximized" alone cannot hold. This wrapper replicates the otty
# treatment: SW_MAXIMIZE + SetForegroundWindow shortly after the window is
# realized, which komorebi's float layer accepts.
#
# Footgun this script guards against: alacritty briefly owns a visible 16x16
# pre-realization window, so window picking ignores anything smaller than
# 150x150.
# ============================================================================

$signature = @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

public static class WinRiceAlc
{
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
    public struct RECT { public int L, T, R, B; }
}
'@
if (-not ('WinRiceAlc' -as [type])) { Add-Type -TypeDefinition $signature }

function Maximize-Focus {
    param([IntPtr]$Hwnd)
    [WinRiceAlc]::ShowWindow($Hwnd, 3)  | Out-Null   # SW_MAXIMIZE
    [WinRiceAlc]::SetForegroundWindow($Hwnd) | Out-Null
    if (-not [WinRiceAlc]::IsZoomed($Hwnd)) {
        [WinRiceAlc]::ShowWindow($Hwnd, 3) | Out-Null
    }
}

function Get-RealWindow {
    # Main window handle of an alacritty process that is a real terminal
    # surface (visible, at least 150x150) - skips the 16x16 pre-window.
    param([System.Diagnostics.Process]$Proc)
    $h = $Proc.MainWindowHandle
    if ($h -eq [IntPtr]::Zero) { return $null }
    $r = New-Object WinRiceAlc+RECT
    $null = [WinRiceAlc]::GetWindowRect($h, [ref]$r)
    if ((($r.R - $r.L) -ge 150) -and (($r.B - $r.T) -ge 150)) { return $h }
    return $null
}

# --- Single instance aware: focus + maximize what is already there ----------
$procs = @(Get-Process -Name alacritty -ErrorAction SilentlyContinue)
foreach ($p in $procs) {
    $h = Get-RealWindow $p
    if ($h) { Maximize-Focus $h; exit 0 }
}

# --- Fresh launch ------------------------------------------------------------
$exe = "$env:USERPROFILE\scoop\apps\alacritty\current\alacritty.exe"
if (-not (Test-Path -LiteralPath $exe)) {
    $exe = (Get-Command alacritty -ErrorAction SilentlyContinue).Source
}
if (-not $exe -or -not (Test-Path -LiteralPath $exe)) { exit 1 }

Start-Process -FilePath $exe

# Wait up to 15s for a REAL alacritty window, then maximize + focus it.
for ($i = 0; $i -lt 76; $i++) {
    Start-Sleep -Milliseconds 200
    $procs = @(Get-Process -Name alacritty -ErrorAction SilentlyContinue)
    foreach ($p in $procs) {
        $h = Get-RealWindow $p
        if ($h) {
            Start-Sleep -Milliseconds 400   # settle: let the surface finish realizing
            Maximize-Focus $h
            exit 0
        }
    }
}

# Window never came up with a real surface; alacritty itself was launched.
exit 0
