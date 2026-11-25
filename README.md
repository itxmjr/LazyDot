# 🟢 LazyDot

![Rust](https://img.shields.io/badge/Rust-1.88+-orange.svg?style=for-the-badge&logo=rust&logoColor=white)
![Ratatui](https://img.shields.io/badge/Ratatui-0.30-5f87ff.svg?style=for-the-badge)
[![Crates.io](https://img.shields.io/crates/v/lazydot.svg?style=for-the-badge&logo=rust)](https://crates.io/crates/lazydot)
[![CI](https://img.shields.io/github/actions/workflow/status/itxmjr/LazyDot/ci.yml?branch=main&style=for-the-badge&logo=github)](https://github.com/itxmjr/LazyDot/actions)
![License](https://img.shields.io/badge/License-MIT-green.svg?style=for-the-badge)

A lazygit-style terminal UI for your dotfiles. LazyDot finds the configs in your home directory, moves them into a git-friendly **`~/.dotfiles`** repo with one keypress, and symlinks them back. The repo uses the **GNU Stow** layout, so there is no lock-in: an existing Stow repo works on day one, and you can stop using LazyDot at any time.

[Report Bug](https://github.com/itxmjr/LazyDot/issues/new?template=bug_report.yml) · [Request Feature](https://github.com/itxmjr/LazyDot/issues/new?template=feature_request.yml) · [Discussions](https://github.com/itxmjr/LazyDot/discussions)

## ✨ Features

- **Auto discovery**: lists `~/.config/*` apps, `~/.*rc` files and common files like `.gitconfig` and `.tmux.conf`. Browser profiles and caches are hidden.
- **Real link status**: every file shows as linked, not linked, conflict, untracked or managed by another tool. Status is read from the filesystem, never guessed.
- **Adopt / Stow / Unstow / Restore**: move configs into the repo, link them on a new machine, remove links, or move everything back out.
- **Preview before change**: every action shows the exact moves and links it will make and waits for `y`.
- **Never destroys data**: files in the way are moved to a timestamped backup folder. A failed step rolls back the steps before it. Links owned by other tools (Nix, home-manager) are left alone.
- **GNU Stow compatible**: same `<repo>/<package>/<path from $HOME>` layout, and understands the relative links Stow creates.
- **Scriptable CLI**: `status`, `adopt`, `stow`, `unstow`, `restore`, all with `--dry-run`.

## 🛠️ Tech Stack

- [Rust](https://www.rust-lang.org/): single static binary, no runtime.
- [Ratatui](https://ratatui.rs/) + [crossterm](https://github.com/crossterm-rs/crossterm): terminal UI.
- [clap](https://github.com/clap-rs/clap): command-line parsing.

## 📦 Installation

Prerequisites: Linux or macOS, and Rust 1.88+ if you build from source.

1. Install from crates.io:

   ```bash
   cargo install lazydot
   ```

2. Or build from source:

   ```bash
   git clone https://github.com/itxmjr/LazyDot.git
   cd LazyDot
   cargo install --path .
   ```

## 🎮 Usage

1. Open the TUI:

   ```bash
   lazydot
   ```

2. Pick a config in the **Packages** pane (`+` means untracked) and press `a` to adopt it. Check the plan and press `y`.
3. On a new machine, clone your dotfiles repo to `~/.dotfiles`, run `lazydot`, select a package and press `s` to stow it. Or link everything at once:

   ```bash
   lazydot stow --all
   ```

4. Try anything safely first with `--dry-run`:

   ```bash
   lazydot --dry-run adopt nvim zshrc
   lazydot status
   ```

### Keybindings

| Key | Action |
|---|---|
| `j` / `k`, `↓` / `↑` | Move selection |
| `g` / `G` | Jump to top / bottom |
| `h` / `l`, `Tab` | Switch between packages and files |
| `J` / `K`, `PgDn` / `PgUp` | Scroll preview |
| `a` | Adopt: move into the repo and symlink back |
| `s` | Stow: link repo files into `$HOME` (conflicts are backed up) |
| `u` / `D` | Unstow: remove links (repo untouched) |
| `R` | Restore: move files back out of the repo |
| `r` | Rescan |
| `?` | Help |
| `q` / `Esc` / `Ctrl-c` | Quit |

### Status icons

`✓` stowed · `◐` partially stowed · `○` unstowed · `!` conflict · `+` untracked

## ⚙️ Configuration

| Setting | Default | Description |
|---|---|---|
| `--repo <PATH>` / `LAZYDOT_REPO` | `~/.dotfiles` | Location of your dotfiles repo |
| `--dry-run` | off | Show plans without changing files |
| Backups | `$XDG_STATE_HOME/lazydot/backups/<timestamp>/` (`~/.local/state/...`) | Where files that were in the way are moved |

A config file with ignore patterns and extra scan paths is planned. See the [roadmap](ROADMAP.md).

## 📂 Project Structure

```text
src/
├── main.rs          # Binary entry point
├── lib.rs           # Library root
├── cli.rs           # clap CLI and subcommands
├── config.rs        # Paths: home, repo, backups
├── core/            # Filesystem logic, no UI code
│   ├── model.rs     # Package / Entry / status types
│   ├── scan.rs      # Repo walk (Stow-style) + $HOME discovery
│   ├── ops.rs       # adopt / stow / unstow / restore → Plan
│   ├── plan.rs      # Plan execution with rollback
│   └── fs_ops.rs    # Safe move, backups, timestamps
└── tui/             # Ratatui front end
    ├── app.rs       # State machine (testable, no I/O)
    ├── ui.rs        # Rendering
    └── theme.rs     # Colors
tests/
└── safety.rs        # End-to-end tests against a fake $HOME
```

## 🤝 Contributing

Contributions are welcome. Start with [CONTRIBUTING.md](CONTRIBUTING.md) and issues labeled [`good first issue`](https://github.com/itxmjr/LazyDot/labels/good%20first%20issue).

1. Fork the repository.
2. Create your feature branch (`git checkout -b feature/AmazingFeature`).
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`).
4. Push to the branch (`git push origin feature/AmazingFeature`).
5. Open a Pull Request.

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🌐 Connect

<div align="center">

<a href="https://mjawad.tech"><img src="https://img.shields.io/badge/M%20Jawad-Portfolio-39ff14?style=for-the-badge&logo=googlechrome&logoColor=white&labelColor=000000" alt="Portfolio" /></a>
<a href="https://linkedin.com/in/itxmjr"><img src="https://img.shields.io/badge/LinkedIn-0A66C2?style=for-the-badge&logo=linkedin&logoColor=white" alt="LinkedIn" /></a>
<a href="https://github.com/itxmjr"><img src="https://img.shields.io/badge/GitHub-181717?style=for-the-badge&logo=github&logoColor=white" alt="GitHub" /></a>
<a href="https://youtube.com/@aibymjr"><img src="https://img.shields.io/badge/YouTube-FF0000?style=for-the-badge&logo=youtube&logoColor=white" alt="YouTube" /></a>
<a href="https://www.fiverr.com/aibymjr"><img src="https://img.shields.io/badge/Fiverr-1DBF73?style=for-the-badge&logo=fiverr&logoColor=white" alt="Fiverr" /></a>
<a href="https://www.upwork.com/freelancers/~011c75cbed1d1a6c4b"><img src="https://img.shields.io/badge/Upwork-14A800?style=for-the-badge&logo=upwork&logoColor=white" alt="Upwork" /></a>

<sub>Built with ❤️ by M Jawad ur Rehman.</sub>

</div>
