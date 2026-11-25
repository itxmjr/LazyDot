//! Plan-then-execute: every mutation is first described as a list of
//! [`Action`]s (shown to the user as a dry run), then executed. If any step
//! fails, completed steps are undone in reverse order.

use crate::config::Config;
use crate::core::fs_ops;
use anyhow::{anyhow, Result};
use std::fs;
use std::io::{self, ErrorKind};
use std::os::unix::fs::symlink;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Move a file or directory (adopt: home → repo, restore: repo → home).
    Move { from: PathBuf, to: PathBuf },
    /// Move something out of the way into the backup directory.
    Backup { from: PathBuf, to: PathBuf },
    /// Create `link` pointing at `points_to`.
    Symlink { link: PathBuf, points_to: PathBuf },
    /// Remove the symlink `link` (which must point at `points_to`).
    RemoveLink { link: PathBuf, points_to: PathBuf },
    /// Remove empty directories under `root` (cleanup, not undoable, never fails).
    PruneEmptyDirs { root: PathBuf },
}

impl Action {
    pub fn describe(&self, config: &Config) -> String {
        let p = |path: &PathBuf| config.display_path(path);
        match self {
            Action::Move { from, to } => format!("move    {} → {}", p(from), p(to)),
            Action::Backup { from, to } => format!("backup  {} → {}", p(from), p(to)),
            Action::Symlink { link, points_to } => {
                format!("link    {} → {}", p(link), p(points_to))
            }
            Action::RemoveLink { link, .. } => format!("unlink  {}", p(link)),
            Action::PruneEmptyDirs { root } => format!("cleanup empty dirs in {}", p(root)),
        }
    }

    fn apply(&self) -> io::Result<()> {
        match self {
            Action::Move { from, to } | Action::Backup { from, to } => fs_ops::move_path(from, to),
            Action::Symlink { link, points_to } => {
                if fs_ops::exists_no_follow(link) {
                    return Err(io::Error::new(
                        ErrorKind::AlreadyExists,
                        format!("{} already exists", link.display()),
                    ));
                }
                if let Some(parent) = link.parent() {
                    fs::create_dir_all(parent)?;
                }
                symlink(points_to, link)
            }
            Action::RemoveLink { link, points_to } => {
                // Safety: only ever delete a symlink that still points at our file.
                let ok = fs_ops::is_symlink(link)
                    && fs_ops::resolve_link(link)
                        .map(|d| fs_ops::same_path(&d, points_to))
                        .unwrap_or(false);
                if !ok {
                    return Err(io::Error::other(format!(
                        "{} is no longer a link to {}; not removing it",
                        link.display(),
                        points_to.display()
                    )));
                }
                fs::remove_file(link)
            }
            Action::PruneEmptyDirs { root } => {
                fs_ops::prune_empty_dirs(root);
                Ok(())
            }
        }
    }

    fn undo(&self) -> io::Result<()> {
        match self {
            Action::Move { from, to } | Action::Backup { from, to } => fs_ops::move_path(to, from),
            Action::Symlink { link, .. } => fs::remove_file(link),
            Action::RemoveLink { link, points_to } => symlink(points_to, link),
            Action::PruneEmptyDirs { .. } => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    /// Short human title, e.g. "Adopt nvim".
    pub title: String,
    pub actions: Vec<Action>,
    /// Entries deliberately left alone, with a reason.
    pub skipped: Vec<String>,
}

impl Plan {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    /// True if executing would change nothing on disk.
    pub fn is_noop(&self) -> bool {
        self.actions
            .iter()
            .all(|a| matches!(a, Action::PruneEmptyDirs { .. }))
    }

    /// Execute all actions. On failure, roll back completed actions and
    /// return an error describing both the failure and the rollback.
    pub fn execute(&self) -> Result<()> {
        for (i, action) in self.actions.iter().enumerate() {
            if let Err(err) = action.apply() {
                let mut rollback_errors = Vec::new();
                for done in self.actions[..i].iter().rev() {
                    if let Err(e) = done.undo() {
                        rollback_errors.push(format!("{done:?}: {e}"));
                    }
                }
                return Err(if rollback_errors.is_empty() {
                    anyhow!(
                        "{} failed at step {}: {err}. All {i} completed step(s) were rolled back.",
                        self.title,
                        i + 1
                    )
                } else {
                    anyhow!(
                        "{} failed at step {}: {err}. Rollback was incomplete, please check: {}",
                        self.title,
                        i + 1,
                        rollback_errors.join("; ")
                    )
                });
            }
        }
        Ok(())
    }
}
