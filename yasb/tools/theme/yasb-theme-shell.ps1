# Easy terminal use of the bar theme switcher. Dot-source from a profile:
#   . "$env:USERPROFILE\.config\yasb\tools\theme\yasb-theme-shell.ps1"
#
#   yt list              # all themes, * marks active
#   yt current           # active theme name
#   yt set Nord          # activate (case-insensitive)
#   yt next | yt prev    # cycle file order (wraps)
function yt {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Args)
    & (Join-Path $PSScriptRoot 'yasb-theme.exe') @Args
}
