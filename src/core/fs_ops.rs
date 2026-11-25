//! Low-level filesystem helpers. Every function here refuses to overwrite
//! existing data; callers must move occupants out of the way explicitly.

use std::fs;
use std::io::{self, ErrorKind};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// True if *anything* exists at `path`, including a dangling symlink.
pub fn exists_no_follow(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

pub fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

/// Resolve where a symlink points, interpreting relative links against the
/// link's parent directory (GNU Stow creates relative links).
pub fn resolve_link(link: &Path) -> io::Result<PathBuf> {
    let dest = fs::read_link(link)?;
    Ok(if dest.is_absolute() {
        dest
    } else {
        link.parent().unwrap_or(Path::new("/")).join(dest)
    })
}

/// Compare two paths, canonicalizing when possible so `a/../b` and symlinked
/// parent directories compare equal.
pub fn same_path(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// Is `path` equal to or inside `root` (after canonicalization)?
pub fn is_within(path: &Path, root: &Path) -> bool {
    let path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let root = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    path.starts_with(root)
}

/// Move `from` to `to`, creating parent directories. Never overwrites `to`.
/// Falls back to copy + delete when crossing filesystems.
pub fn move_path(from: &Path, to: &Path) -> io::Result<()> {
    if exists_no_follow(to) {
        return Err(io::Error::new(
            ErrorKind::AlreadyExists,
            format!("refusing to overwrite {}", to.display()),
        ));
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::CrossesDevices => move_by_copy(from, to),
        Err(e) => Err(e),
    }
}

/// Copy `from` to `to` recursively (preserving symlinks and permissions),
/// then delete `from`. On copy failure the partial copy is removed and
/// `from` is left untouched.
pub fn move_by_copy(from: &Path, to: &Path) -> io::Result<()> {
    if let Err(e) = copy_recursive(from, to) {
        let _ = remove_path(to);
        return Err(e);
    }
    remove_path(from)
}

fn copy_recursive(from: &Path, to: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(from)?;
    let ft = meta.file_type();
    if ft.is_symlink() {
        symlink(fs::read_link(from)?, to)
    } else if ft.is_dir() {
        fs::create_dir(to)?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            copy_recursive(&entry.path(), &to.join(entry.file_name()))?;
        }
        fs::set_permissions(to, meta.permissions())
    } else {
        fs::copy(from, to).map(|_| ())
    }
}

/// Remove a file, symlink, or directory tree.
pub fn remove_path(path: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// Remove empty directories under (and including) `root`, bottom-up.
/// Best effort: non-empty directories and errors are ignored.
pub fn prune_empty_dirs(root: &Path) {
    let Ok(meta) = fs::symlink_metadata(root) else {
        return;
    };
    if !meta.is_dir() {
        return;
    }
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            prune_empty_dirs(&entry.path());
        }
    }
    let _ = fs::remove_dir(root); // fails harmlessly if not empty
}

/// Pick a fresh backup session directory like `<root>/2026-11-25T10-30-00`,
/// appending `-1`, `-2`, ... if one already exists.
pub fn unique_backup_session(root: &Path) -> PathBuf {
    let stamp = utc_timestamp(SystemTime::now());
    let base = root.join(&stamp);
    if !exists_no_follow(&base) {
        return base;
    }
    (1..)
        .map(|n| root.join(format!("{stamp}-{n}")))
        .find(|p| !exists_no_follow(p))
        .expect("infinite iterator always yields")
}

/// Format a time as `YYYY-MM-DDTHH-MM-SS` (UTC, filesystem safe).
pub fn utc_timestamp(t: SystemTime) -> String {
    let secs = t
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (y, m, d) = civil_from_days(days as i64);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}-{:02}-{:02}",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Days since 1970-01-01 → (year, month, day). Howard Hinnant's algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn timestamp_format() {
        let t = UNIX_EPOCH + Duration::from_secs(1_764_066_600); // 2025-11-25 10:30:00 UTC
        assert_eq!(utc_timestamp(t), "2025-11-25T10-30-00");
        assert_eq!(utc_timestamp(UNIX_EPOCH), "1970-01-01T00-00-00");
    }

    #[test]
    fn move_refuses_to_overwrite() {
        let d = tempdir().unwrap();
        let (a, b) = (d.path().join("a"), d.path().join("b"));
        fs::write(&a, "a").unwrap();
        fs::write(&b, "b").unwrap();
        assert_eq!(
            move_path(&a, &b).unwrap_err().kind(),
            ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read_to_string(&b).unwrap(), "b");
        assert!(a.exists());
    }

    #[test]
    fn move_by_copy_preserves_tree_and_symlinks() {
        let d = tempdir().unwrap();
        let src = d.path().join("src");
        fs::create_dir_all(src.join("sub")).unwrap();
        fs::write(src.join("sub/f.txt"), "hello").unwrap();
        symlink("sub/f.txt", src.join("link")).unwrap();
        let dst = d.path().join("nested/dst");
        fs::create_dir_all(dst.parent().unwrap()).unwrap();

        move_by_copy(&src, &dst).unwrap();

        assert!(!src.exists());
        assert_eq!(fs::read_to_string(dst.join("sub/f.txt")).unwrap(), "hello");
        assert_eq!(
            fs::read_link(dst.join("link")).unwrap(),
            Path::new("sub/f.txt")
        );
    }

    #[test]
    fn backup_sessions_are_unique() {
        let d = tempdir().unwrap();
        let first = unique_backup_session(d.path());
        fs::create_dir_all(&first).unwrap();
        let second = unique_backup_session(d.path());
        assert_ne!(first, second);
    }

    #[test]
    fn prune_keeps_non_empty() {
        let d = tempdir().unwrap();
        fs::create_dir_all(d.path().join("p/a/b")).unwrap();
        fs::create_dir_all(d.path().join("p/c")).unwrap();
        fs::write(d.path().join("p/c/keep"), "").unwrap();
        prune_empty_dirs(&d.path().join("p"));
        assert!(!d.path().join("p/a").exists());
        assert!(d.path().join("p/c/keep").exists());
    }
}
