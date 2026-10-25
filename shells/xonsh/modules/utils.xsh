# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: shells\xonsh\modules / utils
# ============================================================================
# ============================================================================
#  xonsh Modules - God Mode Component: Utilities & Functions
# ============================================================================
import os
import subprocess

def update_all():
    """Updates all scoop packages."""
    print("🚀 Updating Scoop packages...")
    subprocess.run(['scoop', 'update', '*'], shell=True)

def clean_system():
    """Cleans temporary files and caches."""
    print("🧹 Cleaning system...")
    subprocess.run(['scoop', 'cleanup', '*'], shell=True)

# Registering functions to the shell
globals()['update_all'] = update_all
globals()['clean_system'] = clean_system
