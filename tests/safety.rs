//! End-to-end safety tests against a fake $HOME in a temp dir.
//! Each "C<n>" test reproduces a data-loss bug found in v0.1.x.

use lazydot::config::Config;
use lazydot::core::model::{EntryState, Occupant, Package, PackageStatus};
use lazydot::core::{ops, scan};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

struct Env {
    _dir: TempDir,
    config: Config,
}

impl Env {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        fs::create_dir_all(home.join(".config")).unwrap();
        let config = Config::new(
            home.clone(),
            home.join(".dotfiles"),
            dir.path().join("state/backups"),
        );
        Self { _dir: dir, config }
    }
    fn home(&self) -> &Path {
        &self.config.home_dir
    }
    fn repo(&self) -> &Path {
        &self.config.dotfiles_repo
    }
    fn write(&self, path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    fn pkg(&self, name: &str, tracked: bool) -> Package {
        scan::scan(&self.config)
            .into_iter()
            .find(|p| p.name == name && p.tracked == tracked)
            .unwrap_or_else(|| panic!("package {name} (tracked={tracked}) not found"))
    }
    fn files_in(&self, dir: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        if let Ok(rd) = fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    out.extend(self.files_in(&p));
                } else {
                    out.push(p);
                }
            }
        }
        out
    }
}

#[test]
fn adopt_dir_and_rc_file_then_scan_reports_stowed() {
    let env = Env::new();
    env.write(
        &env.home().join(".config/nvim/init.lua"),
        "vim.o.number = true",
    );
    env.write(&env.home().join(".bashrc"), "export A=1");

    for name in ["nvim", "bashrc"] {
        ops::plan_adopt(&env.config, &env.pkg(name, false))
            .execute()
            .unwrap();
    }

    let nvim = env.pkg("nvim", true);
    assert_eq!(nvim.status(), PackageStatus::Stowed);
    assert_eq!(nvim.entries[0].target, env.home().join(".config/nvim"));
    assert_eq!(
        fs::read_to_string(env.repo().join("nvim/.config/nvim/init.lua")).unwrap(),
        "vim.o.number = true"
    );
    // Content is still reachable through the link.
    assert_eq!(
        fs::read_to_string(env.home().join(".config/nvim/init.lua")).unwrap(),
        "vim.o.number = true"
    );
    assert_eq!(env.pkg("bashrc", true).status(), PackageStatus::Stowed);
    // Adopted things no longer show up as untracked candidates.
    assert!(scan::scan(&env.config)
        .iter()
        .all(|p| p.tracked || (p.name != "nvim" && p.name != "bashrc")));
}

/// C1: stowing a repo-only package on a fresh machine used to move the repo
/// folder to `.bak` and leave a dangling symlink.
#[test]
fn c1_fresh_machine_stow_links_into_home_and_keeps_repo() {
    let env = Env::new();
    let repo_file = env.repo().join("nvim/.config/nvim/init.lua");
    env.write(&repo_file, "-- from repo");

    let pkg = env.pkg("nvim", true);
    assert_eq!(pkg.status(), PackageStatus::Unstowed);
    ops::plan_stow(&env.config, &pkg).execute().unwrap();

    assert!(repo_file.is_file(), "repo content must be untouched");
    let link = env.home().join(".config/nvim");
    assert!(link.is_symlink());
    assert_eq!(
        fs::read_to_string(link.join("init.lua")).unwrap(),
        "-- from repo"
    );
    assert_eq!(env.pkg("nvim", true).status(), PackageStatus::Stowed);
}

/// C2: conflicts used to be renamed to `<name>.bak` beside the original,
/// silently overwriting any previous `.bak`.
#[test]
fn c2_conflicts_are_backed_up_without_overwriting() {
    let env = Env::new();
    env.write(&env.repo().join("kitty/.config/kitty/kitty.conf"), "repo");
    // ~/.config/kitty is a real folder, so (like GNU Stow) the link unit is
    // the file inside it, and that file is what's in the way.
    let target = env.home().join(".config/kitty/kitty.conf");
    env.write(&target, "local v1");
    // A pre-existing sibling .bak must survive untouched.
    env.write(&env.home().join(".config/kitty.bak"), "precious");

    let pkg = env.pkg("kitty", true);
    assert_eq!(pkg.entries[0].state, EntryState::Conflict(Occupant::File));
    ops::plan_stow(&env.config, &pkg).execute().unwrap();
    assert_eq!(
        fs::read_to_string(env.home().join(".config/kitty.bak")).unwrap(),
        "precious"
    );

    // Second conflict: unstow, recreate a local file, stow again.
    ops::plan_unstow(&env.config, &env.pkg("kitty", true))
        .execute()
        .unwrap();
    env.write(&target, "local v2");
    ops::plan_stow(&env.config, &env.pkg("kitty", true))
        .execute()
        .unwrap();

    let mut backups: Vec<String> = env
        .files_in(&env.config.backup_dir)
        .iter()
        .map(|p| fs::read_to_string(p).unwrap())
        .collect();
    backups.sort();
    assert_eq!(backups, vec!["local v1", "local v2"]);
    assert!(target.is_symlink());
    assert_eq!(fs::read_to_string(&target).unwrap(), "repo");
}

/// C3: adopting a symlink owned by another tool used to replace it with a
/// dangling link.
#[test]
fn c3_adopt_skips_foreign_symlinks() {
    let env = Env::new();
    let elsewhere = env.home().join("nix-store/alacritty");
    env.write(&elsewhere.join("alacritty.toml"), "nix managed");
    symlink(&elsewhere, env.home().join(".config/alacritty")).unwrap();

    let pkg = env.pkg("alacritty", false);
    assert!(matches!(pkg.entries[0].state, EntryState::ForeignLink(_)));
    let plan = ops::plan_adopt(&env.config, &pkg);
    assert!(plan.is_noop());
    assert_eq!(plan.skipped.len(), 1);
    plan.execute().unwrap();

    assert_eq!(
        fs::read_link(env.home().join(".config/alacritty")).unwrap(),
        elsewhere
    );
    assert!(!env.repo().join("alacritty").exists());
}

/// C4: a failure halfway through used to leave a half-adopted package.
#[test]
fn c4_failed_adopt_rolls_back_completed_steps() {
    let env = Env::new();
    env.write(&env.home().join(".config/fish/config.fish"), "set -x A 1");
    let pkg = env.pkg("fish", false);
    let plan = ops::plan_adopt(&env.config, &pkg);
    assert_eq!(plan.actions.len(), 2); // move + symlink

    // Something appears at the destination between planning and executing.
    env.write(&env.repo().join("fish/.config/fish"), "in the way");
    let err = plan.execute().unwrap_err().to_string();
    assert!(
        err.contains("rolled back") || err.contains("step 1"),
        "{err}"
    );

    // Home is exactly as before.
    let home_cfg = env.home().join(".config/fish");
    assert!(!home_cfg.is_symlink());
    assert_eq!(
        fs::read_to_string(home_cfg.join("config.fish")).unwrap(),
        "set -x A 1"
    );
}

#[test]
fn c4_failure_at_later_step_undoes_earlier_steps() {
    let env = Env::new();
    env.write(&env.home().join(".zshrc"), "zsh");
    env.write(&env.home().join(".config/zsh/aliases"), "alias");
    // Build a combined plan for two packages so the 2nd one fails after the 1st succeeded.
    let mut plan = ops::plan_adopt(&env.config, &env.pkg("zshrc", false));
    plan.actions
        .extend(ops::plan_adopt(&env.config, &env.pkg("zsh", false)).actions);
    env.write(&env.repo().join("zsh/.config/zsh"), "blocker");

    assert!(plan.execute().is_err());
    let zshrc = env.home().join(".zshrc");
    assert!(!zshrc.is_symlink(), "first package's adopt must be undone");
    assert_eq!(fs::read_to_string(&zshrc).unwrap(), "zsh");
    assert!(!env.repo().join("zshrc/.zshrc").exists());
}

#[test]
fn unstow_only_removes_our_links_and_keeps_repo() {
    let env = Env::new();
    env.write(
        &env.home().join(".config/helix/config.toml"),
        "theme = \"x\"",
    );
    ops::plan_adopt(&env.config, &env.pkg("helix", false))
        .execute()
        .unwrap();

    ops::plan_unstow(&env.config, &env.pkg("helix", true))
        .execute()
        .unwrap();
    assert!(!env.home().join(".config/helix").exists());
    assert!(env.repo().join("helix/.config/helix/config.toml").is_file());
    assert_eq!(env.pkg("helix", true).status(), PackageStatus::Unstowed);
}

#[test]
fn restore_round_trip_returns_real_files_and_cleans_repo() {
    let env = Env::new();
    env.write(
        &env.home().join(".config/tmux/tmux.conf"),
        "set -g mouse on",
    );
    ops::plan_adopt(&env.config, &env.pkg("tmux", false))
        .execute()
        .unwrap();
    ops::plan_restore(&env.config, &env.pkg("tmux", true))
        .execute()
        .unwrap();

    let dir = env.home().join(".config/tmux");
    assert!(dir.is_dir() && !dir.is_symlink());
    assert_eq!(
        fs::read_to_string(dir.join("tmux.conf")).unwrap(),
        "set -g mouse on"
    );
    assert!(!env.repo().join("tmux").exists());
    assert_eq!(env.pkg("tmux", false).status(), PackageStatus::Untracked);
}

#[test]
fn understands_relative_links_made_by_gnu_stow() {
    let env = Env::new();
    env.write(&env.repo().join("git/.gitconfig"), "[user]");
    // GNU Stow (run from ~/.dotfiles/git) makes: ~/.gitconfig -> .dotfiles/git/.gitconfig
    symlink(".dotfiles/git/.gitconfig", env.home().join(".gitconfig")).unwrap();
    assert_eq!(env.pkg("git", true).status(), PackageStatus::Stowed);
}

#[test]
fn building_plans_never_touches_disk() {
    let env = Env::new();
    env.write(&env.home().join(".config/foot/foot.ini"), "font=x");
    env.write(&env.repo().join("bat/.config/bat/config"), "--theme");
    env.write(&env.home().join(".config/bat/config"), "local");
    let before = scan::scan(&env.config);
    for pkg in &before {
        let _ = ops::plan_adopt(&env.config, pkg);
        let _ = ops::plan_stow(&env.config, pkg);
        let _ = ops::plan_unstow(&env.config, pkg);
        let _ = ops::plan_restore(&env.config, pkg);
    }
    assert_eq!(before, scan::scan(&env.config));
    assert!(!env.config.backup_dir.exists());
}

#[test]
fn discovery_ignores_noise_and_the_repo_itself() {
    let env = Env::new();
    env.write(
        &env.home().join(".config/google-chrome/Default/Cookies"),
        "x",
    );
    env.write(&env.home().join(".config/waybar/config"), "{}");
    let names: Vec<String> = scan::scan(&env.config)
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert!(names.contains(&"waybar".to_string()));
    assert!(!names.contains(&"google-chrome".to_string()));

    // A repo living under ~/.config must not be offered for adoption.
    let mut cfg = env.config.clone();
    cfg.dotfiles_repo = env.home().join(".config/dotfiles");
    fs::create_dir_all(&cfg.dotfiles_repo).unwrap();
    assert!(scan::scan(&cfg).iter().all(|p| p.name != "dotfiles"));
}

#[test]
fn cli_status_and_dry_run_work_against_fake_home() {
    let env = Env::new();
    env.write(&env.home().join(".config/nvim/init.lua"), "x");
    let bin = env!("CARGO_BIN_EXE_lazydot");

    let out = std::process::Command::new(bin)
        .env("HOME", env.home())
        .env_remove("XDG_STATE_HOME")
        .env_remove("LAZYDOT_REPO")
        .args(["--dry-run", "adopt", "nvim"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("dry run"), "{stdout}");
    assert!(!env.home().join(".config/nvim").is_symlink());

    let out = std::process::Command::new(bin)
        .env("HOME", env.home())
        .env_remove("XDG_STATE_HOME")
        .env_remove("LAZYDOT_REPO")
        .arg("status")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("nvim"));

    let out = std::process::Command::new(bin)
        .env("HOME", env.home())
        .env_remove("XDG_STATE_HOME")
        .args(["stow", "does-not-exist"])
        .output()
        .unwrap();
    assert!(!out.status.success());
}

#[test]
fn folder_holding_tracked_files_is_not_listed_twice() {
    let env = Env::new();
    env.write(&env.repo().join("nvim/.config/nvim/init.lua"), "repo");
    env.write(&env.home().join(".config/nvim/lazy-lock.json"), "{}");
    let pkgs = scan::scan(&env.config);
    let nvims: Vec<_> = pkgs.iter().filter(|p| p.name == "nvim").collect();
    assert_eq!(nvims.len(), 1);
    assert!(nvims[0].tracked);
    assert_eq!(
        nvims[0].entries[0].target,
        env.home().join(".config/nvim/init.lua")
    );
}
