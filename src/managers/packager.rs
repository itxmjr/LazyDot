use crate::config::Config;
use crate::managers::scanner::Package;
use crate::managers::symlink;
use anyhow::{Context, Result};
use std::fs;

pub fn adopt_package(config: &Config, package: &Package) -> Result<()> {
    let repo_dir = &config.dotfiles_repo;
    if !repo_dir.exists() {
        fs::create_dir_all(repo_dir)?;
    }

    let pkg_dir = repo_dir.join(&package.name);
    if !pkg_dir.exists() {
        fs::create_dir(&pkg_dir)?;
    }

    let home = &config.home_dir;

    for file in &package.files {
        if let Ok(relative_path) = file.strip_prefix(home) {
            let repo_path = pkg_dir.join(relative_path);

            if let Some(parent) = repo_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // If the file is a symlink, we assume it's already stowed or adopted elsewhere, skip backing up/moving
            if file.read_link().is_err() {
                fs::rename(file, &repo_path)
                    .context(format!("Failed to move file to repo: {:?}", file))?;
            }

            // Create symlink back to home
            symlink::stow_file(&repo_path, file)?;
        }
    }

    Ok(())
}

pub fn stow_package(config: &Config, package: &Package) -> Result<()> {
    let repo_dir = &config.dotfiles_repo;
    let pkg_dir = repo_dir.join(&package.name);
    let home = &config.home_dir;

    for file in &package.files {
        if let Ok(relative_path) = file.strip_prefix(home) {
            let repo_path = pkg_dir.join(relative_path);
            symlink::stow_file(&repo_path, file)?;
        }
    }
    Ok(())
}

pub fn unstow_package(package: &Package) -> Result<()> {
    for file in &package.files {
        symlink::unstow_file(file)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managers::scanner::PackageStatus;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_adopt_package() -> Result<()> {
        let dir = tempdir()?;
        let home = dir.path().join("home");
        let dotfiles = dir.path().join("dotfiles");
        fs::create_dir_all(&home)?;
        fs::create_dir_all(&dotfiles)?;

        let config = Config {
            dotfiles_repo: dotfiles.clone(),
            home_dir: home.clone(),
        };

        let file_path = home.join(".bashrc");
        std::fs::File::create(&file_path)?.write_all(b"export foo=bar")?;

        let package = Package {
            name: "bash".to_string(),
            files: vec![file_path.clone()],
            status: PackageStatus::Unstowed,
        };

        adopt_package(&config, &package)?;

        assert!(file_path.is_symlink());

        let repo_file = dotfiles.join("bash").join(".bashrc");
        assert!(repo_file.exists());
        assert_eq!(std::fs::read_to_string(&repo_file)?, "export foo=bar");

        Ok(())
    }
}
