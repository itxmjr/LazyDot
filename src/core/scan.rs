//! Build the package list from the dotfiles repo (tracked packages) and
//! from well-known locations in $HOME (untracked candidates).
//!
//! The repo layout is GNU Stow compatible:
//! `<repo>/<package>/<path relative to $HOME>`, e.g. `~/.dotfiles/nvim/.config/nvim/init.lua`.

use crate::config::Config;
use crate::core::fs_ops;
use crate::core::model::{Entry, EntryState, Occupant, Package};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Directories in ~/.config that hold caches, app state or huge profiles
/// rather than hand-written configuration. They are hidden from discovery.
const IGNORED_CONFIG_DIRS: &[&str] = &[
    "BraveSoftware",
    "Code",
    "Code - OSS",
    "Cursor",
    "Electron",
    "Signal",
    "Slack",
    "VSCodium",
    "chromium",
    "dconf",
    "discord",
    "enchant",
    "evolution",
    "google-chrome",
    "google-chrome-beta",
    "ibus",
    "lazydot",
    "microsoft-edge",
    "pulse",
    "session",
    "spotify",
    "systemd",
    "vivaldi",
];

/// Single files in $HOME (besides `.*rc`) that are commonly managed.
const KNOWN_HOME_FILES: &[&str] = &[
    ".bash_profile",
    ".bash_aliases",
    ".gitconfig",
    ".gitignore_global",
    ".profile",
    ".tmux.conf",
    ".Xresources",
    ".xprofile",
    ".zprofile",
    ".zshenv",
];

/// Repo entries that are never treated as package contents (Stow ignores these too).
const IGNORED_REPO_NAMES: &[&str] = &[".git", ".stow-local-ignore", ".DS_Store"];

pub fn scan(config: &Config) -> Vec<Package> {
    let mut packages = scan_repo(config);

    let claimed: HashSet<PathBuf> = packages
        .iter()
        .flat_map(|p| p.entries.iter().map(|e| e.target.clone()))
        .collect();
    // A candidate is claimed if it is a tracked target, lives inside one, or
    // contains one (e.g. a real ~/.config/nvim dir holding a linked init.lua).
    let is_claimed = |target: &Path| {
        claimed
            .iter()
            .any(|c| target.starts_with(c) || c.starts_with(target))
    };

    packages.extend(
        discover(config)
            .into_iter()
            .filter(|p| p.entries.iter().all(|e| !is_claimed(&e.target))),
    );

    // Tracked packages first, then alphabetical.
    packages.sort_by(|a, b| {
        b.tracked
            .cmp(&a.tracked)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    packages
}

/// Packages that exist in the dotfiles repo, with real link status.
pub fn scan_repo(config: &Config) -> Vec<Package> {
    let Ok(read) = fs::read_dir(&config.dotfiles_repo) else {
        return Vec::new();
    };
    let mut packages: Vec<Package> = read
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .map(|e| {
            let mut entries = Vec::new();
            collect_units(&e.path(), &config.home_dir, &mut entries);
            Package {
                name: e.file_name().to_string_lossy().into_owned(),
                tracked: true,
                entries,
            }
        })
        .collect();
    packages.sort_by(|a, b| a.name.cmp(&b.name));
    packages
}

/// Walk a package directory like Stow does: descend while the matching home
/// path is a real directory; otherwise the node is a single link unit
/// (a file, or a whole "folded" directory).
fn collect_units(src_dir: &Path, target_dir: &Path, out: &mut Vec<Entry>) {
    let Ok(read) = fs::read_dir(src_dir) else {
        return;
    };
    let mut children: Vec<_> = read.flatten().collect();
    children.sort_by_key(|e| e.file_name());

    for child in children {
        let name = child.file_name();
        if IGNORED_REPO_NAMES.iter().any(|n| name == **n) {
            continue;
        }
        let source = child.path();
        let target = target_dir.join(&name);

        let source_is_real_dir = fs::symlink_metadata(&source)
            .map(|m| m.is_dir())
            .unwrap_or(false);
        let target_is_real_dir = fs::symlink_metadata(&target)
            .map(|m| m.is_dir())
            .unwrap_or(false);

        if source_is_real_dir && target_is_real_dir {
            collect_units(&source, &target, out);
        } else {
            let state = classify(&source, &target);
            out.push(Entry {
                target,
                source: Some(source),
                state,
            });
        }
    }
}

/// Determine the state of one repo file versus its home location.
pub fn classify(source: &Path, target: &Path) -> EntryState {
    let Ok(meta) = fs::symlink_metadata(target) else {
        return EntryState::NotLinked;
    };
    if meta.file_type().is_symlink() {
        return match fs_ops::resolve_link(target) {
            Ok(dest) if fs_ops::same_path(&dest, source) => EntryState::Linked,
            Ok(_) | Err(_) => {
                EntryState::Conflict(Occupant::Symlink(fs::read_link(target).unwrap_or_default()))
            }
        };
    }
    if meta.is_dir() {
        EntryState::Conflict(Occupant::Directory)
    } else {
        EntryState::Conflict(Occupant::File)
    }
}

/// Untracked config candidates in $HOME.
pub fn discover(config: &Config) -> Vec<Package> {
    let home = &config.home_dir;
    let mut found = Vec::new();

    // ~/.config/<app>
    if let Ok(read) = fs::read_dir(home.join(".config")) {
        for entry in read.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if IGNORED_CONFIG_DIRS.contains(&name.as_str()) || !entry.path().is_dir() {
                continue;
            }
            found.push((name, entry.path()));
        }
    }

    // ~/.*rc and well-known single files
    if let Ok(read) = fs::read_dir(home) {
        for entry in read.flatten() {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            let is_rc = file_name.starts_with('.') && file_name.ends_with("rc");
            let known = KNOWN_HOME_FILES.contains(&file_name.as_str());
            if (is_rc || known) && entry.path().is_file() {
                found.push((file_name.trim_start_matches('.').to_string(), entry.path()));
            }
        }
    }

    found
        .into_iter()
        .filter(|(_, path)| {
            !fs_ops::is_within(path, &config.dotfiles_repo)
                && !fs_ops::is_within(&config.dotfiles_repo, path)
                && !fs_ops::is_within(path, &config.backup_dir)
        })
        .map(|(name, target)| {
            let state = if fs_ops::is_symlink(&target) {
                EntryState::ForeignLink(fs::read_link(&target).unwrap_or_default())
            } else {
                EntryState::Untracked
            };
            Package {
                name,
                tracked: false,
                entries: vec![Entry {
                    target,
                    source: None,
                    state,
                }],
            }
        })
        .collect()
}
