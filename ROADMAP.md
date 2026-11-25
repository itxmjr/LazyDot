# Roadmap

LazyDot aims to be **the lazygit of dotfiles**: see every config, adopt it in one keypress, sync it with git, and restore it on any machine. Stow compatible, never destructive.

Want to help? Pick an item, look for its issue, and comment before you start.

## ✅ v0.2 "Safe" (released)
Data-loss fixes, real link status, plan → confirm → execute with rollback, timestamped backups, log pane, CLI with `--dry-run`.

## v0.3 "Useful"
- Config file `~/.config/lazydot/config.toml`: ignore globs, extra scan paths, repo path.
- Curated list of known apps (shown first) as a data file that is easy to contribute to.
- Adopt a single file inside a package.
- `/` fuzzy filter, mouse support, syntax-highlighted preview.
- Warn before adopting sensitive files (`.ssh`, tokens, `.env`).

## v0.4 "Git-native"
- Git panel: status, diff, commit, pull and push from inside the TUI.
- First-run onboarding: create a repo, or clone your existing dotfiles and stow them.

## v0.5 "Multi-machine"
- `lazydot restore <git-url>` to bootstrap a new machine in one command.
- Per-host package selection.
- `lazydot doctor` for broken links, conflicts and uncommitted changes.
- macOS paths (`~/Library/Application Support`).

## v1.0
- Stable config and keymap, themes, undo history, docs site.
- Prebuilt binaries, Homebrew, AUR, Nix flake, `cargo binstall`.

Out of scope until after 1.0: templating and secret management.
