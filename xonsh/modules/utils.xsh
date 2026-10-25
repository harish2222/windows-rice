# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: Xonsh Shell Utility Functions and Filesystem Navigation Helpers
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
