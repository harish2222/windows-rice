// ============================================================================
// Author -: HKDEVS
// Project -: Window-Rice
// Title -: WINDOW-RICE Installer Terminal UI
// ============================================================================
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use crate::app::App;
use crate::detect::{BinaryStatus, LinkStatus};

pub fn render(f: &mut Frame, app: &App) {
    let size = f.area();

    // Vertical layout: Header (3) -> PM status (3) -> Main Content (Min 10) -> Footer (3)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(3), // PM Status
            Constraint::Min(10),   // Dual Pane
            Constraint::Length(3), // Footer / Keybinds
        ])
        .split(size);

    render_header(f, chunks[0]);
    render_pm_status(f, app, chunks[1]);
    render_main_split(f, app, chunks[2]);
    render_footer(f, app, chunks[3]);
}

fn render_header(f: &mut Frame, area: Rect) {
    let header_text = vec![Line::from(vec![
        Span::styled("  WINDOW-RICE INSTALLER  ", Style::default().fg(Color::Rgb(30, 30, 46)).bg(Color::Rgb(137, 180, 250)).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::styled("Interactive Setup & Real-Time Rice Manager", Style::default().fg(Color::Rgb(205, 214, 244)).add_modifier(Modifier::BOLD)),
        Span::styled(" (Catppuccin Mocha Edition)", Style::default().fg(Color::Rgb(166, 227, 161))),
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
            Constraint::Percentage(45), // Left: Item List
            Constraint::Percentage(55), // Right: Details & Log
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
            Constraint::Percentage(50), // Top: Package Details
            Constraint::Percentage(50), // Bottom: Logs
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
    let start = app.logs.len().saturating_sub(height);
    let log_lines: Vec<Line> = app.logs[start..]
        .iter()
        .map(|l| {
            let color = if l.contains("[PASS]") || l.contains("[SYMLINK]") {
                Color::Rgb(166, 227, 161)
            } else if l.contains("[FAIL]") || l.contains("[ERROR]") {
                Color::Rgb(243, 139, 168)
            } else if l.contains("[COPIED]") || l.contains("[INFO]") {
                Color::Rgb(137, 180, 250)
            } else if l.contains("[WARN]") {
                Color::Rgb(249, 226, 175)
            } else {
                Color::Rgb(186, 194, 222)
            };
            Line::from(Span::styled(l, Style::default().fg(color)))
        })
        .collect();

    let p = Paragraph::new(log_lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!(" Activity Console ({} entries) ", app.logs.len()))
            .border_style(Style::default().fg(Color::Rgb(88, 91, 112))))
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let keys = vec![
        Span::styled("[↑/↓/j/k] ", Style::default().fg(Color::Rgb(137, 180, 250)).add_modifier(Modifier::BOLD)),
        Span::raw("Nav "),
        Span::styled("[Space] ", Style::default().fg(Color::Rgb(166, 227, 161)).add_modifier(Modifier::BOLD)),
        Span::raw("Toggle "),
        Span::styled("[a] ", Style::default().fg(Color::Rgb(180, 190, 254)).add_modifier(Modifier::BOLD)),
        Span::raw("All "),
        Span::styled("[m] ", Style::default().fg(Color::Rgb(249, 226, 175)).add_modifier(Modifier::BOLD)),
        Span::raw("Missing "),
        Span::styled("[l] ", Style::default().fg(Color::Rgb(148, 226, 213)).add_modifier(Modifier::BOLD)),
        Span::raw("Link "),
        Span::styled("[i] ", Style::default().fg(Color::Rgb(166, 227, 161)).add_modifier(Modifier::BOLD)),
        Span::raw("Install "),
        Span::styled("[r] ", Style::default().fg(Color::Rgb(203, 166, 247)).add_modifier(Modifier::BOLD)),
        Span::raw("Reload WM "),
        Span::styled("[u] ", Style::default().fg(Color::Rgb(137, 220, 235)).add_modifier(Modifier::BOLD)),
        Span::raw("Updates "),
        Span::styled("[w] ", Style::default().fg(Color::Rgb(249, 226, 175)).add_modifier(Modifier::BOLD)),
        Span::raw("Wallpapers "),
        Span::styled("[v] ", Style::default().fg(Color::Rgb(137, 180, 250)).add_modifier(Modifier::BOLD)),
        Span::raw("Verify "),
        Span::styled("[b] ", Style::default().fg(Color::Rgb(249, 226, 175)).add_modifier(Modifier::BOLD)),
        Span::raw("Boot "),
        Span::styled("[q] ", Style::default().fg(Color::Rgb(243, 139, 168)).add_modifier(Modifier::BOLD)),
        Span::raw("Quit"),
    ];

    let p = Paragraph::new(vec![
        Line::from(keys),
        Line::from(Span::styled(&app.status_message, Style::default().fg(Color::Rgb(205, 214, 244)))),
    ])
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Rgb(137, 180, 250))));

    f.render_widget(p, area);
}
