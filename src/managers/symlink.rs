use anyhow::{Context, Result};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

pub fn stow_file(source: &Path, target: &Path) -> Result<()> {
    if target.exists() || target.read_link().is_ok() {
        if let Ok(link_target) = target.read_link() {
            if link_target == source {
                return Ok(()); // Already properly stowed, do nothing!
            }
            // It's a symlink but points elsewhere, remove it cleanly to recreate
            fs::remove_file(target)?;
        } else if target.exists() {
            // Original real file exists, back it up to avoid data loss
            let mut backup = target.to_path_buf();
            backup.set_extension("bak");
            fs::rename(target, &backup).context("Failed to backup existing file")?;
        }
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    symlink(source, target).context(format!(
        "Failed to create symlink from {:?} to {:?}",
        source, target
    ))?;
    Ok(())
}

pub fn unstow_file(target: &Path) -> Result<()> {
    if target.exists() && target.read_link().is_ok() {
        fs::remove_file(target).context("Failed to remove symlink")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_stow_and_unstow() -> Result<()> {
        let dir = tempdir()?;
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::File::create(&source)?.write_all(b"test")?;

        stow_file(&source, &target)?;
        assert!(target.is_symlink());
        assert_eq!(target.read_link()?, source);

        unstow_file(&target)?;
        assert!(!target.exists());

        Ok(())
    }

    #[test]
    fn test_stow_backup() -> Result<()> {
        let dir = tempdir()?;
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::File::create(&source)?.write_all(b"new")?;
        std::fs::File::create(&target)?.write_all(b"old")?;

        stow_file(&source, &target)?;
        assert!(target.is_symlink());

        let backup = dir.path().join("target.bak");
        assert!(backup.exists());
        assert_eq!(std::fs::read_to_string(backup)?, "old");

        Ok(())
    }
}
