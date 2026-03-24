use crate::config::Config;
use crate::managers::scanner::{self, Package};
use ratatui::widgets::ListState;
use std::fs;

pub enum InputMode {
    Normal,
    ConfirmDialog(ConfirmAction),
}

pub enum ConfirmAction {
    Adopt(usize), // Package index
    Unstow(usize),
}

#[derive(PartialEq)]
pub enum ActivePane {
    Packages,
    Files,
}

pub struct App {
    pub config: Config,
    pub packages: Vec<Package>,
    pub package_list_state: ListState,
    pub file_list_state: ListState,
    pub input_mode: InputMode,
    pub should_quit: bool,
    pub show_help: bool,
    pub active_pane: ActivePane,
    pub file_preview: String,
}

impl App {
    pub fn new() -> App {
        let config = Config::load();
        let packages = scanner::scan_system(&config);

        let mut package_list_state = ListState::default();
        if !packages.is_empty() {
            package_list_state.select(Some(0));
        }

        let mut file_list_state = ListState::default();
        file_list_state.select(Some(0));

        let mut app = App {
            config,
            packages,
            package_list_state,
            file_list_state,
            input_mode: InputMode::Normal,
            should_quit: false,
            show_help: false,
            active_pane: ActivePane::Packages,
            file_preview: String::new(),
        };
        app.update_file_selection();
        app
    }

    pub fn on_tick(&mut self) {}

    pub fn next(&mut self) {
        match self.active_pane {
            ActivePane::Packages => {
                let i = match self.package_list_state.selected() {
                    Some(i) => {
                        if i >= self.packages.len().saturating_sub(1) {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.package_list_state.select(Some(i));
                self.file_list_state.select(Some(0));
                self.update_file_selection();
            }
            ActivePane::Files => {
                if let Some(pkg_idx) = self.package_list_state.selected() {
                    let files_len = self.packages[pkg_idx].files.len();
                    if files_len > 0 {
                        let i = match self.file_list_state.selected() {
                            Some(i) => {
                                if i >= files_len.saturating_sub(1) {
                                    0
                                } else {
                                    i + 1
                                }
                            }
                            None => 0,
                        };
                        self.file_list_state.select(Some(i));
                        self.update_file_selection();
                    }
                }
            }
        }
    }

    pub fn previous(&mut self) {
        match self.active_pane {
            ActivePane::Packages => {
                let i = match self.package_list_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.packages.len().saturating_sub(1)
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.package_list_state.select(Some(i));
                self.file_list_state.select(Some(0));
                self.update_file_selection();
            }
            ActivePane::Files => {
                if let Some(pkg_idx) = self.package_list_state.selected() {
                    let files_len = self.packages[pkg_idx].files.len();
                    if files_len > 0 {
                        let i = match self.file_list_state.selected() {
                            Some(i) => {
                                if i == 0 {
                                    files_len.saturating_sub(1)
                                } else {
                                    i - 1
                                }
                            }
                            None => 0,
                        };
                        self.file_list_state.select(Some(i));
                        self.update_file_selection();
                    }
                }
            }
        }
    }

    pub fn toggle_pane(&mut self) {
        self.active_pane = match self.active_pane {
            ActivePane::Packages => ActivePane::Files,
            ActivePane::Files => ActivePane::Packages,
        };
    }

    pub fn update_file_selection(&mut self) {
        self.file_preview.clear();
        if let Some(pkg_idx) = self.package_list_state.selected() {
            if let Some(pkg) = self.packages.get(pkg_idx) {
                if let Some(file_idx) = self.file_list_state.selected() {
                    if let Some(file_path) = pkg.files.get(file_idx) {
                        if file_path.is_dir() {
                            let mut preview =
                                format!("Directory: {}\n\nContents:\n", file_path.display());
                            if let Ok(entries) = fs::read_dir(file_path) {
                                for entry in entries.flatten() {
                                    if let Ok(meta) = entry.metadata() {
                                        let prefix = if meta.is_dir() { "[DIR]" } else { "[FILE]" };
                                        preview.push_str(&format!(
                                            "  {} {}\n",
                                            prefix,
                                            entry.file_name().to_string_lossy()
                                        ));
                                    }
                                }
                            }
                            self.file_preview = preview;
                        } else {
                            self.file_preview = fs::read_to_string(file_path)
                                .unwrap_or_else(|e| format!("Could not read file:\n{}", e));
                        }
                    }
                }
            }
        }
    }

    pub fn refresh_packages(&mut self) {
        self.packages = scanner::scan_system(&self.config);
        self.update_file_selection();
    }
}
