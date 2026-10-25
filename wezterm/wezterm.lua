-- =============================================================================
--  WezTerm — visually uniform with Windows Terminal, Otty, and Alacritty
--  Theme: Catppuccin Mocha on Windows 11 Acrylic Frosted Glass
-- =============================================================================

local wezterm = require("wezterm")
local mux = wezterm.mux
local act = wezterm.action
local config = wezterm.config_builder()

-- Maximized Focus on Startup
wezterm.on("gui-startup", function(cmd)
	local tab, pane, window = mux.spawn_window(cmd or {})
	window:gui_window():maximize()
end)

-- Renderer & Performance
config.front_end = "OpenGL"
config.max_fps = 144
config.animation_fps = 1
config.term = "xterm-256color"
config.prefer_egl = true

-- Font: Agave Nerd Font Mono
config.font = wezterm.font("Agave Nerd Font Mono")
config.font_size = 15.0
config.cell_width = 1.0

-- Cursor (SteadyUnderline matching Windows Terminal & Otty)
config.default_cursor_style = "SteadyUnderline"
config.cursor_blink_rate = 0

-- Windows 11 Frosted Glass (Acrylic with 0.90 opacity to eliminate wallpaper tint)
config.win32_system_backdrop = "Acrylic"
config.window_background_opacity = 0.90

-- Window Padding (8px uniform)
config.window_padding = {
	left = 8,
	right = 8,
	top = 8,
	bottom = 8,
}

-- Tabs & Frame
config.hide_tab_bar_if_only_one_tab = true
config.use_fancy_tab_bar = false
config.window_decorations = "NONE | RESIZE"
config.default_prog = { "nu.exe" }
config.initial_cols = 132
config.initial_rows = 50

-- Color Scheme: Catppuccin Mocha
config.color_scheme = "Catppuccin Mocha"

config.colors = {
	background = "#1E1E2E",
	foreground = "#CDD6F4",
	cursor_bg = "#F5E0DC",
	cursor_fg = "#1E1E2E",
	cursor_border = "#F5E0DC",
	selection_bg = "#F5E0DC",
	selection_fg = "#1E1E2E",

	tab_bar = {
		background = "#181825",
		active_tab = {
			bg_color = "#1E1E2E",
			fg_color = "#CDD6F4",
			intensity = "Bold",
			underline = "None",
			italic = false,
			strikethrough = false,
		},
		inactive_tab = {
			bg_color = "#181825",
			fg_color = "#A6ADC8",
			intensity = "Normal",
			underline = "None",
			italic = false,
			strikethrough = false,
		},
		new_tab = {
			bg_color = "#181825",
			fg_color = "#CDD6F4",
		},
	},
}

-- Keymaps
config.keys = {
	{
		key = "h",
		mods = "CTRL|SHIFT|ALT",
		action = wezterm.action.SplitPane({
			direction = "Right",
			size = { Percent = 50 },
		}),
	},
	{
		key = "v",
		mods = "CTRL|SHIFT|ALT",
		action = wezterm.action.SplitPane({
			direction = "Down",
			size = { Percent = 50 },
		}),
	},
	{
		key = "U",
		mods = "CTRL|SHIFT",
		action = act.AdjustPaneSize({ "Left", 5 }),
	},
	{
		key = "I",
		mods = "CTRL|SHIFT",
		action = act.AdjustPaneSize({ "Down", 5 }),
	},
	{
		key = "O",
		mods = "CTRL|SHIFT",
		action = act.AdjustPaneSize({ "Up", 5 }),
	},
	{
		key = "P",
		mods = "CTRL|SHIFT",
		action = act.AdjustPaneSize({ "Right", 5 }),
	},
	{ key = "9", mods = "CTRL", action = act.PaneSelect },
	{ key = "L", mods = "CTRL", action = act.ShowDebugOverlay },
	{
		key = "O",
		mods = "CTRL|ALT",
		action = wezterm.action_callback(function(window, _)
			local overrides = window:get_config_overrides() or {}
			if overrides.window_background_opacity == 1.0 then
				overrides.window_background_opacity = 0.90
			else
				overrides.window_background_opacity = 1.0
			end
			window:set_config_overrides(overrides)
		end),
	},
}

return config
