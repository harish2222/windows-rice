### HKDEVS PowerShell 5.1 Profile - Carapace + Oh-My-Posh
$env:CARAPACE_BRIDGES = 'zsh,fish,bash,inshellisense'

# Carapace completions (official setup) — init script cached so the carapace
# process isn't spawned on every startup (regenerated when carapace.exe changes).
if (Test-Path 'C:\Users\haris\scoop\apps\carapace\current\carapace.exe') {
    Set-Alias -Name carapace -Value 'C:\Users\haris\scoop\apps\carapace\current\carapace.exe' -Force
    Set-PSReadLineOption -Colors @{ "Selection" = "`e[7m" }
    Set-PSReadlineKeyHandler -Key Tab -Function MenuComplete

    $carapaceCacheDir = Join-Path $env:LOCALAPPDATA 'carapace'
    $carapaceCache = Join-Path $carapaceCacheDir 'init-windows-powershell-cached.ps1'
    $carapaceCacheItem = Get-Item -LiteralPath $carapaceCache -ErrorAction SilentlyContinue
    $carapaceExeTime = (Get-Item -LiteralPath 'C:\Users\haris\scoop\apps\carapace\current\carapace.exe').LastWriteTimeUtc
    if (-not $carapaceCacheItem -or $carapaceCacheItem.LastWriteTimeUtc -lt $carapaceExeTime) {
        $null = New-Item -ItemType Directory -Path $carapaceCacheDir -Force
        carapace _carapace | Set-Content -LiteralPath $carapaceCache -Encoding UTF8
    }
    Invoke-Expression (Get-Content -LiteralPath $carapaceCache -Raw)
}

# Oh-My-Posh prompt — init cached the same way (regenerated when theme/exe change).
$ompTheme = Join-Path $env:USERPROFILE 'Documents\PowerShell\lambda_new.omp.json'
if (Test-Path $ompTheme) {
    $ompCache = Join-Path $env:LOCALAPPDATA 'oh-my-posh\init-windows-powershell-cached.ps1'
    $ompCacheItem = Get-Item -LiteralPath $ompCache -ErrorAction SilentlyContinue
    $ompExe = (Get-Command oh-my-posh -ErrorAction SilentlyContinue).Source
    $ompExeTime = if ($ompExe) { (Get-Item -LiteralPath $ompExe).LastWriteTimeUtc } else { [datetime]::MinValue }
    $ompThemeTime = (Get-Item -LiteralPath $ompTheme).LastWriteTimeUtc
    if (-not $ompCacheItem -or $ompCacheItem.LastWriteTimeUtc -lt $ompExeTime -or $ompCacheItem.LastWriteTimeUtc -lt $ompThemeTime) {
        $null = New-Item -ItemType Directory -Path (Split-Path -Parent $ompCache) -Force
        oh-my-posh init pwsh --config $ompTheme | Set-Content -LiteralPath $ompCache -Encoding UTF8
    }
    Invoke-Expression (Get-Content -LiteralPath $ompCache -Raw)
}

function prompt { $marker = if (([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { '#' } else { '$' }; "[C:\Users\haris\.config\yasb] $marker " }
# Guarded: this file no longer exists (yasb scripts dir removed) — was throwing CommandNotFoundException on EVERY powershell.exe spawn (startup script, GlazeWM volume/brightness keybinds, etc.)
$yasbThemeShell = "C:\Users\haris\.config\yasb\scripts\yasb-theme-shell.ps1"
if (Test-Path -LiteralPath $yasbThemeShell) { . $yasbThemeShell }
