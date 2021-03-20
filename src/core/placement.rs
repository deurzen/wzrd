use crate::decoration::Decoration;
use crate::zone::ZoneContent;
use crate::zone::ZoneId;

use winsys::geometry::Region;
use winsys::window::Window;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PlacementMethod {
    Free,
    Tile,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PlacementClass<T> {
    Free(T),
    Tile(T),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PlacementTarget {
    Client(Window),
    Tab(usize),
    Layout,
}

impl PlacementTarget {
    pub fn from_zone_content(content: &ZoneContent) -> Self {
        match content {
            ZoneContent::Client(window) => PlacementTarget::Client(*window),
            ZoneContent::Tab(zones) => PlacementTarget::Tab(zones.len()),
            ZoneContent::Layout(..) => PlacementTarget::Layout,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PlacementRegion {
    NoRegion,
    FreeRegion,
    NewRegion(Region),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Placement {
    pub method: PlacementMethod,
    pub kind: PlacementTarget,
    pub zone: ZoneId,
    pub region: PlacementRegion,
    pub decoration: Decoration,
}
