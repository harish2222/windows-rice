// ============================================================================
// Author -: HKDEVS
// Project -: Window-Rice
// Title -: WINDOW-RICE Installer Terminal UI
// ============================================================================
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};
use crate::app::App;
use crate::detect::{BinaryStatus, LinkStatus};

pub fn render(f: &mut Frame, app: &App) {
    let size = f.area();

    // Vertical layout: Header (3) -> PM status (3) -> Main Content (Min 8) -> Progress/Status Gauge (3) -> Footer (3)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(3), // PM Status
            Constraint::Min(8),    // Dual Pane (Item List & Details/Console)
            Constraint::Length(3), // Real-time Progress Bar / Operational Status
            Constraint::Length(3), // Dynamic Footer & Keybindings
        ])
        .split(size);

    render_header(f, chunks[0]);
    render_pm_status(f, app, chunks[1]);
    render_main_split(f, app, chunks[2]);
    render_progress_bar(f, app, chunks[3]);
    render_footer(f, app, chunks[4]);

    // Full-revoke confirmation modal renders LAST as an overlay on everything.
    if app.confirm_uninstall {
        render_confirm_modal(f, app, size);
    }
}

/// Centered sub-rectangle used by overlay modals.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let pad_y = (100 - percent_y.min(100)) / 2;
    let pad_x = (100 - percent_x.min(100)) / 2;
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(pad_y),
            Constraint::Percentage(percent_y),
            Constraint::Percentage(pad_y),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(pad_x),
            Constraint::Percentage(percent_x),
            Constraint::Percentage(pad_x),
        ])
        .split(v[1])[1]
}

fn render_confirm_modal(f: &mut Frame, _app: &App, area: Rect) {
    let modal = centered_rect(72, 58, area);
    let red = Color::Rgb(243, 139, 168);

    f.render_widget(Clear, modal);
    let text = vec![
        Line::from(Span::styled(
            "                 FULL REVOKE - UNINSTALL EVERYTHING",
            Style::default().fg(red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("This removes EVERYTHING the Window-Rice installer placed on your PC:", Style::default().fg(Color::Rgb(205, 214, 244)))),
        Line::from(vec![
            Span::raw("  x "),
            Span::styled("Every config target installed by this repo (symlinks + copies)", Style::default().fg(Color::Rgb(249, 226, 175))),
        ]),
        Line::from(vec![
            Span::raw("  x "),
            Span::styled("Wallpaper entries the installer created (entries-only)", Style::default().fg(Color::Rgb(249, 226, 175))),
        ]),
        Line::from(""),
        Line::from(Span::styled("What stays SAFE - never touched:", Style::default().fg(Color::Rgb(166, 227, 161)))),
        Line::from("  + Program binaries (installed tools remain available)"),
        Line::from("  + Your own configs: user-modified copies are PRESERVED & archived"),
        Line::from("  + Repo-native targets (repo root == config location)"),
        Line::from("  + Any wallpaper folder containing your own media"),
        Line::from(""),
        Line::from(Span::styled("User files with backup records are restored to their originals.", Style::default().fg(Color::Rgb(148, 226, 213)))),
        Line::from(""),
        Line::from(vec![
            Span::styled(" [Y] YES, revoke everything ", Style::default().fg(Color::Rgb(30, 30, 46)).bg(red).add_modifier(Modifier::BOLD)),
            Span::raw("   "),
            Span::styled(" [N / Esc] Cancel ", Style::default().fg(Color::Rgb(205, 214, 244)).bg(Color::Rgb(49, 50, 68)).add_modifier(Modifier::BOLD)),
        ]),
    ];

    let block = Paragraph::new(text)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(" CONFIRM FULL REVOKE ")
            .border_style(Style::default().fg(red).add_modifier(Modifier::BOLD)))
        .wrap(Wrap { trim: false });
    f.render_widget(block, modal);
}

fn render_header(f: &mut Frame, area: Rect) {
    let header_text = vec![Line::from(vec![
        Span::styled("  WINDOW-RICE INSTALLER  ", Style::default().fg(Color::Rgb(30, 30, 46)).bg(Color::Rgb(137, 180, 250)).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::styled("Interactive Setup & Real-Time Rice Manager", Style::default().fg(Color::Rgb(205, 214, 244)).add_modifier(Modifier::BOLD)),
        Span::styled(" (Catppuccin Mocha Multi-Threaded Edition)", Style::default().fg(Color::Rgb(166, 227, 161))),
    ])];

    let p = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Rgb(137, 180, 250))));
    f.render_widget(p, area);
}

fn render_pm_status(f: &mut Frame, app: &App, area: Rect) {
    let pm = &app.pm_status;
    let mut pm_spans = vec![
        Span::raw(" PMs: "),
        badge("Scoop", pm.scoop),
        Span::raw(" "),
        badge("Winget", pm.winget),
        Span::raw(" "),
        badge("Choco", pm.choco),
        Span::raw(" "),
        badge("Cargo", pm.cargo),
        Span::raw(" "),
        badge("UV", pm.uv),
        Span::raw(" "),
        badge("Vcpkg", pm.vcpkg),
        Span::raw(" "),
        badge("Python", pm.python),
        Span::raw(" "),
        badge("NPM", pm.npm),
        Span::raw("  │  "),
    ];

    if let Some(status) = &app.git_update_status {
        if status.contains("UP-TO-DATE") {
            pm_spans.push(Span::styled(format!("[REPO: {}]", status), Style::default().fg(Color::Rgb(166, 227, 161)).add_modifier(Modifier::BOLD)));
        } else {
            pm_spans.push(Span::styled(format!("[REPO: {}]", status), Style::default().fg(Color::Rgb(249, 226, 175)).add_modifier(Modifier::BOLD)));
        }
    } else {
        pm_spans.push(Span::styled("[REPO: READY]", Style::default().fg(Color::Rgb(148, 226, 213))));
    }

    let p = Paragraph::new(Line::from(pm_spans))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Rgb(88, 91, 112))));
    f.render_widget(p, area);
}

fn badge(name: &str, ok: bool) -> Span<'static> {
    if ok {
        Span::styled(format!("[{} OK]", name), Style::default().fg(Color::Rgb(166, 227, 161)).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(format!("[{} MISSING]", name), Style::default().fg(Color::Rgb(243, 139, 168)))
    }
}

fn render_main_split(f: &mut Frame, app: &App, area: Rect) {
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(42), // Left: Item List
            Constraint::Percentage(58), // Right: Details & Activity Console
        ])
        .split(area);

    render_item_list(f, app, h_chunks[0]);
    render_right_pane(f, app, h_chunks[1]);
}

fn render_item_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let is_current = idx == app.selected_index;
            let check_box = if item.selected { "[x] " } else { "[ ] " };

            let bin_badge = match &item.inspection.binary_status {
                BinaryStatus::Installed(_) => Span::styled("BIN", Style::default().fg(Color::Rgb(166, 227, 161))),
                BinaryStatus::Missing => Span::styled("NO-BIN", Style::default().fg(Color::Rgb(243, 139, 168))),
            };

            let link_badge = match &item.inspection.link_status {
                LinkStatus::LinkedSymlink(_) => Span::styled("LINK", Style::default().fg(Color::Rgb(148, 226, 213))),
                LinkStatus::LinkedCopy(_) => Span::styled("COPY", Style::default().fg(Color::Rgb(137, 180, 250))),
                LinkStatus::NotLinked => Span::styled("UNLINK", Style::default().fg(Color::Rgb(249, 226, 175))),
                LinkStatus::NotApplicable => Span::styled("---", Style::default().fg(Color::Rgb(108, 112, 134))),
            };

            let prefix_style = if is_current {
                Style::default().fg(Color::Rgb(137, 180, 250)).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Rgb(205, 214, 244))
            };

            let spans = vec![
                Span::styled(check_box, if item.selected { Style::default().fg(Color::Rgb(166, 227, 161)).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Rgb(108, 112, 134)) }),
                Span::styled(format!("{:<18} ", item.package.name), prefix_style),
                Span::raw("["),
                bin_badge,
                Span::raw(" | "),
                link_badge,
                Span::raw("]"),
            ];

            let row_style = if is_current {
                Style::default().bg(Color::Rgb(49, 50, 68))
            } else {
                Style::default()
            };

            ListItem::new(Line::from(spans)).style(row_style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!(" Packages & Configs ({}/{}) ", app.items.iter().filter(|i| i.selected).count(), app.items.len()))
            .border_style(Style::default().fg(Color::Rgb(180, 190, 254))));

    f.render_widget(list, area);
}

fn render_right_pane(f: &mut Frame, app: &App, area: Rect) {
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(45), // Top: Package Details
            Constraint::Percentage(55), // Bottom: Live Streaming Activity Console
        ])
        .split(area);

    render_details(f, app, v_chunks[0]);
    render_logs(f, app, v_chunks[1]);
}

fn render_details(f: &mut Frame, app: &App, area: Rect) {
    if let Some(item) = app.items.get(app.selected_index) {
        let pkg = &item.package;
        let mut text = Vec::new();

        text.push(Line::from(vec![
            Span::styled("Name:        ", Style::default().fg(Color::Rgb(137, 180, 250)).add_modifier(Modifier::BOLD)),
            Span::styled(pkg.name, Style::default().fg(Color::Rgb(205, 214, 244)).add_modifier(Modifier::BOLD)),
            Span::styled(format!("  [{}]", pkg.category.as_str()), Style::default().fg(Color::Rgb(180, 190, 254))),
        ]));

        text.push(Line::from(vec![
            Span::styled("Description: ", Style::default().fg(Color::Rgb(137, 180, 250))),
            Span::styled(pkg.description, Style::default().fg(Color::Rgb(186, 194, 222))),
        ]));

        let bin_info = match &item.inspection.binary_status {
            BinaryStatus::Installed(p) => Span::styled(format!("[INSTALLED] {}", p.display()), Style::default().fg(Color::Rgb(166, 227, 161))),
            BinaryStatus::Missing => Span::styled("[NOT FOUND IN PATH]", Style::default().fg(Color::Rgb(243, 139, 168)).add_modifier(Modifier::BOLD)),
        };
        text.push(Line::from(vec![
            Span::styled("Binary:      ", Style::default().fg(Color::Rgb(137, 180, 250))),
            bin_info,
        ]));

        text.push(Line::from(vec![
            Span::styled("Configs:     ", Style::default().fg(Color::Rgb(137, 180, 250))),
            Span::styled(if pkg.configs.is_empty() { "No config files mapped (standalone tool)" } else { "Source -> Target" }, Style::default().fg(Color::Rgb(148, 226, 213))),
        ]));

        for cfg in &pkg.configs {
            text.push(Line::from(vec![
                Span::raw("  • "),
                Span::styled(cfg.source_rel, Style::default().fg(Color::Rgb(249, 226, 175))),
                Span::raw(" -> "),
                Span::styled(cfg.target_pattern, Style::default().fg(Color::Rgb(205, 214, 244))),
            ]));
        }

        text.push(Line::from(vec![
            Span::styled("Install Cmds:", Style::default().fg(Color::Rgb(137, 180, 250))),
        ]));
        if let Some(cmd) = pkg.scoop_cmd {
            text.push(Line::from(vec![Span::raw("  • Scoop:  "), Span::styled(cmd, Style::default().fg(Color::Rgb(166, 227, 161)))]));
        }
        if let Some(cmd) = pkg.winget_cmd {
            text.push(Line::from(vec![Span::raw("  • Winget: "), Span::styled(cmd, Style::default().fg(Color::Rgb(137, 180, 250)))]));
        }
        if let Some(cmd) = pkg.cargo_cmd {
            text.push(Line::from(vec![Span::raw("  • Cargo:  "), Span::styled(cmd, Style::default().fg(Color::Rgb(249, 226, 175)))]));
        }
        if let Some(cmd) = pkg.choco_cmd {
            text.push(Line::from(vec![Span::raw("  • Choco:  "), Span::styled(cmd, Style::default().fg(Color::Rgb(180, 190, 254)))]));
        }

        let p = Paragraph::new(text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(" Item Details ")
                .border_style(Style::default().fg(Color::Rgb(137, 180, 250))))
            .wrap(Wrap { trim: false });
        f.render_widget(p, area);
    }
}

fn render_logs(f: &mut Frame, app: &App, area: Rect) {
    let height = area.height.saturating_sub(2) as usize;
    let total_logs = app.logs.len();

    let end_idx = total_logs.saturating_sub(app.log_scroll_offset);
    let start_idx = end_idx.saturating_sub(height);

    let visible_logs = if start_idx < end_idx && end_idx <= total_logs {
        &app.logs[start_idx..end_idx]
    } else if !app.logs.is_empty() {
        let s = total_logs.saturating_sub(height);
        &app.logs[s..]
    } else {
        &[]
    };

    let log_lines: Vec<Line> = visible_logs
        .iter()
        .map(|l| {
            let color = if l.contains("[PASS]") || l.contains("[SYMLINK]") || l.contains("[COMPLETED") {
                Color::Rgb(166, 227, 161)
            } else if l.contains("[FAIL]") || l.contains("[ERROR]") || l.contains("[ERR]") {
                Color::Rgb(243, 139, 168)
            } else if l.contains("[COPIED]") || l.contains("[INFO]") || l.contains("[STEP") || l.contains("[START") {
                Color::Rgb(137, 180, 250)
            } else if l.contains("[WARN]") {
                Color::Rgb(249, 226, 175)
            } else if l.contains("[OUT]") {
                Color::Rgb(180, 190, 254)
            } else if l.contains("[CANCEL") {
                Color::Rgb(235, 160, 172)
            } else {
                Color::Rgb(186, 194, 222)
            };
            Line::from(Span::styled(l, Style::default().fg(color)))
        })
        .collect();

    let scroll_indicator = if app.log_scroll_offset > 0 {
        format!(" (scroll: -{}) ", app.log_scroll_offset)
    } else {
        " (live) ".to_string()
    };

    let p = Paragraph::new(log_lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!(" Activity & Debug Console ({} entries){} ", total_logs, scroll_indicator))
            .border_style(if app.is_busy { Style::default().fg(Color::Rgb(137, 180, 250)) } else { Style::default().fg(Color::Rgb(88, 91, 112)) }))
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn render_progress_bar(f: &mut Frame, app: &App, area: Rect) {
    if app.is_busy {
        let task_name = app.active_task_name.as_deref().unwrap_or("Processing");
        let title = format!(" [ACTIVE TASK] {} ({}%) ", task_name, app.progress_percentage);
        let detail = if !app.progress_detail.is_empty() {
            format!("{} | {}", app.progress_label, app.progress_detail)
        } else {
            app.progress_label.clone()
        };

        let gauge = Gauge::default()
            .block(Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Rgb(137, 180, 250)).add_modifier(Modifier::BOLD)))
            .gauge_style(Style::default().fg(Color::Rgb(166, 227, 161)).bg(Color::Rgb(49, 50, 68)))
            .percent(app.progress_percentage.min(100))
            .label(Span::styled(
                detail,
                Style::default().fg(Color::Rgb(205, 214, 244)).add_modifier(Modifier::BOLD)
            ));
        f.render_widget(gauge, area);
    } else {
        let status_text = if app.status_message.is_empty() {
            "Ready. Select components with [Space] and press [i] to Install or [l] to Link configs."
        } else {
            &app.status_message
        };

        let p = Paragraph::new(Line::from(vec![
            Span::styled(" [IDLE] ", Style::default().fg(Color::Rgb(30, 30, 46)).bg(Color::Rgb(166, 227, 161)).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(status_text, Style::default().fg(Color::Rgb(205, 214, 244))),
        ]))
        .block(Block::default()
            .borders(Borders::ALL)
            .title(" Operational Status ")
            .border_style(Style::default().fg(Color::Rgb(88, 91, 112))));
        f.render_widget(p, area);
    }
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let keys = if app.is_busy {
        vec![
            Span::styled("[Esc] ", Style::default().fg(Color::Rgb(243, 139, 168)).add_modifier(Modifier::BOLD)),
            Span::raw("Cancel/Abort Task  "),
            Span::styled("[↑/↓/j/k] ", Style::default().fg(Color::Rgb(137, 180, 250)).add_modifier(Modifier::BOLD)),
            Span::raw("Navigate List  "),
            Span::styled("[Space] ", Style::default().fg(Color::Rgb(166, 227, 161)).add_modifier(Modifier::BOLD)),
            Span::raw("Toggle  "),
            Span::styled("[[/]] ", Style::default().fg(Color::Rgb(249, 226, 175)).add_modifier(Modifier::BOLD)),
            Span::raw("Scroll Console  "),
            Span::styled("[q] ", Style::default().fg(Color::Rgb(243, 139, 168)).add_modifier(Modifier::BOLD)),
            Span::raw("Quit"),
        ]
    } else {
        vec![
            Span::styled("[↑/↓] ", Style::default().fg(Color::Rgb(137, 180, 250)).add_modifier(Modifier::BOLD)),
            Span::raw("Nav "),
            Span::styled("[Space/Enter] ", Style::default().fg(Color::Rgb(166, 227, 161)).add_modifier(Modifier::BOLD)),
            Span::raw("Toggle "),
            Span::styled("[a] ", Style::default().fg(Color::Rgb(180, 190, 254)).add_modifier(Modifier::BOLD)),
            Span::raw("All "),
            Span::styled("[m] ", Style::default().fg(Color::Rgb(249, 226, 175)).add_modifier(Modifier::BOLD)),
            Span::raw("Missing "),
            Span::styled("[i] ", Style::default().fg(Color::Rgb(166, 227, 161)).add_modifier(Modifier::BOLD)),
            Span::raw("Install "),
            Span::styled("[l] ", Style::default().fg(Color::Rgb(148, 226, 213)).add_modifier(Modifier::BOLD)),
            Span::raw("Link "),
            Span::styled("[u/x] ", Style::default().fg(Color::Rgb(243, 139, 168)).add_modifier(Modifier::BOLD)),
            Span::raw("Unlink "),
            Span::styled("[b] ", Style::default().fg(Color::Rgb(249, 226, 175)).add_modifier(Modifier::BOLD)),
            Span::raw("Boot "),
            Span::styled("[r] ", Style::default().fg(Color::Rgb(203, 166, 247)).add_modifier(Modifier::BOLD)),
            Span::raw("Reload "),
            Span::styled("[c] ", Style::default().fg(Color::Rgb(137, 220, 235)).add_modifier(Modifier::BOLD)),
            Span::raw("Update "),
            Span::styled("[v] ", Style::default().fg(Color::Rgb(137, 180, 250)).add_modifier(Modifier::BOLD)),
            Span::raw("Verify "),
            Span::styled("[0] ", Style::default().fg(Color::Rgb(243, 139, 168)).add_modifier(Modifier::BOLD)),
            Span::raw("Revoke "),
            Span::styled("[[/]] ", Style::default().fg(Color::Rgb(249, 226, 175)).add_modifier(Modifier::BOLD)),
            Span::raw("Scroll "),
            Span::styled("[q] ", Style::default().fg(Color::Rgb(243, 139, 168)).add_modifier(Modifier::BOLD)),
            Span::raw("Quit"),
        ]
    };

    let p = Paragraph::new(vec![
        Line::from(keys),
        Line::from(Span::styled(&app.status_message, Style::default().fg(Color::Rgb(205, 214, 244)))),
    ])
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Rgb(137, 180, 250))));

    f.render_widget(p, area);
}
