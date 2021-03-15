use winsys::common::Dim;
use winsys::common::Extents;
use winsys::common::Padding;
use winsys::common::Window;

use std::ops::Add;

#[macro_export]
macro_rules! WM_NAME (
    () => { "wzrd" };
);

pub const MIN_WINDOW_DIM: Dim = Dim {
    w: 75,
    h: 50,
};

pub const NO_DECORATION: Decoration = Decoration {
    border: None,
    frame: None,
};

pub const FREE_DECORATION: Decoration = Decoration {
    border: None,
    frame: Some(Frame {
        extents: Extents {
            left: 3,
            right: 1,
            top: 1,
            bottom: 1,
        },
        colors: ColorScheme::DEFAULT,
    }),
};

pub enum StateChangeError {
    EarlyStop,
    LimitReached,
    StateUnchanged,
}

pub type Color = u32;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ColorScheme {
    pub regular: Color,
    pub focused: Color,
    pub urgent: Color,
    pub rdisowned: Color,
    pub fdisowned: Color,
    pub rsticky: Color,
    pub fsticky: Color,
}

impl ColorScheme {
    const DEFAULT: Self = Self {
        regular: 0x333333,
        focused: 0xe78a53,
        urgent: 0xfbcb97,
        rdisowned: 0x999999,
        fdisowned: 0xc1c1c1,
        rsticky: 0x444444,
        fsticky: 0x5f8787,
    };
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Border {
    pub width: u32,
    pub colors: ColorScheme,
}

impl Add<Border> for Padding {
    type Output = Self;

    fn add(
        self,
        _: Border,
    ) -> Self::Output {
        Self::Output {
            left: self.left + 1,
            right: self.right + 1,
            top: self.top + 1,
            bottom: self.bottom + 1,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Frame {
    pub extents: Extents,
    pub colors: ColorScheme,
}

impl Add<Frame> for Padding {
    type Output = Self;

    fn add(
        self,
        frame: Frame,
    ) -> Self::Output {
        Self::Output {
            left: self.left + frame.extents.left,
            right: self.right + frame.extents.right,
            top: self.top + frame.extents.top,
            bottom: self.bottom + frame.extents.bottom,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Decoration {
    pub border: Option<Border>,
    pub frame: Option<Frame>,
}

impl Default for Decoration {
    fn default() -> Self {
        Self {
            border: None,
            frame: None,
        }
    }
}

impl Decoration {
    pub fn extents(&self) -> Extents {
        Extents {
            left: 0,
            right: 0,
            top: 0,
            bottom: 0,
        } + *self
    }
}

impl Add<Decoration> for Padding {
    type Output = Self;

    fn add(
        mut self,
        decoration: Decoration,
    ) -> Self::Output {
        if let Some(border) = decoration.border {
            self = self + border;
        }

        if let Some(frame) = decoration.frame {
            self = self + frame;
        }

        self
    }
}

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
