use crate::geometry::Dim;
use crate::geometry::Pos;
use crate::geometry::Ratio;
use crate::window::IcccmWindowState;
use crate::window::Window;

#[derive(Debug, Copy, Clone, PartialOrd)]
pub struct SizeHints {
    pub by_user: bool,
    pub pos: Option<Pos>,
    pub min_width: Option<i32>,
    pub min_height: Option<i32>,
    pub max_width: Option<i32>,
    pub max_height: Option<i32>,
    pub base_width: Option<i32>,
    pub base_height: Option<i32>,
    pub inc_width: Option<i32>,
    pub inc_height: Option<i32>,
    pub min_ratio: Option<f64>,
    pub max_ratio: Option<f64>,
    pub min_ratio_vulgar: Option<Ratio>,
    pub max_ratio_vulgar: Option<Ratio>,
}

impl SizeHints {
    fn new(
        by_user: bool,
        pos: Option<Pos>,
        min_width: Option<i32>,
        min_height: Option<i32>,
        max_width: Option<i32>,
        max_height: Option<i32>,
        base_width: Option<i32>,
        base_height: Option<i32>,
        inc_width: Option<i32>,
        inc_height: Option<i32>,
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
        let mut dest_width = dim.w;
        let mut dest_height = dim.h;

        if let Some(min_width) = self.min_width {
            dest_width = std::cmp::max(dest_width, min_width);
        }

        if let Some(min_height) = self.min_height {
            dest_height = std::cmp::max(dest_height, min_height);
        }

        if let Some(max_width) = self.max_width {
            dest_width = std::cmp::min(dest_width, max_width);
        }

        if let Some(max_height) = self.max_height {
            dest_height = std::cmp::min(dest_height, max_height);
        }

        let base_width = if let Some(base_width) = self.base_width {
            base_width
        } else {
            0
        };

        let base_height = if let Some(base_height) = self.base_height {
            base_height
        } else {
            0
        };

        let mut width = if base_width < dest_width {
            dest_width - base_width
        } else {
            dest_width
        };

        let mut height = if base_height < dest_height {
            dest_height - base_height
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

                dest_width = width + base_width;
                dest_height = height + base_height;
            }
        }

        if let Some(inc_height) = self.inc_height {
            if dest_height >= base_height {
                dest_height -= base_height;
                dest_height -= dest_height % inc_height;
                dest_height += base_height;
            }
        }

        if let Some(inc_width) = self.inc_width {
            if dest_width >= base_width {
                dest_width -= base_width;
                dest_width -= dest_width % inc_width;
                dest_width += base_width;
            }
        }

        dim.w = std::cmp::max(dest_width, 0i32);
        dim.h = std::cmp::max(dest_height, 0i32);
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
