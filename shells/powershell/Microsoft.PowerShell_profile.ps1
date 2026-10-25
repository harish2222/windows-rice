### HKDEVS PowerShell profile — unified shell/terminal engineering edition
#
# Startup budget (warm, time-to-usable-prompt):
#   * Nothing on this file imports a module on the critical path; everything
#     heavy is on demand.
#   * oh-my-posh initialises the prompt via its own cached init script.
#   * Terminal-Icons, PSFzf, Carapace and the custom argument completers are
#     loaded on demand.

# ============================================================================
# FASTFETCH — on demand, never on the startup critical path
# ============================================================================
function fetch {
    $ffConfig = Join-Path $env:USERPROFILE '.config\fastfetch\config.jsonc'
    if (Test-Path $ffConfig) {
        fastfetch --config $ffConfig
    }
    else {
        fastfetch
    }
}
Set-Alias -Name ffetch -Value fetch -Force
Set-Alias -Name neofetch -Value fetch -Force
Set-Alias -Name ls -Value lsd -Force
$script:ProfileRoot = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Path $PROFILE.CurrentUserCurrentHost -Parent }
$script:CustomProfile = Join-Path -Path $script:ProfileRoot -ChildPath 'CTTcustom.ps1'

if (Test-Path -Path $script:CustomProfile -PathType Leaf) {
    . $script:CustomProfile
}

function Test-InteractiveShell {
    try {
        return $Host.Name -eq 'ConsoleHost' -and
        -not [Console]::IsInputRedirected -and
        -not [Console]::IsOutputRedirected
    }
    catch {
        return $false
    }
}

function Get-ProfileDir {
    switch ($PSVersionTable.PSEdition) {
        'Core' { Join-Path ([Environment]::GetFolderPath('MyDocuments')) 'PowerShell'; break }
        'Desktop' { Join-Path ([Environment]::GetFolderPath('MyDocuments')) 'WindowsPowerShell'; break }
        default {
            throw "Unsupported PowerShell edition: $($PSVersionTable.PSEdition)"
        }
    }
}

function Test-Command {
    param([Parameter(Mandatory)][string]$Name)
    $null -ne (Get-Command -Name $Name -ErrorAction SilentlyContinue)
}

# Fast existence probe for known install roots. Get-Command walks every PATH
# directory (~30-80 ms per call), so the well-known locations are checked first.
function Test-FastCommand {
    param([Parameter(Mandatory)][string]$Name)
    foreach ($dir in @((Join-Path $HOME 'scoop\shims'), (Join-Path $HOME '.local\bin'), (Join-Path $env:LOCALAPPDATA 'Programs'))) {
        foreach ($ext in '.exe', '.cmd', '.bat', '.ps1') {
            if (Test-Path -LiteralPath (Join-Path $dir ($Name + $ext))) { return $true }
        }
    }
    return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

function Save-UriToFile {
    param(
        [Parameter(Mandatory)][string]$Uri,
        [Parameter(Mandatory)][string]$OutFile
    )

    $client = New-Object System.Net.WebClient
    try {
        $client.DownloadFile($Uri, $OutFile)
    }
    finally {
        $client.Dispose()
    }
}

function Get-UriContent {
    param([Parameter(Mandatory)][string]$Uri)

    $client = New-Object System.Net.WebClient
    try {
        $client.DownloadString($Uri)
    }
    finally {
        $client.Dispose()
    }
}

$isInteractiveShell = Test-InteractiveShell
$debug = if ($null -ne $debug_Override) { [bool]$debug_Override } else { $false }
$repo_root = if ($repo_root_Override) { $repo_root_Override } else { 'https://raw.githubusercontent.com/ChrisTitusTech' }
$profileDir = Get-ProfileDir
$showHelpOnLaunch = if ($null -ne $show_help_Override) { [bool]$show_help_Override } else { $false }

function Debug-Message {
    if (Get-Command -Name 'Debug-Message_Override' -ErrorAction SilentlyContinue) {
        Debug-Message_Override
        return
    }

    Write-Host '#######################################' -ForegroundColor Red
    Write-Host '#           Debug mode enabled        #' -ForegroundColor Red
    Write-Host '#          ONLY FOR DEVELOPMENT       #' -ForegroundColor Red
    Write-Host '#######################################' -ForegroundColor Red
}

if ($debug) {
    Debug-Message
}

function Update-PowerShell {
    [CmdletBinding(SupportsShouldProcess)]
    param()

    if (Get-Command -Name 'Update-PowerShell_Override' -ErrorAction SilentlyContinue) {
        Update-PowerShell_Override @PSBoundParameters
        return
    }

    if (-not (Test-Command winget)) {
        Write-Warning 'winget is required to update PowerShell automatically.'
        return
    }

    try {
        $release = Invoke-RestMethod -Uri 'https://api.github.com/repos/PowerShell/PowerShell/releases/latest' -ErrorAction Stop
        $currentVersion = [version]$PSVersionTable.PSVersion
        $latestVersion = [version]($release.tag_name -replace '^v', '')

        if ($currentVersion -ge $latestVersion) {
            Write-Host "PowerShell $currentVersion is up to date." -ForegroundColor Green
            return
        }

        if ($PSCmdlet.ShouldProcess("PowerShell $currentVersion", "Upgrade to $latestVersion")) {
            winget upgrade --id Microsoft.PowerShell --exact --accept-source-agreements --accept-package-agreements
            if ($LASTEXITCODE -ne 0) {
                Write-Error "winget failed to update PowerShell. Exit code: $LASTEXITCODE"
                return
            }
            Write-Host 'PowerShell has been updated. Restart your shell to use the new version.' -ForegroundColor Magenta
        }
    }
    catch {
        Write-Error "Failed to update PowerShell. Error: $_"
    }
}

function Clear-Cache {
    [CmdletBinding(SupportsShouldProcess)]
    param()

    if (Get-Command -Name 'Clear-Cache_Override' -ErrorAction SilentlyContinue) {
        Clear-Cache_Override @PSBoundParameters
        return
    }

    $paths = @(
        "$env:SystemRoot\Prefetch\*",
        "$env:SystemRoot\Temp\*",
        "$env:TEMP\*",
        "$env:LOCALAPPDATA\Microsoft\Windows\INetCache\*"
    )

    foreach ($path in $paths) {
        if ($PSCmdlet.ShouldProcess($path, 'Remove cached files')) {
            Remove-Item -Path $path -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

# Terminal-Icons is imported on demand (Enable-Icons): importing it at startup
# cost ~1.7s because it rewrites every FileSystem formatting view.
$script:IconsLoaded = $false
function Enable-Icons {
    if ($script:IconsLoaded) { return }
    $script:IconsLoaded = $true
    Import-Module -Name Terminal-Icons -ErrorAction SilentlyContinue
}

function Initialize-OptionalModule {
    if (-not $isInteractiveShell) {
        return
    }

    # Chocolatey's helpers\chocolateyProfile.psm1 was imported here and cost
    # ~1s per launch just to add `choco` tab completion. Choco works fine via
    # its shim without it, so it stays out of the startup path.
    $null = $env:ChocolateyInstall
}

function Resolve-Editor {
    if ($EDITOR_Override) {
        return $EDITOR_Override
    }
    if ($env:EDITOR) {
        return $env:EDITOR
    }

    foreach ($candidate in 'nvim', 'pvim', 'vim', 'vi', 'code', 'codium', 'notepad++', 'sublime_text') {
        if (Test-FastCommand $candidate) {
            return $candidate
        }
    }

    return 'notepad'
}

Initialize-OptionalModule

$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
$EDITOR = Resolve-Editor
Set-Alias -Name vim -Value $EDITOR -Force

if ($isInteractiveShell) {
    try {
        $adminSuffix = if ($isAdmin) { ' [ADMIN]' } else { '' }
        $Host.UI.RawUI.WindowTitle = "PowerShell $($PSVersionTable.PSVersion)$adminSuffix"
    }
    catch {
        Write-Verbose "Unable to set console title: $_"
    }
}

function Edit-Profile {
    & $EDITOR $PROFILE.CurrentUserAllHosts
}
Set-Alias -Name ep -Value Edit-Profile -Force

function Invoke-Profile {
    . $PROFILE.CurrentUserCurrentHost
}

function touch {
    param([Parameter(Mandatory)][string]$File)

    if (Test-Path -Path $File) {
        (Get-Item -Path $File).LastWriteTime = Get-Date
    }
    else {
        New-Item -Path $File -ItemType File -Force | Out-Null
    }
}

function mkcd {
    param([Parameter(Mandatory)][string]$Path)
    New-Item -Path $Path -ItemType Directory -Force | Out-Null
    Set-Location -Path $Path
}

function ff {
    param([Parameter(Mandatory)][string]$Name)
    Get-ChildItem -Recurse -Filter "*$Name*" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty FullName
}

function pubip {
    (Get-UriContent -Uri 'https://ifconfig.me/ip').Trim()
}

function winutil {
    & ([ScriptBlock]::Create((Invoke-RestMethod -Uri 'https://christitus.com/win'))) @args
}

function winutildev {
    if (Get-Command -Name 'WinUtilDev_Override' -ErrorAction SilentlyContinue) {
        WinUtilDev_Override @args
        return
    }

    & ([ScriptBlock]::Create((Invoke-RestMethod -Uri 'https://christitus.com/windev'))) @args
}

function admin {
    $cwd = (Get-Location).ProviderPath
    $shell = if (Test-Command pwsh) { 'pwsh.exe' } else { 'powershell.exe' }
    $shellArgs = if ($args.Count -gt 0) { @('-NoExit', '-Command', ($args -join ' ')) } else { @('-NoExit') }

    if (Test-Command wt) {
        Start-Process wt -Verb RunAs -ArgumentList (@('-d', $cwd, $shell) + $shellArgs)
    }
    else {
        Start-Process $shell -Verb RunAs -WorkingDirectory $cwd -ArgumentList $shellArgs
    }
}
Set-Alias -Name su -Value admin -Force

function uptime {
    $boot = if (Get-Command Get-Uptime -ErrorAction SilentlyContinue) {
        Get-Uptime -Since
    }
    else {
        (Get-CimInstance -ClassName Win32_OperatingSystem).LastBootUpTime
    }

    (Get-Date) - $boot | Select-Object Days, Hours, Minutes, Seconds
}

function unzip {
    param([Parameter(Mandatory)][string]$File)

    if (-not (Test-Path -Path $File -PathType Leaf)) {
        Write-Error "File not found: $File"
        return
    }

    Expand-Archive -Path $File -DestinationPath (Get-Location) -Force
}

function grep {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory, Position = 0)][string]$Pattern,
        [Parameter(Position = 1)][string]$Path,
        [Parameter(ValueFromPipeline)][object]$InputObject
    )

    begin {
        $pipelineInput = [System.Collections.Generic.List[object]]::new()
    }

    process {
        if ($PSBoundParameters.ContainsKey('InputObject')) {
            $pipelineInput.Add($InputObject)
        }
    }

    end {
        if ($Path) {
            Get-ChildItem -Path $Path -Recurse -File -ErrorAction SilentlyContinue | Select-String -Pattern $Pattern
        }
        elseif ($pipelineInput.Count -gt 0) {
            $pipelineInput | Select-String -Pattern $Pattern
        }
        else {
            Write-Error 'Usage: grep <pattern> [path] or pipe input to grep'
        }
    }
}

function df { Get-Volume }

function sed {
    param(
        [Parameter(Mandatory)][string]$File,
        [Parameter(Mandatory)][string]$Find,
        [Parameter(Mandatory)][string]$Replace
    )

    (Get-Content -Path $File).Replace($Find, $Replace) | Set-Content -Path $File
}

function which {
    param([Parameter(Mandatory)][string]$Name)
    Get-Command -Name $Name | Select-Object -ExpandProperty Definition
}

function export {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Value
    )
    Set-Item -Path "env:$Name" -Value $Value -Force
}

function pkill {
    param([Parameter(Mandatory)][string]$Name)
    Get-Process -Name $Name -ErrorAction SilentlyContinue | Stop-Process -Force
}

function pgrep {
    param([Parameter(Mandatory)][string]$Name)
    Get-Process -Name $Name -ErrorAction SilentlyContinue
}

function head {
    param([Parameter(Mandatory)][string]$Path, [int]$n = 10)
    Get-Content -Path $Path -Head $n
}

function tail {
    param([Parameter(Mandatory)][string]$Path, [int]$n = 10, [switch]$f)
    Get-Content -Path $Path -Tail $n -Wait:$f
}

function nf {
    param([Parameter(Mandatory)][string]$Name)
    New-Item -ItemType File -Path . -Name $Name -Force | Out-Null
}

function trash {
    param([Parameter(Mandatory)][string]$Path)

    $resolvedPath = Resolve-Path -LiteralPath $Path -ErrorAction SilentlyContinue
    if (-not $resolvedPath) {
        Write-Error "Item not found: $Path"
        return
    }

    $fullPath = $resolvedPath.ProviderPath
    $item = Get-Item -LiteralPath $fullPath
    $parentPath = if ($item.PSIsContainer) {
        if ($item.Parent) { $item.Parent.FullName } else { Split-Path -Path $item.FullName -Parent }
    }
    else {
        $item.DirectoryName
    }

    if ([string]::IsNullOrWhiteSpace($parentPath)) {
        Write-Error "Cannot move root path to Recycle Bin: $fullPath"
        return
    }

    $shell = New-Object -ComObject 'Shell.Application'
    $shellFolder = $shell.NameSpace($parentPath)
    $shellItem = if ($shellFolder) { $shellFolder.ParseName($item.Name) } else { $null }

    if ($shellItem) {
        $shellItem.InvokeVerb('delete')
    }
    else {
        Write-Error "Could not move item to Recycle Bin: $fullPath"
    }
}

function docs {
    Set-Location -Path ([Environment]::GetFolderPath('MyDocuments'))
}

function dtop {
    Set-Location -Path ([Environment]::GetFolderPath('Desktop'))
}

function k9 { param([Parameter(Mandatory)][string]$Name) pkill $Name }
function la { Enable-Icons; Get-ChildItem | Format-Table -AutoSize }
function ll { Enable-Icons; Get-ChildItem -Force | Format-Table -AutoSize }
function gs { git status }
function ga { git add . }
function gc { git commit -m ($args -join ' ') }
function gpush { git push @args }
function gpull { git pull @args }
function gcl { git clone @args }
function profile {Set-Location -Path ~/OneDrive/Documents/PowerShell}
Set-Alias sf -Value superfile
function g {
    if (Get-Command __zoxide_z -ErrorAction SilentlyContinue) {
        __zoxide_z github
    }
    elseif (Test-Path -Path "$HOME\github") {
        Set-Location "$HOME\github"
    }
}

function gcom {
    git add .
    git commit -m ($args -join ' ')
}

function lazyg {
    git add .
    git commit -m ($args -join ' ')
    git push
}

function sysinfo { Get-ComputerInfo }

function flushdns {
    Clear-DnsClientCache
    Write-Host 'DNS has been flushed'
}

function cpy { Set-Clipboard ($args -join ' ') }
function pst { Get-Clipboard }

function Set-PSReadLineOptionsCompat {
    [CmdletBinding(SupportsShouldProcess)]
    param([Parameter(Mandatory)][hashtable]$Options)

    $safeOptions = $Options.Clone()
    if ($PSVersionTable.PSEdition -ne 'Core') {
        $safeOptions.Remove('PredictionSource')
        $safeOptions.Remove('PredictionViewStyle')
    }

    if ($PSCmdlet.ShouldProcess('PSReadLine', 'Set PSReadLine options')) {
        Set-PSReadLineOption @safeOptions
    }
}

function Set-PredictionSource {
    [CmdletBinding(SupportsShouldProcess)]
    param()

    if (Get-Command -Name 'Set-PredictionSource_Override' -ErrorAction SilentlyContinue) {
        Set-PredictionSource_Override
        return
    }

    if ($PSCmdlet.ShouldProcess('PSReadLine', 'Set prediction source')) {
        if ($PSVersionTable.PSEdition -eq 'Core') {
            Set-PSReadLineOption -PredictionSource HistoryAndPlugin
        }

        Set-PSReadLineOption -MaximumHistoryCount 10000
    }
}

function Initialize-PSReadLine {
    if (-not $isInteractiveShell) {
        return
    }
    if (-not (Get-Module -Name PSReadLine)) {
        try { Import-Module PSReadLine -ErrorAction Stop }
        catch { return }
    }

    $script:EscPressCount = 0
    $script:LastEscPressTime = [datetime]::MinValue
    $script:PendingSudo = $false

    $options = @{
        EditMode                      = 'Windows'
        HistoryNoDuplicates           = $true
        HistorySearchCursorMovesToEnd = $true
        PredictionSource              = 'HistoryAndPlugin'
        PredictionViewStyle           = 'ListView'
        BellStyle                     = 'None'
        Colors                        = @{
            Command   = '#87CEEB'
            Parameter = '#98FB98'
            Operator  = '#FFB6C1'
            Variable  = '#DDA0DD'
            String    = '#FFDAB9'
            Number    = '#B0E0E6'
            Type      = '#F0E68C'
            Comment   = '#D3D3D3'
            Keyword   = '#8367c7'
            Error     = '#FF6347'
        }
    }

    Set-PSReadLineOptionsCompat -Options $options
    Set-PSReadLineKeyHandler -Key UpArrow -Function HistorySearchBackward
    Set-PSReadLineKeyHandler -Key DownArrow -Function HistorySearchForward
    Set-PSReadLineKeyHandler -Key Tab -Function MenuComplete
    Set-PSReadLineKeyHandler -Chord 'Ctrl+d' -Function DeleteChar
    Set-PSReadLineKeyHandler -Chord 'Ctrl+w' -Function BackwardDeleteWord
    Set-PSReadLineKeyHandler -Chord 'Alt+d' -Function DeleteWord
    Set-PSReadLineKeyHandler -Chord 'Ctrl+LeftArrow' -Function BackwardWord
    Set-PSReadLineKeyHandler -Chord 'Ctrl+RightArrow' -Function ForwardWord
    Set-PSReadLineKeyHandler -Chord 'Ctrl+z' -Function Undo
    Set-PSReadLineKeyHandler -Chord 'Ctrl+y' -Function Redo

    Set-PSReadLineKeyHandler -Key Escape -ScriptBlock {
        $now = Get-Date
        if (($now - $script:LastEscPressTime).TotalMilliseconds -lt 500) {
            $script:EscPressCount++
        } else {
            $script:EscPressCount = 1
        }
        $script:LastEscPressTime = $now

        if ($script:EscPressCount -eq 3) {
            $lastCommand = (Get-History -Count 1).CommandLine
            if ($lastCommand) {
                [Microsoft.PowerShell.PSConsoleReadLine]::DeleteLine()
                [Microsoft.PowerShell.PSConsoleReadLine]::Insert("sudo run $lastCommand")
            }
            $script:EscPressCount = 0
        }
        elseif ($script:EscPressCount -eq 2) {
            # We wait for a potential 3rd press.
            # To implement the "timeout" without blocking the shell,
            # we'll trigger the 'sudo' action on the NEXT key press
            # if the count was 2.
            $script:PendingSudo = $true
        }
        else {
            # If we had a pending sudo and this press (or any other)
            # breaks the sequence, we apply it now.
            if ($script:PendingSudo) {
                $lastCommand = (Get-History -Count 1).CommandLine
                if ($lastCommand) {
                    [Microsoft.PowerShell.PSConsoleReadLine]::DeleteLine()
                    [Microsoft.PowerShell.PSConsoleReadLine]::Insert("sudo $lastCommand")
                }
                $script:PendingSudo = $false
            }
        }
    }

    Set-PSReadLineOption -AddToHistoryHandler {
        param([string]$line)
        $line -notmatch '(?i)(password|secret|token|apikey|connectionstring)'
    }

    Set-PredictionSource
}

function Register-CustomCompletion {
    if (-not $isInteractiveShell) {
        return
    }

    $completionMap = @{
        git  = @('status', 'add', 'commit', 'push', 'pull', 'clone', 'checkout')
        npm  = @('install', 'start', 'run', 'test', 'build')
        deno = @('run', 'compile', 'bundle', 'test', 'lint', 'fmt', 'cache', 'info', 'doc', 'upgrade')
    }

    Register-ArgumentCompleter -Native -CommandName git, npm, deno -ScriptBlock {
        param($wordToComplete, $commandAst, $cursorPosition)
        $null = $cursorPosition
        $completionWord = $wordToComplete
        $map = $completionMap
        $command = $commandAst.CommandElements[0].Value
        if ($map.ContainsKey($command)) {
            $map[$command] |
            Where-Object { $_ -like "$completionWord*" } |
            ForEach-Object { [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_) }
        }
    }.GetNewClosure()

    if (Test-Command dotnet) {
        Register-ArgumentCompleter -Native -CommandName dotnet -ScriptBlock {
            param($wordToComplete, $commandAst, $cursorPosition)
            $null = $wordToComplete
            dotnet complete --position $cursorPosition $commandAst.ToString() |
            ForEach-Object { [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_) }
        }
    }
}

function lockmyscreen {
    rundll32.exe user32.dll,LockWorkStation
}

# ============================================================================
# DELAYED SYSTEM ACTIONS — shutdown / reboot / lock / sleep / signout
# Duration syntax: plain seconds or e.g., 20s / 10m / 2h. Every action also
# accepts `cancel` to abort its pending instance:
#   shutdown cancel / reboot cancel -> shutdown.exe /a (aborts either
#                                      schedule; exit 0 = aborted, 1116 =
#                                      nothing was pending)
#   lock/sleep/signout cancel      -> kills the hidden wait worker below
# ============================================================================
function ConvertTo-DelaySeconds {
    param([Parameter(Mandatory)][string]$Time)

    $time = $Time.ToLower()
    if ($time -match '^(\d+)([smh])$') {
        $value = [int]$matches[1]
        switch ($matches[2]) {
            's' { return $value }
            'm' { return $value * 60 }
            'h' { return $value * 3600 }
        }
    }
    if ($time -match '^\d+$') {
        return [int]$time
    }

    throw 'Invalid time format. Use plain seconds or e.g., 20s, 10m, 2h.'
}

function Start-DelayedSystemAction {
    # Lock, sleep and sign-out have no native "/t seconds" switch, so a
    # detached hidden powershell.exe performs the wait. Detached via
    # Start-Process so closing this shell does not cancel it (matching how
    # shutdown.exe schedules survive). The trailing #hkdevs-delayed-* token
    # is the plaintext tag Stop-DelayedSystemAction hunts for. No quoting is
    # needed: powershell.exe treats everything after -Command as one script
    # and every token below is space-free, so they join to
    #   Start-Sleep -Seconds N ; <action> ; #hkdevs-delayed-<action>
    param(
        [Parameter(Mandatory)][ValidateSet('lock', 'sleep', 'signout')][string]$Action,
        [Parameter(Mandatory)][int]$Seconds
    )

    if ($Seconds -lt 1) {
        throw 'Delay must be at least 1 second.'
    }

    $tokens = switch ($Action) {
        'lock' {
            'Start-Sleep', '-Seconds', "$Seconds", ';',
            'rundll32.exe', 'user32.dll,LockWorkStation', ';', '#hkdevs-delayed-lock'
        }
        'sleep' {
            # SetSuspendState(False, True, False): standby, not hibernate.
            'Start-Sleep', '-Seconds', "$Seconds", ';',
            'rundll32.exe', 'powrprof.dll,SetSuspendState', '0,1,0', ';', '#hkdevs-delayed-sleep'
        }
        'signout' {
            # logoff.exe is the /l logoff of this console session.
            'Start-Sleep', '-Seconds', "$Seconds", ';',
            'logoff.exe', ';', '#hkdevs-delayed-signout'
        }
    }

    Start-Process -FilePath 'powershell.exe' -WindowStyle Hidden `
        -ArgumentList (@('-NoProfile', '-Command') + $tokens)
}

function Stop-DelayedSystemAction {
    # Kills the hidden wait workers carrying the #hkdevs-delayed-<action>
    # tag on their command line; returns how many were killed (0 = none
    # pending). WQL does the whole match, so no $ variables are involved;
    # %hk-delayed-probe9f7% keeps this query's own probe children (which run
    # with a plain `powershell -Command` and would otherwise self-match)
    # from being killed.
    param([ValidateSet('lock', 'sleep', 'signout')][string]$Action)

    $filter = "Name = 'powershell.exe' AND CommandLine LIKE '%hkdevs-delayed-$Action%' " +
        "AND NOT (CommandLine LIKE '%hk-delayed-probe9f7%')"
    $ids = Get-CimInstance -ClassName Win32_Process -Filter $filter -ErrorAction SilentlyContinue |
        Select-Object -ExpandProperty ProcessId
    $cancelled = 0
    foreach ($id in $ids) {
        Stop-Process -Id $id -Force -ErrorAction SilentlyContinue
        $cancelled++
    }
    return $cancelled
}

function shutdown {
    # `shutdown <time>` schedules; `shutdown cancel` aborts any pending
    # shutdown or restart schedule.
    param([string]$Operation)

    if ($Operation -match '^(?i)cancel$') {
        shutdown.exe /a 2>&1 | Out-Null
        if ($LASTEXITCODE -eq 0) {
            Write-Host 'Pending shutdown cancelled.' -ForegroundColor Green
        }
        else {
            Write-Host 'No pending shutdown to cancel.' -ForegroundColor Yellow
        }
        return
    }
    if ($null -eq $Operation) {
        Write-Host 'usage: shutdown [seconds | 20s|10m|2h | cancel]'
        return
    }

    shutdown.exe /s /f /t (ConvertTo-DelaySeconds $Operation)
}

function reboot {
    param([string]$Operation)

    if ($Operation -match '^(?i)cancel$') {
        shutdown.exe /a 2>&1 | Out-Null
        if ($LASTEXITCODE -eq 0) {
            Write-Host 'Pending restart cancelled.' -ForegroundColor Green
        }
        else {
            Write-Host 'No pending restart to cancel.' -ForegroundColor Yellow
        }
        return
    }
    if ($null -eq $Operation) {
        Write-Host 'usage: reboot [seconds | 20s|10m|2h | cancel]'
        return
    }

    shutdown.exe /r /f /t (ConvertTo-DelaySeconds $Operation)
}

function lock {
    # `lock <time>` schedules; `lock cancel` aborts; a bare `lock` locks now.
    param([string]$Time)

    if ($Time -match '^(?i)cancel$') {
        $cancelled = Stop-DelayedSystemAction -Action lock
        if ($cancelled -gt 0) {
            Write-Host "Cancelled $cancelled delayed lock job(s)." -ForegroundColor Green
        }
        else {
            Write-Host 'No delayed lock pending.' -ForegroundColor Yellow
        }
        return
    }
    if ($null -eq $Time) {
        lockmyscreen
        return
    }

    Start-DelayedSystemAction -Action lock -Seconds (ConvertTo-DelaySeconds $Time)
}

function sleep {
    # Overrides the built-in Start-Sleep alias: `sleep <time>` suspends the
    # system after the delay, `sleep cancel` aborts it. Use `Start-Sleep`
    # directly for in-session waits.
    param([string]$Time)

    if ($Time -match '^(?i)cancel$') {
        $cancelled = Stop-DelayedSystemAction -Action sleep
        if ($cancelled -gt 0) {
            Write-Host "Cancelled $cancelled delayed sleep job(s)." -ForegroundColor Green
        }
        else {
            Write-Host 'No delayed sleep pending.' -ForegroundColor Yellow
        }
        return
    }
    if ($null -eq $Time) {
        Write-Host 'usage: sleep [seconds | 20s|10m|2h | cancel]'
        return
    }

    Start-DelayedSystemAction -Action sleep -Seconds (ConvertTo-DelaySeconds $Time)
}

function signout {
    # `signout <time>` schedules a delayed sign-out, `signout cancel` aborts
    # it, and a bare `signout` signs off immediately (logoff.exe).
    param([string]$Time)

    if ($Time -match '^(?i)cancel$') {
        $cancelled = Stop-DelayedSystemAction -Action signout
        if ($cancelled -gt 0) {
            Write-Host "Cancelled $cancelled delayed sign-out job(s)." -ForegroundColor Green
        }
        else {
            Write-Host 'No delayed sign-out pending.' -ForegroundColor Yellow
        }
        return
    }
    if ($null -eq $Time) {
        logoff.exe
        return
    }

    Start-DelayedSystemAction -Action signout -Seconds (ConvertTo-DelaySeconds $Time)
}

# Aliases outrank functions in command resolution, so the built-in
# Start-Sleep alias would shadow the function above unless removed.
Remove-Item Alias:sleep -Force -ErrorAction SilentlyContinue

# Memory-friendly aliases: logout == signout, restart == reboot (both accept
# the same delay or `cancel`, so e.g. `logout cancel` and `restart cancel`
# behave exactly like their canonical verbs).
Set-Alias logout signout
Set-Alias restart reboot

function Show-Help {

@'
PowerShell Profile Help
=======================

Profile:
  Edit-Profile      Open the current user's all-hosts profile for editing.
  Invoke-Profile    Reload this profile in the current session.
  Update-PowerShell Check for the latest PowerShell release and update with winget.

Git:
  g                 Go to the GitHub directory with zoxide fallback.
  ga                git add .
  gc <message>      git commit -m <message>
  gcl <repo>        git clone <repo>
  gcom <message>    git add .; git commit -m <message>
  gp/gpush          git push
  gpull             git pull
  gs                git status
  lazyg <message>   git add .; git commit -m <message>; git push

Shortcuts:
  cpy <text>        Copy text to the clipboard.
  df                Show volume information.
  docs/dtop         Go to Documents/Desktop.
  ff <name>         Find files recursively by name.
  flushdns          Clear the DNS cache.
  grep <regex> [p]  Search files or piped input.
  head/tail         Show the first or last lines of a file.
  k9/pkill <name>   Kill processes by name.
  la/ll             List visible/all files.
  mkcd <dir>        Create and enter a directory.
  nf/touch <file>   Create a file.
  pgrep <name>      Find processes by name.
  pst               Paste clipboard text.
  sed <f> <a> <b>   Replace text in a file.
  sysinfo           Show system information.
  unzip <file>      Extract a zip file here.
  uptime            Show system uptime.
  which <name>      Show command path.
  shutdown <time>   Schedule forceful shutdown (e.g., 20s, 10m, 2h).
  reboot <time>     Schedule a restart using the same duration syntax.
  lock <time>       Lock the workstation after the delay.
  sleep <time>      Go to standby after the delay (overrides Start-Sleep).
  signout <time>    Sign out after the delay (bare: signs off now).
  <cmd> cancel      Abort: shutdown/reboot cancel -> shutdown /a; the
                    lock/sleep/signout pair kills its hidden wait worker.
  lockmyscreen      Lock the screen immediately.
  logout/restart    Same as signout/reboot (alias keywords for memory).
  winutil           Run the latest WinUtil release script.
  winutildev        Run the latest WinUtil prerelease script.
  music             Start mpd under ~/Music (if needed) and open ncmpc.
'@ | Write-Host
}

Set-Alias -Name gp -Value gpush -Force

# ============================================================================
# MUSIC — mpd under ~/Music + the ncmpc TUI (same `music` as nu / xonsh)
# ============================================================================
function Test-MpdUp {
    try {
        $client = [Net.Sockets.TcpClient]::new()
        $task = $client.ConnectAsync('127.0.0.1', 6600)
        $ok = $task.Wait(400) -and $client.Connected
        $client.Dispose()
        return [bool]$ok
    }
    catch {
        return $false
    }
}

function music {
    $musicDir = Join-Path $HOME 'Music'

    if (-not (Test-MpdUp)) {
        # Hidden window: mpd runs detached from this console.
        Start-Process -FilePath 'mpd' -WorkingDirectory $musicDir -WindowStyle Hidden
        foreach ($i in 1..15) {
            Start-Sleep -Milliseconds 300
            if (Test-MpdUp) { break }
        }
    }

    if (-not (Test-MpdUp)) {
        Write-Warning 'mpd is not responding on 127.0.0.1:6600'
        return
    }

    Push-Location $musicDir
    try { ncmpc @args } finally { Pop-Location }
}

Initialize-PSReadLine
Register-CustomCompletion

# ============================================================================
# CARAPACE — Fish-style completions for 600+ commands
# ============================================================================
$carapacePath = Join-Path $env:USERPROFILE 'scoop\shims\carapace.exe'
if (Test-Path $carapacePath) {
    Set-Alias -Name carapace -Value $carapacePath -Force
    $env:CARAPACE_BRIDGES = 'zsh,fish,bash,inshellisense'

    # Carapace's generated script registers hundreds of native-argument
    # completers; evaluating it up front added ~1s to every shell. It is now
    # loaded on the first Tab press, not on startup.
    $script:CarapaceCache = Join-Path $env:LOCALAPPDATA 'carapace\init\init-powershell-cached.ps1'
    $script:CarapaceLoaded = $false

    function Enable-Carapace {
        if ($script:CarapaceLoaded) { return }
        $script:CarapaceLoaded = $true
        $cacheItem = Get-Item -LiteralPath $script:CarapaceCache -ErrorAction SilentlyContinue
        $exeTime = (Get-Item -LiteralPath $carapacePath).LastWriteTimeUtc
        if (-not $cacheItem -or $cacheItem.LastWriteTimeUtc -lt $exeTime) {
            $null = New-Item -ItemType Directory -Path (Split-Path -Parent $script:CarapaceCache) -Force
            & $carapacePath _carapace powershell | Set-Content -LiteralPath $script:CarapaceCache -Encoding UTF8
        }
        Invoke-Expression (Get-Content -LiteralPath $script:CarapaceCache -Raw)
        # Universal fallback completer for any native command (current or future) via Carapace
        Register-ArgumentCompleter -Native -CommandName * -ScriptBlock {
            param($wordToComplete, $commandAst, $cursorPosition)
            if (-not (Get-Command carapace -ErrorAction SilentlyContinue)) { return }
            $commandElements = $commandAst.CommandElements
            $elems = @()
            foreach ($elem in $commandElements) {
                if ($elem.Extent.StartOffset -gt $cursorPosition) { break }
                $t = $elem.Extent.Text
                if ($elem.Extent.EndOffset -gt $cursorPosition) {
                    $t = $t.Substring(0, $elem.Extent.Text.Length - ($elem.Extent.EndOffset - $cursorPosition))
                }
                $elems += $t.Trim("'`"")
            }
            if ($elems.Count -eq 0) { return }
            $cmd = ($elems[0] -replace '\.exe$', '')
            try {
                $json = if (-not $wordToComplete) {
                    & carapace $cmd powershell @elems '' 2>$null
                } else {
                    & carapace $cmd powershell @elems 2>$null
                }
                if ($json) {
                    $parsed = $json | ConvertFrom-Json
                    $parsed | ForEach-Object {
                        [System.Management.Automation.CompletionResult]::new($_.CompletionText, $_.ListItemText, [System.Management.Automation.CompletionResultType]::ParameterValue, $_.ToolTip)
                    }
                }
            } catch { }
        }
        # Universal fallback completer for any native command (current or future) via Carapace
        Register-ArgumentCompleter -Native -CommandName * -ScriptBlock {
            param($wordToComplete, $commandAst, $cursorPosition)
            if (-not (Get-Command carapace -ErrorAction SilentlyContinue)) { return }
            $commandElements = $commandAst.CommandElements
            $elems = @()
            foreach ($elem in $commandElements) {
                if ($elem.Extent.StartOffset -gt $cursorPosition) { break }
                $t = $elem.Extent.Text
                if ($elem.Extent.EndOffset -gt $cursorPosition) {
                    $t = $t.Substring(0, $elem.Extent.Text.Length - ($elem.Extent.EndOffset - $cursorPosition))
                }
                $elems += $t.Trim("'`"")
            }
            if ($elems.Count -eq 0) { return }
            $cmd = ($elems[0] -replace '\.exe$', '')
            try {
                $json = if (-not $wordToComplete) {
                    & carapace $cmd powershell @elems '' 2>$null
                } else {
                    & carapace $cmd powershell @elems 2>$null
                }
                if ($json) {
                    $parsed = $json | ConvertFrom-Json
                    $parsed | ForEach-Object {
                        [System.Management.Automation.CompletionResult]::new($_.CompletionText, $_.ListItemText, [System.Management.Automation.CompletionResultType]::ParameterValue, $_.ToolTip)
                    }
                }
            } catch { }
        }
    }

    if ($isInteractiveShell) {
        Set-PSReadLineKeyHandler -Key Tab -ScriptBlock {
            Enable-Carapace
            [Microsoft.PowerShell.PSConsoleReadLine]::MenuComplete()
        }
    }
}

# ============================================================================
# OH-MY-POSH — half-life theme (cached init script for instant prompt load)
# ============================================================================
$env:POSH_THEME = Join-Path $env:USERPROFILE '.config\oh-my-posh\half-life.omp.json'
if ($isInteractiveShell -and (Test-Path $env:POSH_THEME)) {
    $poshCacheDir = Join-Path $env:LOCALAPPDATA 'oh-my-posh'
    $poshCache = Join-Path $poshCacheDir 'init-pwsh-half-life.ps1'
    if (-not (Test-Path $poshCacheDir)) {
        New-Item -ItemType Directory -Path $poshCacheDir -Force | Out-Null
    }
    if (-not (Test-Path $poshCache) -or (Get-Item $env:POSH_THEME).LastWriteTimeUtc -gt (Get-Item $poshCache).LastWriteTimeUtc) {
        oh-my-posh init pwsh --config $env:POSH_THEME | Set-Content -LiteralPath $poshCache -Encoding utf8
    }
    . $poshCache
}

if ($showHelpOnLaunch) {
    Show-Help
}
elseif ($isInteractiveShell) {
    Write-Host "Use 'Show-Help' to display help" -ForegroundColor Yellow
}


# Microsoft.WinGet.CommandNotFound intentionally not imported:
# it cost ~12s of the ~20s profile launch (it loads WinGet at startup).
. "C:\Users\haris\.config\yasb\tools\theme\yasb-theme-shell.ps1"


# ============================================================================
# FZF — Interactive Fuzzy Autocompletion & Suggestions
# ============================================================================
if (Get-Command fzf -ErrorAction SilentlyContinue) {
    if ($isInteractiveShell -and (Get-Module -Name PSReadLine -ErrorAction SilentlyContinue)) {
        # Ctrl+Space: Interactive fuzzy completion across all parameters, cmdlets, paths
        Set-PSReadLineKeyHandler -Chord 'Ctrl+Space' -ScriptBlock {
            Enable-Carapace
            Enable-Carapace
            if (Get-Command Enable-PSFzf -ErrorAction SilentlyContinue) { Enable-PSFzf }
            if (Get-Command Invoke-FzfTabCompletion -ErrorAction SilentlyContinue) {
                Invoke-FzfTabCompletion
            } else {
                [Microsoft.PowerShell.PSConsoleReadLine]::MenuComplete()
            }
        }
        # F2: Interactive Git branch switcher
        Set-PSReadLineKeyHandler -Key F2 -ScriptBlock {
            if (Get-Command Enable-PSFzf -ErrorAction SilentlyContinue) { Enable-PSFzf }
            if (Get-Command Invoke-PsFzfGitBranches -ErrorAction SilentlyContinue) {
                Invoke-PsFzfGitBranches
            }
        }
    }

    # Environment variables fuzzy finder
    function Invoke-FuzzyEnv {
        $selected = Get-ChildItem env: | ForEach-Object { "$($_.Name)=$($_.Value)" } | fzf --prompt 'Env❯ ' --height 40% --layout reverse --border rounded
        if ($selected) {
            $varName = ($selected -split '=')[0]
            if (Get-Module -Name PSReadLine -ErrorAction SilentlyContinue) {
                [Microsoft.PowerShell.PSConsoleReadLine]::Insert("`$env:$varName")
            } else {
                "`$env:$varName"
            }
        }
    }
    Set-Alias -Name fenv -Value Invoke-FuzzyEnv -Force

    # Git commit log browser with preview
    function Invoke-FuzzyGitLog {
        $hash = git log --oneline --color=always 2>$null | fzf --ansi --prompt 'Commit❯ ' --height 50% --layout reverse --border rounded --preview 'git show --color=always {1}'
        if ($hash) {
            $cleanHash = ($hash -split '\s+')[0]
            if (Get-Module -Name PSReadLine -ErrorAction SilentlyContinue) {
                [Microsoft.PowerShell.PSConsoleReadLine]::Insert($cleanHash)
            } else {
                $cleanHash
            }
        }
    }
    Set-Alias -Name fgl -Value Invoke-FuzzyGitLog -Force
}
