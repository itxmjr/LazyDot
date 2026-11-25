use crate::core::model::{EntryState, Occupant, PackageStatus};
use crate::tui::app::{App, Focus, Level, Mode};
use crate::tui::theme;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App) {
    let [main, status] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(f.area());
    let [left, middle, right] = Layout::horizontal([
        Constraint::Percentage(25),
        Constraint::Percentage(30),
        Constraint::Percentage(45),
    ])
    .areas(main);
    let [preview, log] = Layout::vertical([Constraint::Min(5), Constraint::Length(8)]).areas(right);

    draw_packages(f, app, left);
    draw_entries(f, app, middle);
    draw_preview(f, app, preview);
    draw_log(f, app, log);
    draw_status_bar(f, app, status);

    match &app.mode {
        Mode::Help => draw_help(f),
        Mode::Confirm(_) => draw_confirm(f, app),
        Mode::Normal => {}
    }
}

fn panel(title: String, active: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(theme::border(active))
        .title(Span::styled(title, theme::title(active)))
}

fn status_badge(status: PackageStatus) -> (&'static str, Style) {
    match status {
        PackageStatus::Stowed => ("✓", Style::default().fg(theme::ACCENT)),
        PackageStatus::Partial => ("◐", Style::default().fg(theme::WARN)),
        PackageStatus::Unstowed => ("○", Style::default().fg(theme::INFO)),
        PackageStatus::Conflict => ("!", Style::default().fg(theme::ERROR)),
        PackageStatus::Untracked => ("+", Style::default().fg(theme::DIM)),
    }
}

fn draw_packages(f: &mut Frame, app: &mut App, area: Rect) {
    let active = app.focus == Focus::Packages && app.mode == Mode::Normal;
    let mut items = Vec::new();
    for pkg in &app.packages {
        let (icon, style) = status_badge(pkg.status());
        let name_style = if pkg.tracked {
            Style::default()
        } else {
            Style::default().fg(theme::DIM)
        };
        items.push(ListItem::new(Line::from(vec![
            Span::styled(format!("{icon} "), style),
            Span::styled(pkg.name.clone(), name_style),
        ])));
    }
    let tracked = app.packages.iter().filter(|p| p.tracked).count();
    let title = format!(" [1] Packages {tracked}/{} ", app.packages.len());
    let list = if items.is_empty() {
        List::new(vec![ListItem::new(Span::styled(
            "No configs found",
            Style::default().fg(theme::DIM),
        ))])
    } else {
        List::new(items)
    };
    f.render_stateful_widget(
        list.block(panel(title, active))
            .highlight_style(theme::selected()),
        area,
        &mut app.package_state,
    );
}

fn draw_entries(f: &mut Frame, app: &mut App, area: Rect) {
    let active = app.focus == Focus::Files && app.mode == Mode::Normal;
    let config = &app.config;
    let items: Vec<ListItem> = app
        .selected_package()
        .map(|pkg| {
            pkg.entries
                .iter()
                .map(|e| {
                    let (icon, color, note) = match &e.state {
                        EntryState::Linked => ("✓", theme::ACCENT, String::new()),
                        EntryState::NotLinked => ("○", theme::INFO, " (not linked)".into()),
                        EntryState::Conflict(Occupant::File) => {
                            ("!", theme::ERROR, " (file in the way)".into())
                        }
                        EntryState::Conflict(Occupant::Directory) => {
                            ("!", theme::ERROR, " (folder in the way)".into())
                        }
                        EntryState::Conflict(Occupant::Symlink(_)) => {
                            ("!", theme::ERROR, " (other link in the way)".into())
                        }
                        EntryState::Untracked => ("+", theme::DIM, String::new()),
                        EntryState::ForeignLink(_) => {
                            ("~", theme::WARN, " (symlink, other tool)".into())
                        }
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(format!("{icon} "), Style::default().fg(color)),
                        Span::raw(config.display_path(&e.target)),
                        Span::styled(note, Style::default().fg(theme::DIM)),
                    ]))
                })
                .collect()
        })
        .unwrap_or_default();
    let title = match app.selected_package() {
        Some(p) => format!(" [2] {} · {} ", p.name, p.status().label()),
        None => " [2] Files ".to_string(),
    };
    f.render_stateful_widget(
        List::new(items)
            .block(panel(title, active))
            .highlight_style(theme::selected()),
        area,
        &mut app.file_state,
    );
}

fn draw_preview(f: &mut Frame, app: &App, area: Rect) {
    let title = app
        .selected_entry()
        .map(|e| format!(" Preview · {} ", app.config.display_path(&e.target)))
        .unwrap_or_else(|| " Preview ".into());
    f.render_widget(
        Paragraph::new(app.preview.as_str())
            .block(panel(title, false))
            .wrap(Wrap { trim: false })
            .scroll((app.preview_scroll, 0)),
        area,
    );
}

fn draw_log(f: &mut Frame, app: &App, area: Rect) {
    let visible = area.height.saturating_sub(2) as usize;
    let lines: Vec<Line> = app
        .log
        .iter()
        .skip(app.log.len().saturating_sub(visible))
        .map(|l| {
            let color = match l.level {
                Level::Info => theme::DIM,
                Level::Success => theme::ACCENT,
                Level::Warn => theme::WARN,
                Level::Error => theme::ERROR,
            };
            Line::from(Span::styled(l.text.clone(), Style::default().fg(color)))
        })
        .collect();
    f.render_widget(
        Paragraph::new(lines)
            .block(panel(" Log ".into(), false))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn hint(key: &str, label: &str) -> [Span<'static>; 2] {
    [
        Span::styled(format!(" {key} "), Style::default().fg(theme::KEY)),
        Span::styled(format!("{label} "), Style::default().fg(theme::DIM)),
    ]
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let mut spans: Vec<Span> = Vec::new();
    match &app.mode {
        Mode::Confirm(_) => {
            spans.extend(hint("y/enter", "confirm"));
            spans.extend(hint("n/esc", "cancel"));
        }
        Mode::Help => spans.extend(hint("esc", "close")),
        Mode::Normal => {
            match app.selected_package().map(|p| p.status()) {
                Some(PackageStatus::Untracked) => spans.extend(hint("a", "adopt")),
                Some(PackageStatus::Stowed) => {
                    spans.extend(hint("u", "unstow"));
                    spans.extend(hint("R", "restore"));
                }
                Some(_) => {
                    spans.extend(hint("s", "stow"));
                    spans.extend(hint("u", "unstow"));
                    spans.extend(hint("R", "restore"));
                }
                None => {}
            }
            spans.extend(hint("r", "rescan"));
            spans.extend(hint("?", "help"));
            spans.extend(hint("q", "quit"));
        }
    }
    let version = format!(
        "{}v{} ",
        if app.dry_run { "DRY RUN · " } else { "" },
        env!("CARGO_PKG_VERSION")
    );
    let [left, right] = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(version.len() as u16 + 2),
    ])
    .areas(area);
    f.render_widget(Paragraph::new(Line::from(spans)), left);
    f.render_widget(
        Paragraph::new(Span::styled(version, Style::default().fg(theme::DIM))).right_aligned(),
        right,
    );
}

fn draw_confirm(f: &mut Frame, app: &App) {
    let Mode::Confirm(plan) = &app.mode else {
        return;
    };
    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "The following changes will be made:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    for action in &plan.actions {
        lines.push(Line::from(format!("  {}", action.describe(&app.config))));
    }
    if !plan.skipped.is_empty() {
        lines.push(Line::from(""));
        for s in &plan.skipped {
            lines.push(Line::from(Span::styled(
                format!("  skip: {s}"),
                Style::default().fg(theme::WARN),
            )));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("y", Style::default().fg(theme::KEY)),
        Span::raw(" confirm   "),
        Span::styled("n", Style::default().fg(theme::KEY)),
        Span::raw(" cancel"),
    ]));

    let height = (lines.len() as u16 + 2).min(f.area().height.saturating_sub(2));
    let area = centered(f.area(), 80, height);
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme::WARN))
                    .title(Span::styled(
                        format!(" {} ", plan.title),
                        Style::default()
                            .fg(theme::WARN)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}

const HELP: &[(&str, &str)] = &[
    ("j / k  ↓ / ↑", "Move selection"),
    ("g / G", "Jump to top / bottom"),
    ("h / l  Tab", "Switch between packages and files"),
    ("J / K  PgDn / PgUp", "Scroll preview"),
    ("a", "Adopt: move into repo and symlink back"),
    ("s", "Stow: link repo files into $HOME (backs up conflicts)"),
    ("u / D", "Unstow: remove symlinks (repo untouched)"),
    ("R", "Restore: move files back out of the repo"),
    ("r", "Rescan disk"),
    ("?", "Toggle this help"),
    ("q / Esc / Ctrl-c", "Quit"),
];

fn draw_help(f: &mut Frame) {
    let mut lines: Vec<Line> = HELP
        .iter()
        .map(|(k, d)| {
            Line::from(vec![
                Span::styled(format!("{k:<20}"), Style::default().fg(theme::KEY)),
                Span::raw(*d),
            ])
        })
        .collect();
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("✓", Style::default().fg(theme::ACCENT)),
        Span::raw(" stowed  "),
        Span::styled("◐", Style::default().fg(theme::WARN)),
        Span::raw(" partial  "),
        Span::styled("○", Style::default().fg(theme::INFO)),
        Span::raw(" unstowed  "),
        Span::styled("!", Style::default().fg(theme::ERROR)),
        Span::raw(" conflict  "),
        Span::styled("+", Style::default().fg(theme::DIM)),
        Span::raw(" untracked"),
    ]));
    let area = centered(f.area(), 76, lines.len() as u16 + 2);
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).block(panel(" Keybindings ".into(), true)),
        area,
    );
}

fn centered(r: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(r.width);
    let h = height.min(r.height);
    Rect::new(r.x + (r.width - w) / 2, r.y + (r.height - h) / 2, w, h)
}
