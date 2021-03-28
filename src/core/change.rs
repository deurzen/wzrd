use crate::decoration::Decoration;

use winsys::geometry::Region;

use std::ops::Add;
use std::ops::Mul;
use std::ops::Sub;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Toggle {
    On,
    Off,
    Reverse,
}

impl From<bool> for Toggle {
    #[inline(always)]
    fn from(toggle: bool) -> Self {
        match toggle {
            true => Toggle::On,
            false => Toggle::Off,
        }
    }
}

impl Toggle {
    #[inline(always)]
    pub fn eval(
        self,
        current: bool,
    ) -> bool {
        match self {
            Toggle::On => true,
            Toggle::Off => false,
            Toggle::Reverse => !current,
        }
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
pub enum Change<T>
where
    T: Add<Output = T> + Sub<Output = T> + Mul<Output = T>,
{
    Inc(T),
    Dec(T),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Disposition {
    Unchanged(Decoration),
    Changed(Region, Decoration),
}
