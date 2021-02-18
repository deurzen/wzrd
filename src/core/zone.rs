use crate::client::Client;
use crate::common::Ident;
use crate::common::Identify;
use crate::common::Placement;
use crate::cycle::Cycle;
use crate::layout::Layout;

use winsys::common::Region;
use winsys::common::Window;

use std::vec::Vec;
use std::collections::HashMap;
use std::sync::atomic;

static INSTANCE_COUNT: atomic::AtomicUsize = atomic::AtomicUsize::new(1);
fn next_id() -> usize { INSTANCE_COUNT.fetch_add(1, atomic::Ordering::Relaxed) }

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

#[derive(Debug)]
pub enum Zone {
    Layout(Cycle<Window>),
    Recurse(Cycle<LayoutZone>),
}

#[derive(Debug)]
pub struct LayoutZone {
    number: usize,
    zone: Zone,
    layouts: Cycle<Layout>,
    previous_layout: LayoutKind,
}

// impl LayoutZone {
//     pub fn new() -> Self {
//         Self {
//             number: next_id(),
//         }
//     }
// }

pub trait Arrange {
    fn arrange(
        &self,
        client_map: &HashMap<Window, Client>,
        focus: Option<Window>,
        screen: &Region,
    ) -> Vec<Placement>;
}

impl Arrange for Zone {
    fn arrange(
        &self,
        client_map: &HashMap<Window, Client>,
        focus: Option<Window>,
        screen: &Region,
    ) -> Vec<Placement> {
        match self {
            Zone::Layout(clients) => {
                vec![]
            },
            Zone::Recurse(zones) => {
                vec![]
            },
        }
    }
}

impl std::cmp::PartialEq<Self> for LayoutZone {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.number == other.number
    }
}

impl Identify for LayoutZone {
    fn id(&self) -> Ident {
        self.number as Ident
    }
}
