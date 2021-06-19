use winsys::geometry::Extents;
use winsys::geometry::Padding;

use std::ops::Add;

pub type Color = u32;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ColorScheme {
    pub focused: Color,
    pub fdisowned: Color,
    pub fsticky: Color,
    pub unfocused: Color,
    pub udisowned: Color,
    pub usticky: Color,
    pub urgent: Color,
}

impl ColorScheme {
    pub const DEFAULT: Self = Self {
        focused: 0xe78a53,
        fdisowned: 0xc1c1c1,
        fsticky: 0x5f8787,
        unfocused: 0x333333,
        udisowned: 0x999999,
        usticky: 0x444444,
        urgent: 0xfbcb97,
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
        border: Border,
    ) -> Self::Output {
        Self::Output {
            left: self.left + border.width as i32,
            right: self.right + border.width as i32,
            top: self.top + border.width as i32,
            bottom: self.bottom + border.width as i32,
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
