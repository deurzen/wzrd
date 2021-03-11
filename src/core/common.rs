use winsys::common::Dim;
use winsys::common::Extents;
use winsys::common::Pos;
use winsys::common::Region;
use winsys::common::Window;

pub type Color = u32;

#[macro_export]
macro_rules! WM_NAME (
    () => { "wzrd" };
);

pub const FOCUSED_FRAME_COLOR: Color = 0xD7005F;
pub const URGENT_FRAME_COLOR: Color = 0xD08928;
pub const REGULAR_DISOWNED_FRAME_COLOR: Color = 0x707070;
pub const FOCUSED_DISOWNED_FRAME_COLOR: Color = 0x00AA80;
pub const REGULAR_STICKY_FRAME_COLOR: Color = 0x6C9EF8;
pub const FOCUSED_STICKY_FRAME_COLOR: Color = 0xB77FDB;
pub const REGULAR_FRAME_COLOR: Color = 0x191A2A;

pub const MIN_WINDOW_DIM: Dim = Dim {
    w: 75,
    h: 50,
};
pub const BORDER_SIZE: u32 = 3;

pub const FREE_EXTENTS: Extents = Extents {
    left: 3,
    right: 1,
    top: 1,
    bottom: 1,
};

pub const NO_EXTENTS: Extents = Extents {
    left: 0,
    right: 0,
    top: 0,
    bottom: 0,
};

pub type Ident = u32;
pub type Index = usize;

pub trait Identify: PartialEq {
    fn id(&self) -> Ident;
}

impl Identify for Window {
    fn id(&self) -> Ident {
        *self as Ident
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}

impl Direction {
    pub fn rev(&self) -> Self {
        match self {
            Self::Forward => Self::Backward,
            Self::Backward => Self::Forward,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Change {
    Inc,
    Dec,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BorderState {
    Urgent,
    Focused,
    Unfocused,
    Disowned,
    Sticky,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct BorderSize {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
}
