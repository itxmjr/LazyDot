use crate::config::Config;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub files: Vec<PathBuf>,
    pub status: PackageStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PackageStatus {
    Unstowed,
    Stowed,
    #[allow(dead_code)]
    Conflict,
}

pub fn scan_system(config: &Config) -> Vec<Package> {
    let mut packages = Vec::new();
    let home = &config.home_dir;

    // 1. Scan ~/.config
    let config_dir = home.join(".config");
    if config_dir.exists() && config_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&config_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().unwrap().to_string_lossy().into_owned();
                    // Basic heuristic: Some folders aren't config to manage, but user can pick them
                    packages.push(Package {
                        name,
                        files: vec![path],
                        status: PackageStatus::Unstowed,
                    });
                }
            }
        }
    }

    // 2. Scan ~/.*rc
    if let Ok(entries) = fs::read_dir(home) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
                if (file_name.starts_with('.') && file_name.ends_with("rc"))
                    || file_name == ".gitconfig"
                    || file_name == ".tmux.conf"
                {
                    let name = file_name.trim_start_matches('.').to_string();
                    packages.push(Package {
                        name,
                        files: vec![path],
                        status: PackageStatus::Unstowed,
                    });
                }
            }
        }
    }

    // 3. Scan ~/.dotfiles (already adopted packages)
    let dotfiles_repo = &config.dotfiles_repo;
    if dotfiles_repo.exists() && dotfiles_repo.is_dir() {
        if let Ok(entries) = fs::read_dir(dotfiles_repo) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().unwrap().to_string_lossy().into_owned();
                    if name == ".git" {
                        continue;
                    }

                    // Check if this package is already listed from scanning HOME
                    // If it is, update status or merge files. For now, just add if not present.
                    let existing = packages.iter_mut().find(|p| p.name == name);
                    if let Some(pkg) = existing {
                        // We will refine status later via symlink checker
                        pkg.status = PackageStatus::Stowed;
                    } else {
                        // Package exists in dotfiles but not found in typical ~ locations
                        // That usually means it's stowed!
                        packages.push(Package {
                            name,
                            files: vec![path],
                            status: PackageStatus::Stowed,
                        });
                    }
                }
            }
        }
    }

    // Sort alphabetically
    packages.sort_by(|a, b| a.name.cmp(&b.name));

    packages
}
