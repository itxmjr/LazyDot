# Changelog

All notable changes to this project are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.0] - 2026-11-25

A safety-focused rewrite. **Upgrading from 0.1.x is strongly recommended.**

### Fixed
- Stowing a package that exists only in the repo (the new-machine case) no longer moves the repo folder to `.bak` and leaves a dangling link.
- Files in the way are no longer renamed to `<name>.bak` beside the original (which overwrote earlier backups). They are moved to `$XDG_STATE_HOME/lazydot/backups/<timestamp>/`.
- Adopting a symlink owned by another tool (Nix, home-manager, …) no longer replaces it with a dangling link. It is skipped with an explanation.
- Moving across filesystems falls back to copy + delete instead of failing.
- A failure partway through an operation rolls back the steps already done.
- Errors are shown in the new log pane instead of being ignored.
- The terminal is restored on errors and panics.
- Selecting a package after the list shrinks no longer panics.
- Unstow removes dangling links too, and only ever removes links that point into the repo.

### Added
- Real per-file link status read from disk: linked, not linked, conflict, untracked, managed by another tool. Packages show stowed / partial / unstowed / conflict.
- GNU Stow compatible repo scanning, including Stow's relative symlinks.
- Confirm dialog listing every move, link and backup before anything happens.
- `R` restore: move a package back out of the repo as regular files.
- Log pane, context key hints, `r` rescan, `g`/`G`, preview scrolling.
- CLI: `status`, `adopt`, `stow [--all]`, `unstow`, `restore`, `--repo`, `--dry-run`, `--version`.
- Preview is size-limited and detects binary files.
- Discovery hides browser profiles and caches, and covers more common files.

### Changed
- Removed unused dependencies (`tokio`, `serde`, `serde_json`, `toml`). Upgraded to ratatui 0.30.
- Mouse capture is off, so normal terminal text selection works.
- Clear compile error on unsupported platforms (Windows).

## [0.1.1] - 2026-03-24
## [0.1.0] - 2026-03-23
- Initial release.

[Unreleased]: https://github.com/itxmjr/LazyDot/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/itxmjr/LazyDot/compare/v0.1.1...v0.2.0
