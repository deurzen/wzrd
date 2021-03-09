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

pub type ZoneId = u32;
type Color = u32;

static INSTANCE_COUNT: atomic::AtomicU32 = atomic::AtomicU32::new(1);
fn next_id() -> ZoneId {
    INSTANCE_COUNT.fetch_add(1, atomic::Ordering::Relaxed) as ZoneId
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ColorScheme {
    pub regular: Color,
    pub focused: Color,
    pub urgent: Color,
    pub rdisowned: Color,
    pub fdisowned: Color,
    pub rsticky: Color,
    pub fsticky: Color,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            regular: 0x333333,
            focused: 0xe78a53,
            urgent: 0xfbcb97,
            rdisowned: 0x999999,
            fdisowned: 0xc1c1c1,
            rsticky: 0x444444,
            fsticky: 0x5f8787,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Border {
    pub width: u32,
    pub colors: ColorScheme,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Frame {
    pub extents: Extents,
    pub colors: ColorScheme,
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Disposition {
    Unchanged,
    Changed(Region, Decoration),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PlacementKind {
    Client(Window),
    Tab(usize),
    Layout,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Placement {
    pub kind: PlacementKind,
    pub zone: ZoneId,
    pub region: Option<Region>,
    pub decoration: Decoration,
}

type LayoutFn = fn(&Region, &LayoutData, Vec<bool>) -> Vec<(Disposition, bool)>;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LayoutMethod {
    /// Does not inhibit free placement of clients
    Free,

    /// Arranges clients along a predefined layout
    Tile,

    /// Semi-adjustable tree-based layout
    Tree,
}

#[non_exhaustive]
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

    fn config(&self) -> LayoutConfig {
        match *self {
            // TODO
            LayoutKind::Float => LayoutConfig::default(),
            LayoutKind::SingleFloat => LayoutConfig::default(),
            LayoutKind::Center => LayoutConfig::default(),
            LayoutKind::Monocle => LayoutConfig::default(),
            LayoutKind::Paper => LayoutConfig::default(),
            LayoutKind::SStack => LayoutConfig::default(),
            LayoutKind::Stack => LayoutConfig::default(),

            #[allow(unreachable_patterns)]
            _ => unimplemented!(
                "{:?} does not have an associated configuration",
                self
            ),
        }
    }

    fn func(&self) -> LayoutFn {
        match *self {
            // TODO
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

            #[allow(unreachable_patterns)]
            _ => unimplemented!(
                "{:?} does not have an associated function",
                self
            ),
        }
    }
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Clone, Copy)]
struct LayoutConfig {
    method: LayoutMethod,
    decoration: Decoration,
    root_only: bool,
    free: bool,
    persistent: bool,
    single: bool,
    wraps: bool,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            method: LayoutMethod::Free,
            decoration: Default::default(),
            root_only: true,
            free: true,
            persistent: false,
            single: false,
            wraps: true,
        }
    }
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Clone, Copy)]
struct LayoutData {
    /// Generic layout data
    margin: Option<Padding>,
    gap_size: u32,

    /// Tiled layout data
    main_count: u32,
    main_factor: f32,
}

impl Default for LayoutData {
    fn default() -> Self {
        Self {
            margin: None,
            gap_size: 0u32,

            main_count: 0u32,
            main_factor: 0f32,
        }
    }
}

#[derive(Clone)]
pub struct Layout {
    kind: LayoutKind,
    prev_kind: LayoutKind,
    data: HashMap<LayoutKind, LayoutData>,
    default_data: HashMap<LayoutKind, LayoutData>,
}

impl Layout {
    pub fn new() -> Self {
        let mut data = HashMap::with_capacity(LayoutKind::COUNT);
        let mut default_data = HashMap::with_capacity(LayoutKind::COUNT);

        for kind in LayoutKind::iter() {
            data.insert(kind, Default::default());
            default_data.insert(kind, Default::default());
        }

        Self {
            kind: LayoutKind::Float,
            prev_kind: LayoutKind::Float,
            data,
            default_data,
        }
    }

    pub fn with_kind(kind: LayoutKind) -> Self {
        let mut data = HashMap::with_capacity(LayoutKind::COUNT);
        let mut default_data = HashMap::with_capacity(LayoutKind::COUNT);

        for kind in LayoutKind::iter() {
            data.insert(kind, Default::default());
            default_data.insert(kind, Default::default());
        }

        Self {
            kind,
            prev_kind: kind,
            data,
            default_data,
        }
    }

    fn get_data(&self) -> &LayoutData {
        self.data.get(&self.kind).unwrap()
    }

    fn get_data_mut(&mut self) -> &mut LayoutData {
        self.data.get_mut(&self.kind).unwrap()
    }

    fn get_default_data(&self) -> &LayoutData {
        self.default_data.get(&self.kind).unwrap()
    }

    fn set_kind(
        &mut self,
        kind: LayoutKind,
    ) {
        self.prev_kind = self.kind;
        self.kind = kind;
    }
}

trait Apply {
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
        (self.kind.func())(
            region,
            &self.data.get(&self.kind).unwrap(),
            active_map,
        )
    }
}

#[derive(Debug, PartialEq)]
pub enum ZoneContent {
    Empty,
    Client(Window),
    Tab(Cycle<ZoneId>),
    Layout(Layout, Cycle<ZoneId>),
}

// impl ZoneContent {
//     fn set_visible(
//         &mut self,
//         is_visible: bool,
//     ) {
//         match self {
//             ZoneContent::Tab(zones) | ZoneContent::Layout(_, zones) => {
//                 zones.iter_mut().for_each(|zone| {
//                     zone.set_visible(is_visible);
//                 });
//             },
//             _ => {},
//         }
//     }

//     fn n_regions(&self) -> usize {
//         match self {
//             ZoneContent::Empty => 0,
//             ZoneContent::Client(_) => 1,
//             ZoneContent::Tab(_) => 1,
//             ZoneContent::Layout(_, zones) => zones.len(),
//         }
//     }

//     fn n_zones(&self) -> usize {
//         match self {
//             ZoneContent::Empty => 0,
//             ZoneContent::Client(_) => 1,
//             ZoneContent::Tab(zones) | ZoneContent::Layout(_, zones) => {
//                 zones.len()
//             },
//         }
//     }

//     fn n_subzones(&self) -> usize {
//         match self {
//             ZoneContent::Empty => 0,
//             ZoneContent::Client(_) => 1,
//             ZoneContent::Tab(zones) | ZoneContent::Layout(_, zones) => zones
//                 .iter()
//                 .fold(0, |len, zone| len + zone.content.n_subzones()),
//         }
//     }
// }

#[derive(Debug)]
pub struct Zone {
    id: ZoneId,
    parent: Option<ZoneId>,
    content: ZoneContent,
    region: Region,
    decoration: Decoration,
    is_active: bool,
    is_visible: bool,
}

impl Zone {
    pub fn new(
        parent: Option<ZoneId>,
        content: ZoneContent,
        region: Region,
        is_active: bool,
        is_visible: bool,
    ) -> (ZoneId, Self) {
        let id = next_id();

        (id, Self {
            id,
            parent,
            content,
            region,
            decoration: Decoration {
                border: None,
                frame: None,
            },
            is_active,
            is_visible,
        })
    }

    pub fn set_visible(
        &mut self,
        is_visible: bool,
    ) {
        self.is_visible = is_visible;
        // self.content.set_visible(is_visible);
    }
}

pub trait Arrange {
    fn arrange(
        &mut self,
        client_map: &HashMap<Window, Client>,
        region: &Region,
    ) -> Vec<Placement>;
}

enum ZoneChange {
    Visible(bool),
    Active(bool),
    Region(Region),
    Decoration(Decoration),
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

    pub fn set_visible(
        &mut self,
        zone: ZoneId,
        is_visible: bool,
    ) {
        let subzones = self.gather_subzones(zone, true);

        subzones.iter().for_each(|zone| {
            if let Some(zone) = self.zone_map.get_mut(&zone) {
                zone.is_visible = is_visible;
            }
        });

        if let Some(zone) = self.zone_map.get_mut(&zone) {
            zone.is_visible = is_visible;
        }
    }

    fn gather_subzones(
        &self,
        zone: ZoneId,
        recurse: bool,
    ) -> Vec<ZoneId> {
        if let Some(zone) = self.zone_map.get(&zone) {
            match &zone.content {
                ZoneContent::Empty => {},
                ZoneContent::Client(_) => {},
                ZoneContent::Tab(zones) | ZoneContent::Layout(_, zones) => {
                    let mut zones = zones.as_vec();

                    if recurse {
                        let mut subzones = Vec::new();

                        zones.iter().for_each(|&zone| {
                            subzones
                                .extend(self.gather_subzones(zone, recurse));
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
    ) -> Vec<Placement> {
        let id = zone;
        let zone = self.zone_map.get_mut(&id);
        let region;

        if let Some(zone) = zone {
            region = zone.region;
            zone.is_active = true;
        } else {
            return Vec::with_capacity(0);
        }

        self.arrange_subzones(id, region)
    }

    fn arrange_subzones(
        &mut self,
        zone: ZoneId,
        region: Region,
    ) -> Vec<Placement> {
        let id = zone;
        let zone = self.zone_map.get(&id).unwrap();

        let region = zone.region;
        let decoration = zone.decoration;
        let content = &zone.content;

        let mut zone_changes: Vec<(ZoneId, ZoneChange)> = Vec::new();

        let placements = match &content {
            ZoneContent::Empty => {
                return Vec::with_capacity(0);
            },
            ZoneContent::Client(window) => {
                return vec![Placement {
                    kind: PlacementKind::Client(*window),
                    zone: id,
                    region: Some(region),
                    decoration,
                }];
            },
            ZoneContent::Tab(zones) => {
                let mut placements = vec![Placement {
                    kind: PlacementKind::Tab(zones.len()),
                    zone: id,
                    region: Some(region),
                    decoration,
                }];

                zone_changes.extend(
                    self.gather_subzones(id, true)
                        .iter()
                        .map(|&id| (id, ZoneChange::Visible(false)))
                        .collect::<Vec<(ZoneId, ZoneChange)>>(),
                );
                
                match zones.active_element() {
                    None => placements,
                    Some(&zone) => {
                        let subzones = self.gather_subzones(zone, true);

                        zone_changes.extend(
                            subzones
                                .iter()
                                .map(|&id| (id, ZoneChange::Visible(true)))
                                .collect::<Vec<(ZoneId, ZoneChange)>>(),
                        );

                        zone_changes.extend(
                            subzones
                                .iter()
                                .map(|&id| (id, ZoneChange::Region(region)))
                                .collect::<Vec<(ZoneId, ZoneChange)>>(),
                        );

                        placements.extend(self.arrange_subzones(id, region));
                        placements
                    },
                }
            },
            ZoneContent::Layout(layout, zones) => {
                let mut placements = vec![Placement {
                    kind: PlacementKind::Layout,
                    zone: id,
                    region: Some(region),
                    decoration,
                }];

                let application = layout
                    .apply(&region, zones.iter().map(|id| {
                        let zone = self.zone_map.get(id).unwrap();
                        zone.is_active
                    }).collect());

                zones.iter().zip(application.iter()).for_each(
                    |(id, (disposition, is_visible))| {
                        let zone = self.zone_map.get(id).unwrap();

                        let (region, decoration) = match disposition {
                            Disposition::Unchanged => {
                                (zone.region, zone.decoration)
                            },
                            Disposition::Changed(region, decoration) => {
                                zone_changes.push((*id, ZoneChange::Visible(true)));
                                zone_changes.push((*id, ZoneChange::Region(*region)));
                                zone_changes.push((*id, ZoneChange::Decoration(*decoration)));

                                (*region, *decoration)
                            },
                        };


                });


                placements
            },
        };

        zone_changes.iter().for_each(|(id, change)| {
            let zone = self.zone_map.get_mut(id).unwrap();

            match *change {
                ZoneChange::Visible(is_visible) => zone.is_visible = is_visible,
                ZoneChange::Active(is_active) => zone.is_active = is_active,
                ZoneChange::Region(region) => zone.region = region,
                ZoneChange::Decoration(decoration) => zone.decoration = decoration,
            };
        });

        placements
    }
}

// impl Arrange for Zone {
//     fn arrange(
//         &mut self,
//         client_map: &HashMap<Window, Client>,
//         region: &Region,
//     ) -> Vec<Placement> {
//         match &mut self.content {
//             ZoneContent::Empty => {
//                 self.set_visible(true);

//                 Vec::new()
//             },
//             ZoneContent::Client(_) => {
//                 self.set_visible(true);

//                 vec![Placement {
//                     zone: self.id,
//                     region: Some(*region),
//                     decoration: self.decoration,
//                 }]
//             },
//             ZoneContent::Tab(ref mut zones) => {
//                 let mut tab = vec![Placement {
//                     zone: self.id,
//                     region: Some(*region),
//                     decoration: self.decoration,
//                 }];

//                 zones.on_all_mut(|zone| {
//                     zone.set_visible(false);
//                 });

//                 match zones.active_element_mut() {
//                     None => tab,
//                     Some(zone) => {
//                         zone.set_visible(true);

//                         tab.append(&mut zone.arrange(client_map, region));

//                         tab
//                     },
//                 }
//             },
//             ZoneContent::Layout(layout, ref mut zones) => {
//                 let id = self.id;
//                 let application = layout
//                     .apply(region, zones.iter().map(|z| z.is_active).collect());
//                 let mut placements = Vec::with_capacity(zones.len());

//                 zones.iter_mut().zip(application.iter()).for_each(
//                     |(ref mut zone, (disposition, is_visible))| {
//                         let (region, decoration) = match disposition {
//                             Disposition::Unchanged => {
//                                 (zone.region, zone.decoration)
//                             },
//                             Disposition::Changed(region, decoration) => {
//                                 zone.region = *region;
//                                 zone.decoration = *decoration;

//                                 (*region, *decoration)
//                             },
//                         };

//                         zone.set_visible(*is_visible);

//                         if *is_visible {
//                             placements.push(Placement {
//                                 zone: id,
//                                 region: Some(region),
//                                 decoration,
//                             });

//                             placements
//                                 .append(&mut zone.arrange(client_map, &region));
//                         }
//                     },
//                 );

//                 placements
//             },
//         }
//     }
// }

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
