pub use crate::Result;

use crate::geometry::Dim;
use crate::geometry::Pos;
use crate::geometry::Region;
use crate::input::Grip;
use crate::input::KeyCode;
use crate::input::MouseEvent;
use crate::screen::Screen;
use crate::window::Window;
use crate::window::WindowState;
use crate::window::WindowType;

#[derive(Debug, Clone)]
pub enum Event {
    Mouse {
        event: MouseEvent,
    },
    Key {
        key_code: KeyCode,
    },
    MapRequest {
        window: Window,
        ignore: bool,
    },
    Map {
        window: Window,
        ignore: bool,
    },
    Enter {
        window: Window,
        root_rpos: Pos,
        window_rpos: Pos,
    },
    Leave {
        window: Window,
        root_rpos: Pos,
        window_rpos: Pos,
    },
    Destroy {
        window: Window,
    },
    Expose {
        window: Window,
    },
    Unmap {
        window: Window,
        ignore: bool,
    },
    StateRequest {
        window: Window,
        state: WindowState,
        action: ToggleAction,
        on_root: bool,
    },
    FocusRequest {
        window: Window,
        on_root: bool,
    },
    CloseRequest {
        window: Window,
        on_root: bool,
    },
    WorkspaceRequest {
        window: Option<Window>,
        index: usize,
        on_root: bool,
    },
    PlacementRequest {
        window: Window,
        pos: Option<Pos>,
        dim: Option<Dim>,
        on_root: bool,
    },
    GripRequest {
        window: Window,
        pos: Pos,
        grip: Option<Grip>,
        on_root: bool,
    },
    RestackRequest {
        window: Window,
        sibling: Window,
        mode: StackMode,
        on_root: bool,
    },
    Configure {
        window: Window,
        region: Region,
        on_root: bool,
    },
    Property {
        window: Window,
        kind: PropertyKind,
        on_root: bool,
    },
    FrameExtentsRequest {
        window: Window,
        on_root: bool,
    },
    Mapping {
        request: u8,
    },
    ScreenChange,
    Randr,
}

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum StackMode {
    Above,
    Below,
}

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum ToggleAction {
    Toggle,
    Add,
    Remove,
}

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum PropertyKind {
    Name,
    Class,
    Size,
    Strut,
}
