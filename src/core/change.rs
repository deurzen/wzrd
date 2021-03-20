use crate::decoration::Decoration;

use winsys::geometry::Region;

use std::ops::Add;
use std::ops::Mul;
use std::ops::Sub;

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
