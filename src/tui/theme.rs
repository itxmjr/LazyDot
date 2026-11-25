use ratatui::style::{Color, Modifier, Style};

pub const ACCENT: Color = Color::LightGreen;
pub const DIM: Color = Color::DarkGray;
pub const KEY: Color = Color::Cyan;
pub const WARN: Color = Color::Yellow;
pub const ERROR: Color = Color::LightRed;
pub const INFO: Color = Color::Blue;

pub fn border(active: bool) -> Style {
    if active {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(DIM)
    }
}

pub fn title(active: bool) -> Style {
    if active {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

pub fn selected() -> Style {
    Style::default()
        .fg(ACCENT)
        .add_modifier(Modifier::BOLD | Modifier::REVERSED)
}
