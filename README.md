# LazyDot 🟢

A modern, fast, and gorgeous TUI (Terminal User Interface) dotfiles manager. LazyDot automates creating a dotfiles repository and symlinking (like GNU Stow), but with the UX of tools like lazygit.

## 🎯 Features
- **Auto Discovery**: Automatically detects your dotfiles in `~/.config/`, `~/.*rc`, and more!
- **3-Pane Interface**: Left pane for packages, center for configurations, right for live file previews.
- **Adopt Mode**: Move your files to `~/.dotfiles` and automatically replace them with a symlink. No manual moving required!
- **Stow Engine**: Restore dotfiles to any machine safely without overwriting data by accident.
- **Vibrant UI**: Lazygit-inspired neon green interface.

## ⌨️ Shortcuts
| Key Action | Description |
|---|---|
| `q` or `Esc` | Quit the application |
| `↑` / `k` | Move up |
| `↓` / `j` | Move down |
| `←`/`→` or `h`/`l` or `Tab` | Toggle between Package List and File List |
| `a` | **Adopt** the selected package (moves files to `~/.dotfiles`) |
| `s` | **Stow** the selected package (creates symlinks back to `~`) |
| `D` or `u` | **Unstow** the selected package (removes symlinks) |

## 🚀 Installation & Usage
Make sure you have Rust installed.
```bash
cargo install --path .
```
Then simply run:
```bash
lazydot
```

Your dotfiles repository will automatically be tracked at `~/.dotfiles/`.
