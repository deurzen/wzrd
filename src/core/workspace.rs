use crate::client::Client;
use crate::common::Change;
use crate::common::Direction;
use crate::common::Ident;
use crate::common::Identify;
use crate::common::Index;
use crate::cycle::Cycle;
use crate::cycle::InsertPos;
use crate::cycle::Selector;
use crate::zone::LayoutMethod;
use crate::zone::Placement;
use crate::zone::ZoneId;
use crate::zone::ZoneManager;

use winsys::common::Edge;
use winsys::common::Grip;
use winsys::common::Pos;
use winsys::common::Region;
use winsys::common::Window;

use std::collections::HashMap;
use std::collections::VecDeque;

#[derive(Clone, Copy)]
pub enum ClientSelector {
    AtActive,
    AtMaster,
    AtIndex(Index),
    AtIdent(Window),
    First,
    Last,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum BufferKind {
    Move,
    Resize,
    Scratchpad,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buffer {
    kind: BufferKind,
    handle: Window,
    window: Option<Window>,
    grip: Option<Grip>,
    grip_pos: Option<Pos>,
    window_region: Option<Region>,
}

impl Buffer {
    pub fn new(
        kind: BufferKind,
        handle: Window,
    ) -> Self {
        Self {
            kind,
            handle,
            window: None,
            grip: None,
            grip_pos: None,
            window_region: None,
        }
    }

    pub fn set(
        &mut self,
        window: Window,
        grip: Grip,
        pos: Pos,
        region: Region,
    ) {
        self.window = Some(window);
        self.grip = Some(grip);
        self.grip_pos = Some(pos);
        self.window_region = Some(region);
    }

    pub fn unset(&mut self) {
        self.window = None;
        self.grip = None;
        self.grip_pos = None;
        self.window_region = None;
    }

    pub fn is_occupied(&self) -> bool {
        self.window.is_some()
    }

    pub fn handle(&self) -> Window {
        self.handle
    }

    pub fn window(&self) -> Option<Window> {
        self.window
    }

    pub fn grip(&self) -> Option<Grip> {
        self.grip
    }

    pub fn grip_pos(&self) -> Option<Pos> {
        self.grip_pos
    }

    pub fn set_grip_pos(
        &mut self,
        pos: Pos,
    ) {
        self.grip_pos = Some(pos);
    }

    pub fn window_region(&self) -> &Option<Region> {
        &self.window_region
    }

    pub fn set_window_region(
        &mut self,
        region: Region,
    ) {
        self.window_region = Some(region);
    }
}

#[derive(Debug, Clone)]
pub struct Scratchpad {
    command: String,
    client: Option<Window>,
    active: bool,
}

#[derive(Debug, Clone)]
pub struct Workspace {
    number: Ident,
    name: String,
    root_zone: ZoneId,
    zones: Cycle<ZoneId>,
    clients: Cycle<Window>,
    icons: Cycle<Window>,
}

impl Workspace {
    pub fn new(
        name: impl Into<String>,
        number: Ident,
        root_zone: ZoneId,
    ) -> Self {
        Self {
            number,
            name: name.into(),
            root_zone,
            zones: Cycle::new(vec![root_zone], true),
            clients: Cycle::new(Vec::new(), true),
            icons: Cycle::new(Vec::new(), true),
        }
    }

    pub fn number(&self) -> Ident {
        self.number
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn root_zone(&self) -> ZoneId {
        self.root_zone
    }

    pub fn set_name(
        &mut self,
        name: impl Into<String>,
    ) {
        self.name = name.into();
    }

    pub fn len(&self) -> usize {
        self.clients.len()
    }

    pub fn contains(
        &self,
        window: Window,
    ) -> bool {
        self.clients.contains(&window)
    }

    pub fn is_empty(&self) -> bool {
        self.clients.len() == 0
    }

    pub fn iter(&self) -> std::collections::vec_deque::Iter<Window> {
        self.clients.iter()
    }

    pub fn iter_mut(&mut self) -> std::collections::vec_deque::IterMut<Window> {
        self.clients.iter_mut()
    }

    pub fn stack(&self) -> &VecDeque<Window> {
        self.clients.stack()
    }

    pub fn stack_after_focus(&self) -> Vec<Window> {
        self.clients.stack_after_focus()
    }

    pub fn active_zone(&self) -> Option<ZoneId> {
        self.zones.active_element().copied()
    }

    pub fn focused_client(&self) -> Option<Window> {
        self.clients.active_element().copied()
    }

    pub fn get_client_for(
        &self,
        sel: &ClientSelector,
    ) -> Option<&Window> {
        let sel = match sel {
            ClientSelector::AtActive => Selector::AtActive,
            ClientSelector::AtMaster => {
                // TODO: zone change
                return None;
            },
            ClientSelector::AtIndex(index) => Selector::AtIndex(*index),
            ClientSelector::AtIdent(window) => {
                Selector::AtIdent(*window as Ident)
            },
            ClientSelector::First => Selector::First,
            ClientSelector::Last => Selector::Last,
        };

        self.clients.get_for(&sel)
    }

    pub fn next_client(
        &self,
        dir: Direction,
    ) -> Option<Window> {
        self.clients.next_element(dir).copied()
    }

    pub fn add_zone(
        &mut self,
        id: ZoneId,
        insert: &InsertPos,
    ) {
        self.zones.insert_at(insert, id);
    }

    pub fn add_client(
        &mut self,
        window: Window,
        insert: &InsertPos,
    ) {
        self.clients.insert_at(insert, window);
    }

    pub fn replace_client(
        &mut self,
        window: Window,
        replacement: Window,
    ) {
        self.clients.remove_for(&Selector::AtIdent(replacement));
        self.clients
            .insert_at(&InsertPos::BeforeIdent(window), replacement);
        self.clients.remove_for(&Selector::AtIdent(window));
    }

    pub fn activate_zone(
        &mut self,
        id: ZoneId,
    ) -> Option<ZoneId> {
        let prev_active = match self.zones.active_element() {
            Some(z) => *z,
            None => return None,
        };

        self.zones.activate_for(&Selector::AtIdent(id));
        Some(prev_active)
    }

    pub fn focus_client(
        &mut self,
        window: Window,
    ) -> Option<Window> {
        let prev_active = match self.clients.active_element() {
            Some(c) => *c,
            None => return None,
        };

        self.clients.activate_for(&Selector::AtIdent(window));
        Some(prev_active)
    }

    pub fn remove_zone(
        &mut self,
        id: ZoneId,
    ) -> Option<Window> {
        self.zones.remove_for(&Selector::AtIdent(id))
    }

    pub fn remove_client(
        &mut self,
        window: Window,
    ) -> Option<Window> {
        self.clients.remove_for(&Selector::AtIdent(window))
    }

    pub fn remove_focused_client(&mut self) -> Option<Window> {
        self.clients.remove_for(&Selector::AtActive)
    }

    pub fn arrange(
        &self,
        zone_manager: &mut ZoneManager,
        screen_region: Region,
    ) -> (LayoutMethod, Vec<Placement>) {
        if !self.clients.is_empty() {
            // TODO: zone change
            let placements = zone_manager.arrange(self.root_zone);

            println!("!!!!! {:#?}", placements);

            placements
        } else {
            (LayoutMethod::Free, Vec::with_capacity(0))
        }
    }

    pub fn set_layout(
        &mut self,
        // kind: LayoutKind,
    ) {
        // TODO: zone change
    }

    pub fn toggle_layout(&mut self) {
        // TODO: zone change
    }

    pub fn cycle_layout(
        &mut self,
        dir: Direction,
    ) {
        // TODO: zone change
    }

    pub fn cycle_focus(
        &mut self,
        dir: Direction,
    ) -> Option<(Window, Window)> {
        if self.clients.len() < 2 {
            return None;
        }

        // TODO: zone change

        let prev_active = *self.clients.active_element()?;
        let now_active = *self.clients.cycle_active(dir)?;

        if prev_active != now_active {
            Some((prev_active, now_active))
        } else {
            None
        }
    }

    pub fn drag_focus(
        &mut self,
        dir: Direction,
    ) -> Option<Window> {
        self.clients.drag_active(dir).copied()
    }

    pub fn rotate_clients(
        &mut self,
        dir: Direction,
    ) -> Option<(Window, Window)> {
        if self.clients.len() < 2 {
            return None;
        }

        let prev_active = *self.clients.active_element()?;

        self.clients.rotate(dir);

        let now_active = *self.clients.active_element()?;

        if prev_active != now_active {
            Some((prev_active, now_active))
        } else {
            None
        }
    }

    pub fn reset_layout(&mut self) {
        // TODO: zone change
    }

    pub fn change_gap_size(
        &mut self,
        change: Change,
        delta: u32,
    ) {
        // TODO: zone change
    }

    pub fn reset_gap_size(&mut self) {
        // TODO: zone change
    }

    pub fn change_main_count(
        &mut self,
        change: Change,
    ) {
        // TODO: zone change
    }

    pub fn change_main_factor(
        &mut self,
        change: Change,
        delta: f32,
    ) {
        // TODO: zone change
    }

    pub fn change_margin(
        &mut self,
        edge: Edge,
        change: Change,
        delta: u32,
    ) {
        // TODO: zone change
    }

    pub fn reset_margin(&mut self) {
        // TODO: zone change
    }

    pub fn focused_icon(&self) -> Option<Window> {
        self.icons.active_element().copied()
    }

    pub fn icon_to_client(
        &mut self,
        window: Window,
    ) {
        if let Some(icon) = self.remove_icon(window) {
            self.add_client(icon, &InsertPos::Back);
        }
    }

    pub fn client_to_icon(
        &mut self,
        window: Window,
    ) {
        if let Some(client) = self.remove_client(window) {
            self.add_icon(client);
        }
    }

    pub fn add_icon(
        &mut self,
        window: Window,
    ) {
        self.icons.insert_at(&InsertPos::Back, window);
    }

    pub fn remove_icon(
        &mut self,
        window: Window,
    ) -> Option<Window> {
        self.icons.remove_for(&Selector::AtIdent(window))
    }
}

impl Identify for Workspace {
    fn id(&self) -> Ident {
        self.number
    }
}

impl PartialEq for Workspace {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.number == other.number
    }
}
