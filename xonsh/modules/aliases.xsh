# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: xonsh\modules / aliases
# ============================================================================
# ============================================================================
#  xonsh Modules - God Mode Component: Aliases
# ============================================================================
# Comprehensive set of high-performance aliases.

# Navigation
aliases['ll'] = 'ls -la'
aliases['la'] = 'ls -A'
aliases['l'] = 'ls -CF'
aliases['..'] = 'cd ..'
aliases['...'] = 'cd ../..'
aliases['....'] = 'cd ../../../'

# System & Process (Modern tool replacements)
aliases['top'] = 'htop'
aliases['grep'] = 'rg'
aliases['cat'] = 'bat'
aliases['find'] = 'fd'

# Music Player Quick-Launch
aliases['kew-launch'] = 'alacritty -e kew'
aliases['cliamp-launch'] = 'alacritty -e cliamp'

# Git Power-Aliasing
aliases['gs'] = 'git status'
aliases['ga'] = 'git add'
aliases['gc'] = 'git commit -m'
aliases['gp'] = 'git push'
aliases['gl'] = 'git log --oneline --graph --all'
