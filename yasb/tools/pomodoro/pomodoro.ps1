# pomodoro-tick: emits the bar label for the 55/5 focus timer.
# YASB custom widget calls this every second; stdout becomes the widget label.
# `tick` folds elapsed wall-clock time into the state file and prints the
# countdown, so no background process is needed.
$exe = Join-Path $PSScriptRoot 'target\release\pomodoro.exe'
$out = & $exe tick 2>$null
if ($LASTEXITCODE -ne 0 -or -not $out) { $out = '55:00' }
Write-Output ([string]$out).Trim()
