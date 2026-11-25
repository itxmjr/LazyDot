//! LazyDot: a lazygit-style TUI for managing dotfiles.
//!
//! The crate is split into:
//! - [`core`]: filesystem logic (scanning, link status, plans, safe file operations).
//!   It has no UI dependencies and is fully unit/integration tested.
//! - [`tui`]: the ratatui front end.
//! - [`cli`]: argument parsing and the non-interactive subcommands.

#[cfg(not(unix))]
compile_error!(
    "LazyDot currently supports Unix-like systems only (Linux, macOS, BSD). \
     Windows support is tracked on the issue tracker; contributions welcome!"
);

pub mod cli;
pub mod config;
pub mod core;
pub mod tui;
