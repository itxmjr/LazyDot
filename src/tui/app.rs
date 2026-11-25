//! TUI state machine. Contains no terminal I/O so it can be unit tested.

use crate::config::Config;
use crate::core::model::{Entry, Package, PackageStatus};
use crate::core::{ops, scan, Plan};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;
use std::fs;
use std::io::Read;
use std::path::Path;

const PREVIEW_LIMIT: u64 = 64 * 1024;
const LOG_CAPACITY: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Packages,
    Files,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Help,
    Confirm(Plan),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Success,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    pub level: Level,
    pub text: String,
}

#[derive(Debug, Clone, Copy)]
enum Op {
    Adopt,
    Stow,
    Unstow,
    Restore,
}

pub struct App {
    pub config: Config,
    pub packages: Vec<Package>,
    pub package_state: ListState,
    pub file_state: ListState,
    pub focus: Focus,
    pub mode: Mode,
    pub preview: String,
    pub preview_scroll: u16,
    pub log: Vec<LogLine>,
    pub dry_run: bool,
    pub should_quit: bool,
}

impl App {
    pub fn new(config: Config, dry_run: bool) -> Self {
        let mut app = Self {
            config,
            packages: Vec::new(),
            package_state: ListState::default(),
            file_state: ListState::default(),
            focus: Focus::Packages,
            mode: Mode::Normal,
            preview: String::new(),
            preview_scroll: 0,
            log: Vec::new(),
            dry_run,
            should_quit: false,
        };
        app.refresh();
        let tracked = app.packages.iter().filter(|p| p.tracked).count();
        app.push_log(
            Level::Info,
            format!(
                "Repo: {}  ({} tracked, {} untracked)",
                app.config.display_path(&app.config.dotfiles_repo),
                tracked,
                app.packages.len() - tracked
            ),
        );
        if dry_run {
            app.push_log(Level::Warn, "Dry-run mode: no changes will be made");
        }
        app
    }

    pub fn selected_package(&self) -> Option<&Package> {
        self.package_state
            .selected()
            .and_then(|i| self.packages.get(i))
    }

    pub fn selected_entry(&self) -> Option<&Entry> {
        let pkg = self.selected_package()?;
        pkg.entries.get(self.file_state.selected()?)
    }

    pub fn push_log(&mut self, level: Level, text: impl Into<String>) {
        self.log.push(LogLine {
            level,
            text: text.into(),
        });
        if self.log.len() > LOG_CAPACITY {
            self.log.drain(..self.log.len() - LOG_CAPACITY);
        }
    }

    /// Rescan disk, keeping the selected package (by name) when possible.
    pub fn refresh(&mut self) {
        let keep = self.selected_package().map(|p| (p.name.clone(), p.tracked));
        self.packages = scan::scan(&self.config);
        let idx = keep
            .and_then(|(name, tracked)| {
                self.packages
                    .iter()
                    .position(|p| p.name == name && p.tracked == tracked)
                    .or_else(|| self.packages.iter().position(|p| p.name == name))
            })
            .or_else(|| self.package_state.selected());
        self.select_package(idx);
    }

    fn select_package(&mut self, idx: Option<usize>) {
        let len = self.packages.len();
        let idx = if len == 0 {
            None
        } else {
            Some(idx.unwrap_or(0).min(len - 1))
        };
        self.package_state.select(idx);
        self.select_file(Some(0));
    }

    fn select_file(&mut self, idx: Option<usize>) {
        let len = self.selected_package().map_or(0, |p| p.entries.len());
        let idx = if len == 0 {
            None
        } else {
            Some(idx.unwrap_or(0).min(len - 1))
        };
        self.file_state.select(idx);
        self.update_preview();
    }

    fn move_selection(&mut self, delta: isize) {
        let (len, current) = match self.focus {
            Focus::Packages => (self.packages.len(), self.package_state.selected()),
            Focus::Files => (
                self.selected_package().map_or(0, |p| p.entries.len()),
                self.file_state.selected(),
            ),
        };
        if len == 0 {
            return;
        }
        let next = match current {
            Some(i) => (i as isize + delta).rem_euclid(len as isize) as usize,
            None => 0,
        };
        match self.focus {
            Focus::Packages => self.select_package(Some(next)),
            Focus::Files => self.select_file(Some(next)),
        }
    }

    fn jump(&mut self, to_end: bool) {
        let idx = if to_end { usize::MAX } else { 0 };
        match self.focus {
            Focus::Packages => self.select_package(Some(idx)),
            Focus::Files => self.select_file(Some(idx)),
        }
    }

    pub fn update_preview(&mut self) {
        self.preview_scroll = 0;
        self.preview = match self.selected_entry() {
            None => String::new(),
            Some(entry) => {
                let path = entry
                    .source
                    .as_deref()
                    .filter(|s| s.exists())
                    .unwrap_or(&entry.target);
                render_preview(path)
            }
        };
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }
        match std::mem::replace(&mut self.mode, Mode::Normal) {
            Mode::Help => {
                if !matches!(
                    key.code,
                    KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')
                ) {
                    self.mode = Mode::Help;
                }
            }
            Mode::Confirm(plan) => match key.code {
                KeyCode::Char('y') | KeyCode::Enter => self.execute(plan),
                KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
                    self.push_log(Level::Info, format!("{}: cancelled", plan.title));
                }
                _ => self.mode = Mode::Confirm(plan),
            },
            Mode::Normal => self.handle_normal_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('?') => self.mode = Mode::Help,
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Char('g') | KeyCode::Home => self.jump(false),
            KeyCode::Char('G') | KeyCode::End => self.jump(true),
            KeyCode::Tab
            | KeyCode::BackTab
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Char('h')
            | KeyCode::Char('l') => {
                self.focus = match self.focus {
                    Focus::Packages => Focus::Files,
                    Focus::Files => Focus::Packages,
                };
            }
            KeyCode::PageDown | KeyCode::Char('J') => {
                self.preview_scroll = self.preview_scroll.saturating_add(10)
            }
            KeyCode::PageUp | KeyCode::Char('K') => {
                self.preview_scroll = self.preview_scroll.saturating_sub(10)
            }
            KeyCode::Char('r') => {
                self.refresh();
                self.push_log(Level::Info, "Rescanned");
            }
            KeyCode::Char('a') => self.propose(Op::Adopt),
            KeyCode::Char('s') => self.propose(Op::Stow),
            KeyCode::Char('u') | KeyCode::Char('D') => self.propose(Op::Unstow),
            KeyCode::Char('R') => self.propose(Op::Restore),
            _ => {}
        }
    }

    fn propose(&mut self, op: Op) {
        let Some(pkg) = self.selected_package() else {
            return;
        };
        let plan = match op {
            Op::Adopt => ops::plan_adopt(&self.config, pkg),
            Op::Stow => ops::plan_stow(&self.config, pkg),
            Op::Unstow => ops::plan_unstow(&self.config, pkg),
            Op::Restore => ops::plan_restore(&self.config, pkg),
        };
        if plan.is_noop() {
            let reason = if plan.skipped.is_empty() {
                match (op, pkg.status()) {
                    (Op::Stow, PackageStatus::Stowed) => "already stowed".to_string(),
                    (Op::Unstow, _) => "nothing is linked".to_string(),
                    _ => "nothing to do".to_string(),
                }
            } else {
                plan.skipped.join("; ")
            };
            self.push_log(Level::Warn, format!("{}: {reason}", plan.title));
            return;
        }
        self.mode = Mode::Confirm(plan);
    }

    fn execute(&mut self, plan: Plan) {
        if self.dry_run {
            self.push_log(
                Level::Warn,
                format!(
                    "{}: dry run, {} step(s) not executed",
                    plan.title,
                    plan.actions.len()
                ),
            );
            return;
        }
        match plan.execute() {
            Ok(()) => {
                self.push_log(Level::Success, format!("{}: done", plan.title));
                for note in &plan.skipped {
                    self.push_log(Level::Warn, format!("  skipped: {note}"));
                }
            }
            Err(e) => self.push_log(Level::Error, format!("{e:#}")),
        }
        self.refresh();
    }
}

/// Text preview for a file or directory. Bounded in size; binary-safe.
fn render_preview(path: &Path) -> String {
    let Ok(meta) = fs::metadata(path) else {
        return match fs::read_link(path) {
            Ok(dest) => format!("Broken symlink → {}", dest.display()),
            Err(e) => format!("Cannot read {}: {e}", path.display()),
        };
    };
    if meta.is_dir() {
        let mut names: Vec<String> = fs::read_dir(path)
            .map(|rd| {
                rd.flatten()
                    .map(|e| {
                        let dir = e.path().is_dir();
                        format!(
                            "{}{}",
                            e.file_name().to_string_lossy(),
                            if dir { "/" } else { "" }
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        names.sort_by_key(|n| n.to_lowercase());
        let total = names.len();
        names.truncate(500);
        let mut out = format!("{}/  ({total} entries)\n\n", path.display());
        for n in names {
            out.push_str("  ");
            out.push_str(&n);
            out.push('\n');
        }
        return out;
    }

    let mut buf = Vec::new();
    match fs::File::open(path).and_then(|f| f.take(PREVIEW_LIMIT).read_to_end(&mut buf)) {
        Err(e) => format!("Cannot read {}: {e}", path.display()),
        Ok(_) if buf.iter().take(8192).any(|&b| b == 0) => {
            format!("Binary file ({} bytes)", meta.len())
        }
        Ok(_) => {
            let mut text = String::from_utf8_lossy(&buf).into_owned();
            if meta.len() > PREVIEW_LIMIT {
                text.push_str(&format!(
                    "\n… truncated ({} of {} bytes shown)",
                    PREVIEW_LIMIT,
                    meta.len()
                ));
            }
            text
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyEvent;
    use std::fs;

    fn key(c: char) -> KeyEvent {
        KeyEvent::from(KeyCode::Char(c))
    }

    fn app_with(configs: &[&str]) -> (tempfile::TempDir, App) {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        for c in configs {
            fs::create_dir_all(home.join(".config").join(c)).unwrap();
            fs::write(home.join(".config").join(c).join("conf"), "x").unwrap();
        }
        let config = Config::new(home.clone(), home.join(".dotfiles"), dir.path().join("bk"));
        (dir, App::new(config, false))
    }

    /// C7: selection must stay in bounds when the list shrinks.
    #[test]
    fn selection_is_clamped_after_refresh() {
        let (dir, mut app) = app_with(&["a", "b", "c"]);
        app.handle_key(key('G'));
        assert_eq!(app.package_state.selected(), Some(2));
        fs::remove_dir_all(dir.path().join("home/.config/c")).unwrap();
        fs::remove_dir_all(dir.path().join("home/.config/b")).unwrap();
        app.handle_key(key('r'));
        assert_eq!(app.package_state.selected(), Some(0));
        assert!(app.selected_package().is_some());
    }

    #[test]
    fn empty_home_does_not_panic() {
        let (_dir, mut app) = app_with(&[]);
        for c in ['j', 'k', 'l', 'j', 'a', 's', 'u', 'R', 'G', 'g'] {
            app.handle_key(key(c));
        }
        assert!(app.selected_package().is_none());
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn adopt_goes_through_confirm_and_can_be_cancelled() {
        let (_dir, mut app) = app_with(&["nvim"]);
        app.handle_key(key('a'));
        assert!(matches!(app.mode, Mode::Confirm(_)));
        app.handle_key(key('n'));
        assert_eq!(app.mode, Mode::Normal);
        assert!(!app.packages[0].tracked);

        app.handle_key(key('a'));
        app.handle_key(key('y'));
        assert!(app.packages[0].tracked, "log: {:?}", app.log);
        assert_eq!(app.packages[0].status(), PackageStatus::Stowed);
    }

    #[test]
    fn dry_run_never_executes() {
        let (dir, _) = app_with(&["nvim"]);
        let home = dir.path().join("home");
        let mut app = App::new(
            Config::new(home.clone(), home.join(".dotfiles"), dir.path().join("bk")),
            true,
        );
        app.handle_key(key('a'));
        app.handle_key(key('y'));
        assert!(!home.join(".config/nvim").is_symlink());
    }

    #[test]
    fn preview_is_bounded_and_binary_safe() {
        let dir = tempfile::tempdir().unwrap();
        let big = dir.path().join("big");
        fs::write(&big, "a".repeat(200_000)).unwrap();
        assert!(render_preview(&big).contains("truncated"));
        let bin = dir.path().join("bin");
        fs::write(&bin, [0u8, 1, 2, 3]).unwrap();
        assert!(render_preview(&bin).starts_with("Binary file"));
    }

    #[test]
    fn renders_without_panicking() {
        use ratatui::{backend::TestBackend, Terminal};
        let (_dir, mut app) = app_with(&["nvim", "kitty"]);
        let mut term = Terminal::new(TestBackend::new(120, 30)).unwrap();
        term.draw(|f| crate::tui::ui::draw(f, &mut app)).unwrap();
        app.handle_key(key('?'));
        term.draw(|f| crate::tui::ui::draw(f, &mut app)).unwrap();
        app.handle_key(KeyEvent::from(KeyCode::Esc));
        app.handle_key(key('a'));
        term.draw(|f| crate::tui::ui::draw(f, &mut app)).unwrap();
        let text: String = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("Adopt"));
        // Tiny terminals must not panic either.
        let mut tiny = Terminal::new(TestBackend::new(10, 4)).unwrap();
        tiny.draw(|f| crate::tui::ui::draw(f, &mut app)).unwrap();
    }
}
