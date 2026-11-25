use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Runtime configuration: where $HOME is, where the dotfiles repo lives,
/// and where backups of displaced files go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub home_dir: PathBuf,
    pub dotfiles_repo: PathBuf,
    pub backup_dir: PathBuf,
}

impl Config {
    /// Build a config from explicit paths (used by tests and embedders).
    pub fn new(home_dir: PathBuf, dotfiles_repo: PathBuf, backup_dir: PathBuf) -> Self {
        Self {
            home_dir,
            dotfiles_repo,
            backup_dir,
        }
    }

    /// Resolve the config for the current user.
    ///
    /// `repo_override` comes from `--repo` / `LAZYDOT_REPO`; defaults to `~/.dotfiles`.
    pub fn load(repo_override: Option<PathBuf>) -> Result<Self> {
        let home_dir =
            dirs::home_dir().context("could not determine your home directory (is $HOME set?)")?;
        let dotfiles_repo = match repo_override {
            Some(p) => absolutize(&p)?,
            None => home_dir.join(".dotfiles"),
        };
        let backup_dir = dirs::state_dir()
            .or_else(dirs::data_local_dir)
            .unwrap_or_else(|| home_dir.join(".local").join("state"))
            .join("lazydot")
            .join("backups");
        Ok(Self::new(home_dir, dotfiles_repo, backup_dir))
    }

    /// Render a path relative to `$HOME` as `~/...` for display.
    pub fn display_path(&self, path: &Path) -> String {
        match path.strip_prefix(&self.home_dir) {
            Ok(rel) if rel.as_os_str().is_empty() => "~".to_string(),
            Ok(rel) => format!("~/{}", rel.display()),
            Err(_) => path.display().to_string(),
        }
    }
}

fn absolutize(p: &Path) -> Result<PathBuf> {
    if p.is_absolute() {
        Ok(p.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_path_shortens_home() {
        let c = Config::new("/h".into(), "/h/.dotfiles".into(), "/b".into());
        assert_eq!(c.display_path(Path::new("/h/.bashrc")), "~/.bashrc");
        assert_eq!(c.display_path(Path::new("/h")), "~");
        assert_eq!(c.display_path(Path::new("/etc/x")), "/etc/x");
    }
}
