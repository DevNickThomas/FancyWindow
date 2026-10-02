//! Pure data and rules. Nothing here knows about Win32.

mod geometry;
mod hotkey;
mod ids;
mod layout;
mod node;
mod rebuild;
mod settings;

pub use geometry::{Point, Rect, ZoneRect};
pub use hotkey::{Chord, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN};
pub use ids::{SplitId, ZoneId};
pub use layout::{GridLayout, JoinDirection, LayoutError};
pub use settings::{MAX_MARGIN, Settings, WORKSPACE_SLOTS, WindowBounds, Workspace, crash_log_name, profile_from_args, settings_file_name};
pub use node::{GridNode, Orientation, SPLITTER_THICKNESS, SplitChild, SplitNode, SplitterHandle};
