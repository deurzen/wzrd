use crate::change::Disposition;
use crate::cycle::Cycle;
use crate::cycle::InsertPos;
use crate::cycle::Selector;
use crate::decoration::Border;
use crate::decoration::Decoration;
use crate::error::StateChangeError;
use crate::identify::Ident;
use crate::identify::Identify;
use crate::layout::Apply;
use crate::layout::Layout;
use crate::layout::LayoutConfig;
use crate::layout::LayoutData;
use crate::layout::LayoutKind;
use crate::placement::Placement;
use crate::placement::PlacementMethod;
use crate::placement::PlacementRegion;
use crate::placement::PlacementTarget;

use winsys::geometry::Region;
use winsys::window::Window;

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::atomic;
use std::vec::Vec;

pub type ZoneId = u32;

static INSTANCE_COUNT: atomic::AtomicU32 = atomic::AtomicU32::new(1);

#[derive(Debug, PartialEq)]
pub enum ZoneContent {
    Client(Window),
    Tab(Cycle<ZoneId>),
    Layout(Layout, Cycle<ZoneId>),
}

#[derive(Debug)]
pub struct Zone {
    id: ZoneId,
    parent: Cell<Option<ZoneId>>,
    method: Cell<PlacementMethod>,
    content: ZoneContent,
    region: Cell<Region>,
    decoration: Cell<Decoration>,
    is_visible: Cell<bool>,
}

impl Zone {
    fn next_id() -> ZoneId {
        INSTANCE_COUNT.fetch_add(1, atomic::Ordering::Relaxed) as ZoneId
    }

    fn new(
        parent: Option<ZoneId>,
        content: ZoneContent,
        region: Region,
    ) -> (ZoneId, Self) {
        let id = Self::next_id();

        (id, Self {
            id,
            parent: Cell::new(parent),
            method: Cell::new(PlacementMethod::Free),
            content,
            region: Cell::new(region),
            decoration: Cell::new(Decoration::NO_DECORATION),
            is_visible: Cell::new(true),
        })
    }

    pub fn set_content(
        &mut self,
        content: ZoneContent,
    ) {
        self.content = content;
    }

    fn set_kind(
        &mut self,
        kind: LayoutKind,
    ) -> Result<LayoutKind, StateChangeError> {
        match self.content {
            ZoneContent::Layout(ref mut layout, _) => layout.set_kind(kind),
            _ => Err(StateChangeError::InvalidCaller),
        }
    }

    pub fn prev_kind(&self) -> Result<LayoutKind, StateChangeError> {
        match &self.content {
            ZoneContent::Layout(layout, _) => Ok(layout.prev_kind()),
            _ => Err(StateChangeError::InvalidCaller),
        }
    }

    pub fn kind(&self) -> Result<LayoutKind, StateChangeError> {
        match &self.content {
            ZoneContent::Layout(layout, _) => Ok(layout.kind()),
            _ => Err(StateChangeError::InvalidCaller),
        }
    }

    pub fn set_region(
        &self,
        region: Region,
    ) {
        self.region.set(region);
    }

    pub fn set_method(
        &self,
        method: PlacementMethod,
    ) {
        self.method.set(method);
    }

    pub fn default_data(&self) -> Option<LayoutData> {
        match &self.content {
            ZoneContent::Layout(layout, _) => Some(layout.default_data()),
            _ => None,
        }
    }

    pub fn data(&self) -> Option<&LayoutData> {
        match self.content {
            ZoneContent::Layout(ref layout, _) => Some(layout.data()),
            _ => None,
        }
    }

    pub fn data_mut(&mut self) -> Option<&mut LayoutData> {
        match self.content {
            ZoneContent::Layout(ref mut layout, _) => Some(layout.data_mut()),
            _ => None,
        }
    }

    pub fn prev_data(&self) -> Option<&LayoutData> {
        match self.content {
            ZoneContent::Layout(ref layout, _) => Some(layout.prev_data()),
            _ => None,
        }
    }

    pub fn config(&self) -> Option<LayoutConfig> {
        match self.content {
            ZoneContent::Layout(ref layout, _) => Some(layout.config()),
            _ => None,
        }
    }

    pub fn method(&self) -> PlacementMethod {
        self.method.get()
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
    persistent_data_copy: bool,
}

impl ZoneManager {
    pub fn new() -> Self {
        Self {
            zone_map: HashMap::new(),
            persistent_data_copy: true,
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

    pub fn remove_zone(
        &mut self,
        id: ZoneId,
    ) {
        let cycle = self.nearest_cycle(id);
        let cycle = self.zone_map.get_mut(&cycle).unwrap();

        match &mut cycle.content {
            ZoneContent::Tab(zones) | ZoneContent::Layout(_, zones) => {
                zones.remove_for(&Selector::AtIdent(id));
            },
            _ => {},
        }
    }

    pub fn activate_zone(
        &self,
        id: ZoneId,
    ) {
        if let Some(cycle_id) = self.next_cycle(id) {
            let cycle = self.zone(cycle_id);

            match cycle.content {
                ZoneContent::Tab(ref zones) | ZoneContent::Layout(_, ref zones) => {
                    zones.activate_for(&Selector::AtIdent(id));
                    self.activate_zone(cycle_id);
                },
                _ => {},
            }
        }
    }

    pub fn set_kind(
        &mut self,
        id: ZoneId,
        kind: LayoutKind,
    ) -> Result<LayoutKind, StateChangeError> {
        let persistent_data_copy = self.persistent_data_copy;
        let cycle = self.nearest_cycle(id);
        let cycle = self.zone_mut(cycle);

        let prev_kind = cycle.set_kind(kind)?;

        if persistent_data_copy {
            let prev_data = *cycle.prev_data().unwrap();
            let data = cycle.data_mut().unwrap();
            *data = prev_data;
        }

        Ok(prev_kind)
    }

    pub fn set_prev_kind(
        &mut self,
        id: ZoneId,
    ) -> Result<LayoutKind, StateChangeError> {
        let persistent_data_copy = self.persistent_data_copy;
        let cycle = self.nearest_cycle(id);
        let cycle = self.zone_mut(cycle);

        let kind = cycle.prev_kind()?;
        let prev_kind = cycle.set_kind(kind)?;

        if persistent_data_copy {
            let prev_data = *cycle.prev_data().unwrap();
            let data = cycle.data_mut().unwrap();
            *data = prev_data;
        }

        Ok(prev_kind)
    }

    pub fn active_default_data(
        &mut self,
        id: ZoneId,
    ) -> Option<LayoutData> {
        let cycle = self.nearest_cycle(id);
        let cycle = self.zone(cycle);

        cycle.default_data()
    }

    pub fn active_prev_data(
        &mut self,
        id: ZoneId,
    ) -> Option<&LayoutData> {
        let cycle = self.nearest_cycle(id);
        let cycle = self.zone_mut(cycle);

        cycle.prev_data()
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
        self.zone_map.get(&id).and_then(|zone| zone.parent.get())
    }

    pub fn cycle_config(
        &self,
        id: ZoneId,
    ) -> Option<(ZoneId, Option<LayoutConfig>)> {
        let cycle = self.next_cycle(id)?;
        let zone = self.zone(cycle);

        Some((cycle, zone.config()))
    }

    pub fn is_cycle(
        &self,
        id: ZoneId,
    ) -> bool {
        let zone = self.zone_map.get(&id).unwrap();

        match zone.content {
            ZoneContent::Tab(_) | ZoneContent::Layout(..) => true,
            _ => false,
        }
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

            if let Some(parent) = zone.parent.get() {
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

    pub fn is_within_persisent(
        &self,
        mut id: ZoneId,
    ) -> bool {
        while let Some(next_id) = self.parent_id(id) {
            let zone = self.zone_map.get(&next_id).unwrap();

            match zone.content {
                ZoneContent::Tab(_) => {
                    return true;
                },
                ZoneContent::Layout(ref layout, _) => {
                    if layout.config().persistent {
                        return true;
                    }
                },
                _ => {},
            }

            id = next_id;
        }

        false
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

        Vec::with_capacity(0)
    }

    /// Arrange a zone and all of its subzones within the region
    /// of the supplied zone
    pub fn arrange(
        &self,
        zone: ZoneId,
        to_ignore: &Vec<ZoneId>,
    ) -> Vec<Placement> {
        let cycle = self.nearest_cycle(zone);
        let zone = self.zone_map.get(&cycle).unwrap();
        let region = zone.region.get();
        let decoration = zone.decoration.get();

        let method = match &zone.content {
            ZoneContent::Tab(_) => PlacementMethod::Tile,
            ZoneContent::Layout(layout, _) => layout.config().method,
            _ => panic!("attempting to derive method from non-cycle"),
        };

        self.arrange_subzones(cycle, region, decoration, method, to_ignore)
    }

    fn arrange_subzones(
        &self,
        id: ZoneId,
        region: Region,
        decoration: Decoration,
        method: PlacementMethod,
        to_ignore: &Vec<ZoneId>,
    ) -> Vec<Placement> {
        let zone = self.zone_map.get(&id).unwrap();
        let content = &zone.content;

        let mut zone_changes: Vec<(ZoneId, ZoneChange)> = Vec::new();

        let placements = match &content {
            ZoneContent::Client(window) => {
                return vec![Placement {
                    method,
                    kind: PlacementTarget::Client(*window),
                    zone: id,
                    region: if method == PlacementMethod::Free {
                        PlacementRegion::FreeRegion
                    } else {
                        PlacementRegion::NewRegion(region)
                    },
                    decoration,
                }];
            },
            ZoneContent::Tab(zones) => {
                let mut placements = vec![Placement {
                    method,
                    kind: PlacementTarget::Tab(zones.len()),
                    zone: id,
                    region: if method == PlacementMethod::Free {
                        PlacementRegion::FreeRegion
                    } else {
                        zone_changes.push((id, ZoneChange::Region(region)));
                        PlacementRegion::NewRegion(region)
                    },
                    decoration,
                }];

                let mut region = region;
                Layout::adjust_for_border(&mut region, 1, &Zone::MIN_ZONE_DIM);

                let active_element = zones.active_element().copied();
                let zones: Vec<ZoneId> = zones
                    .iter()
                    .filter(|&id| !to_ignore.contains(id))
                    .copied()
                    .collect();

                zones.into_iter().for_each(|id| {
                    let is_active_element = Some(id) == active_element;
                    let subzones = self.gather_subzones(id, !is_active_element);
                    let method = PlacementMethod::Tile;

                    subzones.into_iter().for_each(|id| {
                        zone_changes.push((id, ZoneChange::Visible(is_active_element)));
                        zone_changes.push((id, ZoneChange::Region(region)));
                        zone_changes.push((id, ZoneChange::Method(method)));
                    });

                    placements.extend(self.arrange_subzones(
                        id,
                        region,
                        Decoration {
                            frame: None,
                            border: Some(Border {
                                width: 1,
                                colors: Default::default(),
                            }),
                        },
                        method,
                        to_ignore,
                    ));
                });

                placements
            },
            ZoneContent::Layout(layout, zones) => {
                let active_element = zones.active_element();
                let mut subplacements = Vec::new();
                let mut placements = vec![Placement {
                    method,
                    kind: PlacementTarget::Layout,
                    zone: id,
                    region: if method == PlacementMethod::Free {
                        PlacementRegion::FreeRegion
                    } else {
                        PlacementRegion::NewRegion(region)
                    },
                    decoration,
                }];

                let zones: Vec<ZoneId> = zones
                    .iter()
                    .filter(|&id| layout.config().single || !to_ignore.contains(id))
                    .copied()
                    .collect();

                let (method, application) = layout.apply(
                    region,
                    zones.iter().map(|id| Some(id) == active_element).collect(),
                );

                zones.into_iter().zip(application.into_iter()).for_each(
                    |(id, (disposition, is_visible))| {
                        let (region, decoration) = match disposition {
                            Disposition::Unchanged(decoration) => {
                                let zone = self.zone_map.get(&id).unwrap();
                                (zone.region.get(), decoration)
                            },
                            Disposition::Changed(region, decoration) => (region, decoration),
                        };

                        if !is_visible {
                            let subzones = self.gather_subzones(id, true);

                            placements.push(Placement {
                                method,
                                kind: PlacementTarget::from_zone_content(&self.zone(id).content),
                                zone: id,
                                region: PlacementRegion::NoRegion,
                                decoration,
                            });

                            zone_changes.extend(
                                subzones
                                    .into_iter()
                                    .map(|id| {
                                        placements.push(Placement {
                                            method,
                                            kind: PlacementTarget::from_zone_content(
                                                &self.zone(id).content,
                                            ),
                                            zone: id,
                                            region: PlacementRegion::NoRegion,
                                            decoration,
                                        });

                                        (id, ZoneChange::Visible(false))
                                    })
                                    .collect::<Vec<(ZoneId, ZoneChange)>>(),
                            );
                        } else {
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

        {
            let zone = self.zone_map.get(&id).unwrap();
            zone.region.set(region);
            zone.decoration.set(decoration);
            zone.method.set(method);
        }

        zone_changes.into_iter().for_each(|(id, change)| {
            let zone = self.zone_map.get(&id).unwrap();

            match change {
                ZoneChange::Visible(is_visible) => {
                    zone.is_visible.set(is_visible);
                },
                ZoneChange::Region(region) => {
                    zone.region.set(region);
                },
                ZoneChange::Decoration(decoration) => {
                    zone.decoration.set(decoration);
                },
                ZoneChange::Method(method) => {
                    zone.method.set(method);
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
