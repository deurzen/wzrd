use crate::change::Disposition;
use crate::decoration::Decoration;
use crate::decoration::Frame;
use crate::error::StateChangeError;
use crate::identify::Ident;
use crate::identify::Identify;
use crate::placement::PlacementMethod;
use crate::zone::Zone;

use winsys::geometry::Dim;
use winsys::geometry::Extents;
use winsys::geometry::Padding;
use winsys::geometry::Pos;
use winsys::geometry::Region;

use strum::EnumCount;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use strum_macros::ToString;

use std::collections::HashMap;
use std::string::ToString;
use std::vec::Vec;

type LayoutFn = fn(&Region, &LayoutData, Vec<bool>) -> Vec<(Disposition, bool)>;

#[non_exhaustive]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct LayoutConfig {
    pub method: PlacementMethod,
    pub decoration: Decoration,
    pub root_only: bool,
    pub margin: bool,
    pub gap: bool,
    pub persistent: bool,
    pub single: bool,
    pub wraps: bool,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            method: PlacementMethod::Free,
            decoration: Default::default(),
            root_only: true,
            margin: false,
            gap: false,
            persistent: false,
            single: false,
            wraps: true,
        }
    }
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct LayoutData {
    /// Generic layout data
    pub margin: Padding,
    pub gap_size: u32,

    /// Tiled layout data
    pub main_count: u32,
    pub main_factor: f32,
}

impl Default for LayoutData {
    fn default() -> Self {
        Self {
            margin: Default::default(),
            gap_size: 0u32,

            main_count: 1u32,
            main_factor: 0.50f32,
        }
    }
}

#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, EnumIter, EnumCount, ToString)]
pub enum LayoutKind {
    /// Free layouts
    Float = b'f',
    BLFloat = b'F',
    SingleFloat = b'z',
    BLSingleFloat = b'Z',

    /// Tiled layouts
    // Overlapping
    Center = b';',
    Monocle = b'%',
    // Non-overlapping
    Paper = b'p',
    SPaper = b'P',
    Stack = b's',
    SStack = b'S',
    BStack = b'b',
    SBStack = b'B',
    Horz = b'h',
    SHorz = b'H',
    Vert = b'v',
    SVert = b'V',
}

impl LayoutKind {
    pub fn symbol(&self) -> char {
        (*self as u8) as char
    }

    pub fn name(&self) -> String {
        self.to_string()
    }

    pub fn config(&self) -> LayoutConfig {
        match *self {
            LayoutKind::Float => LayoutConfig {
                method: PlacementMethod::Free,
                decoration: Decoration::FREE_DECORATION,
                root_only: true,
                margin: false,
                gap: false,
                persistent: false,
                single: false,
                wraps: true,
            },
            LayoutKind::BLFloat => LayoutConfig {
                method: PlacementMethod::Free,
                decoration: Decoration::NO_DECORATION,
                root_only: true,
                margin: false,
                gap: false,
                persistent: false,
                single: false,
                wraps: true,
            },
            LayoutKind::SingleFloat => LayoutConfig {
                method: PlacementMethod::Free,
                decoration: Decoration::FREE_DECORATION,
                root_only: true,
                margin: false,
                gap: false,
                persistent: true,
                single: true,
                wraps: true,
            },
            LayoutKind::BLSingleFloat => LayoutConfig {
                method: PlacementMethod::Free,
                decoration: Decoration::NO_DECORATION,
                root_only: true,
                margin: false,
                gap: false,
                persistent: true,
                single: true,
                wraps: true,
            },
            LayoutKind::Center => LayoutConfig {
                method: PlacementMethod::Tile,
                decoration: Decoration::NO_DECORATION,
                root_only: false,
                margin: true,
                gap: true,
                persistent: false,
                single: false,
                wraps: true,
            },
            LayoutKind::Monocle => LayoutConfig {
                method: PlacementMethod::Tile,
                decoration: Decoration::NO_DECORATION,
                root_only: false,
                margin: true,
                gap: true,
                persistent: false,
                single: false,
                wraps: true,
            },
            LayoutKind::Paper => LayoutConfig {
                method: PlacementMethod::Tile,
                decoration: Decoration {
                    frame: Some(Frame {
                        extents: Extents {
                            left: 1,
                            right: 1,
                            top: 0,
                            bottom: 0,
                        },
                        colors: Default::default(),
                    }),
                    border: None,
                },
                root_only: false,
                margin: true,
                gap: true,
                persistent: true,
                single: false,
                wraps: false,
            },
            LayoutKind::SPaper => LayoutConfig {
                method: PlacementMethod::Tile,
                decoration: Decoration {
                    frame: Some(Frame {
                        extents: Extents {
                            left: 1,
                            right: 1,
                            top: 0,
                            bottom: 0,
                        },
                        colors: Default::default(),
                    }),
                    border: None,
                },
                root_only: false,
                margin: true,
                gap: false,
                persistent: true,
                single: false,
                wraps: false,
            },
            LayoutKind::Stack => LayoutConfig {
                method: PlacementMethod::Tile,
                decoration: Decoration {
                    frame: Some(Frame {
                        extents: Extents {
                            left: 0,
                            right: 0,
                            top: 3,
                            bottom: 0,
                        },
                        colors: Default::default(),
                    }),
                    border: None,
                },
                root_only: false,
                margin: true,
                gap: true,
                persistent: false,
                single: false,
                wraps: true,
            },
            LayoutKind::SStack => LayoutConfig {
                method: PlacementMethod::Tile,
                decoration: Decoration {
                    frame: Some(Frame {
                        extents: Extents {
                            left: 0,
                            right: 0,
                            top: 3,
                            bottom: 0,
                        },
                        colors: Default::default(),
                    }),
                    border: None,
                },
                root_only: false,
                margin: true,
                gap: false,
                persistent: false,
                single: false,
                wraps: true,
            },
            LayoutKind::BStack => LayoutConfig {
                method: PlacementMethod::Tile,
                decoration: Decoration {
                    frame: Some(Frame {
                        extents: Extents {
                            left: 0,
                            right: 0,
                            top: 3,
                            bottom: 0,
                        },
                        colors: Default::default(),
                    }),
                    border: None,
                },
                root_only: false,
                margin: true,
                gap: true,
                persistent: false,
                single: false,
                wraps: true,
            },
            LayoutKind::SBStack => LayoutConfig {
                method: PlacementMethod::Tile,
                decoration: Decoration {
                    frame: Some(Frame {
                        extents: Extents {
                            left: 0,
                            right: 0,
                            top: 3,
                            bottom: 0,
                        },
                        colors: Default::default(),
                    }),
                    border: None,
                },
                root_only: false,
                margin: true,
                gap: false,
                persistent: false,
                single: false,
                wraps: true,
            },
            LayoutKind::Horz => LayoutConfig {
                method: PlacementMethod::Tile,
                decoration: Decoration {
                    frame: Some(Frame {
                        extents: Extents {
                            left: 0,
                            right: 0,
                            top: 3,
                            bottom: 0,
                        },
                        colors: Default::default(),
                    }),
                    border: None,
                },
                root_only: false,
                margin: true,
                gap: true,
                persistent: false,
                single: false,
                wraps: true,
            },
            LayoutKind::SHorz => LayoutConfig {
                method: PlacementMethod::Tile,
                decoration: Decoration {
                    frame: Some(Frame {
                        extents: Extents {
                            left: 0,
                            right: 0,
                            top: 3,
                            bottom: 0,
                        },
                        colors: Default::default(),
                    }),
                    border: None,
                },
                root_only: false,
                margin: true,
                gap: false,
                persistent: false,
                single: false,
                wraps: true,
            },
            LayoutKind::Vert => LayoutConfig {
                method: PlacementMethod::Tile,
                decoration: Decoration {
                    frame: Some(Frame {
                        extents: Extents {
                            left: 0,
                            right: 0,
                            top: 3,
                            bottom: 0,
                        },
                        colors: Default::default(),
                    }),
                    border: None,
                },
                root_only: false,
                margin: true,
                gap: true,
                persistent: false,
                single: false,
                wraps: true,
            },
            LayoutKind::SVert => LayoutConfig {
                method: PlacementMethod::Tile,
                decoration: Decoration {
                    frame: Some(Frame {
                        extents: Extents {
                            left: 0,
                            right: 0,
                            top: 3,
                            bottom: 0,
                        },
                        colors: Default::default(),
                    }),
                    border: None,
                },
                root_only: false,
                margin: true,
                gap: false,
                persistent: false,
                single: false,
                wraps: true,
            },

            #[allow(unreachable_patterns)]
            _ => unimplemented!("{:?} does not have an associated configuration", self),
        }
    }

    fn default_data(&self) -> LayoutData {
        match *self {
            LayoutKind::Float => Default::default(),
            LayoutKind::BLFloat => Default::default(),
            LayoutKind::SingleFloat => Default::default(),
            LayoutKind::BLSingleFloat => Default::default(),
            LayoutKind::Center => LayoutData {
                main_count: 5u32,
                main_factor: 0.40f32,
                ..Default::default()
            },
            LayoutKind::Monocle => Default::default(),
            LayoutKind::Paper => Default::default(),
            LayoutKind::SPaper => Default::default(),
            LayoutKind::Stack => LayoutData {
                main_count: 1u32,
                main_factor: 0.50f32,
                ..Default::default()
            },
            LayoutKind::SStack => LayoutData {
                main_count: 1u32,
                main_factor: 0.50f32,
                ..Default::default()
            },
            LayoutKind::BStack => LayoutData {
                main_count: 1u32,
                main_factor: 0.50f32,
                ..Default::default()
            },
            LayoutKind::SBStack => LayoutData {
                main_count: 1u32,
                main_factor: 0.50f32,
                ..Default::default()
            },
            LayoutKind::Horz => Default::default(),
            LayoutKind::SHorz => Default::default(),
            LayoutKind::Vert => Default::default(),
            LayoutKind::SVert => Default::default(),

            #[allow(unreachable_patterns)]
            _ => unimplemented!("{:?} does not have associated default data", self),
        }
    }

    #[inline]
    fn stack_split(
        n: usize,
        n_main: u32,
    ) -> (i32, i32) {
        let n_main = n_main as i32;
        let n = n as i32;

        if n <= n_main {
            (n, 0i32)
        } else {
            (n_main, n - n_main)
        }
    }

    fn func(&self) -> LayoutFn {
        match *self {
            LayoutKind::Float => |_, _, active_map| {
                let config = &LayoutKind::Float.config();
                vec![(Disposition::Unchanged(config.decoration), true); active_map.len()]
            },
            LayoutKind::BLFloat => |_, _, active_map| {
                let config = &LayoutKind::BLFloat.config();
                vec![(Disposition::Unchanged(config.decoration), true); active_map.len()]
            },
            LayoutKind::SingleFloat => |_, _, active_map| {
                let config = &LayoutKind::SingleFloat.config();
                active_map
                    .into_iter()
                    .map(|b| (Disposition::Unchanged(config.decoration), b))
                    .collect()
            },
            LayoutKind::BLSingleFloat => |_, _, active_map| {
                let config = &LayoutKind::BLSingleFloat.config();
                active_map
                    .into_iter()
                    .map(|b| (Disposition::Unchanged(config.decoration), b))
                    .collect()
            },
            LayoutKind::Center => |region, data, active_map| {
                let config = &LayoutKind::Center.config();
                let (pos, dim) = region.values();

                let h_comp = Layout::MAX_MAIN_COUNT + 1;
                let w_ratio: f32 = data.main_factor / 0.95;
                let h_ratio: f32 = (h_comp - data.main_count) as f32 / h_comp as f32;

                active_map
                    .into_iter()
                    .map(|_| {
                        (
                            Disposition::Changed(
                                Region {
                                    pos,
                                    dim,
                                }
                                .from_absolute_inner_center(Dim {
                                    w: (dim.w as f32 * w_ratio) as i32,
                                    h: (dim.h as f32 * h_ratio) as i32,
                                }),
                                config.decoration,
                            ),
                            true,
                        )
                    })
                    .collect()
            },
            LayoutKind::Monocle => |region, _, active_map| {
                let config = &LayoutKind::Monocle.config();
                let (pos, dim) = region.values();

                active_map
                    .into_iter()
                    .map(|_| {
                        (
                            Disposition::Changed(
                                Region {
                                    pos,
                                    dim,
                                },
                                config.decoration,
                            ),
                            true,
                        )
                    })
                    .collect()
            },
            LayoutKind::Paper => |region, data, active_map| {
                const MIN_W_RATIO: f32 = 0.5;

                let config = &LayoutKind::Paper.config();
                let (pos, dim) = region.values();
                let n = active_map.len();

                if n == 1 {
                    return vec![(
                        Disposition::Changed(*region, Decoration::NO_DECORATION),
                        true,
                    )];
                }

                let cw = (dim.w as f32
                    * if data.main_factor > MIN_W_RATIO {
                        data.main_factor
                    } else {
                        MIN_W_RATIO
                    }) as i32;

                let w = ((dim.w - cw) as usize / (n - 1)) as i32;
                let mut after_active = false;

                active_map
                    .into_iter()
                    .enumerate()
                    .map(|(i, active)| {
                        let i = i as i32;

                        (
                            Disposition::Changed(
                                if active {
                                    after_active = true;
                                    Region::new(pos.x + i * w, pos.y, cw, dim.h)
                                } else {
                                    let mut x = pos.x + i * w;

                                    if after_active {
                                        x += cw - w;
                                    }

                                    Region::new(x, pos.y, w, dim.h)
                                },
                                config.decoration,
                            ),
                            true,
                        )
                    })
                    .collect()
            },
            LayoutKind::SPaper => |region, data, active_map| {
                let mut region = region.clone();
                Layout::adjust_for_gap_size(&mut region, data.gap_size, &Zone::MIN_ZONE_DIM);

                (Self::Paper.func())(&region, data, active_map)
            },
            LayoutKind::Stack => |region, data, active_map| {
                let (pos, dim) = region.values();
                let n = active_map.len();

                if n == 1 {
                    return vec![(
                        Disposition::Changed(*region, Decoration::NO_DECORATION),
                        true,
                    )];
                }

                let (n_main, n_stack) = Self::stack_split(n, data.main_count);
                let h_stack = if n_stack > 0 { dim.h / n_stack } else { 0 };
                let h_main = if n_main > 0 { dim.h / n_main } else { 0 };

                let div = if data.main_count > 0 {
                    (dim.w as f32 * data.main_factor) as i32
                } else {
                    0
                };

                let config = &LayoutKind::Stack.config();
                let main_count = data.main_count as i32;

                active_map
                    .into_iter()
                    .enumerate()
                    .map(|(i, _)| {
                        let i = i as i32;

                        (
                            Disposition::Changed(
                                if i < main_count {
                                    Region::new(
                                        pos.x,
                                        pos.y + (i * h_main),
                                        if n_stack == 0 { dim.w } else { div },
                                        h_main,
                                    )
                                } else {
                                    Region::new(
                                        pos.x + div,
                                        pos.y + (i - main_count) * h_stack,
                                        dim.w - div,
                                        h_stack,
                                    )
                                },
                                config.decoration,
                            ),
                            true,
                        )
                    })
                    .collect()
            },
            LayoutKind::SStack => |region, data, active_map| {
                let mut region = region.clone();
                Layout::adjust_for_gap_size(&mut region, data.gap_size, &Zone::MIN_ZONE_DIM);

                (Self::Stack.func())(&region, data, active_map)
            },
            LayoutKind::BStack => |region, data, active_map| {
                let (pos, dim) = region.values();
                let n = active_map.len();

                if n == 1 {
                    return vec![(
                        Disposition::Changed(*region, Decoration::NO_DECORATION),
                        true,
                    )];
                }

                let (n_main, n_stack) = Self::stack_split(n, data.main_count);

                let div = if data.main_count > 0 {
                    (dim.w as f32 * data.main_factor) as i32
                } else {
                    0
                };

                let h_main = if n_main > 0 {
                    (if n_stack > 0 { div } else { dim.h }) / n_main
                } else {
                    0
                };

                let w_stack = if n_stack > 0 { dim.w / n_stack } else { 0 };

                let config = &LayoutKind::Stack.config();
                let main_count = data.main_count as i32;

                active_map
                    .into_iter()
                    .enumerate()
                    .map(|(i, _)| {
                        let i = i as i32;

                        (
                            Disposition::Changed(
                                if i < main_count {
                                    Region::new(pos.x, pos.y + (i * h_main), dim.w, h_main)
                                } else {
                                    Region::new(
                                        pos.x + ((i - main_count) * w_stack),
                                        pos.y + div,
                                        w_stack,
                                        dim.h - div,
                                    )
                                },
                                config.decoration,
                            ),
                            true,
                        )
                    })
                    .collect()
            },
            LayoutKind::SBStack => |region, data, active_map| {
                let mut region = region.clone();
                Layout::adjust_for_gap_size(&mut region, data.gap_size, &Zone::MIN_ZONE_DIM);

                (Self::BStack.func())(&region, data, active_map)
            },
            LayoutKind::Horz => |_region, _data, _active_map| todo!(),
            LayoutKind::SHorz => |_region, _data, _active_map| todo!(),
            LayoutKind::Vert => |_region, _data, _active_map| todo!(),
            LayoutKind::SVert => |_region, _data, _active_map| todo!(),

            #[allow(unreachable_patterns)]
            _ => unimplemented!("{:?} does not have an associated function", self),
        }
    }
}

pub struct Layout {
    kind: LayoutKind,
    prev_kind: LayoutKind,
    data: HashMap<LayoutKind, LayoutData>,
}

impl Layout {
    #[inline]
    pub fn new() -> Self {
        let kind = LayoutKind::Stack;
        let mut data = HashMap::with_capacity(LayoutKind::COUNT);

        for kind in LayoutKind::iter() {
            data.insert(kind, kind.default_data());
        }

        Self {
            kind,
            prev_kind: kind,
            data,
        }
    }

    #[inline]
    pub fn with_kind(kind: LayoutKind) -> Self {
        let mut data = HashMap::with_capacity(LayoutKind::COUNT);

        for kind in LayoutKind::iter() {
            data.insert(kind, kind.default_data());
        }

        Self {
            kind,
            prev_kind: kind,
            data,
        }
    }

    #[inline]
    pub fn kind(&self) -> LayoutKind {
        self.kind
    }

    #[inline]
    pub fn prev_kind(&self) -> LayoutKind {
        self.prev_kind
    }

    #[inline]
    pub fn config(&self) -> LayoutConfig {
        self.kind.config()
    }

    #[inline]
    pub fn prev_data(&self) -> &LayoutData {
        self.data.get(&self.prev_kind).unwrap()
    }

    #[inline]
    pub fn data(&self) -> &LayoutData {
        self.data.get(&self.kind).unwrap()
    }

    #[inline]
    pub fn data_mut(&mut self) -> &mut LayoutData {
        self.data.get_mut(&self.kind).unwrap()
    }

    #[inline]
    pub fn default_data(&self) -> LayoutData {
        self.kind.default_data()
    }

    #[inline]
    pub fn set_kind(
        &mut self,
        kind: LayoutKind,
    ) -> Result<LayoutKind, StateChangeError> {
        if kind == self.kind {
            return Err(StateChangeError::EarlyStop);
        }

        self.prev_kind = self.kind;
        self.kind = kind;

        Ok(self.prev_kind)
    }

    #[inline]
    pub fn adjust_for_margin(
        region: Region,
        extents: &Extents,
    ) -> Region {
        Region {
            pos: Pos {
                x: region.pos.x + extents.left as i32,
                y: region.pos.y + extents.top as i32,
            },
            dim: Dim {
                w: region.dim.w - extents.left - extents.right,
                h: region.dim.h - extents.top - extents.bottom,
            },
        }
    }

    #[inline]
    pub fn adjust_for_gap_size(
        region: &mut Region,
        gap_size: u32,
        min_dim: &Dim,
    ) {
        let gap_size = gap_size as i32;
        let dim_gap = 2 * gap_size;

        let new_w = region.dim.w - dim_gap;
        if new_w < min_dim.w {
            region.pos.x += ((region.dim.w - min_dim.w) as f32 / 2f32) as i32;
            region.dim.w = min_dim.w;
        } else {
            region.dim.w = new_w;
            region.pos.x += gap_size;
        }

        let new_h = region.dim.h - dim_gap;
        if new_h < min_dim.h {
            region.pos.y += ((region.dim.h - min_dim.h) as f32 / 2f32) as i32;
            region.dim.h = min_dim.h;
        } else {
            region.dim.h = new_h;
            region.pos.y += gap_size;
        }
    }

    #[inline]
    pub fn adjust_for_border(
        region: &mut Region,
        border_width: u32,
        min_dim: &Dim,
    ) {
        let border_padding = 2 * border_width as i32;

        let new_w = region.dim.w - border_padding;
        region.dim.w = std::cmp::max(min_dim.w, new_w);

        let new_h = region.dim.h - border_padding;
        region.dim.h = std::cmp::max(min_dim.h, new_h);
    }
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            kind: LayoutKind::Stack,
            prev_kind: LayoutKind::Stack,
            data: HashMap::new(),
        }
    }
}

pub trait Apply {
    fn apply(
        &self,
        region: Region,
        active_map: Vec<bool>,
    ) -> (PlacementMethod, Vec<(Disposition, bool)>);
}

impl Apply for Layout {
    #[inline]
    fn apply(
        &self,
        region: Region,
        active_map: Vec<bool>,
    ) -> (PlacementMethod, Vec<(Disposition, bool)>) {
        let config = self.kind.config();
        let data = self.data();

        let region = if config.margin {
            Self::adjust_for_margin(region, &data.margin)
        } else {
            region
        };

        (
            config.method,
            (self.kind.func())(&region, &data, active_map)
                .into_iter()
                .map(|(mut disposition, is_visible)| {
                    match disposition {
                        Disposition::Unchanged(_) => {},
                        Disposition::Changed(ref mut region, decoration) => {
                            if let Some(border) = decoration.border {
                                Self::adjust_for_gap_size(
                                    region,
                                    border.width,
                                    &Zone::MIN_ZONE_DIM,
                                );
                            }

                            if config.gap {
                                Self::adjust_for_gap_size(
                                    region,
                                    data.gap_size,
                                    &Zone::MIN_ZONE_DIM,
                                );
                            }
                        },
                    }

                    (disposition, is_visible)
                })
                .collect(),
        )
    }
}

impl std::cmp::PartialEq<Self> for Layout {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.kind == other.kind && self.data.get(&self.kind) == other.data.get(&other.kind)
    }
}

impl Identify for Layout {
    fn id(&self) -> Ident {
        self.kind as Ident
    }
}

impl std::fmt::Debug for Layout {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("Layout")
            .field("kind", &self.kind)
            .field("prev_kind", &self.prev_kind)
            .field("data", &self.data.get(&self.kind))
            .finish()
    }
}
