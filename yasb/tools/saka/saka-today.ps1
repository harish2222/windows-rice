# saka-today: emits today's Indian national (Saka) calendar date.
# YASB custom widget calls this; stdout becomes the widget label.
$d = Get-Date
$exe = Join-Path $PSScriptRoot 'target\release\saka.exe'
$out = & $exe ('{0:yyyy-MM-dd}' -f $d)
# prepend the Om glyph; Rust output stays pure "4 Ashwin 1948"
$glyph = [char]0x0950
Write-Output ("$glyph " + $out.Trim())
