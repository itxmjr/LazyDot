# Contributing to LazyDot

Thanks for helping! LazyDot touches people's home directories, so **safety comes before features**. This guide explains how the code is laid out and what a good PR looks like.

## Quick start

```bash
git clone https://github.com/itxmjr/LazyDot.git
cd LazyDot
cargo test
```

Run against a throwaway home so you never touch your real dotfiles:

```bash
export T=$(mktemp -d)
mkdir -p $T/home/.config/nvim && echo 'set number' > $T/home/.config/nvim/init.vim
HOME=$T/home XDG_STATE_HOME=$T/state cargo run
```

## Before you open a PR

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

CI runs the same commands on Linux and macOS.

## Architecture

| Path | What lives there |
|---|---|
| `src/core/model.rs` | `Package`, `Entry`, `EntryState`, `PackageStatus` |
| `src/core/scan.rs` | Walks the repo like GNU Stow, classifies links, discovers untracked configs in `$HOME` |
| `src/core/ops.rs` | Turns an intent (adopt / stow / unstow / restore) into a `Plan`. **Never touches disk.** |
| `src/core/plan.rs` | Executes a `Plan` step by step and rolls back on failure |
| `src/core/fs_ops.rs` | Low-level helpers that refuse to overwrite anything |
| `src/tui/app.rs` | TUI state machine. No terminal I/O, so it is unit tested |
| `src/tui/ui.rs` | Ratatui rendering |
| `src/cli.rs` | clap CLI and subcommands |

### Safety rules (please read)

1. **Every mutation goes through a `Plan`.** Do not call `fs::rename`, `fs::remove_*` or `symlink` from UI or CLI code.
2. **Never delete or overwrite user data.** Anything in the way is moved into the backup directory.
3. **Status comes from the filesystem**, not from names or caches.
4. **Every new operation needs a test** in `tests/safety.rs` using the fake-`$HOME` helper.

## Good first contributions

- **Add a known config location or ignore entry**: edit `IGNORED_CONFIG_DIRS` or `KNOWN_HOME_FILES` in `src/core/scan.rs` and add a test.
- Look at issues labeled [`good first issue`](https://github.com/itxmjr/LazyDot/labels/good%20first%20issue) and [`help wanted`](https://github.com/itxmjr/LazyDot/labels/help%20wanted).
- Comment on an issue before starting so nobody duplicates work.

## Commits and PRs

- Use [Conventional Commits](https://www.conventionalcommits.org/): `fix(scan): ...`, `feat(tui): ...`, `docs: ...`.
- Keep PRs focused. One fix or feature per PR.
- Add a line to `CHANGELOG.md` under **Unreleased**.
- UI changes: include a screenshot or a short recording.

## Questions

Ask in [Discussions](https://github.com/itxmjr/LazyDot/discussions). Use issues for bugs and concrete feature requests.

By contributing you agree that your contributions are licensed under the MIT License and that you follow the [Code of Conduct](CODE_OF_CONDUCT.md).
