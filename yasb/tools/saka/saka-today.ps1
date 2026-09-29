# saka-today: emits today's Indian national (Saka) calendar date.
# YASB custom widget calls this; stdout becomes the widget label.
# Long form = era-correct: "Ashwin 4, Saka 1948"
$d = Get-Date
$exe = Join-Path $PSScriptRoot 'target\release\saka.exe'
$out = & $exe ('{0:yyyy-MM-dd}' -f $d) --long
Write-Output $out.Trim()
