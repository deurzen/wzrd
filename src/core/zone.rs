use crate::common::Decoration;
use crate::common::Frame;
use crate::common::Ident;
use crate::common::Identify;
use crate::common::FREE_DECORATION;
use crate::common::NO_DECORATION;
use crate::cycle::Cycle;
use crate::cycle::InsertPos;
use crate::cycle::Selector;

use winsys::common::Dim;
use winsys::common::Extents;
use winsys::common::Padding;
use winsys::common::Pos;
use winsys::common::Region;
use winsys::common::Window;

use strum::EnumCount;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use strum_macros::ToString;

use std::collections::HashMap;
use std::string::ToString;
use std::sync::atomic;
use std::vec::Vec;

pub type ZoneId = u32;

pub const MAX_MAIN_COUNT: u32 = 15;
pub const MAX_GAP_SIZE: u32 = 300;
pub const MAX_MARGIN: Padding = Padding {
    left: 700,
    right: 700,
    top: 400,
    bottom: 400,
};

const MIN_ZONE_DIM: Dim = Dim {
    w: 25,
    h: 25,
};

static INSTANCE_COUNT: atomic::AtomicU32 = atomic::AtomicU32::new(1);
fn next_id() -> ZoneId {
    INSTANCE_COUNT.fetch_add(1, atomic::Ordering::Relaxed) as ZoneId
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Disposition {
    Unchanged,
    Changed(Region, Decoration),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PlacementMethod {
    /// Does not inhibit free placement of clients
    Free,

    /// Arranges clients along a predefined layout
    Tile,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PlacementKind {
    Client(Window),
    Tab(usize),
    Layout,
}

impl PlacementKind {
    pub fn from_zone_content(content: &ZoneContent) -> Self {
        match content {
            ZoneContent::Client(window) => PlacementKind::Client(*window),
            ZoneContent::Tab(zones) => PlacementKind::Tab(zones.len()),
            ZoneContent::Layout(..) => PlacementKind::Layout,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Placement {
    pub method: PlacementMethod,
    pub kind: PlacementKind,
    pub zone: ZoneId,
    pub region: Option<Region>,
    pub decoration: Decoration,
}

impl Placement {
    pub fn inner_region(&self) -> Option<Region> {
        if let Some(region) = self.region {
            if let Some(frame) = self.decoration.frame {
                let extents = frame.extents;

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

type LayoutFn = fn(&Region, &LayoutData, Vec<bool>) -> Vec<(Disposition, bool)>;

#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, EnumIter, EnumCount, ToString)]
pub enum LayoutKind {
    /// Free layouts
    Float = b'F',
    SingleFloat = b'Z',

    /// Tiled layouts
    Center = b';',
    Monocle = b'M',
    Paper = b'/',
    SStack = b'+',
    Stack = b'S',
    Horz = b'H',
    Vert = b'V',
}

impl LayoutKind {
    pub fn symbol(&self) -> char {
        (*self as u8) as char
    }

    pub fn name(&self) -> String {
        self.to_string()
    }

    fn config(&self) -> LayoutConfig {
        match *self {
            LayoutKind::Float => LayoutConfig {
                method: PlacementMethod::Free,
                decoration: FREE_DECORATION,
                root_only: true,
                gap: false,
                persistent: false,
                single: false,
                wraps: true,
            },
            LayoutKind::SingleFloat => LayoutConfig {
                method: PlacementMethod::Free,
                decoration: FREE_DECORATION,
                root_only: true,
                gap: false,
                persistent: true,
                single: true,
                wraps: true,
            },
            LayoutKind::Center => LayoutConfig {
                method: PlacementMethod::Tile,
                decoration: NO_DECORATION,
                root_only: false,
                gap: true,
                persistent: false,
                single: false,
                wraps: true,
            },
            LayoutKind::Monocle => LayoutConfig {
                method: PlacementMethod::Tile,
                decoration: NO_DECORATION,
                root_only: false,
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
                gap: true,
                persistent: true,
                single: false,
                wraps: false,
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
                gap: false,
                persistent: false,
                single: false,
                wraps: true,
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
                gap: true,
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
                gap: true,
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
                gap: true,
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
            LayoutKind::SingleFloat => Default::default(),
            LayoutKind::Center => LayoutData {
                main_count: 5u32,
                main_factor: 0.40f32,
                ..Default::default()
            },
            LayoutKind::Monocle => Default::default(),
            LayoutKind::Paper => Default::default(),
            LayoutKind::SStack => LayoutData {
                main_count: 1u32,
                main_factor: 0.50f32,
                ..Default::default()
            },
            LayoutKind::Stack => LayoutData {
                main_count: 1u32,
                main_factor: 0.50f32,
                ..Default::default()
            },
            LayoutKind::Horz => Default::default(),
            LayoutKind::Vert => Default::default(),

            #[allow(unreachable_patterns)]
            _ => unimplemented!("{:?} does not have associated default data", self),
        }
    }

    #[inline]
    fn stack_split(
        n: usize,
        n_main: u32,
    ) -> (u32, u32) {
        let n = n as u32;

        if n <= n_main {
            (n, 0)
        } else {
            (n_main, n - n_main)
        }
    }

    fn func(&self) -> LayoutFn {
        match *self {
            LayoutKind::Float => {
                |_, _, active_map| vec![(Disposition::Unchanged, true); active_map.len()]
            },
            LayoutKind::SingleFloat => |_, _, active_map| {
                active_map
                    .into_iter()
                    .map(|b| (Disposition::Unchanged, b))
                    .collect()
            },
            LayoutKind::Stack => |region, data, active_map| {
                let (pos, dim) = region.values();
                let n = active_map.len();

                if n == 1 {
                    return vec![(Disposition::Changed(*region, NO_DECORATION), true)];
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
                active_map
                    .into_iter()
                    .enumerate()
                    .map(|(i, _)| {
                        let i = i as u32;

                        if i < data.main_count {
                            let w = if n_stack == 0 { dim.w } else { div as u32 };

                            (
                                Disposition::Changed(
                                    Region::new(pos.x, pos.y + (i * h_main) as i32, w, h_main),
                                    config.decoration,
                                ),
                                true,
                            )
                        } else {
                            let sn = (i - data.main_count) as i32;

                            (
                                Disposition::Changed(
                                    Region::new(
                                        pos.x + div,
                                        pos.y + sn * h_stack as i32,
                                        dim.w - div as u32,
                                        h_stack,
                                    ),
                                    config.decoration,
                                ),
                                true,
                            )
                        }
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
            LayoutKind::Center => |region, data, active_map| {
                let config = &LayoutKind::Center.config();
                let (pos, dim) = region.values();

                let h_comp = MAX_MAIN_COUNT + 1;
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
                                .from_absolute_inner_center(&Dim {
                                    w: (dim.w as f32 * w_ratio) as u32,
                                    h: (dim.h as f32 * h_ratio) as u32,
                                }),
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
                    return vec![(Disposition::Changed(*region, NO_DECORATION), true)];
                }

                let cw = (dim.w as f32
                    * if data.main_factor > MIN_W_RATIO {
                        data.main_factor
                    } else {
                        MIN_W_RATIO
                    }) as u32;

                let w = ((dim.w - cw) as usize / (n - 1)) as i32;
                let mut after_active = false;

                active_map
                    .into_iter()
                    .enumerate()
                    .map(|(i, active)| {
                        if active {
                            after_active = true;

                            (
                                Disposition::Changed(
                                    Region::new(pos.x + i as i32 * w, pos.y, cw, dim.h),
                                    config.decoration,
                                ),
                                true,
                            )
                        } else {
                            let mut x = pos.x + i as i32 * w;

                            if after_active {
                                x += cw as i32 - w;
                            }

                            (
                                Disposition::Changed(
                                    Region::new(x, pos.y, w as u32, dim.h),
                                    config.decoration,
                                ),
                                true,
                            )
                        }
                    })
                    .collect()
            },

            #[allow(unreachable_patterns)]
            _ => unimplemented!("{:?} does not have an associated function", self),
        }
    }
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct LayoutConfig {
    pub method: PlacementMethod,
    pub decoration: Decoration,
    pub root_only: bool,
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
    fn get_config(&self) -> LayoutConfig {
        self.kind.config()
    }

    #[inline]
    fn get_data(&self) -> &LayoutData {
        self.data.get(&self.kind).unwrap()
    }

    #[inline]
    fn get_data_mut(&mut self) -> &mut LayoutData {
        self.data.get_mut(&self.kind).unwrap()
    }

    #[inline]
    fn get_default_data(&self) -> LayoutData {
        self.kind.default_data()
    }

    #[inline]
    fn set_kind(
        &mut self,
        kind: LayoutKind,
    ) {
        self.prev_kind = self.kind;
        self.kind = kind;
    }

    #[inline]
    fn adjust_for_margin(
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
    fn adjust_for_gap_size(
        region: &mut Region,
        gap_size: u32,
        min_dim: &Dim,
    ) {
        let dim_gap = 2 * gap_size as i32;

        let new_w = region.dim.w as i32 - dim_gap;
        if new_w < min_dim.w as i32 {
            region.pos.x += ((region.dim.w as i32 - min_dim.w as i32) as f32 / 2f32) as i32;
            region.dim.w = min_dim.w;
        } else {
            region.dim.w = new_w as u32;
            region.pos.x += gap_size as i32;
        }

        let new_h = region.dim.h as i32 - dim_gap;
        if new_h < min_dim.h as i32 {
            region.pos.y += ((region.dim.h as i32 - min_dim.h as i32) as f32 / 2f32) as i32;
            region.dim.h = min_dim.h;
        } else {
            region.dim.h = new_h as u32;
            region.pos.y += gap_size as i32;
        }
    }
}

trait Apply {
    fn apply(
        &self,
        region: Region,
        active_map: Vec<bool>,
    ) -> (PlacementMethod, Vec<(Disposition, bool)>);
}

impl Apply for Layout {
    fn apply(
        &self,
        region: Region,
        active_map: Vec<bool>,
    ) -> (PlacementMethod, Vec<(Disposition, bool)>) {
        let config = self.kind.config();
        let data = self.get_data();

        let region = if config.gap {
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
                        Disposition::Unchanged => {},
                        Disposition::Changed(ref mut region, _) => {
                            Self::adjust_for_gap_size(region, data.gap_size, &MIN_ZONE_DIM);
                        },
                    }

                    (disposition, is_visible)
                })
                .collect(),
        )
    }
}

#[derive(Debug, PartialEq)]
pub enum ZoneContent {
    Client(Window),
    Tab(Cycle<ZoneId>),
    Layout(Layout, Cycle<ZoneId>),
}

#[derive(Debug)]
pub struct Zone {
    id: ZoneId,
    parent: Option<ZoneId>,
    method: PlacementMethod,
    content: ZoneContent,
    region: Region,
    decoration: Decoration,
    is_visible: bool,
}

impl Zone {
    fn new(
        parent: Option<ZoneId>,
        content: ZoneContent,
        region: Region,
    ) -> (ZoneId, Self) {
        let id = next_id();

        (id, Self {
            id,
            parent,
            method: PlacementMethod::Free,
            content,
            region,
            decoration: NO_DECORATION,
            is_visible: true,
        })
    }

    pub fn set_content(
        &mut self,
        content: ZoneContent,
    ) {
        self.content = content;
    }

    pub fn set_kind(
        &mut self,
        kind: LayoutKind,
    ) {
        match self.content {
            ZoneContent::Layout(ref mut layout, _) => {
                layout.prev_kind = layout.kind;
                layout.kind = kind;
            },
            _ => {},
        }
    }

    pub fn get_prev_kind(&self) -> LayoutKind {
        match &self.content {
            ZoneContent::Layout(layout, _) => layout.prev_kind,
            _ => panic!("attempting to obtain layout kind from non-layout"),
        }
    }

    pub fn get_kind(&self) -> LayoutKind {
        match &self.content {
            ZoneContent::Layout(layout, _) => layout.kind,
            _ => panic!("attempting to obtain layout kind from non-layout"),
        }
    }

    pub fn set_region(
        &mut self,
        region: Region,
    ) {
        self.region = region;
    }

    pub fn default_data(&self) -> Option<LayoutData> {
        match &self.content {
            ZoneContent::Layout(layout, _) => Some(layout.get_default_data()),
            _ => None,
        }
    }

    pub fn data(&self) -> Option<&LayoutData> {
        match self.content {
            ZoneContent::Layout(ref layout, _) => Some(layout.get_data()),
            _ => None,
        }
    }

    pub fn data_mut(&mut self) -> Option<&mut LayoutData> {
        match self.content {
            ZoneContent::Layout(ref mut layout, _) => Some(layout.get_data_mut()),
            _ => None,
        }
    }

    pub fn config(&self) -> Option<LayoutConfig> {
        match self.content {
            ZoneContent::Layout(ref layout, _) => Some(layout.kind.config()),
            _ => None,
        }
    }

    pub fn method(&self) -> PlacementMethod {
        self.method
    }
}

enum ZoneChange {
    Visible(bool),
    Region(Region),
    Decoration(Decoration),
    Method(PlacementMethod),
}

pub struct ZoneManager {
    zone_map: HashMap<ZoneId, Zone>,
}

impl ZoneManager {
    pub fn new() -> Self {
        Self {
            zone_map: HashMap::new(),
        }
    }

    pub fn new_zone(
        &mut self,
        parent: Option<ZoneId>,
        content: ZoneContent,
    ) -> ZoneId {
        let (id, zone) = Zone::new(parent, content, Region::new(0, 0, 0, 0));
        let parent = parent.and_then(|p| self.zone_map.get_mut(&p));

        if let Some(parent) = parent {
            match &mut parent.content {
                ZoneContent::Tab(zones) | ZoneContent::Layout(_, zones) => {
                    zones.insert_at(&InsertPos::AfterActive, id)
                },
                _ => panic!("attempted to insert into non-cycle"),
            }
        }

        self.zone_map.insert(id, zone);
        id
    }

    pub fn activate_zone(
        &mut self,
        id: ZoneId,
    ) {
        if let Some(cycle_id) = self.next_cycle(id) {
            let cycle = self.zone_mut(cycle_id);

            match cycle.content {
                ZoneContent::Tab(ref mut zones) | ZoneContent::Layout(_, ref mut zones) => {
                    zones.activate_for(&Selector::AtIdent(id));
                    self.activate_zone(cycle_id);
                },
                _ => {},
            }
        }
    }

    pub fn active_default_data(
        &mut self,
        id: ZoneId,
    ) -> Option<LayoutData> {
        let cycle = self.nearest_cycle(id);
        let cycle = self.zone(cycle);

        cycle.default_data()
    }

    pub fn active_data_mut(
        &mut self,
        id: ZoneId,
    ) -> Option<&mut LayoutData> {
        let cycle = self.nearest_cycle(id);
        let cycle = self.zone_mut(cycle);

        cycle.data_mut()
    }

    pub fn active_layoutconfig(
        &self,
        id: ZoneId,
    ) -> Option<LayoutConfig> {
        let cycle = self.nearest_cycle(id);
        let cycle = self.zone(cycle);

        cycle.config()
    }

    pub fn zone_checked(
        &self,
        id: ZoneId,
    ) -> Option<&Zone> {
        self.zone_map.get(&id)
    }

    pub fn zone(
        &self,
        id: ZoneId,
    ) -> &Zone {
        self.zone_map.get(&id).unwrap()
    }

    pub fn zone_checked_mut(
        &mut self,
        id: ZoneId,
    ) -> Option<&mut Zone> {
        self.zone_map.get_mut(&id)
    }

    pub fn zone_mut(
        &mut self,
        id: ZoneId,
    ) -> &mut Zone {
        self.zone_map.get_mut(&id).unwrap()
    }

    pub fn parent_id(
        &self,
        id: ZoneId,
    ) -> Option<ZoneId> {
        self.zone_map.get(&id).and_then(|zone| zone.parent)
    }

    pub fn cycle_config(
        &self,
        id: ZoneId,
    ) -> Option<LayoutConfig> {
        let cycle = self.nearest_cycle(id);
        let zone = self.zone(cycle);

        zone.config()
    }

    pub fn nearest_cycle(
        &self,
        id: ZoneId,
    ) -> ZoneId {
        let mut next = id;

        loop {
            let zone = self.zone_map.get(&next).unwrap();

            match zone.content {
                ZoneContent::Tab(_) | ZoneContent::Layout(..) => {
                    return next;
                },
                _ => {},
            }

            if let Some(parent) = zone.parent {
                next = parent;
            } else {
                panic!("no nearest cycle found");
            }
        }
    }

    pub fn next_cycle(
        &self,
        mut id: ZoneId,
    ) -> Option<ZoneId> {
        while let Some(next_id) = self.parent_id(id) {
            let zone = self.zone_map.get(&next_id).unwrap();

            match zone.content {
                ZoneContent::Tab(_) | ZoneContent::Layout(..) => {
                    return Some(next_id);
                },
                _ => id = next_id,
            }
        }

        None
    }

    fn gather_subzones(
        &self,
        zone: ZoneId,
        recurse: bool,
    ) -> Vec<ZoneId> {
        if let Some(zone) = self.zone_map.get(&zone) {
            match &zone.content {
                ZoneContent::Client(_) => {},
                ZoneContent::Tab(zones) | ZoneContent::Layout(_, zones) => {
                    let mut zones = zones.as_vec();

                    if recurse {
                        let mut subzones = Vec::new();

                        zones.iter().for_each(|&zone| {
                            subzones.extend(self.gather_subzones(zone, recurse));
                        });

                        zones.extend(subzones);
                    }

                    return zones;
                },
            }
        }

        return Vec::with_capacity(0);
    }

    /// Arrange a zone and all of its subzones within the region
    /// of the supplied zone
    pub fn arrange(
        &mut self,
        zone: ZoneId,
        to_ignore: &Vec<ZoneId>,
    ) -> Vec<Placement> {
        let cycle = self.nearest_cycle(zone);
        let zone = self.zone_map.get(&cycle).unwrap();
        let region = zone.region;
        let decoration = zone.decoration;

        let method = match &zone.content {
            ZoneContent::Tab(_) => PlacementMethod::Tile,
            ZoneContent::Layout(layout, _) => layout.kind.config().method,
            _ => panic!("attempting to derive method from non-cycle"),
        };

        self.arrange_subzones(cycle, region, decoration, method, to_ignore)
    }

    fn arrange_subzones(
        &mut self,
        zone: ZoneId,
        region: Region,
        decoration: Decoration,
        method: PlacementMethod,
        to_ignore: &Vec<ZoneId>,
    ) -> Vec<Placement> {
        let id = zone;
        let zone = self.zone_map.get(&id).unwrap();
        let content = &zone.content;

        let mut zone_changes: Vec<(ZoneId, ZoneChange)> = Vec::new();

        let placements = match &content {
            ZoneContent::Client(window) => {
                return vec![Placement {
                    method,
                    kind: PlacementKind::Client(*window),
                    zone: id,
                    region: Some(region),
                    decoration,
                }];
            },
            ZoneContent::Tab(zones) => {
                let mut placements = vec![Placement {
                    method,
                    kind: PlacementKind::Tab(zones.len()),
                    zone: id,
                    region: Some(region),
                    decoration,
                }];

                let active_element = zones.active_element();
                active_element.into_iter().for_each(|&id| {
                    zone_changes.push((id, ZoneChange::Visible(true)));

                    self.gather_subzones(id, true).into_iter().for_each(|id| {
                        zone_changes.push((id, ZoneChange::Visible(true)));
                    });
                });

                zones
                    .iter()
                    .filter(|&id| Some(id) != active_element)
                    .for_each(|&id| {
                        zone_changes.push((id, ZoneChange::Visible(false)));

                        self.gather_subzones(id, true).into_iter().for_each(|id| {
                            zone_changes.push((id, ZoneChange::Visible(false)));
                        });
                    });

                match active_element {
                    None => placements,
                    Some(&id) => {
                        let subzones = self.gather_subzones(id, true);
                        let method = PlacementMethod::Tile;

                        subzones.into_iter().for_each(|id| {
                            zone_changes.push((id, ZoneChange::Visible(true)));
                            zone_changes.push((id, ZoneChange::Region(region)));
                            zone_changes.push((id, ZoneChange::Method(method)));
                        });

                        placements.extend(
                            self.arrange_subzones(id, region, decoration, method, to_ignore),
                        );
                        placements
                    },
                }
            },
            ZoneContent::Layout(layout, zones) => {
                let active_element = zones.active_element();
                let mut subplacements = Vec::new();
                let mut placements = vec![Placement {
                    method,
                    kind: PlacementKind::Layout,
                    zone: id,
                    region: Some(region),
                    decoration,
                }];

                let zones: Vec<ZoneId> = zones
                    .iter()
                    .filter(|&id| !to_ignore.contains(id))
                    .map(|&id| id)
                    .collect();

                let (method, application) = layout.apply(
                    region,
                    zones.iter().map(|id| Some(id) == active_element).collect(),
                );

                zones.into_iter().zip(application.into_iter()).for_each(
                    |(id, (disposition, is_visible))| {
                        let zone = self.zone_map.get(&id).unwrap();
                        let subzones = self.gather_subzones(id, true);

                        zone_changes.extend(
                            subzones
                                .into_iter()
                                .map(|id| (id, ZoneChange::Visible(true)))
                                .collect::<Vec<(ZoneId, ZoneChange)>>(),
                        );

                        let (region, decoration) = match disposition {
                            Disposition::Unchanged => (zone.region, zone.decoration),
                            Disposition::Changed(region, decoration) => {
                                zone_changes.push((id, ZoneChange::Region(region)));
                                zone_changes.push((id, ZoneChange::Decoration(decoration)));
                                zone_changes.push((id, ZoneChange::Method(method)));

                                (region, decoration)
                            },
                        };

                        if is_visible {
                            subplacements.push((id, region, decoration));
                        }
                    },
                );

                subplacements
                    .into_iter()
                    .for_each(|(id, region, decoration)| {
                        placements.extend(
                            self.arrange_subzones(id, region, decoration, method, to_ignore),
                        );
                    });

                placements
            },
        };

        zone_changes.into_iter().for_each(|(id, change)| {
            let zone = self.zone_map.get_mut(&id).unwrap();

            match change {
                ZoneChange::Visible(is_visible) => {
                    zone.is_visible = is_visible;
                },
                ZoneChange::Region(region) => {
                    zone.region = region;
                },
                ZoneChange::Decoration(decoration) => {
                    zone.decoration = decoration;
                },
                ZoneChange::Method(method) => {
                    zone.method = method;
                },
            };
        });

        placements
    }
}

impl std::cmp::PartialEq<Self> for Zone {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.id == other.id
    }
}

impl Identify for Zone {
    fn id(&self) -> Ident {
        self.id as Ident
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
