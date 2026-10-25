# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: xonsh\modules / theme
# ============================================================================
# ============================================================================
#  xonsh Modules - God Mode Component: Prompt & Theme
# ============================================================================
import os

def prompt():
    # Catppuccin Mocha colors for the prompt
    user_color = "\x1b[38;2;203;166;247m" # Mauve
    path_color = "\x1b[38;2;137;180;250m" # Blue
    reset = "\x1b[0m"
    return f"{user_color}{reset} {path_color}{os.getcwd().replace(os.environ['HOME'], '~')}{reset} {user_color}{reset} "

# Apply the prompt to xonsh options
options.prompt_text = prompt
