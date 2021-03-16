use crate::client::Client;
use crate::common::Change;
use crate::common::Direction;
use crate::common::Ident;
use crate::common::Identify;
use crate::common::Index;
use crate::common::StateChangeError;
use crate::common::FREE_DECORATION;
use crate::common::NO_DECORATION;
use crate::cycle::Cycle;
use crate::cycle::InsertPos;
use crate::cycle::Selector;
use crate::zone::Placement;
use crate::zone::PlacementKind;
use crate::zone::PlacementMethod;
use crate::zone::PlacementRegion;
use crate::zone::ZoneId;
use crate::zone::ZoneManager;
use crate::zone::MAX_GAP_SIZE;
use crate::zone::MAX_MAIN_COUNT;
use crate::zone::MAX_MARGIN;

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
        zone_manager: &ZoneManager,
    ) -> Option<&Window> {
        let sel = match sel {
            ClientSelector::AtActive => Selector::AtActive,
            ClientSelector::AtMaster => {
                if let Some(&id) = self.zones.active_element() {
                    let cycle = zone_manager.nearest_cycle(id);
                    let cycle = zone_manager.zone(cycle);

                    Selector::AtIndex(std::cmp::min(
                        cycle.data().unwrap().main_count as usize,
                        self.clients.len(),
                    ))
                } else {
                    return None;
                }
            },
            ClientSelector::AtIndex(index) => Selector::AtIndex(*index),
            ClientSelector::AtIdent(window) => Selector::AtIdent(*window as Ident),
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

    pub fn arrange<F>(
        &self,
        zone_manager: &mut ZoneManager,
        client_map: &HashMap<Window, Client>,
        screen_region: Region,
        filter: F,
    ) -> Vec<Placement>
    where
        F: Fn(&Client) -> bool,
    {
        if !self.clients.is_empty() {
            let zone = zone_manager.zone_mut(self.root_zone);
            zone.set_region(screen_region);

            let (to_ignore_ids, to_ignore_clients): (Vec<_>, Vec<_>) = self
                .clients
                .iter()
                .map(|window| client_map.get(window).unwrap())
                .filter(|&client| filter(client))
                .map(|client| (client.zone(), client))
                .unzip();

            zone_manager
                .arrange(self.root_zone, &to_ignore_ids)
                .into_iter()
                .chain(to_ignore_clients.into_iter().map(|client| {
                    let (method, decoration) = if client.is_fullscreen() && !client.is_in_window() {
                        (PlacementMethod::Tile, NO_DECORATION)
                    } else {
                        (PlacementMethod::Free, FREE_DECORATION)
                    };

                    Placement {
                        method,
                        kind: PlacementKind::Client(client.window()),
                        zone: client.zone(),
                        region: if method == PlacementMethod::Tile {
                            PlacementRegion::NewRegion(screen_region)
                        } else {
                            PlacementRegion::FreeRegion
                        },
                        decoration,
                    }
                }))
                .collect()
        } else {
            Vec::with_capacity(0)
        }
    }

    pub fn cycle_focus(
        &mut self,
        dir: Direction,
        client_map: &HashMap<Window, Client>,
        zone_manager: &ZoneManager,
    ) -> Option<(Window, Window)> {
        if self.clients.len() < 2 {
            return None;
        }

        let window = self.clients.active_element().unwrap();
        let id = client_map.get(window).unwrap().zone();
        let config = zone_manager.active_layoutconfig(id);

        if let Some(config) = config {
            if !config.wraps && self.clients.next_will_wrap(dir) {
                return None;
            }
        }

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

    pub fn reset_layout(
        &self,
        zone_manager: &mut ZoneManager,
    ) -> Result<(), StateChangeError> {
        let &id = self
            .zones
            .active_element()
            .ok_or(StateChangeError::EarlyStop)?;

        let default_data = zone_manager
            .active_default_data(id)
            .ok_or(StateChangeError::EarlyStop)?;

        let data = zone_manager
            .active_data_mut(id)
            .ok_or(StateChangeError::EarlyStop)?;

        Ok(*data = default_data)
    }

    pub fn change_gap_size(
        &self,
        change: Change,
        delta: u32,
        zone_manager: &mut ZoneManager,
    ) -> Result<(), StateChangeError> {
        let &id = self
            .zones
            .active_element()
            .ok_or(StateChangeError::EarlyStop)?;

        let data = zone_manager
            .active_data_mut(id)
            .ok_or(StateChangeError::EarlyStop)?;

        let new_gap_size = match change {
            Change::Inc => std::cmp::min(data.gap_size + delta, MAX_GAP_SIZE),
            Change::Dec => std::cmp::max(data.gap_size as i32 - delta as i32, 0) as u32,
        };

        if new_gap_size == data.gap_size {
            return Err(StateChangeError::LimitReached);
        }

        Ok(data.gap_size = new_gap_size)
    }

    pub fn reset_gap_size(
        &self,
        zone_manager: &mut ZoneManager,
    ) -> Result<(), StateChangeError> {
        let &id = self
            .zones
            .active_element()
            .ok_or(StateChangeError::EarlyStop)?;

        let default_data = zone_manager
            .active_default_data(id)
            .ok_or(StateChangeError::EarlyStop)?;

        let data = zone_manager
            .active_data_mut(id)
            .ok_or(StateChangeError::EarlyStop)?;

        Ok(data.gap_size = default_data.gap_size)
    }

    pub fn change_main_count(
        &self,
        change: Change,
        zone_manager: &mut ZoneManager,
    ) -> Result<(), StateChangeError> {
        let &id = self
            .zones
            .active_element()
            .ok_or(StateChangeError::EarlyStop)?;

        let data = zone_manager
            .active_data_mut(id)
            .ok_or(StateChangeError::EarlyStop)?;

        let new_main_count = match change {
            Change::Inc => std::cmp::min(data.main_count + 1, MAX_MAIN_COUNT),
            Change::Dec => std::cmp::max(data.main_count as i32 - 1, 0) as u32,
        };

        if data.main_count == new_main_count {
            Err(StateChangeError::LimitReached)
        } else {
            Ok(data.main_count = new_main_count)
        }
    }

    pub fn change_main_factor(
        &self,
        change: Change,
        delta: f32,
        zone_manager: &mut ZoneManager,
    ) -> Result<(), StateChangeError> {
        let &id = self
            .zones
            .active_element()
            .ok_or(StateChangeError::EarlyStop)?;

        let data = zone_manager
            .active_data_mut(id)
            .ok_or(StateChangeError::EarlyStop)?;

        match change {
            Change::Inc => data.main_factor += delta,
            Change::Dec => data.main_factor -= delta,
        }

        if data.main_factor < 0.05f32 {
            data.main_factor = 0.05f32;
        } else if data.main_factor > 0.95f32 {
            data.main_factor = 0.95f32;
        }

        Ok(())
    }

    pub fn change_margin(
        &self,
        edge: Edge,
        change: Change,
        delta: u32,
        zone_manager: &mut ZoneManager,
    ) -> Result<(), StateChangeError> {
        let &id = self
            .zones
            .active_element()
            .ok_or(StateChangeError::EarlyStop)?;

        let data = zone_manager
            .active_data_mut(id)
            .ok_or(StateChangeError::EarlyStop)?;

        let delta_change = match change {
            Change::Inc => delta as i32,
            Change::Dec => -(delta as i32),
        };

        let (edge_value, edge_max) = match edge {
            Edge::Left => (&mut data.margin.left, MAX_MARGIN.left),
            Edge::Right => (&mut data.margin.right, MAX_MARGIN.right),
            Edge::Top => (&mut data.margin.top, MAX_MARGIN.top),
            Edge::Bottom => (&mut data.margin.bottom, MAX_MARGIN.bottom),
        };

        let edge_changed = *edge_value as i32 + delta_change;
        let edge_changed = std::cmp::max(edge_changed, 0);
        let edge_changed = std::cmp::min(edge_changed, edge_max as i32);

        if *edge_value == edge_changed as u32 {
            Err(StateChangeError::LimitReached)
        } else {
            Ok(*edge_value = edge_changed as u32)
        }
    }

    pub fn reset_margin(
        &self,
        zone_manager: &mut ZoneManager,
    ) -> Result<(), StateChangeError> {
        let &id = self
            .zones
            .active_element()
            .ok_or(StateChangeError::EarlyStop)?;

        let default_data = zone_manager
            .active_default_data(id)
            .ok_or(StateChangeError::EarlyStop)?;

        let data = zone_manager
            .active_data_mut(id)
            .ok_or(StateChangeError::EarlyStop)?;

        Ok(data.margin = default_data.margin)
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
