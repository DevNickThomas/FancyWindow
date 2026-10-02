//! The Win32 edge: owns the window, turns OS messages into `Msg`, runs `Effect`s.

mod chrome;
pub mod crash;
pub mod host;
mod dialogs;
mod hotkeys;
mod indicator;
mod input;
mod menus;
mod placement;
mod preferences;
mod shortcuts;
pub mod storage;
mod window;

pub use window::run;
