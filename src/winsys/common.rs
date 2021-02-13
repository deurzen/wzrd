use std::default::Default;
use std::ops::Add;
use std::ops::AddAssign;
use std::ops::Sub;
use std::ops::SubAssign;

pub type Atom = u32;
pub type Window = u32;
pub type Pid = u32;
pub type Extents = Padding;

pub struct Hex32(pub u32);

impl std::fmt::Debug for Hex32 {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{:#0x}", &self.0)
    }
}

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum IcccmWindowState {
    Withdrawn,
    Normal,
    Iconic,
}

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum WindowState {
    Modal,
    Sticky,
    MaximizedVert,
    MaximizedHorz,
    Shaded,
    SkipTaskbar,
    SkipPager,
    Hidden,
    Fullscreen,
    Above,
    Below,
    DemandsAttention,
}

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum WindowType {
    Desktop,
    Dock,
    Toolbar,
    Menu,
    Utility,
    Splash,
    Dialog,
    DropdownMenu,
    PopupMenu,
    Tooltip,
    Notification,
    Combo,
    Dnd,
    Normal,
}

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum Grip {
    Edge(Edge),
    Corner(Corner),
}

impl Grip {
    pub fn is_top_grip(&self) -> bool {
        *self == Grip::Edge(Edge::Top)
            || *self == Grip::Corner(Corner::TopLeft)
            || *self == Grip::Corner(Corner::TopRight)
    }

    pub fn is_left_grip(&self) -> bool {
        *self == Grip::Edge(Edge::Left)
            || *self == Grip::Corner(Corner::TopLeft)
            || *self == Grip::Corner(Corner::BottomLeft)
    }
}

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
        w: u32,
        h: u32,
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
            && pos.x <= self.pos.x + self.dim.w as i32
            && pos.y <= self.pos.y + self.dim.h as i32
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
                let (left, _) = self
                    .split_at_width((self.dim.w as f64 / 2f64).round() as u32);
                let (topleft, _) = left
                    .split_at_height((left.dim.h as f64 / 2f64).round() as u32);

                Some(Pos::from_center_of_region(topleft))
            },
            (Corner::TopRight, _) => {
                let (_, right) = self
                    .split_at_width((self.dim.w as f64 / 2f64).round() as u32);
                let (topright, _) =
                    right.split_at_height(
                        (right.dim.h as f64 / 2f64).round() as u32
                    );

                Some(Pos::from_center_of_region(topright))
            },
            (Corner::BottomLeft, _) => {
                let (left, _) = self
                    .split_at_width((self.dim.w as f64 / 2f64).round() as u32);
                let (_, bottomleft) = left
                    .split_at_height((left.dim.h as f64 / 2f64).round() as u32);

                Some(Pos::from_center_of_region(bottomleft))
            },
            (Corner::BottomRight, _) => {
                let (_, right) = self
                    .split_at_width((self.dim.w as f64 / 2f64).round() as u32);
                let (_, bottomright) =
                    right.split_at_height(
                        (right.dim.h as f64 / 2f64).round() as u32
                    );

                Some(Pos::from_center_of_region(bottomright))
            },
        }
    }

    pub fn split_at_width(
        &self,
        width: u32,
    ) -> (Self, Self) {
        assert!(width < self.dim.w, "Desired width exceeds divisible width.");

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
                    x: self.pos.x + width as i32,
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
        height: u32,
    ) -> (Self, Self) {
        assert!(
            height < self.dim.h,
            "Desired height exceeds divisible height."
        );

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
                    y: self.pos.y + height as i32,
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
        dim: &Dim,
    ) -> Self {
        if dim.w > self.dim.w || dim.h > self.dim.h {
            return self;
        }

        Self {
            pos: Pos {
                x: self.pos.x + ((self.dim.w - dim.w) as f32 / 2f32) as i32,
                y: self.pos.y + ((self.dim.h - dim.h) as f32 / 2f32) as i32,
            },
            dim: *dim,
        }
    }

    pub fn without_extents(
        mut self,
        extents: &Extents,
    ) -> Self {
        self.pos.x += extents.left as i32;
        self.pos.y += extents.top as i32;
        self.dim.w -= extents.left + extents.right;
        self.dim.h -= extents.top + extents.bottom;
        self
    }

    pub fn with_extents(
        mut self,
        extents: &Extents,
    ) -> Self {
        self.pos.x -= extents.left as i32;
        self.pos.y -= extents.top as i32;
        self.dim.w += extents.left + extents.right;
        self.dim.h += extents.top + extents.bottom;
        self
    }

    pub fn top_right(&self) -> Pos {
        Pos {
            x: self.pos.x + self.dim.w as i32,
            y: self.pos.y,
        }
    }

    pub fn bottom_left(&self) -> Pos {
        Pos {
            x: self.pos.x,
            y: self.pos.y + self.dim.h as i32,
        }
    }

    pub fn bottom_right(&self) -> Pos {
        Pos {
            x: self.pos.x + self.dim.w as i32,
            y: self.pos.y + self.dim.h as i32,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
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
            x: dim.w as i32 / 2,
            y: dim.h as i32 / 2,
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
    pub w: u32,
    pub h: u32,
}

impl Default for Dim {
    fn default() -> Self {
        Self {
            w: 480,
            h: 260,
        }
    }
}

impl Dim {
    pub fn values(&self) -> (u32, u32) {
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
            x: self.x + other.w as i32,
            y: self.y + other.h as i32,
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
            x: self.x - other.w as i32,
            y: self.y - other.h as i32,
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
            w: (self.x as i32 - other.x) as u32,
            h: (self.y as i32 - other.y) as u32,
        }
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

    pub fn pythagorean(&self) -> u32 {
        let dx = self.dx.pow(2) as f64;
        let dy = self.dy.pow(2) as f64;

        (dx + dy).sqrt().round() as u32
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
            w: (self.w as i32 + dist.dx).abs() as u32,
            h: (self.h as i32 + dist.dy).abs() as u32,
        }
    }
}

impl AddAssign<Distance> for Dim {
    fn add_assign(
        &mut self,
        dist: Distance,
    ) {
        *self = Self {
            w: (self.w as i32 + dist.dx).abs() as u32,
            h: (self.h as i32 + dist.dy).abs() as u32,
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
            w: (self.w as i32 - dist.dx).abs() as u32,
            h: (self.h as i32 - dist.dy).abs() as u32,
        }
    }
}

impl SubAssign<Distance> for Dim {
    fn sub_assign(
        &mut self,
        dist: Distance,
    ) {
        *self = Self {
            w: (self.w as i32 - dist.dx).abs() as u32,
            h: (self.h as i32 - dist.dy).abs() as u32,
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

#[derive(Debug, Copy, Clone, PartialOrd)]
pub struct SizeHints {
    pub by_user: bool,
    pub pos: Option<Pos>,
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub base_width: Option<u32>,
    pub base_height: Option<u32>,
    pub inc_width: Option<u32>,
    pub inc_height: Option<u32>,
    pub min_ratio: Option<f64>,
    pub max_ratio: Option<f64>,
    pub min_ratio_vulgar: Option<Ratio>,
    pub max_ratio_vulgar: Option<Ratio>,
}

impl SizeHints {
    fn new(
        by_user: bool,
        pos: Option<Pos>,
        min_width: Option<u32>,
        min_height: Option<u32>,
        max_width: Option<u32>,
        max_height: Option<u32>,
        base_width: Option<u32>,
        base_height: Option<u32>,
        inc_width: Option<u32>,
        inc_height: Option<u32>,
        min_ratio: Option<f64>,
        max_ratio: Option<f64>,
        min_ratio_vulgar: Option<Ratio>,
        max_ratio_vulgar: Option<Ratio>,
    ) -> Self {
        Self {
            by_user,
            pos,
            min_width,
            min_height,
            max_width,
            max_height,
            base_width,
            base_height,
            inc_width,
            inc_height,
            min_ratio,
            max_ratio,
            min_ratio_vulgar,
            max_ratio_vulgar,
        }
    }

    pub fn apply(
        &self,
        dim: &mut Dim,
    ) {
        let mut dest_width = dim.w as i32;
        let mut dest_height = dim.h as i32;

        if let Some(min_width) = self.min_width {
            dest_width = std::cmp::max(dest_width, min_width as i32);
        }

        if let Some(min_height) = self.min_height {
            dest_height = std::cmp::max(dest_height, min_height as i32);
        }

        if let Some(max_width) = self.max_width {
            dest_width = std::cmp::min(dest_width, max_width as i32);
        }

        if let Some(max_height) = self.max_height {
            dest_height = std::cmp::min(dest_height, max_height as i32);
        }

        let base_width = if let Some(base_width) = self.base_width {
            base_width as i32
        } else {
            0
        };

        let base_height = if let Some(base_height) = self.base_height {
            base_height as i32
        } else {
            0
        };

        let mut width = if base_width < dest_width {
            dest_width - base_width as i32
        } else {
            dest_width
        };

        let mut height = if base_height < dest_height {
            dest_height - base_height as i32
        } else {
            dest_height
        };

        if self.min_ratio.is_some() || self.max_ratio.is_some() {
            if height == 0 {
                height = 1;
            }

            let current_ratio = width as f64 / height as f64;
            let mut new_ratio = None;

            if let Some(min_ratio) = self.min_ratio {
                if current_ratio < min_ratio {
                    new_ratio = Some(min_ratio);
                }
            }

            if new_ratio.is_none() {
                if let Some(max_ratio) = self.max_ratio {
                    if current_ratio > max_ratio {
                        new_ratio = Some(max_ratio);
                    }
                }
            }

            if let Some(new_ratio) = new_ratio {
                height = (width as f64 / new_ratio).round() as i32;
                width = (height as f64 * new_ratio).round() as i32;

                dest_width = width + base_width as i32;
                dest_height = height + base_height as i32;
            }
        }

        if let Some(inc_height) = self.inc_height {
            if dest_height >= base_height {
                dest_height -= base_height as i32;
                dest_height -= dest_height % inc_height as i32;
                dest_height += base_height as i32;
            }
        }

        if let Some(inc_width) = self.inc_width {
            if dest_width >= base_width {
                dest_width -= base_width as i32;
                dest_width -= dest_width % inc_width as i32;
                dest_width += base_width as i32;
            }
        }

        dim.w = std::cmp::max(dest_width, 0) as u32;
        dim.h = std::cmp::max(dest_height, 0) as u32;
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Padding {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
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
    pub fn with_each_edge(size: u32) -> Self {
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
                x: self.pos.x - padding.left as i32,
                y: self.pos.y - padding.top as i32,
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
                x: self.pos.x + padding.left as i32,
                y: self.pos.y + padding.top as i32,
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
                x: self.pos.x - padding.left as i32,
                y: self.pos.y - padding.top as i32,
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
                x: self.pos.x + padding.left as i32,
                y: self.pos.y + padding.top as i32,
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

impl PartialEq for SizeHints {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.min_width == other.min_width
            && self.min_height == other.min_height
            && self.max_width == other.max_width
            && self.max_height == other.max_height
            && self.base_width == other.base_width
            && self.base_height == other.base_height
            && self.inc_width == other.inc_width
            && self.inc_height == other.inc_height
            && self.min_ratio_vulgar == other.min_ratio_vulgar
            && self.max_ratio_vulgar == other.max_ratio_vulgar
    }
}

impl Eq for SizeHints {}

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct Hints {
    pub urgent: bool,
    pub input: Option<bool>,
    pub initial_state: Option<IcccmWindowState>,
    pub group: Option<Window>,
}

impl Hints {
    fn new(
        urgent: bool,
        input: Option<bool>,
        initial_state: Option<IcccmWindowState>,
        group: Option<Window>,
    ) -> Self {
        Self {
            urgent,
            input,
            initial_state,
            group,
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
