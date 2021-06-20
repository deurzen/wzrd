use crate::hints::SizeHints;
use crate::window::Window;

use std::ops::Add;
use std::ops::AddAssign;
use std::ops::Sub;
use std::ops::SubAssign;

pub type Extents = Padding;

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

impl Default for Pos {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
        }
    }
}

impl Pos {
    pub fn from_center_of_region(region: Region) -> Self {
        Self {
            x: region.pos.x + (region.dim.w as f32 / 2f32) as i32,
            y: region.pos.y + (region.dim.h as f32 / 2f32) as i32,
        }
    }

    pub fn from_center_of_dim(dim: Dim) -> Self {
        Self {
            x: dim.w / 2,
            y: dim.h / 2,
        }
    }

    pub fn values(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn dist(
        &self,
        pos: Self,
    ) -> Distance {
        Distance {
            dx: (pos.x - self.x),
            dy: (pos.y - self.y),
        }
    }

    pub fn relative_to(
        &self,
        pos: Self,
    ) -> Self {
        Pos {
            x: self.x - pos.x,
            y: self.y - pos.y,
        }
    }

    pub fn is_origin(&self) -> bool {
        *self
            == Pos {
                x: 0,
                y: 0,
            }
    }
}

impl Add<Pos> for Pos {
    type Output = Self;

    fn add(
        self,
        other: Pos,
    ) -> Self::Output {
        Self::Output {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct Dim {
    pub w: i32,
    pub h: i32,
}

impl Default for Dim {
    fn default() -> Self {
        Self {
            w: 0,
            h: 0,
        }
    }
}

impl Dim {
    pub fn values(&self) -> (i32, i32) {
        (self.w, self.h)
    }

    pub fn center(&self) -> Pos {
        Pos {
            x: (self.w as f32 / 2f32) as i32,
            y: (self.h as f32 / 2f32) as i32,
        }
    }

    pub fn nearest_corner(
        &self,
        pos: Pos,
    ) -> Corner {
        let center = self.center();

        if pos.x >= center.x {
            if pos.y >= center.y {
                Corner::BottomRight
            } else {
                Corner::TopRight
            }
        } else {
            if pos.y >= center.y {
                Corner::BottomLeft
            } else {
                Corner::TopLeft
            }
        }
    }
}

impl Add<Dim> for Pos {
    type Output = Self;

    fn add(
        self,
        other: Dim,
    ) -> Self::Output {
        Self::Output {
            x: self.x + other.w,
            y: self.y + other.h,
        }
    }
}

impl Sub<Dim> for Pos {
    type Output = Self;

    fn sub(
        self,
        other: Dim,
    ) -> Self::Output {
        Self::Output {
            x: self.x - other.w,
            y: self.y - other.h,
        }
    }
}

impl Sub for Pos {
    type Output = Dim;

    fn sub(
        self,
        other: Self,
    ) -> Self::Output {
        Self::Output {
            w: self.x - other.x,
            h: self.y - other.y,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct Region {
    pub pos: Pos,
    pub dim: Dim,
}

impl Default for Region {
    fn default() -> Self {
        Self {
            pos: Default::default(),
            dim: Default::default(),
        }
    }
}

impl Region {
    pub fn new(
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> Self {
        Self {
            pos: Pos {
                x,
                y,
            },
            dim: Dim {
                w,
                h,
            },
        }
    }

    pub fn values(&self) -> (Pos, Dim) {
        (self.pos, self.dim)
    }

    pub fn with_size_hints(
        self,
        size_hints: &Option<SizeHints>,
    ) -> Self {
        let mut geometry = self;

        if let Some(size_hints) = size_hints {
            size_hints.apply(&mut geometry.dim);
        }

        geometry
    }

    pub fn encompasses(
        &self,
        pos: Pos,
    ) -> bool {
        pos.x >= self.pos.x
            && pos.y >= self.pos.y
            && pos.x <= self.pos.x + self.dim.w
            && pos.y <= self.pos.y + self.dim.h
    }

    pub fn contains(
        &self,
        region: Region,
    ) -> bool {
        self.encompasses(region.pos) && self.encompasses(region.bottom_right())
    }

    pub fn occludes(
        &self,
        region: Region,
    ) -> bool {
        self.encompasses(region.pos) || region.encompasses(self.pos)
    }

    pub fn nearest_corner(
        &self,
        mut pos: Pos,
    ) -> Corner {
        pos += self.pos.dist(Pos {
            x: 0,
            y: 0,
        });
        self.dim.nearest_corner(pos)
    }

    pub fn quadrant_center_from_pos(
        &self,
        pos: Pos,
    ) -> Option<Pos> {
        if self.encompasses(pos) {
            return None;
        }

        let mut dists = vec![
            (Corner::TopLeft, self.pos.dist(pos).pythagorean()),
            (Corner::TopRight, self.top_right().dist(pos).pythagorean()),
            (
                Corner::BottomLeft,
                self.bottom_left().dist(pos).pythagorean(),
            ),
            (
                Corner::BottomRight,
                self.bottom_right().dist(pos).pythagorean(),
            ),
        ];

        dists.sort_by_key(|&(corner, dist)| dist);

        match dists.first().unwrap() {
            (Corner::TopLeft, _) => {
                let (left, _) = self.split_at_width((self.dim.w as f64 / 2f64).round() as i32);
                let (topleft, _) = left.split_at_height((left.dim.h as f64 / 2f64).round() as i32);

                Some(Pos::from_center_of_region(topleft))
            },
            (Corner::TopRight, _) => {
                let (_, right) = self.split_at_width((self.dim.w as f64 / 2f64).round() as i32);
                let (topright, _) =
                    right.split_at_height((right.dim.h as f64 / 2f64).round() as i32);

                Some(Pos::from_center_of_region(topright))
            },
            (Corner::BottomLeft, _) => {
                let (left, _) = self.split_at_width((self.dim.w as f64 / 2f64).round() as i32);
                let (_, bottomleft) =
                    left.split_at_height((left.dim.h as f64 / 2f64).round() as i32);

                Some(Pos::from_center_of_region(bottomleft))
            },
            (Corner::BottomRight, _) => {
                let (_, right) = self.split_at_width((self.dim.w as f64 / 2f64).round() as i32);
                let (_, bottomright) =
                    right.split_at_height((right.dim.h as f64 / 2f64).round() as i32);

                Some(Pos::from_center_of_region(bottomright))
            },
        }
    }

    pub fn split_at_width(
        &self,
        width: i32,
    ) -> (Self, Self) {
        let width = std::cmp::min(width, self.dim.w);

        (
            Self {
                dim: Dim {
                    w: width,
                    ..self.dim
                },
                ..*self
            },
            Self {
                pos: Pos {
                    x: self.pos.x + width,
                    ..self.pos
                },
                dim: Dim {
                    w: self.dim.w - width,
                    ..self.dim
                },
            },
        )
    }

    pub fn split_at_height(
        &self,
        height: i32,
    ) -> (Self, Self) {
        let height = std::cmp::min(height, self.dim.h);

        (
            Self {
                dim: Dim {
                    h: height,
                    ..self.dim
                },
                ..*self
            },
            Self {
                pos: Pos {
                    y: self.pos.y + height,
                    ..self.pos
                },
                dim: Dim {
                    h: self.dim.h - height,
                    ..self.dim
                },
            },
        )
    }

    pub fn with_minimum_dim(
        self,
        minimum_dim: &Dim,
    ) -> Self {
        Self {
            pos: self.pos,
            dim: Dim {
                w: std::cmp::max(minimum_dim.w, self.dim.w),
                h: std::cmp::max(minimum_dim.h, self.dim.h),
            },
        }
    }

    pub fn with_maximum_dim(
        self,
        maximum_dim: &Dim,
    ) -> Self {
        Self {
            pos: self.pos,
            dim: Dim {
                w: std::cmp::min(maximum_dim.w, self.dim.w),
                h: std::cmp::min(maximum_dim.h, self.dim.h),
            },
        }
    }

    pub fn from_absolute_inner_center(
        self,
        dim: Dim,
    ) -> Self {
        Self {
            pos: Pos {
                x: if dim.w > self.dim.w {
                    self.pos.x
                } else {
                    self.pos.x + ((self.dim.w - dim.w) as f32 / 2f32) as i32
                },
                y: if dim.h > self.dim.h {
                    self.pos.y
                } else {
                    self.pos.y + ((self.dim.h - dim.h) as f32 / 2f32) as i32
                },
            },
            dim,
        }
    }

    pub fn without_extents(
        mut self,
        extents: Extents,
    ) -> Self {
        self.pos.x += extents.left;
        self.pos.y += extents.top;
        self.dim.w -= extents.left + extents.right;
        self.dim.h -= extents.top + extents.bottom;
        self
    }

    pub fn with_extents(
        mut self,
        extents: Extents,
    ) -> Self {
        self.pos.x -= extents.left;
        self.pos.y -= extents.top;
        self.dim.w += extents.left + extents.right;
        self.dim.h += extents.top + extents.bottom;
        self
    }

    pub fn top_right(&self) -> Pos {
        Pos {
            x: self.pos.x + self.dim.w,
            y: self.pos.y,
        }
    }

    pub fn bottom_left(&self) -> Pos {
        Pos {
            x: self.pos.x,
            y: self.pos.y + self.dim.h,
        }
    }

    pub fn bottom_right(&self) -> Pos {
        Pos {
            x: self.pos.x + self.dim.w,
            y: self.pos.y + self.dim.h,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Padding {
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
}

impl Default for Padding {
    fn default() -> Self {
        Self {
            left: 0,
            right: 0,
            top: 0,
            bottom: 0,
        }
    }
}

impl Padding {
    pub fn with_each_edge(size: i32) -> Self {
        Self {
            left: size,
            right: size,
            top: size,
            bottom: size,
        }
    }
}

impl Add<Padding> for Region {
    type Output = Self;

    fn add(
        self,
        padding: Padding,
    ) -> Self::Output {
        Self::Output {
            pos: Pos {
                x: self.pos.x - padding.left,
                y: self.pos.y - padding.top,
            },
            dim: Dim {
                w: self.dim.w + padding.left + padding.right,
                h: self.dim.h + padding.top + padding.bottom,
            },
        }
    }
}

impl Sub<Padding> for Region {
    type Output = Self;

    fn sub(
        self,
        padding: Padding,
    ) -> Self::Output {
        Self::Output {
            pos: Pos {
                x: self.pos.x + padding.left,
                y: self.pos.y + padding.top,
            },
            dim: Dim {
                w: self.dim.w - padding.left - padding.right,
                h: self.dim.h - padding.top - padding.bottom,
            },
        }
    }
}

impl AddAssign<Padding> for Region {
    fn add_assign(
        &mut self,
        padding: Padding,
    ) {
        *self = Self {
            pos: Pos {
                x: self.pos.x - padding.left,
                y: self.pos.y - padding.top,
            },
            dim: Dim {
                w: self.dim.w + padding.left + padding.right,
                h: self.dim.h + padding.top + padding.bottom,
            },
        };
    }
}

impl SubAssign<Padding> for Region {
    fn sub_assign(
        &mut self,
        padding: Padding,
    ) {
        *self = Self {
            pos: Pos {
                x: self.pos.x + padding.left,
                y: self.pos.y + padding.top,
            },
            dim: Dim {
                w: self.dim.w - padding.left - padding.right,
                h: self.dim.h - padding.top - padding.bottom,
            },
        };
    }
}

impl Add<Padding> for Dim {
    type Output = Self;

    fn add(
        self,
        padding: Padding,
    ) -> Self::Output {
        Self::Output {
            w: self.w + padding.left + padding.right,
            h: self.h + padding.top + padding.bottom,
        }
    }
}

impl Sub<Padding> for Dim {
    type Output = Self;

    fn sub(
        self,
        padding: Padding,
    ) -> Self::Output {
        Self::Output {
            w: self.w - padding.left - padding.right,
            h: self.h - padding.top - padding.bottom,
        }
    }
}

impl AddAssign<Padding> for Dim {
    fn add_assign(
        &mut self,
        padding: Padding,
    ) {
        *self = Self {
            w: self.w + padding.left + padding.right,
            h: self.h + padding.top + padding.bottom,
        };
    }
}

impl SubAssign<Padding> for Dim {
    fn sub_assign(
        &mut self,
        padding: Padding,
    ) {
        *self = Self {
            w: self.w - padding.left - padding.right,
            h: self.h - padding.top - padding.bottom,
        };
    }
}

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct Distance {
    pub dx: i32,
    pub dy: i32,
}

impl Distance {
    pub fn values(&self) -> (i32, i32) {
        (self.dx, self.dy)
    }

    pub fn pythagorean(&self) -> i32 {
        let dx = self.dx.pow(2) as f64;
        let dy = self.dy.pow(2) as f64;

        (dx + dy).sqrt().round() as i32
    }
}

impl Add<Distance> for Pos {
    type Output = Self;

    fn add(
        self,
        dist: Distance,
    ) -> Self::Output {
        Self::Output {
            x: self.x + dist.dx,
            y: self.y + dist.dy,
        }
    }
}

impl AddAssign<Distance> for Pos {
    fn add_assign(
        &mut self,
        dist: Distance,
    ) {
        *self = Self {
            x: self.x + dist.dx,
            y: self.y + dist.dy,
        };
    }
}

impl Sub<Distance> for Pos {
    type Output = Self;

    fn sub(
        self,
        dist: Distance,
    ) -> Self::Output {
        Self::Output {
            x: self.x - dist.dx,
            y: self.y - dist.dy,
        }
    }
}

impl SubAssign<Distance> for Pos {
    fn sub_assign(
        &mut self,
        dist: Distance,
    ) {
        *self = Self {
            x: self.x - dist.dx,
            y: self.y - dist.dy,
        };
    }
}

impl Add<Distance> for Dim {
    type Output = Self;

    fn add(
        self,
        dist: Distance,
    ) -> Self::Output {
        Self::Output {
            w: (self.w + dist.dx).abs(),
            h: (self.h + dist.dy).abs(),
        }
    }
}

impl AddAssign<Distance> for Dim {
    fn add_assign(
        &mut self,
        dist: Distance,
    ) {
        *self = Self {
            w: (self.w + dist.dx).abs(),
            h: (self.h + dist.dy).abs(),
        };
    }
}

impl Sub<Distance> for Dim {
    type Output = Self;

    fn sub(
        self,
        dist: Distance,
    ) -> Self::Output {
        Self::Output {
            w: (self.w - dist.dx).abs(),
            h: (self.h - dist.dy).abs(),
        }
    }
}

impl SubAssign<Distance> for Dim {
    fn sub_assign(
        &mut self,
        dist: Distance,
    ) {
        *self = Self {
            w: (self.w - dist.dx).abs(),
            h: (self.h - dist.dy).abs(),
        };
    }
}

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct Ratio {
    pub numerator: i32,
    pub denominator: i32,
}

impl Ratio {
    pub fn new(
        numerator: i32,
        denominator: i32,
    ) -> Self {
        Self {
            numerator,
            denominator,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Strut {
    pub window: Window,
    pub width: u32,
}

impl Strut {
    pub fn new(
        window: Window,
        width: u32,
    ) -> Self {
        Self {
            window,
            width,
        }
    }
}

impl PartialOrd for Strut {
    fn partial_cmp(
        &self,
        other: &Self,
    ) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Strut {
    fn cmp(
        &self,
        other: &Self,
    ) -> std::cmp::Ordering {
        other.width.cmp(&self.width)
    }
}
