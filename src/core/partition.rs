use crate::identify::Ident;
use crate::identify::Identify;
use crate::identify::Index;

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

    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    pub fn screen_mut(&mut self) -> &mut Screen {
        &mut self.screen
    }

    pub fn index(&self) -> Index {
        self.index
    }
}

impl Identify for Partition {
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
