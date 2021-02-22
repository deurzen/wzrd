use crate::client::Client;
use crate::common::Ident;
use crate::common::Identify;
use crate::cycle::Cycle;

use winsys::common::Extents;
use winsys::common::Padding;
use winsys::common::Region;
use winsys::common::Window;

use std::collections::HashMap;
use std::string::ToString;
use std::sync::atomic;
use std::vec::Vec;

use strum::EnumCount;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use strum_macros::ToString;

static INSTANCE_COUNT: atomic::AtomicUsize = atomic::AtomicUsize::new(1);
fn next_id() -> usize {
    INSTANCE_COUNT.fetch_add(1, atomic::Ordering::Relaxed)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Frame {
    pub border: Option<u32>,
    pub extents: Option<Extents>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Disposition {
    Unchanged,
    Changed(Region, Frame),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Placement {
    pub zone: usize,
    pub region: Option<Region>,
    pub frame: Frame,
}

pub type LayoutFn =
    fn(&Region, &LayoutData, Vec<bool>) -> Vec<(Disposition, bool)>;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LayoutMethod {
    /// Does not inhibit free placement of clients
    Free,

    /// Arranges clients along a predefined layout
    Tile,

    /// Semi-adjustable tree-based layout
    Tree,
}

#[repr(u8)]
#[derive(
    Debug, Hash, PartialEq, Eq, Clone, Copy, EnumIter, EnumCount, ToString
)]
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
            LayoutKind::Float => LayoutConfig::default(),
            LayoutKind::SingleFloat => LayoutConfig::default(),
            LayoutKind::Center => LayoutConfig::default(),
            LayoutKind::Monocle => LayoutConfig::default(),
            LayoutKind::Paper => LayoutConfig::default(),
            LayoutKind::SStack => LayoutConfig::default(),
            LayoutKind::Stack => LayoutConfig::default(),
            _ => unimplemented!(
                "layout kind {:?} does not have an associated configuration",
                self
            ),
        }
    }
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct LayoutConfig {
    pub method: LayoutMethod,
    pub gap: bool,
    pub persistent: bool,
    pub single: bool,
    pub wraps: bool,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            method: LayoutMethod::Free,
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
    pub frame_extents: (Option<Extents>, Option<u32>),
    pub margin: Option<Padding>,
    pub gap_size: u32,

    /// Tiled layout data
    pub main_count: u32,
    pub main_factor: f32,
}

#[derive(Clone)]
pub struct Layout {
    pub kind: LayoutKind,
    pub prev_kind: LayoutKind,

    data: HashMap<LayoutKind, LayoutData>,
    default_data: HashMap<LayoutKind, LayoutData>,
    func: LayoutFn,
}

pub trait Apply {
    fn apply(
        &self,
        region: &Region,
        active_map: Vec<bool>,
    ) -> Vec<(Disposition, bool)>;
}

impl Apply for Layout {
    fn apply(
        &self,
        region: &Region,
        active_map: Vec<bool>,
    ) -> Vec<(Disposition, bool)> {
        (self.func)(region, &self.data.get(&self.kind).unwrap(), active_map)
    }
}

pub struct LayoutHandler {
    symbol_map: HashMap<LayoutKind, char>,
    name_map: HashMap<LayoutKind, String>,
    config_map: HashMap<LayoutKind, LayoutConfig>,
}

impl LayoutHandler {
    pub fn new() -> Self {
        let mut symbol_map = HashMap::with_capacity(LayoutKind::COUNT);
        let mut name_map = HashMap::with_capacity(LayoutKind::COUNT);
        let mut config_map = HashMap::with_capacity(LayoutKind::COUNT);

        for kind in LayoutKind::iter() {
            symbol_map.insert(kind, LayoutKind::symbol(&kind));
            name_map.insert(kind, LayoutKind::name(&kind));
            config_map.insert(kind, LayoutKind::config(&kind));
        }

        Self {
            symbol_map,
            name_map,
            config_map,
        }
    }

    pub fn layout_func(kind: LayoutKind) -> LayoutFn {
        match kind {
            LayoutKind::Float => |_, _, active_map| {
                vec![(Disposition::Unchanged, true); active_map.len()]
            },
            LayoutKind::SingleFloat => |_, _, active_map| {
                active_map
                    .iter()
                    .map(|&b| (Disposition::Unchanged, b))
                    .collect()
            },
            _ => |_, _, _| Vec::with_capacity(0),
            // Center => {},
            // Monocle => {},
            // Paper => {},
            // SStack => {},
            // Stack => {},
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ZoneContent {
    Empty,
    Client(Window),
    Tab(Cycle<Box<Zone>>),
    Layout(Layout, Cycle<Box<Zone>>),
}

impl ZoneContent {
    pub fn set_visible(
        &mut self,
        is_visible: bool,
    ) {
        match self {
            ZoneContent::Tab(zones) => {
                zones.iter_mut().for_each(|mut zone| {
                    zone.set_visible(is_visible);
                });
            },
            ZoneContent::Layout(_, zones) => {
                zones.iter_mut().for_each(|mut zone| {
                    zone.set_visible(is_visible);
                });
            },
            _ => {},
        }
    }

    pub fn n_regions(&self) -> usize {
        match self {
            ZoneContent::Empty => 0,
            ZoneContent::Client(_) => 1,
            ZoneContent::Tab(zones) => 1,
            ZoneContent::Layout(_, zones) => zones.len(),
        }
    }

    pub fn n_zones(&self) -> usize {
        match self {
            ZoneContent::Empty => 0,
            ZoneContent::Client(_) => 1,
            ZoneContent::Tab(zones) | ZoneContent::Layout(_, zones) => {
                zones.len()
            },
        }
    }

    pub fn n_subzones(&self) -> usize {
        match self {
            ZoneContent::Empty => 0,
            ZoneContent::Client(_) => 1,
            ZoneContent::Tab(zones) | ZoneContent::Layout(_, zones) => zones
                .iter()
                .fold(0, |len, zone| len + zone.content.n_subzones()),
        }
    }
}

#[derive(Debug)]
pub struct Zone {
    id: usize,
    parent: usize,
    content: ZoneContent,
    region: Region,
    frame: Frame,
    is_active: bool,
    is_visible: bool,
}

impl Zone {
    pub fn new(
        parent: usize,
        content: ZoneContent,
        region: Region,
        frame: Frame,
        is_active: bool,
        is_visible: bool,
    ) -> Self {
        Self {
            id: next_id(),
            parent,
            content,
            region,
            frame,
            is_active,
            is_visible,
        }
    }
}

impl Zone {
    pub fn set_visible(
        &mut self,
        is_visible: bool,
    ) {
        self.is_visible = is_visible;
        self.content.set_visible(is_visible);
    }
}

pub trait Arrange {
    fn arrange(
        &mut self,
        client_map: &HashMap<Window, Client>,
        focus: Option<Window>,
        region: &Region,
    ) -> Vec<Placement>;
}

impl Arrange for Zone {
    fn arrange(
        &mut self,
        client_map: &HashMap<Window, Client>,
        focus: Option<Window>,
        region: &Region,
    ) -> Vec<Placement> {
        match &mut self.content {
            ZoneContent::Empty => {
                self.set_visible(true);

                Vec::new()
            },
            ZoneContent::Client(window) => {
                self.set_visible(true);

                vec![Placement {
                    zone: self.id,
                    region: Some(*region),
                    frame: self.frame,
                }]
            },
            ZoneContent::Tab(ref mut zones) => {
                let mut tab = vec![Placement {
                    zone: self.id,
                    region: Some(*region),
                    frame: self.frame,
                }];

                zones.on_all_mut(|mut zone| {
                    zone.set_visible(false);
                });

                match zones.active_element_mut() {
                    None => tab,
                    Some(mut zone) => {
                        zone.set_visible(true);

                        tab.append(
                            &mut zone.arrange(client_map, focus, region),
                        );

                        tab
                    },
                }
            },
            ZoneContent::Layout(layout, ref mut zones) => {
                let id = self.id;
                let application = layout
                    .apply(region, zones.iter().map(|z| z.is_active).collect());
                let mut placements = Vec::with_capacity(zones.len());

                zones.iter_mut().zip(application.iter()).map(
                    |(ref mut zone, (disposition, is_visible))| {
                        let (region, frame) = match disposition {
                            Disposition::Unchanged => (zone.region, zone.frame),
                            Disposition::Changed(region, frame) => {
                                zone.region = *region;
                                zone.frame = *frame;

                                (*region, *frame)
                            },
                        };

                        zone.set_visible(*is_visible);

                        if *is_visible {
                            placements.push(Placement {
                                zone: id,
                                region: Some(region),
                                frame,
                            });

                            placements.append(
                                &mut zone.arrange(client_map, focus, &region),
                            );
                        }
                    },
                );

                placements
            },
        }
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

impl Identify for Box<Zone> {
    fn id(&self) -> Ident {
        self.id as Ident
    }
}

impl std::cmp::PartialEq<Self> for Layout {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.kind == other.kind
            && self.data.get(&self.kind) == other.data.get(&other.kind)
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
