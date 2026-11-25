//! Command-line interface. With no subcommand, LazyDot opens the TUI.

use crate::config::Config;
use crate::core::model::{EntryState, Package};
use crate::core::{ops, scan, Plan};
use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "lazydot",
    version,
    about = "A lazygit-style TUI for your dotfiles (GNU Stow compatible)",
    long_about = "Run without arguments to open the TUI.\n\n\
                  Your dotfiles repo uses the GNU Stow layout: <repo>/<package>/<path relative to $HOME>."
)]
pub struct Cli {
    /// Path to the dotfiles repo [default: ~/.dotfiles]
    #[arg(long, global = true, env = "LAZYDOT_REPO", value_name = "PATH")]
    pub repo: Option<PathBuf>,

    /// Show what would change without touching any files
    #[arg(long, global = true)]
    pub dry_run: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List packages and the link state of every file
    Status,
    /// Move config(s) from $HOME into the repo and symlink them back
    Adopt {
        #[arg(required = true)]
        packages: Vec<String>,
    },
    /// Symlink repo package(s) into $HOME (conflicting files are backed up)
    Stow {
        packages: Vec<String>,
        /// Stow every package in the repo
        #[arg(long, conflicts_with = "packages")]
        all: bool,
    },
    /// Remove symlinks for package(s); repo files are untouched
    Unstow {
        #[arg(required = true)]
        packages: Vec<String>,
    },
    /// Move package(s) out of the repo back into $HOME as regular files
    Restore {
        #[arg(required = true)]
        packages: Vec<String>,
    },
}

pub fn run(cli: Cli) -> Result<()> {
    let config = Config::load(cli.repo)?;
    let Some(command) = cli.command else {
        return crate::tui::run(config, cli.dry_run);
    };
    let packages = scan::scan(&config);

    match command {
        Command::Status => {
            print_status(&config, &packages);
            Ok(())
        }
        Command::Adopt { packages: names } => run_plans(
            &config,
            &packages,
            &names,
            false,
            cli.dry_run,
            ops::plan_adopt,
        ),
        Command::Stow {
            packages: names,
            all,
        } => {
            let names = if all {
                packages
                    .iter()
                    .filter(|p| p.tracked)
                    .map(|p| p.name.clone())
                    .collect()
            } else if names.is_empty() {
                bail!("give at least one package name, or use --all");
            } else {
                names
            };
            run_plans(
                &config,
                &packages,
                &names,
                true,
                cli.dry_run,
                ops::plan_stow,
            )
        }
        Command::Unstow { packages: names } => run_plans(
            &config,
            &packages,
            &names,
            true,
            cli.dry_run,
            ops::plan_unstow,
        ),
        Command::Restore { packages: names } => run_plans(
            &config,
            &packages,
            &names,
            true,
            cli.dry_run,
            ops::plan_restore,
        ),
    }
}

fn find<'a>(packages: &'a [Package], name: &str, tracked: bool) -> Option<&'a Package> {
    packages
        .iter()
        .find(|p| p.name == name && p.tracked == tracked)
        .or_else(|| packages.iter().find(|p| p.name == name))
}

fn run_plans(
    config: &Config,
    packages: &[Package],
    names: &[String],
    want_tracked: bool,
    dry_run: bool,
    build: fn(&Config, &Package) -> Plan,
) -> Result<()> {
    let mut failures = 0;
    for name in names {
        let Some(pkg) = find(packages, name, want_tracked) else {
            eprintln!("error: no package named '{name}' (see `lazydot status`)");
            failures += 1;
            continue;
        };
        let plan = build(config, pkg);
        println!("{}", plan.title);
        for action in &plan.actions {
            println!("  {}", action.describe(config));
        }
        for skip in &plan.skipped {
            println!("  skip: {skip}");
        }
        if plan.is_noop() {
            println!("  nothing to do");
            continue;
        }
        if dry_run {
            println!("  (dry run, nothing changed)");
            continue;
        }
        match plan.execute() {
            Ok(()) => println!("  done"),
            Err(e) => {
                eprintln!("error: {e:#}");
                failures += 1;
            }
        }
    }
    if failures > 0 {
        bail!("{failures} package(s) failed");
    }
    Ok(())
}

fn print_status(config: &Config, packages: &[Package]) {
    println!("repo: {}", config.display_path(&config.dotfiles_repo));
    for pkg in packages {
        println!("{:<24} {}", pkg.name, pkg.status().label());
        if !pkg.tracked {
            continue;
        }
        for e in &pkg.entries {
            let state = match &e.state {
                EntryState::Linked => "linked".to_string(),
                EntryState::NotLinked => "not linked".to_string(),
                EntryState::Conflict(o) => format!("conflict ({o:?} in the way)").to_lowercase(),
                EntryState::Untracked => "untracked".to_string(),
                EntryState::ForeignLink(d) => format!("foreign link → {}", d.display()),
            };
            println!("    {:<40} {state}", config.display_path(&e.target));
        }
    }
}
