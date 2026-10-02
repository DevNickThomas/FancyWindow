//! Application logic: what happens in response to each message. Pure, no Win32.

mod attach;
mod command;
mod editor;
mod frame;
mod headers;
mod hotkeys;
mod instance;
mod menu;
mod msg;
mod palette;
mod state;
mod status;
mod update;
mod workspace;

pub use command::Command;
pub use hotkeys::CONFIGURABLE;
pub use attach::{ActiveHighlight, Attachment, WindowId, ZONE_GAP, host_rect, visible_rect};
pub use headers::{HEADER_HEIGHT, HeaderHit, ZoneHeader};
pub use frame::Frame;
pub use menu::{MOUSE_GESTURES, Menu, MenuAction, MenuItem, PRESETS, menu_bar, zone_menu};
pub use msg::{Button, Effect, Modifiers, Msg};
pub use palette::{PaletteEntry, PaletteMatch, filter as filter_palette};
pub use state::{AppState, CursorKind, Drag};
pub use status::{Segment, SegmentKind, StatusBar, StatusClick};
pub use update::update;
pub use workspace::workspace_menu;
