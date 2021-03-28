use crate::identify::Ident;
use crate::identify::Identify;
use crate::identify::Index;

use winsys::geometry::Region;
use winsys::screen::Screen;

#[derive(Clone)]
pub struct Partition {
    screen: Screen,
    index: Index,
}

impl Partition {
    pub fn new(
        screen: Screen,
        index: Index,
    ) -> Self {
        Self {
            screen,
            index,
        }
    }

    #[inline]
    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    #[inline]
    pub fn index(&self) -> Index {
        self.index
    }

    #[inline]
    pub fn full_region(&self) -> Region {
        self.screen.full_region()
    }

    #[inline]
    pub fn placeable_region(&self) -> Region {
        self.screen.placeable_region()
    }
}

impl Identify for Partition {
    #[inline(always)]
    fn id(&self) -> Ident {
        self.screen.number() as Ident
    }
}

impl PartialEq for Partition {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.screen.number() == other.screen.number()
    }
}

#[allow(unused)]
impl std::fmt::Debug for Partition {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("Partition")
            .field("screen", &self.screen)
            .field("workspace", &self.index)
            .finish()
    }
}
