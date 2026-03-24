use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub dotfiles_repo: PathBuf,
    pub home_dir: PathBuf,
}

impl Config {
    pub fn default_home() -> PathBuf {
        let home = std::env::var("HOME").expect("HOME environment variable not set");
        PathBuf::from(home)
    }

    pub fn load() -> Self {
        let home_dir = Self::default_home();
        Config {
            dotfiles_repo: home_dir.join(".dotfiles"),
            home_dir,
        }
    }
}
