//! Terminal front end.

pub mod app;
pub mod theme;
pub mod ui;

use crate::config::Config;
use anyhow::Result;
use app::App;
use ratatui::crossterm::event::{self, Event, KeyEventKind};
use std::time::Duration;

/// Run the interactive UI. The terminal is restored on normal exit, on
/// error, and on panic (ratatui installs a panic hook in `init`).
pub fn run(config: Config, dry_run: bool) -> Result<()> {
    let mut app = App::new(config, dry_run);
    let mut terminal = ratatui::init();
    let result = (|| -> Result<()> {
        while !app.should_quit {
            terminal.draw(|f| ui::draw(f, &mut app))?;
            if event::poll(Duration::from_millis(250))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        app.handle_key(key);
                    }
                }
            }
        }
        Ok(())
    })();
    ratatui::restore();
    result
}
