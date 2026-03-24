use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{ActivePane, App, ConfirmAction, InputMode};
use crate::managers::scanner::PackageStatus;
use crate::ui::theme;

pub fn draw(f: &mut Frame, app: &mut App) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)].as_ref())
        .split(f.area());

    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(20),
                Constraint::Percentage(35),
                Constraint::Percentage(45),
            ]
            .as_ref(),
        )
        .split(main_layout[0]);

    draw_left_panel(f, app, content_chunks[0]);
    draw_center_panel(f, app, content_chunks[1]);
    draw_right_panel(f, app, content_chunks[2]);

    // Status bar at the bottom
    draw_status_bar(f, main_layout[1]);

    if app.show_help {
        draw_help_modal(f);
    } else if let InputMode::ConfirmDialog(_) = app.input_mode {
        draw_dialog(f, app);
    }
}

fn draw_status_bar(f: &mut Frame, area: Rect) {
    let left_text = Line::from(vec![Span::styled(
        "nav: j/k | switch: h/l | adopt: a | stow: s | quit: q | ?: keybindings",
        Style::default().fg(theme::TEXT_DIM),
    )]);

    let right_text = Line::from(vec![
        Span::styled("Donate ", Style::default().fg(theme::ACCENT)),
        Span::styled("Ask_Question ", Style::default().fg(theme::HIGHLIGHT)),
        Span::styled(
            format!("v{} ", env!("CARGO_PKG_VERSION")),
            Style::default().fg(theme::TEXT_DIM),
        ),
    ]);

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
        .split(area);

    let left_para = Paragraph::new(left_text).alignment(Alignment::Left);
    let right_para = Paragraph::new(right_text).alignment(Alignment::Right);

    f.render_widget(left_para, layout[0]);
    f.render_widget(right_para, layout[1]);
}

fn draw_left_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let is_active = app.active_pane == ActivePane::Packages
        && matches!(app.input_mode, InputMode::Normal)
        && !app.show_help;

    let border_style = if is_active {
        theme::active_border_style()
    } else {
        theme::inactive_border_style()
    };

    let title_style = if is_active {
        theme::active_title_style()
    } else {
        theme::inactive_title_style()
    };

    let items: Vec<ListItem> = app
        .packages
        .iter()
        .map(|pkg| {
            let status_icon = match pkg.status {
                PackageStatus::Stowed => "[S]",
                PackageStatus::Unstowed => "[ ]",
                PackageStatus::Conflict => "[!]",
            };

            let color = match pkg.status {
                PackageStatus::Stowed => theme::SELECTED,
                PackageStatus::Conflict => theme::ERROR,
                PackageStatus::Unstowed => theme::TEXT_STD,
            };

            let content = Line::from(vec![
                Span::styled(format!("{} ", status_icon), Style::default().fg(color)),
                Span::raw(pkg.name.clone()),
            ]);
            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(" Packages ", title_style))
                .border_style(border_style),
        )
        .highlight_style(Style::default().fg(theme::SELECTED))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, &mut app.package_list_state);
}

fn draw_center_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let is_active = app.active_pane == ActivePane::Files
        && matches!(app.input_mode, InputMode::Normal)
        && !app.show_help;

    let border_style = if is_active {
        theme::active_border_style()
    } else {
        theme::inactive_border_style()
    };

    let title_style = if is_active {
        theme::active_title_style()
    } else {
        theme::inactive_title_style()
    };

    let selected_pkg = app.package_list_state.selected().map(|i| &app.packages[i]);

    let items: Vec<ListItem> = if let Some(pkg) = selected_pkg {
        pkg.files
            .iter()
            .map(|fp| ListItem::new(fp.display().to_string()))
            .collect()
    } else {
        vec![]
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(" Config Locations ", title_style))
                .border_style(border_style),
        )
        .highlight_style(Style::default().fg(theme::SELECTED))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, &mut app.file_list_state);
}

fn draw_right_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let border_style = theme::inactive_border_style();
    let title_style = theme::inactive_title_style();

    let p = Paragraph::new(app.file_preview.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(" File Contents / Log ", title_style))
                .border_style(border_style),
        )
        .style(Style::default().fg(theme::TEXT_STD))
        .wrap(Wrap { trim: false });

    f.render_widget(p, area);
}

fn draw_dialog(f: &mut Frame, app: &mut App) {
    let area = centered_rect(60, 20, f.area());
    let prompt = match app.input_mode {
        InputMode::ConfirmDialog(ConfirmAction::Adopt(_)) => {
            "Are you sure you want to adopt this package?\nIt will be moved to ~/.dotfiles. (y/n)"
        }
        InputMode::ConfirmDialog(ConfirmAction::Unstow(_)) => {
            "Are you sure you want to unstow this package?\nSymlinks will be removed. (y/n)"
        }
        _ => "",
    };

    let p = Paragraph::new(prompt)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    " Confirm ",
                    Style::default().fg(theme::HIGHLIGHT),
                ))
                .border_style(Style::default().fg(theme::HIGHLIGHT)),
        )
        .style(Style::default().fg(theme::TEXT_STD))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(Clear, area);
    f.render_widget(p, area);
}

fn draw_help_modal(f: &mut Frame) {
    let area = centered_rect(50, 60, f.area());
    let text = vec![
        Line::from(vec![
            Span::styled("q / Esc    ", Style::default().fg(theme::HIGHLIGHT)),
            Span::raw("Quit the application"),
        ]),
        Line::from(vec![
            Span::styled("j / ↓      ", Style::default().fg(theme::HIGHLIGHT)),
            Span::raw("Move down"),
        ]),
        Line::from(vec![
            Span::styled("k / ↑      ", Style::default().fg(theme::HIGHLIGHT)),
            Span::raw("Move up"),
        ]),
        Line::from(vec![
            Span::styled("h/l / ←/→  ", Style::default().fg(theme::HIGHLIGHT)),
            Span::raw("Toggle focus (Packages <-> Files)"),
        ]),
        Line::from(vec![
            Span::styled("Tab        ", Style::default().fg(theme::HIGHLIGHT)),
            Span::raw("Toggle focus"),
        ]),
        Line::from(vec![
            Span::styled("a          ", Style::default().fg(theme::HIGHLIGHT)),
            Span::raw("Adopt selected package to dotfiles repo"),
        ]),
        Line::from(vec![
            Span::styled("s          ", Style::default().fg(theme::HIGHLIGHT)),
            Span::raw("Stow selected package (create symlinks)"),
        ]),
        Line::from(vec![
            Span::styled("u / D      ", Style::default().fg(theme::HIGHLIGHT)),
            Span::raw("Unstow selected package (remove symlinks)"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press ? or Esc to close",
            Style::default().fg(theme::TEXT_DIM),
        )),
    ];

    let p = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    " LazyDot Shortcuts ",
                    theme::active_title_style(),
                ))
                .border_style(theme::active_border_style()),
        )
        .style(Style::default().fg(theme::TEXT_STD))
        .wrap(Wrap { trim: true });

    f.render_widget(Clear, area);
    f.render_widget(p, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
