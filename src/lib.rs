//! Fancy Window: a tiling host for other applications' windows.
//!
//! Flow (Model-View-Update): Win32 event -> `Msg` -> `update` -> effects -> repaint.

pub mod app;
pub mod model;
pub mod platform;
pub mod view;
