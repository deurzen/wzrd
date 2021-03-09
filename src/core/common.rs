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

pub const REGULAR_FRAME_COLOR: Color = 0x333333;
pub const FOCUSED_FRAME_COLOR: Color = 0xe78a53;
pub const URGENT_FRAME_COLOR: Color = 0xfbcb97;
pub const REGULAR_DISOWNED_FRAME_COLOR: Color = 0x999999;
pub const FOCUSED_DISOWNED_FRAME_COLOR: Color = 0xc1c1c1;
pub const REGULAR_STICKY_FRAME_COLOR: Color = 0x444444;
pub const FOCUSED_STICKY_FRAME_COLOR: Color = 0x5f8787;

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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Placement {
    pub window: Window,
    pub region: Option<Region>,
    pub extents: Option<Extents>,
}

impl Placement {
    pub fn new(
        window: Window,
        region: Option<Region>,
        extents: Option<Extents>,
    ) -> Self {
        Self {
            window,
            region,
            extents,
        }
    }

    pub fn inner_region(&self) -> Option<Region> {
        if let Some(region) = self.region {
            if let Some(extents) = self.extents {
                return Some(Region {
                    pos: Pos {
                        x: extents.left as i32,
                        y: extents.top as i32,
                    },
                    dim: Dim {
                        w: region.dim.w - extents.left - extents.right,
                        h: region.dim.h - extents.top - extents.bottom,
                    },
                });
            } else {
                return Some(Region {
                    pos: Pos {
                        x: 0,
                        y: 0,
                    },
                    dim: region.dim,
                });
            }
        }

        None
    }
}
