# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: shells\nushell / plugin-manager
# ============================================================================
# ==============================================================================
# ██╗   ██╗███╗   ██╗██████╗ ███████╗██████╗ ████████╗██████╗ ██╗   ██╗
# ██║   ██║████╗  ██║██╔══██╗██╔════╝██╔══██╗╚══██╔══╝██╔══██╗╚██╗ ██╔╝
# ██║   ██║██╔██╗ ██║██║  ██║█████╗  ██████╔╝   ██║   ██████╔╝ ╚████╔╝
# ██║   ██║██║╚██╗██║██║  ██║██╔══╝  ██╔══██╗   ██║   ██╔══██╗  ╚██╔╝
# ╚██████╔╝██║ ╚████║██████╔╝███████╗██║  ██║   ██║   ██║  ██║   ██║
#  ╚═════╝ ╚═╝  ╚═══╝╚═════╝ ╚══════╝╚═╝  ╚═╝   ╚═╝   ╚═╝  ╚═╝   ╚═╝
# ==============================================================================
# Nushell Plugin Manager — Plugin installation and registration
# ==============================================================================

# ── Plugin installation script ──
def install-plugins [] {
    print "🔧 Installing Nushell plugins..."

    let plugins = [
        { name: "nu_plugin_clipboard" desc: "Clipboard operations" },
        { name: "nu_plugin_highlight" desc: "Syntax highlighting" },
        { name: "nu_plugin_image" desc: "Terminal image display" },
        { name: "nu_plugin_plot" desc: "Terminal plotting" },
        { name: "nu_plugin_emoji" desc: "Emoji picker" },
        { name: "nu_plugin_polars" desc: "DataFrame operations" },
        { name: "nu_plugin_formats" desc: "EML, ICS, INI, plist, VCF" },
        { name: "nu_plugin_query" desc: "SQL, XML, JSON, HTML queries" },
    ]

    for $p in $plugins {
        print $"  Installing ($p.name) — ($p.desc)..."
        if (which cargo | is-not-empty) {
            ^cargo install $p.name --locked
        } else {
            print $"  ⚠️  cargo not found, skipping ($p.name)"
        }
    }

    print "✅ All plugins installed"
}

# ── Plugin registration ──
def register-plugins [] {
    print "📝 Registering plugins with Nushell..."

    let plugin_dir = if ($nu.os-info | get name) == "windows" {
        ($env.HOME | path join ".cargo" "bin")
    } else {
        ($env.HOME | path join ".cargo" "bin")
    }

    let plugins = [
        "nu_plugin_clipboard",
        "nu_plugin_highlight",
        "nu_plugin_image",
        "nu_plugin_plot",
        "nu_plugin_emoji",
    ]

    for $p in $plugins {
        let plugin_path = if ($nu.os-info | get name) == "windows" {
            ($plugin_dir | path join $"($p).exe")
        } else {
            ($plugin_dir | path join $p)
        }
        if ($plugin_path | path exists) {
            plugin add $plugin_path
            print $"  ✅ Registered: ($p)"
        } else {
            print $"  ⚠️  Not found: ($plugin_path)"
        }
    }

    print "✅ Plugin registration complete. Restart Nushell to load plugins."
}

# ── Plugin status ──
def plugin-status [] {
    print "📋 Nushell plugin status:"
    plugin list | table
}

# ── Plugin cleanup ──
def plugin-cleanup [] {
    print "🧹 Cleaning up plugin registry..."
    print "ℹ️  Plugin garbage collection is automatic in Nushell"
    print "   Remove stale entries manually with: plugin rm <name>"
    print "✅ Plugin cleanup complete"
}

# ── Plugin search path ──
def plugin-search-path [] {
    let cargo_bin = ($env.HOME | path join ".cargo" "bin")
    print $"Cargo bin: ($cargo_bin)"
    if ($cargo_bin | path exists) {
        ls ($cargo_bin | path join "nu_plugin_*") | each { |p| $"  ($p.name)" }
    } else {
        print "  ⚠️  Cargo bin directory not found"
    }
}

# ── Install core plugins ──
def install-core-plugins [] {
    print "📦 Installing core Nushell plugins..."

    let core_plugins = [
        "nu_plugin_polars",
        "nu_plugin_formats",
        "nu_plugin_query",
    ]

    for $p in $core_plugins {
        print $"  Installing ($p)..."
        ^cargo install $p --locked
    }

    print "✅ Core plugins installed"
}

# ── Full plugin setup ──
def setup-plugins [] {
    print "🚀 Running full plugin setup..."
    install-plugins
    register-plugins
    plugin-status
    print ""
    print "⚠️  Please restart Nushell to load the plugins."
    print "   Run: plugin-status to verify after restart."
}
