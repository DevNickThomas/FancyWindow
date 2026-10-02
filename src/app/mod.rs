//! Application logic: what happens in response to each message. Pure, no Win32.

mod attach;
mod command;
mod frame;
mod headers;
mod hotkeys;
mod menu;
mod msg;
mod state;
mod status;
mod update;
mod workspace;

pub use command::Command;
pub use hotkeys::CONFIGURABLE;
pub use attach::{ActiveHighlight, Attachment, WindowId, host_rect, visible_rect};
pub use headers::{HEADER_HEIGHT, HeaderHit, ZoneHeader};
pub use frame::Frame;
pub use menu::{MOUSE_GESTURES, Menu, MenuAction, MenuItem, PRESETS, menu_bar, zone_menu};
pub use msg::{Button, Effect, Modifiers, Msg};
pub use state::{AppState, CursorKind, Drag};
pub use status::{Segment, StatusBar};
pub use update::update;
pub use workspace::workspace_menu;
