mod app;
mod config;
mod event;
mod managers;
mod ui;

use crate::managers::packager;

use crate::app::{App, ConfirmAction, InputMode};
use crate::event::{Event, Events};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{error::Error, io, time::Duration};

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new();
    let events = Events::new(Duration::from_millis(250));

    loop {
        terminal.draw(|f| ui::layout::draw(f, &mut app))?;

        if app.should_quit {
            break;
        }

        match events.next()? {
            Event::Input(key) => {
                if app.show_help {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                            app.show_help = false
                        }
                        _ => {}
                    }
                    continue;
                }

                match app.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                        KeyCode::Char('?') => app.show_help = true,
                        KeyCode::Down | KeyCode::Char('j') => app.next(),
                        KeyCode::Up | KeyCode::Char('k') => app.previous(),
                        KeyCode::Tab
                        | KeyCode::Right
                        | KeyCode::Left
                        | KeyCode::Char('l')
                        | KeyCode::Char('h') => {
                            app.toggle_pane();
                        }
                        KeyCode::Char('a') => {
                            if let Some(i) = app.package_list_state.selected() {
                                app.input_mode = InputMode::ConfirmDialog(ConfirmAction::Adopt(i));
                            }
                        }
                        KeyCode::Char('s') => {
                            if let Some(i) = app.package_list_state.selected() {
                                let pkg = app.packages[i].clone();
                                let _ = packager::stow_package(&app.config, &pkg);
                                app.refresh_packages();
                            }
                        }
                        KeyCode::Char('u') | KeyCode::Char('D') => {
                            if let Some(i) = app.package_list_state.selected() {
                                app.input_mode = InputMode::ConfirmDialog(ConfirmAction::Unstow(i));
                            }
                        }
                        _ => {}
                    },
                    InputMode::ConfirmDialog(ref action) => match key.code {
                        KeyCode::Char('y') => {
                            match action {
                                ConfirmAction::Adopt(i) => {
                                    let pkg = app.packages[*i].clone();
                                    let _ = packager::adopt_package(&app.config, &pkg);
                                }
                                ConfirmAction::Unstow(i) => {
                                    let pkg = app.packages[*i].clone();
                                    let _ = packager::unstow_package(&pkg);
                                }
                            }
                            app.refresh_packages();
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Char('n') | KeyCode::Esc => app.input_mode = InputMode::Normal,
                        _ => {}
                    },
                }
            }
            Event::Tick => app.on_tick(),
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
