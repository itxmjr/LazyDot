//! Turn user intents (adopt / stow / unstow / restore) into [`Plan`]s.
//! Building a plan never touches the filesystem, so plans double as dry runs.

use crate::config::Config;
use crate::core::fs_ops;
use crate::core::model::{EntryState, Occupant, Package};
use crate::core::plan::{Action, Plan};

/// Move an untracked package into the repo and symlink it back.
pub fn plan_adopt(config: &Config, pkg: &Package) -> Plan {
    let mut plan = Plan::new(format!("Adopt {}", pkg.name));
    if pkg.tracked {
        plan.skipped
            .push(format!("{} is already in your dotfiles repo", pkg.name));
        return plan;
    }
    let pkg_dir = config.dotfiles_repo.join(&pkg.name);

    for entry in &pkg.entries {
        let shown = config.display_path(&entry.target);
        match &entry.state {
            EntryState::Untracked => {}
            EntryState::ForeignLink(dest) => {
                plan.skipped.push(format!(
                    "{shown} is a symlink to {} (managed by another tool?)",
                    dest.display()
                ));
                continue;
            }
            _ => continue,
        }
        let Ok(rel) = entry.target.strip_prefix(&config.home_dir) else {
            plan.skipped
                .push(format!("{shown} is outside your home directory"));
            continue;
        };
        let dest = pkg_dir.join(rel);
        if fs_ops::exists_no_follow(&dest) {
            plan.skipped.push(format!(
                "{} already exists in the repo",
                config.display_path(&dest)
            ));
            continue;
        }
        plan.actions.push(Action::Move {
            from: entry.target.clone(),
            to: dest.clone(),
        });
        plan.actions.push(Action::Symlink {
            link: entry.target.clone(),
            points_to: dest,
        });
    }
    plan
}

/// Link every repo file of a tracked package into $HOME. Anything in the way
/// is moved into a timestamped backup directory first, never deleted.
pub fn plan_stow(config: &Config, pkg: &Package) -> Plan {
    let mut plan = Plan::new(format!("Stow {}", pkg.name));
    if !pkg.tracked {
        plan.skipped.push(format!(
            "{} is not in your dotfiles repo yet; adopt it first",
            pkg.name
        ));
        return plan;
    }
    let session = fs_ops::unique_backup_session(&config.backup_dir);

    for entry in &pkg.entries {
        let Some(source) = &entry.source else {
            continue;
        };
        match &entry.state {
            EntryState::Linked => continue,
            EntryState::NotLinked => {}
            EntryState::Conflict(occupant) => {
                let rel = entry
                    .target
                    .strip_prefix(&config.home_dir)
                    .unwrap_or(&entry.target);
                let rel = rel.strip_prefix("/").unwrap_or(rel);
                plan.actions.push(Action::Backup {
                    from: entry.target.clone(),
                    to: session.join(rel),
                });
                if let Occupant::Symlink(dest) = occupant {
                    plan.skipped.push(format!(
                        "note: existing link {} → {} will be backed up",
                        config.display_path(&entry.target),
                        dest.display()
                    ));
                }
            }
            EntryState::Untracked | EntryState::ForeignLink(_) => continue,
        }
        plan.actions.push(Action::Symlink {
            link: entry.target.clone(),
            points_to: source.clone(),
        });
    }
    plan
}

/// Remove symlinks that point into the repo. Repo files are untouched.
pub fn plan_unstow(_config: &Config, pkg: &Package) -> Plan {
    let mut plan = Plan::new(format!("Unstow {}", pkg.name));
    for entry in &pkg.entries {
        if let (EntryState::Linked, Some(source)) = (&entry.state, &entry.source) {
            plan.actions.push(Action::RemoveLink {
                link: entry.target.clone(),
                points_to: source.clone(),
            });
        }
    }
    plan
}

/// Un-adopt: move repo files back to their home locations as real files and
/// remove the (now empty) package directory. The inverse of adopt.
pub fn plan_restore(config: &Config, pkg: &Package) -> Plan {
    let mut plan = Plan::new(format!("Restore {}", pkg.name));
    if !pkg.tracked {
        plan.skipped
            .push(format!("{} is not in your dotfiles repo", pkg.name));
        return plan;
    }
    for entry in &pkg.entries {
        let Some(source) = &entry.source else {
            continue;
        };
        match &entry.state {
            EntryState::Linked => plan.actions.push(Action::RemoveLink {
                link: entry.target.clone(),
                points_to: source.clone(),
            }),
            EntryState::NotLinked => {}
            EntryState::Conflict(_) => {
                plan.skipped.push(format!(
                    "{} is occupied by something else; left in repo",
                    config.display_path(&entry.target)
                ));
                continue;
            }
            EntryState::Untracked | EntryState::ForeignLink(_) => continue,
        }
        plan.actions.push(Action::Move {
            from: source.clone(),
            to: entry.target.clone(),
        });
    }
    if !plan.actions.is_empty() {
        plan.actions.push(Action::PruneEmptyDirs {
            root: config.dotfiles_repo.join(&pkg.name),
        });
    }
    plan
}
