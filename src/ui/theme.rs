use ratatui::style::{Color, Modifier, Style};

#[allow(dead_code)]
pub const BG: Color = Color::Reset;
pub const TEXT_STD: Color = Color::Reset;
pub const SELECTED: Color = Color::LightGreen;
pub const BORDER_ACTIVE: Color = Color::LightGreen;
pub const BORDER_INACTIVE: Color = Color::DarkGray;
pub const TEXT_DIM: Color = Color::DarkGray;
pub const HIGHLIGHT: Color = Color::Yellow;
pub const ERROR: Color = Color::Red;
pub const ACCENT: Color = Color::Magenta;

pub fn active_border_style() -> Style {
    Style::default()
        .fg(BORDER_ACTIVE)
        .add_modifier(Modifier::BOLD)
}

pub fn inactive_border_style() -> Style {
    Style::default().fg(BORDER_INACTIVE)
}

pub fn active_title_style() -> Style {
    Style::default()
        .fg(BORDER_ACTIVE)
        .add_modifier(Modifier::BOLD)
}

pub fn inactive_title_style() -> Style {
    Style::default().fg(TEXT_STD)
}
