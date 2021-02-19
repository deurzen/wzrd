use crate::client::Client;
use crate::common::Ident;
use crate::common::Identify;
use crate::cycle::Cycle;

use winsys::common::Extents;
use winsys::common::Padding;
use winsys::common::Region;
use winsys::common::Window;

use std::collections::HashMap;
use std::sync::atomic;
use std::vec::Vec;

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
pub struct Placement {
    pub zone: usize,
    pub region: Option<Region>,
    pub frame: Frame,
}

pub type LayoutFn = fn(&Region, &LayoutData) -> Vec<(Region, Frame, bool)>;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LayoutMethod {
    /// Does not inhibit free placement of clients
    Free,

    /// Arranges clients along a predefined layout
    Tile,

    /// Semi-adjustable tree-based layout
    Tree,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LayoutKind {
    /// Free layouts
    Float,
    SingleFloat,

    /// Tiled layouts
    Center,
    Monocle,
    Paper,
    SStack,
    Stack,
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
    pub symbol: char,
    pub name: String,

    pub kind: LayoutKind,
    pub prev_kind: LayoutKind,
    pub config: LayoutConfig,

    data: LayoutData,
    default_data: LayoutData,
    func: LayoutFn,
}

pub trait Apply {
    fn apply(
        &self,
        region: &Region,
        n_zones: usize,
    ) -> Vec<(Region, Frame, bool)>;
}

impl Apply for Layout {
    fn apply(
        &self,
        region: &Region,
        n_zones: usize,
    ) -> Vec<(Region, Frame, bool)> {
        (self.func)(region, &self.data)
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
    visible: bool,
}

impl Zone {
    pub fn new(
        parent: usize,
        content: ZoneContent,
        region: Region,
        frame: Frame,
        visible: bool,
    ) -> Self {
        Self {
            id: next_id(),
            parent,
            content,
            region,
            frame,
            visible,
        }
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
            ZoneContent::Empty => Vec::new(),
            ZoneContent::Client(window) => {
                vec![Placement {
                    zone: self.id,
                    region: Some(*region),
                    frame: self.frame,
                }]
            },
            ZoneContent::Tab(ref mut zones) => {
                zones.active_element_mut().map_or(Vec::new(), |mut zone| {
                    zone.arrange(client_map, focus, region)
                })
            },
            ZoneContent::Layout(layout, ref mut zones) => {
                let id = self.id;
                let regions = layout.apply(region, zones.len());
                let mut placements = Vec::new();

                zones.iter_mut().zip(regions.iter()).map(
                    |(ref mut zone, (region, frame, visible))| {
                        zone.region = *region;
                        zone.visible = *visible;

                        if *visible {
                            placements.push(Placement {
                                zone: id,
                                region: Some(*region),
                                frame: *frame,
                            });

                            placements.append(
                                &mut zone.arrange(client_map, focus, region),
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
            && self.symbol == other.symbol
            && self.name == other.name
            && self.config == other.config
            && self.data == other.data
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
            .field("symbol", &self.symbol)
            .field("name", &self.name)
            .field("kind", &self.kind)
            .field("prev_kind", &self.prev_kind)
            .field("config", &self.config)
            .field("data", &self.data)
            .finish()
    }
}
