//! The Nerd Font helper behind the settings card (Step 12o, ADR-055).
//!
//! The terminal, not TokenRadar or Claude Code, picks the font the statusline is drawn with.
//! So the helper does what can be done from outside: it finds out whether a Nerd Font is
//! installed ([`fonts`]), downloads and installs a pinned one with the user's consent
//! ([`download`], [`install`]), and selects it in Windows Terminal and VS Code
//! ([`terminals`]).

pub mod download;
pub mod fonts;
pub mod install;
pub mod terminals;

pub use fonts::{FontScan, NerdTier};
pub use terminals::{TargetId, TerminalKind};
