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
