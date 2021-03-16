use crate::binding::KeyBindings;
use crate::binding::MouseBindings;
use crate::client::Client;
use crate::common::Change;
use crate::common::Direction;
use crate::common::Index;
use crate::common::StateChangeError;
use crate::common::FREE_DECORATION;
use crate::common::MIN_WINDOW_DIM;
use crate::common::NO_DECORATION;
use crate::consume::get_spawner_pid;
use crate::cycle::Cycle;
use crate::cycle::InsertPos;
use crate::cycle::Selector;
use crate::jump::JumpCriterium;
use crate::jump::MatchMethod;
use crate::partition::Partition;
use crate::rule::Rules;
use crate::stack::StackLayer;
use crate::stack::StackManager;
use crate::workspace::Buffer;
use crate::workspace::BufferKind;
use crate::workspace::Workspace;
use crate::zone::Layout;
use crate::zone::LayoutKind;
use crate::zone::Placement;
use crate::zone::PlacementKind;
use crate::zone::PlacementMethod;
use crate::zone::PlacementRegion;
use crate::zone::ZoneContent;
use crate::zone::ZoneManager;

#[allow(unused_imports)]
use crate::util::Util;

use winsys::common::Corner;
use winsys::common::Dim;
use winsys::common::Edge;
use winsys::common::Grip;
use winsys::common::Hints;
use winsys::common::IcccmWindowState;
use winsys::common::Pid;
use winsys::common::Pos;
use winsys::common::Region;
use winsys::common::Window;
use winsys::common::WindowState;
use winsys::common::WindowType;
use winsys::connection::Connection;
use winsys::event::Event;
use winsys::event::PropertyKind;
use winsys::event::StackMode;
use winsys::event::ToggleAction;
use winsys::input::EventTarget;
use winsys::input::KeyCode;
use winsys::input::MouseEvent;
use winsys::input::MouseEventKey;
use winsys::input::MouseEventKind;
use winsys::screen::Screen;

use std::collections::HashMap;

pub struct Model<'a> {
    conn: &'a mut dyn Connection,
    stack: StackManager,
    stacking_order: Vec<Window>,
    zone_manager: ZoneManager,
    pid_map: HashMap<Pid, Window>,
    client_map: HashMap<Window, Client>,
    window_map: HashMap<Window, Window>,
    frame_map: HashMap<Window, Window>,
    sticky_clients: Vec<Window>,
    unmanaged_windows: Vec<Window>,
    fullscreen_regions: HashMap<Window, Region>,
    partitions: Cycle<Partition>,
    workspaces: Cycle<Workspace>,
    move_buffer: Buffer,
    resize_buffer: Buffer,
    prev_partition: Index,
    prev_workspace: Index,
    running: bool,
    focus: Option<Window>,
    jumped_from: Option<Window>,
}

impl<'a> Model<'a> {
    pub fn new(
        conn: &'a mut dyn Connection,
        key_bindings: &KeyBindings,
        mouse_bindings: &MouseBindings,
    ) -> Self {
        let move_handle = conn.create_handle();
        let resize_handle = conn.create_handle();

        Self::init(
            Self {
                conn,
                stack: StackManager::new(),
                stacking_order: Vec::new(),
                zone_manager: ZoneManager::new(),
                pid_map: HashMap::new(),
                client_map: HashMap::new(),
                window_map: HashMap::new(),
                frame_map: HashMap::new(),
                sticky_clients: Vec::new(),
                unmanaged_windows: Vec::new(),
                fullscreen_regions: HashMap::new(),
                partitions: Cycle::new(Vec::new(), false),
                workspaces: Cycle::new(Vec::with_capacity(10), false),
                move_buffer: Buffer::new(BufferKind::Move, move_handle),
                resize_buffer: Buffer::new(BufferKind::Resize, resize_handle),
                prev_partition: 0,
                prev_workspace: 0,
                running: true,
                focus: None,
                jumped_from: None,
            },
            key_bindings,
            mouse_bindings,
        )
    }

    fn init(
        mut model: Self,
        key_bindings: &KeyBindings,
        mouse_bindings: &MouseBindings,
    ) -> Self {
        info!("initializing window manager");
        model.acquire_partitions();

        let workspaces = ["main", "web", "term", "4", "5", "6", "7", "8", "9", "10"];

        for (i, &workspace_name) in workspaces.iter().enumerate() {
            let region = model
                .partitions
                .active_element()
                .unwrap()
                .screen()
                .placeable_region();

            let id = model.zone_manager.new_zone(
                None,
                ZoneContent::Layout(Layout::new(), Cycle::new(Vec::new(), true)),
            );

            model
                .workspaces
                .push_back(Workspace::new(workspace_name, i as u32, id));

            model.zone_manager.zone_mut(id).set_region(region);
        }

        model.workspaces.activate_for(&Selector::AtIndex(0));

        model.conn.init_wm_properties(WM_NAME!(), &workspaces);
        model.conn.set_current_desktop(0);

        model.conn.grab_bindings(
            &key_bindings
                .keys()
                .cloned()
                .collect::<Vec<winsys::input::KeyCode>>(),
            &mouse_bindings
                .keys()
                .into_iter()
                .collect::<Vec<&(winsys::input::MouseEventKey, winsys::input::MouseShortcut)>>(),
        );

        model
            .conn
            .top_level_windows()
            .into_iter()
            .for_each(|window| {
                if model.conn.must_manage_window(window) {
                    model.manage(window, false);
                }
            });

        if cfg!(not(debug_assertions)) {
            let nonblocking = concat!("$HOME/.config/", WM_NAME!(), "/nonblocking_autostart &");
            let blocking = concat!("$HOME/.config/", WM_NAME!(), "/blocking_autostart");

            info!("executing startup scripts");
            Util::spawn_shell(nonblocking);
            Util::spawn_shell(blocking);
        }

        model
    }

    pub fn run(
        &mut self,
        mut key_bindings: KeyBindings,
        mut mouse_bindings: MouseBindings,
    ) {
        while self.running {
            if let Some(event) = self.conn.step() {
                trace!("received event: {:?}", event);

                match event {
                    Event::Mouse {
                        event,
                    } => self.handle_mouse(event, &mut mouse_bindings),
                    Event::Key {
                        key_code,
                    } => self.handle_key(key_code, &mut key_bindings),
                    Event::MapRequest {
                        window,
                        ignore,
                    } => self.handle_map_request(window, ignore),
                    Event::Map {
                        window,
                        ignore,
                    } => self.handle_map(window, ignore),
                    Event::Enter {
                        window,
                        root_rpos,
                        window_rpos,
                    } => self.handle_enter(window, root_rpos, window_rpos),
                    Event::Leave {
                        window,
                        root_rpos,
                        window_rpos,
                    } => self.handle_leave(window, root_rpos, window_rpos),
                    Event::Destroy {
                        window,
                    } => self.handle_destroy(window),
                    Event::Expose {
                        window,
                    } => self.handle_expose(window),
                    Event::Unmap {
                        window,
                        ignore,
                    } => self.handle_unmap(window, ignore),
                    Event::Configure {
                        window,
                        region,
                        on_root,
                    } => self.handle_configure(window, region, on_root),
                    Event::StateRequest {
                        window,
                        state,
                        action,
                        on_root,
                    } => self.handle_state_request(window, state, action, on_root),
                    Event::FocusRequest {
                        window,
                        on_root,
                    } => self.handle_focus_request(window, on_root),
                    Event::CloseRequest {
                        window,
                        on_root,
                    } => self.handle_close_request(window, on_root),
                    Event::WorkspaceRequest {
                        window,
                        index,
                        on_root,
                    } => self.handle_workspace_request(window, index, on_root),
                    Event::PlacementRequest {
                        window,
                        pos,
                        dim,
                        on_root,
                    } => self.handle_placement_request(window, pos, dim, on_root),
                    Event::GripRequest {
                        window,
                        pos,
                        grip,
                        on_root,
                    } => self.handle_grip_request(window, pos, grip, on_root),
                    Event::RestackRequest {
                        window,
                        sibling,
                        mode,
                        on_root,
                    } => self.handle_restack_request(window, sibling, mode, on_root),
                    Event::Property {
                        window,
                        kind,
                        on_root,
                    } => self.handle_property(window, kind, on_root),
                    Event::FrameExtentsRequest {
                        window,
                        on_root,
                    } => self.handle_frame_extents_request(window, on_root),
                    Event::Mapping {
                        request,
                    } => self.handle_mapping(request),
                    Event::ScreenChange => self.handle_screen_change(),
                    Event::Randr => self.handle_randr(),
                }
            }

            self.conn.flush();
        }
    }

    fn acquire_partitions(&mut self) {
        info!("acquiring partitions");

        let partitions: Vec<Partition> = self
            .conn
            .connected_outputs()
            .into_iter()
            .enumerate()
            .map(|(i, mut s)| {
                s.compute_placeable_region();
                Partition::new(s, i)
            })
            .collect();

        if partitions == self.partitions.as_vec() {
            return;
        }

        self.partitions = Cycle::new(partitions, false);
    }

    fn apply_layout(
        &mut self,
        index: Index,
        must_apply_stack: bool,
    ) {
        info!("applying layout on workspace {}", index);

        if index != self.active_workspace() {
            return;
        }

        let workspace = match self.workspaces.get(index) {
            Some(workspace) => workspace,
            None => return,
        };

        // TODO: zone change
        let region = self.active_screen().placeable_region();

        let placements =
            workspace.arrange(&mut self.zone_manager, &self.client_map, region, |client| {
                !Self::is_applyable(client)
            });

        let (show, hide): (Vec<&Placement>, Vec<&Placement>) = placements
            .iter()
            .partition(|&placement| placement.region != PlacementRegion::NoRegion);

        for placement in show {
            match placement.kind {
                PlacementKind::Client(window) => {
                    let frame = self.frame(window).unwrap();

                    self.update_client_placement(&placement);
                    self.place_client(window, placement.method);
                    self.map_client(frame);
                },
                PlacementKind::Tab(_) => {},
                PlacementKind::Layout => {},
            };
        }

        for placement in hide {
            match placement.kind {
                PlacementKind::Client(window) => {
                    let frame = self.frame(window).unwrap();
                    self.unmap_client(frame);
                },
                PlacementKind::Tab(_) => {},
                PlacementKind::Layout => {},
            };
        }

        if must_apply_stack {
            self.apply_stack(index);
        }
    }

    fn apply_stack(
        &mut self,
        index: Index,
    ) {
        info!("applying stack on workspace {}", index);

        let workspace = match self.workspaces.get(index) {
            Some(workspace) => workspace,
            None => return,
        };

        let desktop = self.stack.layer_windows(StackLayer::Desktop);
        let below = self.stack.layer_windows(StackLayer::Below);
        let dock = self.stack.layer_windows(StackLayer::Dock);

        let stack: Vec<Window> = workspace
            .stack_after_focus()
            .into_iter()
            .map(|window| self.frame(window).unwrap())
            .collect();

        let (regular, fullscreen): (Vec<Window>, Vec<Window>) =
            stack.iter().partition(|&&window| {
                self.client(window).map_or(false, |client| {
                    !client.is_fullscreen() || client.is_in_window()
                })
            });

        let (free, regular): (Vec<Window>, Vec<Window>) =
            regular.into_iter().partition(|&window| {
                self.client(window).map_or(true, |client| {
                    self.is_free(client)
                })
            });

        let above = self.stack.layer_windows(StackLayer::Above);
        let notification = self.stack.layer_windows(StackLayer::Notification);

        let mut windows: Vec<Window> = desktop
            .into_iter()
            .chain(below.into_iter())
            .chain(dock.into_iter())
            .chain(regular.into_iter())
            .chain(fullscreen.into_iter())
            .chain(free.into_iter())
            .chain(above.into_iter())
            .chain(notification)
            .into_iter()
            .collect();

        {
            // handle above-other relationships
            for &window in self.stack.above_other().keys() {
                let index = windows.iter().position(|&w| w == window);

                if let Some(index) = index {
                    windows.remove(index);
                }
            }

            for (&window, &sibling) in self.stack.above_other() {
                let index = windows.iter().position(|&w| w == sibling);

                if let Some(index) = index {
                    if index < windows.len() {
                        windows.insert(index + 1, window);
                    }
                }
            }
        }

        {
            // handle below-other relationships
            for &window in self.stack.below_other().keys() {
                let index = windows.iter().position(|&w| w == window);

                if let Some(index) = index {
                    windows.remove(index);
                }
            }

            for (&window, &sibling) in self.stack.below_other() {
                let index = windows.iter().position(|&w| w == sibling);

                if let Some(index) = index {
                    windows.insert(index, window);
                }
            }
        }

        let mut stack_walk = windows.iter();
        let mut order_changed = false;
        let mut prev_window = stack_walk.next().cloned();

        for (i, &window) in stack_walk.enumerate() {
            order_changed |= self.stacking_order.get(i + 1) != Some(&window);

            if order_changed {
                self.conn.stack_window_above(window, prev_window);
            }

            prev_window = Some(window);
        }

        self.stacking_order = windows;

        if !order_changed {
            return;
        }

        let mut client_list: Vec<&Client> = self.client_map.values().collect::<Vec<&Client>>();
        client_list.sort_by_key(|&a| a.managed_since());

        let client_list: Vec<Window> = client_list
            .into_iter()
            .map(|client| client.window())
            .collect();
        self.conn.update_client_list(&client_list);

        let stack_windows: Vec<Window> = stack
            .into_iter()
            .map(|window| self.window(window).unwrap())
            .collect();

        let mut client_list_stacking = client_list;
        client_list_stacking.retain(|&window| !stack_windows.contains(&window));
        client_list_stacking = client_list_stacking
            .iter()
            .chain(stack_windows.iter())
            .copied()
            .collect();

        self.conn.update_client_list_stacking(&client_list_stacking);
    }

    fn window(
        &self,
        window: Window,
    ) -> Option<Window> {
        if self.window_map.contains_key(&window) {
            return Some(window);
        }

        Some(*self.frame_map.get(&window)?)
    }

    fn frame(
        &self,
        window: Window,
    ) -> Option<Window> {
        if self.frame_map.contains_key(&window) {
            return Some(window);
        }

        Some(*self.window_map.get(&window)?)
    }

    fn client_any(
        &self,
        mut window: Window,
    ) -> Option<&Client> {
        if let Some(inside) = self.frame_map.get(&window) {
            window = *inside;
        }

        self.client_map.get(&window)
    }

    fn client(
        &self,
        window: Window,
    ) -> Option<&Client> {
        self.client_any(window).and_then(|client| {
            if client.is_managed() {
                Some(client)
            } else {
                None
            }
        })
    }

    fn client_any_mut(
        &mut self,
        mut window: Window,
    ) -> Option<&mut Client> {
        if let Some(inside) = self.frame_map.get(&window) {
            window = *inside;
        }

        self.client_map.get_mut(&window)
    }

    fn client_mut(
        &mut self,
        window: Window,
    ) -> Option<&mut Client> {
        self.client_any_mut(window).and_then(|client| {
            if client.is_managed() {
                Some(client)
            } else {
                None
            }
        })
    }

    fn workspace(
        &self,
        index: Index,
    ) -> &Workspace {
        self.workspaces.get(index).unwrap()
    }

    fn workspace_mut(
        &mut self,
        index: Index,
    ) -> &mut Workspace {
        self.workspaces.get_mut(index).unwrap()
    }

    fn detect_rules(
        &self,
        instance: &str,
    ) -> Rules {
        const PREFIX: &str = &concat!(WM_NAME!(), ":");
        const PREFIX_LEN: usize = PREFIX.len();

        let mut rules: Rules = Default::default();

        match (instance.get(..PREFIX_LEN), instance.get(PREFIX_LEN..)) {
            (Some(PREFIX), flags) => {
                if let Some(flags) = flags {
                    let mut invert = false;

                    for i in 0..flags.len() {
                        let flag = &flags[i..=i];

                        match flag {
                            "!" => {
                                invert = true;
                                continue;
                            },
                            "f" => rules.float = Some(!invert),
                            "c" => rules.center = Some(!invert),
                            _ => {},
                        }

                        invert = false;
                    }
                }
            },
            _ => {},
        }

        rules
    }

    fn manage(
        &mut self,
        window: Window,
        ignore: bool,
    ) {
        if ignore {
            if self.conn.window_is_mappable(window) {
                self.conn.map_window(window);
            }

            self.conn.init_unmanaged(window);
            self.unmanaged_windows.push(window);

            return;
        }

        let pid = self.conn.get_window_pid(window);

        let ppid = pid.and_then(|pid| {
            get_spawner_pid(
                pid,
                std::process::id() as Pid,
                &self.pid_map,
                &self.client_map,
            )
        });

        let name = self.conn.get_icccm_window_name(window);
        let class = self.conn.get_icccm_window_class(window);
        let instance = self.conn.get_icccm_window_instance(window);

        let preferred_state = self.conn.get_window_preferred_state(window);
        let preferred_type = self.conn.get_window_preferred_type(window);

        let geometry = self.conn.get_window_geometry(window);

        if geometry.is_err() {
            return;
        }

        self.stop_moving();
        self.stop_resizing();

        let original_geometry = geometry.unwrap();
        let mut geometry = original_geometry;

        let frame = self.conn.create_frame(geometry);
        let rules = self.detect_rules(&instance);
        let hints = self.conn.get_icccm_window_hints(window);
        let (_, size_hints) =
            self.conn
                .get_icccm_window_size_hints(window, Some(MIN_WINDOW_DIM), &None);

        geometry = if size_hints.is_some() {
            geometry
                .with_size_hints(&size_hints)
                .with_extents(&FREE_DECORATION.extents())
        } else {
            geometry
                .with_minimum_dim(&MIN_WINDOW_DIM)
                .with_extents(&FREE_DECORATION.extents())
        };

        let parent = self.conn.get_icccm_window_transient_for(window);

        let leader = self
            .conn
            .get_icccm_window_client_leader(window)
            .and_then(|leader| self.client_any(leader));

        // TODO: startup sequence/notification
        // TODO: MOTIF decorations for old-style applications

        let context = 0;
        let workspace = self
            .conn
            .get_window_desktop(window)
            .map_or(self.active_workspace(), |d| {
                if d < self.workspaces.len() {
                    d
                } else {
                    self.active_workspace()
                }
            });

        let mut center = false;

        // TODO: retrieve screen of new client's workspace
        let screen = self.active_screen();

        if rules.center(&mut center)
            || (size_hints.is_none() || !size_hints.unwrap().by_user)
                && original_geometry.pos
                    == (Pos {
                        x: 0,
                        y: 0,
                    })
        {
            geometry = screen
                .full_region()
                .from_absolute_inner_center(&geometry.dim);
        }

        let mut client = Client::new(
            window,
            frame,
            name,
            class,
            instance,
            preferred_type,
            pid,
            ppid,
        );

        let fullscreen = self.conn.window_is_fullscreen(window);
        let sticky = self.conn.window_is_sticky(window);
        let mut floating = self.conn.must_free_window(window);

        if let Some(parent) = parent {
            floating = true;
            client.set_parent(parent);
        }

        if let Some(leader) = leader {
            let leader_window = leader.window();
            if leader_window != window {
                floating = true;
                client.set_leader(leader_window);
            }
        }

        if let Some(hints) = hints {
            client.set_urgent(hints.urgent);
        }

        rules.float(&mut floating);

        client.set_floating(floating);
        client.set_free_region(&geometry);
        client.set_size_hints(size_hints);
        client.set_context(context);
        client.set_workspace(workspace);

        let extents = FREE_DECORATION.extents();
        self.conn.reparent_window(window, frame, Pos {
            x: extents.left as i32,
            y: extents.top as i32,
        });

        self.conn
            .set_icccm_window_state(window, IcccmWindowState::Normal);

        if let Some(current_workspace) = self.workspaces.get(workspace) {
            let parent_zone = current_workspace
                .active_zone()
                .map(|id| self.zone_manager.nearest_cycle(id));

            let id = self
                .zone_manager
                .new_zone(parent_zone, ZoneContent::Client(window));

            client.set_zone(id);

            let current_workspace = self.workspaces.get_mut(workspace).unwrap();
            current_workspace.add_client(window, &InsertPos::Back);
            current_workspace.add_zone(id, &InsertPos::AfterActive);
        }

        if let Some(parent) = parent {
            if let Some(parent) = self.client_any_mut(parent) {
                let parent_frame = parent.frame();
                parent.add_child(window);
                self.stack.add_above_other(frame, parent_frame);
            }
        }

        if let Some(pid) = pid {
            self.pid_map.insert(pid, window);
        }

        info!("managing client {:#?}", client);

        self.client_map.insert(window, client);
        self.frame_map.insert(frame, window);
        self.window_map.insert(window, frame);

        self.conn.insert_window_in_save_set(window);
        self.conn.init_window(window, false); // TODO config.focus_follows_mouse
        self.conn.init_frame(frame, false); // TODO: config.focus_follows_mouse
        self.conn.set_window_border_width(window, 0);
        self.conn.set_window_desktop(window, workspace);

        self.apply_layout(workspace, false);
        self.focus(window);

        if let Some(ppid) = ppid {
            if let Some(ppid_window) = self.pid_map.get(&ppid) {
                let ppid_window = *ppid_window;
                if let Some(ppid_client) = self.client(ppid_window) {
                    if ppid_client.is_producing() {
                        self.consume_client(window, ppid_window);
                    }
                }
            }
        }

        if let Some(state) = preferred_state {
            match state {
                WindowState::DemandsAttention => {
                    self.handle_state_request(window, state, ToggleAction::Add, false)
                },
                _ => {},
            }
        }

        if sticky {
            self.stick(window);
        }

        if fullscreen {
            self.fullscreen(window);
        }

        let client = self.client_any(window).unwrap();
        let active_region = client.active_region();
        let current_pos = self.conn.get_pointer_position();

        if let Some(warp_pos) = active_region.quadrant_center_from_pos(current_pos) {
            self.conn.warp_pointer(warp_pos);
        }
    }

    fn remanage(
        &mut self,
        window: Window,
        must_alter_workspace: bool,
    ) {
        if let Some(client) = self.client_any(window) {
            if client.is_managed() {
                return;
            }

            info!("remanaging client with window {:#0x}", client.window());

            let window = client.window();
            let active_workspace = self.active_workspace();
            let mut workspace = active_workspace;

            if must_alter_workspace {
                let leader = client.leader();

                if let Some(leader) = leader {
                    if let Some(leader) = self.client(leader) {
                        workspace = leader.workspace();
                    }
                }

                {
                    let workspace = self.workspace_mut(workspace);

                    if !workspace.contains(window) {
                        workspace.add_client(window, &InsertPos::Back);
                    }
                }

                let client = self.client_any_mut(window).unwrap();
                client.set_workspace(workspace);
            }

            let client = self.client_any_mut(window).unwrap();
            client.set_managed(true);

            let client = self.client_any(window).unwrap();
            if client.is_sticky() {
                let client = self.client_any_mut(window).unwrap();
                client.set_sticky(false);

                self.stick(window);
                self.map_client(window);
            }
        }
    }

    fn unmanage(
        &mut self,
        window: Window,
    ) {
        if let Some(client) = self.client(window) {
            info!("unmanaging client with window {:#0x}", client.window());

            if client.is_sticky() {
                self.unstick(window);

                let client = self.client_mut(window).unwrap();
                client.set_sticky(true);
            }

            let client = self.client(window).unwrap();
            let window = client.window();
            let workspace = client.workspace();

            self.unmap_client(window);

            {
                let workspace = self.workspace_mut(workspace);

                if workspace.contains(window) {
                    workspace.remove_client(window);
                }
            }

            let client = self.client_mut(window).unwrap();
            client.set_managed(false);
        }
    }

    fn is_applyable(client: &Client) -> bool {
        !client.is_floating()
            && !client.is_disowned()
            && client.is_managed()
            && (!client.is_fullscreen() || client.is_in_window())
    }

    fn is_free(
        &self,
        client: &Client,
    ) -> bool {
        client.is_floating() && (!client.is_fullscreen() || client.is_in_window()) || {
            let id = client.zone();
            let zone = self.zone_manager.zone(id);

            zone.method() == PlacementMethod::Free
        }
    }

    fn is_focusable(
        &self,
        window: Window,
    ) -> bool {
        self.client(window).map_or(false, |client| {
            !client.is_disowned() && !client.is_iconified()
        })
    }

    fn remove_window(
        &mut self,
        window: Window,
    ) {
        let client = self.client(window);

        if client.is_none() {
            return;
        }

        let client = client.unwrap();
        let (window, frame) = client.windows();
        let parent = client.parent();
        let producer = client.producer();
        let workspace = client.workspace();
        let id = client.zone();

        info!("removing client with window {:#0x}", window);

        if client.is_sticky() {
            self.unstick(window);
        }

        if Some(window) == self.jumped_from {
            self.jumped_from = None;
        }

        if producer.is_some() {
            self.unconsume_client(window);
        }

        if let Some(parent) = parent {
            if let Some(parent) = self.client_mut(parent) {
                parent.remove_child(window);
            }
        }

        self.workspaces.get_mut(workspace).map(|w| {
            w.remove_zone(id);
            w.remove_icon(window);
        });

        self.stack.remove_window(window);
        self.frame_map.remove(&frame);
        self.window_map.remove(&window);
        self.client_map.remove(&window);
        self.pid_map.remove(&window);
        self.fullscreen_regions.remove(&window);

        self.sync_focus();
    }

    fn redraw_client(
        &self,
        window: Window,
    ) {
        if let Some(client) = self.client(window) {
            let decoration = client.decoration();

            if let Some(border) = decoration.border {
                self.conn
                    .set_window_border_width(client.frame(), border.width);

                self.conn.set_window_border_color(
                    client.frame(),
                    if client.is_focused() {
                        if client.is_sticky() {
                            border.colors.fsticky
                        } else {
                            border.colors.focused
                        }
                    } else if client.is_urgent() {
                        border.colors.urgent
                    } else if client.is_sticky() {
                        border.colors.rsticky
                    } else {
                        border.colors.regular
                    },
                );
            }

            if let Some(frame) = decoration.frame {
                self.conn.set_window_background_color(
                    client.frame(),
                    if client.is_focused() {
                        if client.is_sticky() {
                            frame.colors.fsticky
                        } else {
                            frame.colors.focused
                        }
                    } else if client.is_urgent() {
                        frame.colors.urgent
                    } else if client.is_sticky() {
                        frame.colors.rsticky
                    } else {
                        frame.colors.regular
                    },
                );
            }
        }
    }

    fn update_client_placement(
        &mut self,
        placement: &Placement,
    ) {
        match placement.kind {
            PlacementKind::Client(window) => {
                let client = self.client_mut(window).unwrap();
                let region = match placement.region {
                    PlacementRegion::FreeRegion => *client.free_region(),
                    PlacementRegion::NewRegion(region) => region,
                    PlacementRegion::NoRegion => return,
                };

                client.set_decoration(placement.decoration);

                match placement.method {
                    PlacementMethod::Free => {
                        let id = client.zone();
                        client.set_free_region(&region);

                        let zone = self.zone_manager.zone_mut(id);
                        zone.set_region(region);
                        zone.set_method(placement.method);
                    },
                    PlacementMethod::Tile => {
                        let id = client.zone();
                        client.set_tile_region(&region);

                        let zone = self.zone_manager.zone_mut(id);
                        zone.set_method(placement.method);
                    },
                };
            },
            _ => panic!("attempting to update non-client placement"),
        }
    }

    fn place_client(
        &self,
        window: Window,
        method: PlacementMethod,
    ) {
        let client = self.client(window).unwrap();

        let (window, frame) = client.windows();
        let inner_region = client.inner_region();

        self.conn.place_window(window, inner_region);

        self.conn.place_window(frame, match method {
            PlacementMethod::Free => &client.free_region(),
            PlacementMethod::Tile => &client.tile_region(),
        });

        self.redraw_client(window);
        self.conn.update_window_offset(window, frame);
    }

    fn map_client(
        &mut self,
        window: Window,
    ) {
        if let Some(client) = self.client(window) {
            if !client.is_mapped() {
                let (window, frame) = client.windows();

                info!("mapping client with window {:#0x}", window);
                self.conn.map_window(window);
                self.conn.map_window(frame);
                self.redraw_client(window);

                let client = self.client_mut(window).unwrap();
                client.set_mapped(true);
            }
        }
    }

    fn unmap_client(
        &mut self,
        window: Window,
    ) {
        if let Some(client) = self.client(window) {
            if client.is_mapped() {
                let client = self.client_mut(window).unwrap();
                let (window, frame) = client.windows();

                info!("unmapping client with window {:#0x}", window);
                client.set_mapped(false);
                client.expect_unmap();

                self.conn.unmap_window(frame);
            }
        }
    }

    fn consume_client(
        &mut self,
        consumer: Window,
        producer: Window,
    ) {
        let consumer_window = consumer;
        let producer_window = producer;

        let consumer = self.client_any(consumer_window);
        let producer = self.client_any(producer_window);

        if consumer.is_none() || producer.is_none() {
            return;
        }

        info!(
            "consuming client with window {:#0x} and producer window {:#0x}",
            consumer_window, producer_window
        );

        let consumer = consumer.unwrap();
        let producer = producer.unwrap();
        let producer_workspace_index = producer.workspace();

        if producer.is_iconified() || consumer.is_iconified() {
            return;
        }

        let consumer_len = producer.consumer_len();
        let consumer_workspace_index = consumer.workspace();
        let consumer = self.client_any_mut(consumer_window).unwrap();
        consumer.set_consuming(true);
        consumer.set_producer(producer_window);

        if consumer_len == 0 {
            let producer_workspace = self.workspace_mut(producer_workspace_index);

            if producer_workspace_index == consumer_workspace_index {
                producer_workspace.replace_client(producer_window, consumer_window);
            } else {
                producer_workspace.remove_client(producer_window);
            }

            self.apply_layout(consumer_workspace_index, true);
        }

        let producer = self.client_any_mut(producer_window).unwrap();
        producer.add_consumer(consumer_window);
        self.unmanage(producer_window);
    }

    fn unconsume_client(
        &mut self,
        consumer: Window,
    ) {
        let consumer_window = consumer;
        let consumer = self.client_any(consumer_window);

        if consumer.is_none() {
            return;
        }

        let consumer = consumer.unwrap();
        let producer_window = consumer.producer();
        let consumer_workspace = consumer.workspace();

        if producer_window.is_none() {
            return;
        }

        let producer_window = producer_window.unwrap();

        info!(
            "unconsuming client with window {:#0x} and producer window {:#0x}",
            consumer_window, producer_window
        );

        if self.client_map.contains_key(&producer_window) {
            let producer = self.client_any_mut(producer_window).unwrap();
            producer.remove_consumer(consumer_window);
            let consumer_len = producer.consumer_len();

            if consumer_len == 0 {
                producer.set_workspace(consumer_workspace);

                if let Some(workspace) = self.workspaces.get_mut(consumer_workspace) {
                    if workspace.contains(consumer_window) {
                        workspace.replace_client(consumer_window, producer_window);
                    } else {
                        workspace.add_client(producer_window, &InsertPos::Back);
                    }
                }

                self.remanage(producer_window, false);

                if consumer_workspace == self.active_workspace() {
                    self.map_client(producer_window);
                }

                self.apply_layout(consumer_workspace, true);
            }
        }

        let consumer = self.client_any_mut(consumer_window).unwrap();
        consumer.unset_producer();
        consumer.set_consuming(false);
    }

    pub fn kill_focus(&mut self) {
        if let Some(focus) = self.focus {
            self.kill_client(focus);
        }
    }

    pub fn kill_client(
        &mut self,
        mut window: Window,
    ) {
        if let Some(client) = self.client_any(window) {
            window = client.window();

            if client.is_invincible() {
                return;
            }
        } else {
            return;
        }

        info!("killing client with window {:#0x}", window);

        self.conn.kill_window(window);
        self.conn.flush();
    }

    pub fn cycle_focus(
        &mut self,
        dir: Direction,
    ) {
        let workspace = self.active_workspace();
        let client_map = &self.client_map;
        let zone_manager = &self.zone_manager;

        let windows = self
            .workspaces
            .get_mut(workspace)
            .and_then(|ws| ws.cycle_focus(dir, client_map, zone_manager));

        if let Some((_, window)) = windows {
            self.focus(window);
            self.sync_focus();
        }
    }

    pub fn drag_focus(
        &mut self,
        dir: Direction,
    ) {
        if let Some(focus) = self.focus {
            let workspace_index = self.active_workspace();
            self.workspaces
                .get_mut(workspace_index)
                .and_then(|ws| ws.drag_focus(dir));

            self.apply_layout(workspace_index, false);
            self.focus(focus);
        }
    }

    pub fn rotate_clients(
        &mut self,
        dir: Direction,
    ) {
        let workspace_index = self.active_workspace();
        let workspace = self.workspace_mut(workspace_index);
        let next_window = workspace.next_client(dir.rev());

        workspace.rotate_clients(dir);
        self.apply_layout(workspace_index, false);

        if let Some(window) = next_window {
            self.focus(window);
        }
    }

    pub fn center_client(
        &mut self,
        window: Window,
    ) {
        if let Some(client) = self.client(window) {
            if self.is_free(client) {
                let screen = self.partitions.active_element().unwrap().screen();

                let center = screen
                    .full_region()
                    .from_absolute_inner_center(&client.free_region().dim);

                let mut free_region = *client.free_region();
                free_region.pos = center.pos;

                info!("centering client with window {:#0x}", client.window());

                self.conn.move_window(client.frame(), center.pos);
                self.client_mut(window)
                    .unwrap()
                    .set_free_region(&free_region);
            }
        }
    }

    pub fn center_focus(&mut self) {
        if let Some(focus) = self.focus {
            self.center_client(focus);
        }
    }

    pub fn apply_float_retain_region(&mut self) {
        let workspace_index = self.active_workspace();
        let workspace = self.workspace(workspace_index);
        let windows = workspace.iter().copied().collect::<Vec<Window>>();

        windows.into_iter().for_each(|w| {
            let client = self.client(w).unwrap();
            let active_region = *client.active_region();

            let client = self.client_mut(w).unwrap();
            client.set_free_region(&active_region);
        });

        self.set_layout(LayoutKind::Float);
        self.apply_layout(workspace_index, false);
    }

    pub fn move_focus_to_next_workspace(&mut self) {
        if let Some(focus) = self.focus {
            self.move_client_to_next_workspace(focus);
        }
    }

    pub fn move_focus_to_prev_workspace(&mut self) {
        if let Some(focus) = self.focus {
            self.move_client_to_prev_workspace(focus);
        }
    }

    pub fn move_focus_to_workspace(
        &mut self,
        index: Index,
    ) {
        if let Some(focus) = self.focus {
            self.move_client_to_workspace(focus, index);
        }
    }

    pub fn move_client_to_next_workspace(
        &mut self,
        window: Window,
    ) {
        let index = self.active_workspace() + 1;
        let index = index % self.workspaces.len();

        self.move_client_to_workspace(window, index);
    }

    pub fn move_client_to_prev_workspace(
        &mut self,
        window: Window,
    ) {
        let index = if self.active_workspace() == 0 {
            self.workspaces.len() - 1
        } else {
            self.active_workspace() - 1
        };

        self.move_client_to_workspace(window, index);
    }

    fn move_client_to_workspace(
        &mut self,
        window: Window,
        index: Index,
    ) {
        if index == self.active_workspace() || index >= self.workspaces.len() {
            return;
        }

        let (window, current_index) = match self.client(window) {
            Some(client) => {
                if client.is_sticky() {
                    return;
                } else {
                    (client.window(), client.workspace())
                }
            },
            _ => return,
        };

        info!(
            "moving client with window {:#0x} to workspace {}",
            window, index
        );

        // add client to requested workspace
        let workspace = self.workspace_mut(index);
        workspace.add_client(window, &InsertPos::Back);

        // remove client from current_index workspace
        let workspace = self.workspace_mut(current_index);
        workspace.remove_client(window);
        self.unmap_client(window);
        self.apply_layout(current_index, true);
        self.sync_focus();

        let client = self.client_mut(window).unwrap();
        client.set_workspace(index);
    }

    pub fn toggle_screen_struts(&mut self) {
        let screen = self.active_screen_mut();

        if screen.showing_struts() {
            let struts = screen.hide_and_yield_struts();

            for strut in struts {
                self.conn.unmap_window(strut);
            }
        } else {
            let struts = screen.show_and_yield_struts();

            for strut in struts {
                self.conn.map_window(strut);
            }
        }

        // TODO: apply layout to workspace active on screen
        let workspace_index = self.active_workspace();
        self.apply_layout(workspace_index, false);
    }

    pub fn toggle_workspace(&mut self) {
        self.activate_workspace(self.prev_workspace);
    }

    pub fn activate_next_workspace(&mut self) {
        let index = self.active_workspace() + 1;
        let index = index % self.workspaces.len();

        self.activate_workspace(index);
    }

    pub fn activate_prev_workspace(&mut self) {
        let index = if self.active_workspace() == 0 {
            self.workspaces.len() - 1
        } else {
            self.active_workspace() - 1
        };

        self.activate_workspace(index);
    }

    pub fn activate_workspace(
        &mut self,
        index: Index,
    ) {
        if index == self.active_workspace() || index >= self.workspaces.len() {
            return;
        }

        info!("activating workspace {}", index);

        self.stop_moving();
        self.stop_resizing();

        self.prev_workspace = self.workspaces.active_index();
        let mut clients_to_map = Vec::with_capacity(20);
        let mut windows_to_unmap = Vec::with_capacity(20);

        let workspace_index = self.active_workspace();
        let workspace = self.workspace(workspace_index);

        workspace
            .iter()
            .map(|&window| self.client(window).unwrap())
            .for_each(|client| {
                if client.is_mapped() && !client.is_sticky() {
                    windows_to_unmap.push(client.window());
                }
            });

        self.workspaces.activate_for(&Selector::AtIndex(index));

        let workspace_index = self.active_workspace();
        let workspace = self.workspace(workspace_index);

        workspace
            .iter()
            .map(|&window| self.client(window).unwrap())
            .for_each(|client| {
                if !client.is_mapped() {
                    clients_to_map.push(client.window());
                }
            });

        clients_to_map
            .into_iter()
            .for_each(|window| self.map_client(window));

        windows_to_unmap
            .into_iter()
            .for_each(|window| self.unmap_client(window));

        let sticky_windows = self.sticky_clients.iter().copied().collect::<Vec<_>>();
        sticky_windows.into_iter().for_each(|window| {
            if let Some(client) = self.client_mut(window) {
                client.set_workspace(index);
            }
        });

        self.apply_layout(self.active_workspace(), true);
        self.sync_focus();
        self.conn.set_current_desktop(index);
    }

    pub fn change_gap_size(
        &mut self,
        change: Change,
    ) -> Result<(), StateChangeError> {
        let workspace_index = self.active_workspace();

        if let Some(workspace) = self.workspaces.get(workspace_index) {
            workspace.change_gap_size(change, 5, &mut self.zone_manager)?;
        }

        self.apply_layout(workspace_index, true);
        Ok(())
    }

    pub fn reset_layout(&mut self) -> Result<(), StateChangeError> {
        let workspace_index = self.active_workspace();

        if let Some(workspace) = self.workspaces.get(workspace_index) {
            workspace.reset_layout(&mut self.zone_manager)?;
        }

        self.apply_layout(workspace_index, true);
        Ok(())
    }

    pub fn reset_gap_size(&mut self) -> Result<(), StateChangeError> {
        let workspace_index = self.active_workspace();

        if let Some(workspace) = self.workspaces.get(workspace_index) {
            workspace.reset_gap_size(&mut self.zone_manager)?;
        }

        self.apply_layout(workspace_index, true);
        Ok(())
    }

    pub fn change_main_count(
        &mut self,
        change: Change,
    ) -> Result<(), StateChangeError> {
        let workspace_index = self.active_workspace();

        if let Some(workspace) = self.workspaces.get(workspace_index) {
            workspace.change_main_count(change, &mut self.zone_manager)?;
        }

        self.apply_layout(workspace_index, true);
        Ok(())
    }

    pub fn change_main_factor(
        &mut self,
        change: Change,
    ) -> Result<(), StateChangeError> {
        let workspace_index = self.active_workspace();

        if let Some(workspace) = self.workspaces.get(workspace_index) {
            workspace.change_main_factor(change, 0.05f32, &mut self.zone_manager)?;
        }

        self.apply_layout(workspace_index, true);
        Ok(())
    }

    pub fn change_margin(
        &mut self,
        edge: Edge,
        change: Change,
    ) -> Result<(), StateChangeError> {
        let workspace_index = self.active_workspace();

        if let Some(workspace) = self.workspaces.get(workspace_index) {
            workspace.change_margin(edge, change, 5, &mut self.zone_manager)?;
        }

        self.apply_layout(workspace_index, true);
        Ok(())
    }

    pub fn reset_margin(&mut self) -> Result<(), StateChangeError> {
        let workspace_index = self.active_workspace();

        if let Some(workspace) = self.workspaces.get(workspace_index) {
            workspace.reset_margin(&mut self.zone_manager)?;
        }

        self.apply_layout(workspace_index, true);
        Ok(())
    }

    pub fn set_layout(
        &mut self,
        kind: LayoutKind,
    ) {
        let workspace_index = self.active_workspace();
        let workspace = self.workspace_mut(workspace_index);

        if let Some(id) = workspace.active_zone() {
            let cycle = self.zone_manager.nearest_cycle(id);
            let cycle = self.zone_manager.zone_mut(cycle);

            info!(
                "activating layout {:?} on workspace {}",
                kind, workspace_index
            );

            cycle.set_kind(kind);
            self.apply_layout(workspace_index, true);
        }
    }

    pub fn toggle_layout(&mut self) {
        let workspace_index = self.active_workspace();
        let workspace = self.workspace_mut(workspace_index);

        if let Some(id) = workspace.active_zone() {
            let cycle = self.zone_manager.nearest_cycle(id);
            let cycle = self.zone_manager.zone_mut(cycle);
            let prev_kind = cycle.get_prev_kind();

            info!(
                "activating layout {:?} on workspace {}",
                prev_kind, workspace_index
            );

            cycle.set_kind(prev_kind);
            self.apply_layout(workspace_index, true);
        }
    }

    pub fn toggle_in_window_focus(&mut self) {
        if let Some(focus) = self.focus {
            if let Some(client) = self.client_mut(focus) {
                let must_in_window = !client.is_in_window();
                client.set_in_window(must_in_window);

                if must_in_window {
                    self.unfullscreen(focus);
                } else {
                    self.fullscreen(focus);
                }
            }
        }
    }

    pub fn toggle_invincible_focus(&mut self) {
        if let Some(focus) = self.focus {
            if let Some(client) = self.client_mut(focus) {
                let must_invincible = !client.is_invincible();
                client.set_invincible(must_invincible);
            }
        }
    }

    pub fn toggle_producing_focus(&mut self) {
        if let Some(focus) = self.focus {
            if let Some(client) = self.client_mut(focus) {
                let must_producing = !client.is_producing();
                client.set_producing(must_producing);
            }
        }
    }

    pub fn toggle_float_focus(&mut self) {
        if let Some(focus) = self.focus {
            self.toggle_float_client(focus);
        }
    }

    pub fn toggle_float_client(
        &mut self,
        window: Window,
    ) {
        if let Some(client) = self.client(window) {
            let active_workspace_index = client.workspace();
            let workspace_index = client.workspace();

            let client = self.client_mut(window).unwrap();
            let must_float = !client.is_floating();

            info!(
                "{}floating client with window {:#0x}",
                if must_float { "" } else { "un" },
                client.window()
            );

            client.set_floating(must_float);

            if active_workspace_index == workspace_index {
                self.apply_layout(workspace_index, true);
            }
        }
    }

    fn active_partition(&self) -> usize {
        self.partitions.active_index()
    }

    fn active_screen(&self) -> &Screen {
        self.partitions.active_element().unwrap().screen()
    }

    fn active_screen_mut(&mut self) -> &mut Screen {
        self.partitions.active_element_mut().unwrap().screen_mut()
    }

    pub fn active_workspace(&self) -> usize {
        self.workspaces.active_index()
    }

    fn window_workspace(
        &self,
        window: Window,
    ) -> Option<usize> {
        self.client(window).map(|c| c.workspace())
    }

    fn focused_client(&self) -> Option<&Client> {
        self.focus
            .or_else(|| self.workspace(self.active_workspace()).focused_client())
            .and_then(|id| self.client_map.get(&id))
    }

    fn focused_client_mut(&mut self) -> Option<&mut Client> {
        self.focus
            .or_else(|| self.workspace(self.active_workspace()).focused_client())
            .and_then(move |id| self.client_map.get_mut(&id))
    }

    fn focus(
        &mut self,
        window: Window,
    ) {
        if let Some(frame) = self.frame(window) {
            if let Some(window) = self.window(window) {
                if Some(window) == self.focus {
                    return;
                }

                info!("focusing client with window {:#0x}", window);

                let active_workspace_index = self.active_workspace();
                let client = self.client(window);

                if !self.is_focusable(window) {
                    return;
                }

                let client = client.unwrap();
                let client_workspace_index = client.workspace();
                let id = client.zone();

                if client_workspace_index != active_workspace_index {
                    self.activate_workspace(client_workspace_index);
                }

                if let Some(prev_focus) = self.focus {
                    self.unfocus(prev_focus);
                }

                self.conn.ungrab_buttons(frame);

                if let Some(client) = self.client_mut(window) {
                    client.set_focused(true);
                    client.set_urgent(false);
                }

                self.zone_manager.activate_zone(id);

                self.workspaces
                    .get_mut(client_workspace_index)
                    .and_then(|ws| ws.focus_client(window));

                if let Some(config) = self.zone_manager.cycle_config(id) {
                    if config.persistent {
                        self.apply_layout(client_workspace_index, false);
                    }
                }

                if self.conn.get_focused_window() != window {
                    self.conn.focus_window(window);
                }

                self.focus = Some(window);
                self.redraw_client(window);
                self.apply_stack(client_workspace_index);
            }
        }
    }

    fn unfocus(
        &mut self,
        window: Window,
    ) {
        if let Some(client) = self.client(window) {
            let (window, frame) = client.windows();
            let current_pos = self.conn.get_pointer_position();

            info!("unfocusing client with window {:#0x}", window);

            self.conn.regrab_buttons(frame);

            let client = self.client_mut(window).unwrap();
            client.set_warp_pos(current_pos);
            client.set_focused(false);
            self.redraw_client(window);
        }
    }

    fn sync_focus(&mut self) {
        let workspace_index = self.active_workspace();
        let workspace = self.workspace_mut(workspace_index);

        if !workspace.is_empty() {
            if let Some(ws_focus) = workspace.focused_client() {
                if Some(ws_focus) != self.focus {
                    self.focus(ws_focus);
                }
            }
        } else {
            self.conn.unfocus();
            self.focus = None;
        }
    }

    pub fn toggle_fullscreen_focus(&mut self) {
        if let Some(focus) = self.focus {
            self.toggle_fullscreen_client(focus);
        }
    }

    pub fn toggle_fullscreen_client(
        &mut self,
        window: Window,
    ) {
        if let Some(client) = self.client(window) {
            let must_fullscreen = !client.is_fullscreen();

            if must_fullscreen {
                self.fullscreen(window);
            } else {
                self.unfullscreen(window);
            }
        }
    }

    pub fn jump_client(
        &mut self,
        criterium: &JumpCriterium,
    ) {
        let mut window = match criterium {
            JumpCriterium::OnWorkspaceBySelector(index, sel) => {
                let index = *index;

                if index >= self.workspaces.len() {
                    return;
                }

                let workspace = self.workspace(index);
                let window = workspace.get_client_for(sel, &self.zone_manager);

                if window.is_none() {
                    return;
                }

                *window.unwrap()
            },
            JumpCriterium::ByName(name, match_method) => {
                let mut clients = self
                    .client_map
                    .iter()
                    .filter(|&(_, client)| {
                        client.is_managed()
                            && match match_method {
                                MatchMethod::Equals => client.name() == *name,
                                MatchMethod::Contains => client.name().contains(name),
                            }
                    })
                    .map(|(_, client)| client)
                    .collect::<Vec<&Client>>();

                clients.sort_by_key(|&c| c.last_focused());

                if let Some(client) = clients.last() {
                    client.window()
                } else {
                    return;
                }
            },
            JumpCriterium::ByClass(class, match_method) => {
                let mut clients = self
                    .client_map
                    .iter()
                    .filter(|&(_, client)| {
                        client.is_managed()
                            && match match_method {
                                MatchMethod::Equals => client.class() == *class,
                                MatchMethod::Contains => client.class().contains(class),
                            }
                    })
                    .map(|(_, client)| client)
                    .collect::<Vec<&Client>>();

                clients.sort_by_key(|&c| c.last_focused());

                if let Some(client) = clients.last() {
                    client.window()
                } else {
                    return;
                }
            },
            JumpCriterium::ByInstance(instance, match_method) => {
                let mut clients = self
                    .client_map
                    .iter()
                    .filter(|&(_, client)| {
                        client.is_managed()
                            && match match_method {
                                MatchMethod::Equals => client.instance() == *instance,
                                MatchMethod::Contains => client.instance().contains(instance),
                            }
                    })
                    .map(|(_, client)| client)
                    .collect::<Vec<&Client>>();

                clients.sort_by_key(|&c| c.last_focused());

                if let Some(client) = clients.last() {
                    client.window()
                } else {
                    return;
                }
            },
            JumpCriterium::ForCond(cond) => {
                let mut clients = self
                    .client_map
                    .iter()
                    .filter(|&(_, client)| client.is_managed() && cond(client))
                    .map(|(_, client)| client)
                    .collect::<Vec<&Client>>();

                clients.sort_by_key(|&c| c.last_focused());

                if let Some(client) = clients.last() {
                    client.window()
                } else {
                    return;
                }
            },
        };

        if let Some(focus) = self.focus {
            if window == focus {
                let jumped_from = self.jumped_from;

                if jumped_from.is_none() || jumped_from == Some(focus) {
                    return;
                }

                if let Some(jumped_from) = jumped_from {
                    window = jumped_from;
                }
            }

            self.jumped_from = Some(focus);
        }

        info!("jumping to client with window {:#0x}", window);
        self.focus(window);
    }

    fn fullscreen(
        &mut self,
        window: Window,
    ) {
        if let Some(client) = self.client(window) {
            let free_region = *client.free_region();
            let window = client.window();

            info!("enabling fullscreen for client with window {:#0x}", window);

            self.conn
                .set_window_state(window, WindowState::Fullscreen, true);
            self.fullscreen_regions.insert(window, free_region);

            let client = self.client_mut(window).unwrap();
            client.set_fullscreen(true);

            let workspace = client.workspace();
            if workspace == self.active_workspace() {
                self.apply_layout(workspace, true);
            }
        }
    }

    fn unfullscreen(
        &mut self,
        window: Window,
    ) {
        if let Some(client) = self.client(window) {
            let window = client.window();
            let free_region = self.fullscreen_regions.get(&window).map(|&region| region);

            info!("disabling fullscreen for client with window {:#0x}", window);

            self.conn
                .set_window_state(window, WindowState::Fullscreen, false);

            let client = self.client_mut(window).unwrap();
            client.set_fullscreen(false);

            if let Some(free_region) = free_region {
                client.set_free_region(&free_region);
            }

            let workspace = client.workspace();

            if workspace == self.active_workspace() {
                self.apply_layout(workspace, true);
            }

            self.fullscreen_regions.remove(&window);
        }
    }

    pub fn toggle_stick_focus(&mut self) {
        if let Some(focus) = self.focus {
            if let Some(client) = self.client(focus) {
                if client.is_sticky() {
                    self.unstick(focus);
                } else {
                    self.stick(focus);
                }
            }
        }
    }

    fn stick(
        &mut self,
        window: Window,
    ) {
        let client = self.client_mut(window);

        if client.is_none() {
            return;
        }

        let client = client.unwrap();
        let window = client.window();
        let workspace_index = client.workspace();

        info!("enabling sticky for client with window {:#0x}", window);

        client.set_sticky(true);
        self.conn
            .set_window_state(window, WindowState::Sticky, true);
        self.sticky_clients.push(window);

        for workspace in self.workspaces.iter_mut() {
            if workspace.number() as Index != workspace_index {
                workspace.add_client(window, &InsertPos::Back);
            }
        }

        self.redraw_client(window);
    }

    fn unstick(
        &mut self,
        window: Window,
    ) {
        let client = self.client_mut(window);

        if client.is_none() {
            return;
        }

        let client = client.unwrap();
        let window = client.window();
        let workspace_index = client.workspace();

        info!("disabling sticky for client with window {:#0x}", window);

        client.set_sticky(false);
        self.conn
            .set_window_state(window, WindowState::Sticky, false);

        if let Some(index) = self.sticky_clients.iter().position(|&s| s == window) {
            self.sticky_clients.remove(index);
        }

        for workspace in self.workspaces.iter_mut() {
            if workspace.number() as Index != workspace_index {
                workspace.remove_client(window);
                workspace.remove_icon(window);
            }
        }

        self.redraw_client(window);
    }

    pub fn iconify_focus(&mut self) {
        if let Some(focus) = self.focus {
            if let Some(client) = self.client(focus) {
                if !client.is_iconified() {
                    self.iconify(focus);
                }
            }
        }
    }

    pub fn pop_deiconify(&mut self) {
        let workspace_index = self.active_workspace();
        let workspace = self.workspace(workspace_index);

        if let Some(icon) = workspace.focused_icon() {
            self.deiconify(icon);
        }
    }

    pub fn deiconify_all(
        &mut self,
        index: Index,
    ) {
        if index >= self.workspaces.len() {
            warn!("attempting to deicony_all from nonexistent workspace");
            return;
        }

        let mut workspace = self.workspace(index);

        while let Some(icon) = workspace.focused_icon() {
            self.deiconify(icon);
            workspace = self.workspace(index);
        }
    }

    fn iconify(
        &mut self,
        window: Window,
    ) {
        let client = self.client(window);

        if client.is_none() {
            return;
        }

        let client = self.client_mut(window).unwrap();
        let window = client.window();
        let workspace_index = client.workspace();

        info!("iconifying client with window {:#0x}", window);

        client.set_iconified(true);
        self.unmap_client(window);
        self.conn
            .set_icccm_window_state(window, IcccmWindowState::Iconic);

        let workspace = self.workspace_mut(workspace_index);
        workspace.client_to_icon(window);
        self.sync_focus();
        self.apply_layout(workspace_index, true);
    }

    fn deiconify(
        &mut self,
        window: Window,
    ) {
        let client = self.client(window);

        if client.is_none() {
            return;
        }

        let client = self.client_mut(window).unwrap();
        let window = client.window();
        let workspace_index = client.workspace();

        info!("deiconifying client with window {:#0x}", window);

        client.set_iconified(false);
        self.map_client(window);
        self.conn
            .set_icccm_window_state(window, IcccmWindowState::Normal);

        let workspace = self.workspace_mut(workspace_index);
        workspace.icon_to_client(window);
        self.sync_focus();
        self.apply_layout(workspace_index, true);
    }

    pub fn snap_focus(
        &mut self,
        edge: Edge,
    ) {
        if let Some(focus) = self.focus {
            self.snap_client(focus, edge);
        }
    }

    fn snap_client(
        &mut self,
        window: Window,
        edge: Edge,
    ) {
        if let Some(client) = self.client(window) {
            if self.is_free(client) {
                let screen = self.active_screen();
                let placeable_region = screen.placeable_region();
                let mut region = *client.free_region();
                let window = client.window();

                info!(
                    "snapping client with window {:#0x} to edge {:?}",
                    window, edge
                );

                match edge {
                    Edge::Left => region.pos.x = placeable_region.pos.x,
                    Edge::Right => {
                        let x = placeable_region.dim.w as i32 + placeable_region.pos.x;

                        region.pos.x = std::cmp::max(0, x - region.dim.w as i32)
                    },
                    Edge::Top => region.pos.y = placeable_region.pos.y,
                    Edge::Bottom => {
                        let y = placeable_region.dim.h as i32 + placeable_region.pos.y;

                        region.pos.y = std::cmp::max(0, y - region.dim.h as i32)
                    },
                }

                let placement = Placement {
                    method: PlacementMethod::Free,
                    kind: PlacementKind::Client(window),
                    zone: client.zone(),
                    region: PlacementRegion::NewRegion(region),
                    decoration: *client.decoration(),
                };

                self.update_client_placement(&placement);
                self.place_client(window, placement.method);
            }
        }
    }

    pub fn nudge_focus(
        &mut self,
        edge: Edge,
        step: i32,
    ) {
        if let Some(focus) = self.focus {
            self.nudge_client(focus, edge, step);
        }
    }

    fn nudge_client(
        &mut self,
        window: Window,
        edge: Edge,
        step: i32,
    ) {
        if let Some(client) = self.client(window) {
            if self.is_free(client) {
                let mut region = *client.free_region();
                let window = client.window();

                info!(
                    "nudging client with window {:#0x} at the {:?} by {}",
                    window, edge, step
                );

                match edge {
                    Edge::Left => region.pos.x -= step,
                    Edge::Right => region.pos.x += step,
                    Edge::Top => region.pos.y -= step,
                    Edge::Bottom => region.pos.y += step,
                }

                let placement = Placement {
                    method: PlacementMethod::Free,
                    kind: PlacementKind::Client(window),
                    zone: client.zone(),
                    region: PlacementRegion::NewRegion(region),
                    decoration: *client.decoration(),
                };

                self.update_client_placement(&placement);
                self.place_client(window, placement.method);
            }
        }
    }

    pub fn grow_ratio_client(
        &mut self,
        window: Window,
        step: i32,
    ) {
        if let Some(client) = self.client(window) {
            if self.is_free(client) {
                let frame_extents = client.frame_extents();
                let original_region = *client.free_region();
                let region = original_region;
                let window = client.window();
                let (width, height) = region.dim.values();

                let fraction = width as f64 / (width + height) as f64;
                let width_inc = fraction * step as f64;
                let height_inc = step as f64 - width_inc;
                let width_inc = width_inc.round() as i32;
                let height_inc = height_inc.round() as i32;

                let mut region = region.without_extents(&frame_extents);

                if (width_inc.is_negative() && -width_inc >= region.dim.w as i32)
                    || (height_inc.is_negative() && -height_inc >= region.dim.h as i32)
                    || (region.dim.w as i32 + width_inc <= MIN_WINDOW_DIM.w as i32)
                    || (region.dim.h as i32 + height_inc <= MIN_WINDOW_DIM.h as i32)
                {
                    return;
                }

                info!(
                    "{} client with window {:#0x} by {}",
                    if step >= 0 { "growing" } else { "shrinking" },
                    window,
                    step.abs()
                );

                region.dim.w = (region.dim.w as i32 + width_inc) as u32;
                region.dim.h = (region.dim.h as i32 + height_inc) as u32;

                let mut region = region.with_extents(&frame_extents);
                let dx = region.dim.w as i32 - original_region.dim.w as i32;
                let dy = region.dim.h as i32 - original_region.dim.h as i32;

                let width_shift = (dx as f64 / 2f64) as i32;
                let height_shift = (dy as f64 / 2f64) as i32;

                region.pos.x -= width_shift;
                region.pos.y -= height_shift;

                let placement = Placement {
                    method: PlacementMethod::Free,
                    kind: PlacementKind::Client(window),
                    zone: client.zone(),
                    region: PlacementRegion::NewRegion(region),
                    decoration: *client.decoration(),
                };

                self.update_client_placement(&placement);
                self.place_client(window, placement.method);
            }
        }
    }

    pub fn stretch_focus(
        &mut self,
        edge: Edge,
        step: i32,
    ) {
        if let Some(focus) = self.focus {
            self.stretch_client(focus, edge, step);
        }
    }

    fn stretch_client(
        &mut self,
        window: Window,
        edge: Edge,
        step: i32,
    ) {
        if let Some(client) = self.client(window) {
            if self.is_free(client) {
                let frame_extents = client.frame_extents();
                let window = client.window();
                let mut region = (*client.free_region()).without_extents(&frame_extents);

                info!(
                    "stretching client with window {:#0x} at the {:?} by {}",
                    window, edge, step
                );

                match edge {
                    Edge::Left => {
                        if step.is_negative() && -step >= region.dim.w as i32 {
                            return;
                        }

                        if region.dim.w as i32 + step <= MIN_WINDOW_DIM.w as i32 {
                            region.pos.x -= MIN_WINDOW_DIM.w as i32 - region.dim.w as i32;
                            region.dim.w = MIN_WINDOW_DIM.w;
                        } else {
                            region.pos.x -= step;
                            region.dim.w = (region.dim.w as i32 + step) as u32;
                        }
                    },
                    Edge::Right => {
                        if step.is_negative() && -step >= region.dim.w as i32 {
                            return;
                        }

                        if region.dim.w as i32 + step <= MIN_WINDOW_DIM.w as i32 {
                            region.dim.w = MIN_WINDOW_DIM.w;
                        } else {
                            region.dim.w = (region.dim.w as i32 + step) as u32;
                        }
                    },
                    Edge::Top => {
                        if step.is_negative() && -step >= region.dim.h as i32 {
                            return;
                        }

                        if region.dim.h as i32 + step <= MIN_WINDOW_DIM.h as i32 {
                            region.pos.y -= MIN_WINDOW_DIM.h as i32 - region.dim.h as i32;
                            region.dim.h = MIN_WINDOW_DIM.h;
                        } else {
                            region.pos.y -= step;
                            region.dim.h = (region.dim.h as i32 + step) as u32;
                        }
                    },
                    Edge::Bottom => {
                        if step.is_negative() && -step >= region.dim.h as i32 {
                            return;
                        }

                        if region.dim.h as i32 + step <= MIN_WINDOW_DIM.h as i32 {
                            region.dim.h = MIN_WINDOW_DIM.h;
                        } else {
                            region.dim.h = (region.dim.h as i32 + step) as u32;
                        }
                    },
                }

                let window = client.window();
                let region = region.with_extents(&frame_extents);
                let placement = Placement {
                    method: PlacementMethod::Free,
                    kind: PlacementKind::Client(window),
                    zone: client.zone(),
                    region: PlacementRegion::NewRegion(region),
                    decoration: *client.decoration(),
                };

                self.update_client_placement(&placement);
                self.place_client(window, placement.method);
            }
        }
    }

    pub fn start_moving(
        &mut self,
        window: Window,
    ) {
        if !self.move_buffer.is_occupied() && !self.resize_buffer.is_occupied() {
            if let Some(client) = self.client(window) {
                let current_pos = self.conn.get_pointer_position();
                let client_region = *client.free_region();

                self.move_buffer.set(
                    window,
                    Grip::Corner(Corner::TopLeft),
                    current_pos,
                    client_region,
                );

                self.conn.confine_pointer(self.move_buffer.handle());
            }
        }
    }

    pub fn stop_moving(&mut self) {
        if self.move_buffer.is_occupied() {
            self.conn.release_pointer();
            self.move_buffer.unset();
        }
    }

    pub fn handle_move(
        &mut self,
        pos: &Pos,
    ) {
        if let Some(client) = self.client(self.move_buffer.window().unwrap()) {
            if self.is_free(client) {
                if let Some(grip_pos) = self.move_buffer.grip_pos() {
                    if let Some(window_region) = self.move_buffer.window_region() {
                        let window = client.window();
                        let region = Region {
                            pos: window_region.pos + grip_pos.dist(*pos),
                            dim: client.free_region().dim,
                        };

                        let placement = Placement {
                            method: PlacementMethod::Free,
                            kind: PlacementKind::Client(window),
                            zone: client.zone(),
                            region: PlacementRegion::NewRegion(region),
                            decoration: *client.decoration(),
                        };

                        self.update_client_placement(&placement);
                        self.place_client(window, placement.method);
                    }
                }
            }
        }
    }

    pub fn start_resizing(
        &mut self,
        window: Window,
    ) {
        if !self.move_buffer.is_occupied() && !self.resize_buffer.is_occupied() {
            if let Some(client) = self.client(window) {
                let current_pos = self.conn.get_pointer_position();
                let client_region = *client.free_region();
                let corner = client.free_region().nearest_corner(current_pos);

                self.resize_buffer
                    .set(window, Grip::Corner(corner), current_pos, client_region);

                self.conn.confine_pointer(self.resize_buffer.handle());
            }
        }
    }

    pub fn stop_resizing(&mut self) {
        if self.resize_buffer.is_occupied() {
            self.conn.release_pointer();
            self.resize_buffer.unset();
        }
    }

    pub fn handle_resize(
        &mut self,
        pos: &Pos,
    ) {
        let window = self.resize_buffer.window().unwrap();

        if let Some(client) = self.client(window) {
            if self.is_free(client) {
                let grip_pos = self.resize_buffer.grip_pos().unwrap();
                let window_region = self.resize_buffer.window_region().unwrap();
                let grip = self.resize_buffer.grip().unwrap();

                let current_pos = *pos;
                let previous_region = *client.previous_region();
                let decoration = client.decoration();
                let (pos, mut dim) = client
                    .free_region()
                    .without_extents(&decoration.extents())
                    .values();

                let top_grip = grip.is_top_grip();
                let left_grip = grip.is_left_grip();
                let delta = grip_pos.dist(current_pos);

                let dest_w = if left_grip {
                    window_region.dim.w as i32 - delta.dx
                } else {
                    window_region.dim.w as i32 + delta.dx
                };

                let dest_h = if top_grip {
                    window_region.dim.h as i32 - delta.dy
                } else {
                    window_region.dim.h as i32 + delta.dy
                };

                dim.w = std::cmp::max(0, dest_w) as u32;
                dim.h = std::cmp::max(0, dest_h) as u32;

                if let Some(size_hints) = client.size_hints() {
                    size_hints.apply(&mut dim);
                }

                let mut region = (Region {
                    pos,
                    dim,
                })
                .with_extents(&decoration.extents());

                if top_grip {
                    region.pos.y =
                        window_region.pos.y + (window_region.dim.h as i32 - region.dim.h as i32);
                }

                if left_grip {
                    region.pos.x =
                        window_region.pos.x + (window_region.dim.w as i32 - region.dim.w as i32);
                }

                if region == previous_region {
                    return;
                }

                let window = client.window();
                let placement = Placement {
                    method: PlacementMethod::Free,
                    kind: PlacementKind::Client(window),
                    zone: client.zone(),
                    region: PlacementRegion::NewRegion(region),
                    decoration: *decoration,
                };

                self.update_client_placement(&placement);
                self.place_client(window, placement.method);
            }
        }
    }

    fn handle_mouse(
        &mut self,
        event: MouseEvent,
        mouse_bindings: &mut MouseBindings,
    ) {
        let mut window = event.window;
        let subwindow = event.subwindow;

        match event.kind {
            MouseEventKind::Release => {
                if self.move_buffer.is_occupied() {
                    self.stop_moving();
                    return;
                } else if self.resize_buffer.is_occupied() {
                    self.stop_resizing();
                    return;
                }
            },
            MouseEventKind::Motion => {
                if self.move_buffer.is_occupied() {
                    self.handle_move(&event.root_rpos);
                } else if self.resize_buffer.is_occupied() {
                    self.handle_resize(&event.root_rpos);
                }

                return;
            },
            _ => {},
        }

        {
            // handle global mouse bindings
            let binding = mouse_bindings.get_mut(&(
                MouseEventKey {
                    kind: event.kind,
                    target: EventTarget::Global,
                },
                event.shortcut.clone(),
            ));

            if let Some((action, moves_focus)) = binding {
                action(self, None);

                if *moves_focus {
                    // TODO: config.focus_follows_mouse
                    if let Some(focus) = self.focus {
                        if window != focus {
                            self.focus(window);
                        }
                    }
                }

                return;
            }
        }

        if event.on_root {
            if let Some(subwindow) = subwindow {
                window = subwindow;
            } else {
                // handle root-targeted mouse bindings
                let binding = mouse_bindings.get_mut(&(
                    MouseEventKey {
                        kind: event.kind,
                        target: EventTarget::Root,
                    },
                    event.shortcut.clone(),
                ));

                if let Some((action, _)) = binding {
                    action(self, None);
                }

                return;
            }
        }

        {
            // handle client-targeted mouse bindings
            let binding = mouse_bindings.get_mut(&(
                MouseEventKey {
                    kind: event.kind,
                    target: EventTarget::Client,
                },
                event.shortcut.clone(),
            ));

            if let Some(window) = self.window(window) {
                if let Some((action, moves_focus)) = binding {
                    action(self, Some(window));

                    if *moves_focus {
                        // TODO: config.focus_follows_mouse
                        if let Some(focus) = self.focus {
                            if window != focus {
                                self.focus(window);
                            }
                        }
                    }
                } else {
                    // TODO: config.focus_follows_mouse
                    if event.kind != MouseEventKind::Release {
                        if let Some(focus) = self.focus {
                            if window != focus {
                                self.focus(window);
                            }
                        }
                    }
                }
            }
        }
    }

    fn handle_key(
        &mut self,
        key_code: KeyCode,
        key_bindings: &mut KeyBindings,
    ) {
        if let Some(action) = key_bindings.get_mut(&key_code.clone()) {
            debug!("processing key binding: {:?}", key_code);
            action(self);
        }
    }

    fn handle_map_request(
        &mut self,
        window: Window,
        ignore: bool,
    ) {
        debug!("MAP_REQUEST for window {:#0x}", window);

        if ignore {
            if let Some(struts) = self.conn.get_window_strut(window) {
                let screen = self.active_screen_mut();
                screen.add_struts(struts);

                if !screen.showing_struts() {
                    self.conn.unmap_window(window);
                } else {
                    screen.compute_placeable_region();
                    self.apply_layout(self.active_workspace(), true);
                }
            }

            let preferred_state = self.conn.get_window_preferred_state(window);
            let preferred_type = self.conn.get_window_preferred_type(window);
            let geometry = self.conn.get_window_geometry(window);

            match (preferred_state, preferred_type) {
                (Some(WindowState::Below), _) => Some(StackLayer::Below),
                (_, WindowType::Desktop) => Some(StackLayer::Desktop),
                (_, WindowType::Dock) => {
                    if let Ok(geometry) = geometry {
                        let screen = self.active_screen_mut();

                        if !screen.contains_window(window) {
                            let strut = match (
                                (geometry.pos.x, geometry.pos.y),
                                (geometry.dim.w, geometry.dim.h),
                            ) {
                                ((0, 0), (w, h)) if w == screen.full_region().dim.w => {
                                    Some((Edge::Top, h))
                                },
                                ((0, 0), (w, h)) if h == screen.full_region().dim.h => {
                                    Some((Edge::Left, w))
                                },
                                ((0, 0), (w, h)) if w > h => Some((Edge::Top, h)),
                                ((0, 0), (w, h)) if w < h => Some((Edge::Left, w)),
                                ((_, y), (_, h))
                                    if y == screen.full_region().dim.h as i32 - h as i32 =>
                                {
                                    Some((Edge::Bottom, h))
                                },
                                ((x, _), (w, _))
                                    if x == screen.full_region().dim.w as i32 - w as i32 =>
                                {
                                    Some((Edge::Right, w))
                                },
                                _ => None,
                            };

                            if let Some((edge, width)) = strut {
                                screen.add_strut(edge, window, width);

                                if !screen.showing_struts() {
                                    self.conn.unmap_window(window);
                                } else {
                                    screen.compute_placeable_region();
                                    self.apply_layout(self.active_workspace(), true);
                                }
                            }
                        }
                    }

                    Some(StackLayer::Dock)
                },
                (_, WindowType::Notification) => Some(StackLayer::Notification),
                (Some(WindowState::Above), _) => Some(StackLayer::Above),
                (..) => None,
            }
            .map(|layer| self.stack.add_window(window, layer));

            self.apply_stack(self.active_workspace());
        }

        if self.client_map.contains_key(&window) {
            return;
        }

        self.manage(window, ignore);
    }

    fn handle_map(
        &mut self,
        window: Window,
        _ignore: bool,
    ) {
        debug!("MAP for window {:#0x}", window);
    }

    fn handle_enter(
        &mut self,
        window: Window,
        _root_rpos: Pos,
        _window_rpos: Pos,
    ) {
        debug!("ENTER for window {:#0x}", window);

        if let Some(window) = self.window(window) {
            if let Some(focus) = self.focus {
                if focus != window {
                    self.unfocus(focus);
                }
            }

            self.focus(window);
        }
    }

    fn handle_leave(
        &mut self,
        window: Window,
        _root_rpos: Pos,
        _window_rpos: Pos,
    ) {
        debug!("LEAVE for window {:#0x}", window);
        self.unfocus(window);
    }

    fn handle_destroy(
        &mut self,
        window: Window,
    ) {
        debug!("DESTROY for window {:#0x}", window);

        let active_workspace = self.active_workspace();
        let screen = self.active_screen_mut();

        if screen.has_strut_window(window) {
            screen.remove_window_strut(window);
            screen.compute_placeable_region();
            self.apply_layout(active_workspace, true);
        }

        if let Some(index) = self.unmanaged_windows.iter().position(|&s| s == window) {
            self.unmanaged_windows.remove(index);
        }

        let client = self.client_any(window);

        if client.is_none() {
            return;
        }

        let client = client.unwrap();
        let is_managed = client.is_managed();
        let (window, frame) = client.windows();

        let client = self.client_any_mut(window).unwrap();
        if client.consume_unmap_if_expecting() {
            return;
        }

        if !is_managed {
            self.remanage(window, true);
        }

        let client = self.client_any(window).unwrap();
        let workspace = client.workspace();

        if let Ok(geometry) = self.conn.get_window_geometry(frame) {
            self.conn.unparent_window(window, geometry.pos);
        }

        self.conn.cleanup_window(window);
        self.conn.destroy_window(frame);

        self.remove_window(window);

        if workspace == active_workspace {
            self.apply_layout(workspace, false);
        }
    }

    fn handle_expose(
        &mut self,
        _window: Window,
    ) {
    }

    fn handle_unmap(
        &mut self,
        window: Window,
        _ignore: bool,
    ) {
        debug!("UNMAP for window {:#0x}", window);

        if self.unmanaged_windows.contains(&window) {
            return;
        }

        self.handle_destroy(window);
    }

    fn handle_configure(
        &mut self,
        window: Window,
        _region: Region,
        on_root: bool,
    ) {
        if on_root {
            debug!("CONFIGURE for window {:#0x}", window);
            self.acquire_partitions();
        }
    }

    fn handle_state_request(
        &mut self,
        window: Window,
        state: WindowState,
        action: ToggleAction,
        on_root: bool,
    ) {
        debug!(
            "STATE_REQUEST for window {:#0x}, with state {:?} and action {:?}",
            window, state, action
        );

        let client = self.client_any(window);

        if client.is_none() {
            return;
        }

        let client = client.unwrap();

        match action {
            ToggleAction::Add => match state {
                WindowState::Fullscreen => self.fullscreen(window),
                WindowState::Sticky => self.stick(window),
                WindowState::DemandsAttention => {
                    let hints = Hints {
                        urgent: true,
                        input: None,
                        initial_state: None,
                        group: None,
                    };

                    self.conn.set_icccm_window_hints(window, hints);

                    if let Some(client) = self.client_any_mut(window) {
                        client.set_urgent(true);
                        self.redraw_client(window);
                    }
                },
                _ => {},
            },
            ToggleAction::Remove => match state {
                WindowState::Fullscreen => self.unfullscreen(window),
                WindowState::Sticky => self.unstick(window),
                WindowState::DemandsAttention => {
                    let hints = Hints {
                        urgent: false,
                        input: None,
                        initial_state: None,
                        group: None,
                    };

                    self.conn.set_icccm_window_hints(window, hints);

                    if let Some(client) = self.client_any_mut(window) {
                        client.set_urgent(false);
                        self.redraw_client(window);
                    }
                },
                _ => {},
            },
            ToggleAction::Toggle => {
                let is_fullscreen = client.is_fullscreen();

                self.handle_state_request(
                    window,
                    state,
                    if is_fullscreen {
                        ToggleAction::Remove
                    } else {
                        ToggleAction::Add
                    },
                    on_root,
                );
            },
        }
    }

    fn handle_focus_request(
        &mut self,
        window: Window,
        on_root: bool,
    ) {
        debug!("FOCUS_REQUEST for window {:#0x}", window);

        if !on_root {
            self.focus(window);
        }
    }

    fn handle_close_request(
        &mut self,
        window: Window,
        on_root: bool,
    ) {
        debug!("CLOSE_REQUEST for window {:#0x}", window);

        if !on_root {
            self.conn.kill_window(window);
        }
    }

    fn handle_workspace_request(
        &mut self,
        _window: Option<Window>,
        index: usize,
        on_root: bool,
    ) {
        debug!("WORKSPACE_REQUEST for workspace {}", index);

        if on_root {
            self.activate_workspace(index);
        }
    }

    fn handle_placement_request(
        &mut self,
        window: Window,
        pos: Option<Pos>,
        dim: Option<Dim>,
        _on_root: bool,
    ) {
        debug!(
            "PLACEMENT_REQUEST for window {:#0x} with pos {:?} and dim {:?}",
            window, pos, dim
        );

        if pos.is_some() || dim.is_some() {
            let event_window = window;

            if let Some(client) = self.client(window) {
                if self.is_free(client) {
                    let window = client.window();
                    let frame_extents = client.frame_extents();

                    let region = if event_window == window {
                        Some(Region {
                            pos: if let Some(pos) = pos {
                                Pos {
                                    x: pos.x - frame_extents.left as i32,
                                    y: pos.y - frame_extents.top as i32,
                                }
                            } else {
                                client.free_region().pos
                            },
                            dim: if let Some(dim) = dim {
                                Dim {
                                    w: dim.w + frame_extents.left + frame_extents.right,
                                    h: dim.h + frame_extents.top + frame_extents.bottom,
                                }
                            } else {
                                client.free_region().dim
                            },
                        })
                    } else {
                        Some(Region {
                            pos: if let Some(pos) = pos {
                                pos
                            } else {
                                client.free_region().pos
                            },
                            dim: if let Some(dim) = dim {
                                dim
                            } else {
                                client.free_region().dim
                            },
                        })
                    }
                    .map(|region| {
                        if client.size_hints().is_some() {
                            region
                                .without_extents(&frame_extents)
                                .with_size_hints(&client.size_hints())
                                .with_extents(&frame_extents)
                        } else {
                            region
                                .without_extents(&frame_extents)
                                .with_minimum_dim(&MIN_WINDOW_DIM)
                                .with_extents(&frame_extents)
                        }
                    });

                    if let Some(region) = region {
                        let placement = Placement {
                            method: PlacementMethod::Free,
                            kind: PlacementKind::Client(window),
                            zone: client.zone(),
                            region: PlacementRegion::NewRegion(region),
                            decoration: *client.decoration(),
                        };

                        self.update_client_placement(&placement);
                        self.place_client(window, placement.method);
                    }
                }
            } else {
                let geometry = self.conn.get_window_geometry(window);

                if let Ok(mut geometry) = geometry {
                    if let Some(pos) = pos {
                        geometry.pos = pos;
                    }

                    if let Some(dim) = dim {
                        geometry.dim = dim;
                    }

                    self.conn.place_window(window, &geometry);
                }
            }
        }
    }

    fn handle_grip_request(
        &mut self,
        window: Window,
        pos: Pos,
        grip: Option<Grip>,
        _on_root: bool,
    ) {
        debug!(
            "GRIP_REQUEST for window {:#0x} with pos {:?} and grip {:?}",
            window, pos, grip
        );

        if let Some(grip) = grip {
            // initiate resize from grip
            self.move_buffer.unset();
            self.resize_buffer.unset();

            if let Some(client) = self.client(window) {
                let current_pos = self.conn.get_pointer_position();
                let client_region = *client.free_region();

                self.resize_buffer
                    .set(window, grip, current_pos, client_region);

                self.conn.confine_pointer(self.resize_buffer.handle());
            }
        } else {
            // initiate move
            self.start_moving(window);
        }
    }

    fn handle_restack_request(
        &mut self,
        window: Window,
        sibling: Window,
        mode: StackMode,
        _on_root: bool,
    ) {
        debug!(
            "RESTACK_REQUEST for window {:#0x} with sibling {:?} and mode {:?}",
            window, sibling, mode
        );

        match mode {
            StackMode::Above => self.stack.add_above_other(window, sibling),
            StackMode::Below => self.stack.add_below_other(window, sibling),
        }

        self.apply_stack(self.active_workspace());
    }

    fn handle_property(
        &mut self,
        window: Window,
        kind: PropertyKind,
        _on_root: bool,
    ) {
        debug!("PROPERTY for window {:#0x} of kind {:?}", window, kind);

        match kind {
            PropertyKind::Name => {
                let name = self.conn.get_icccm_window_name(window);

                if let Some(client) = self.client_any_mut(window) {
                    client.set_name(name);
                }
            },
            PropertyKind::Class => {
                let class = self.conn.get_icccm_window_class(window);
                let instance = self.conn.get_icccm_window_instance(window);

                if let Some(client) = self.client_any_mut(window) {
                    client.set_class(class);
                    client.set_instance(instance);
                }
            },
            PropertyKind::Size => {
                if let Some(client) = self.client_any(window) {
                    let window = client.window();
                    let workspace = client.workspace();
                    let geometry = self.conn.get_window_geometry(window);

                    if geometry.is_err() {
                        return;
                    }

                    let frame_extents = client.frame_extents();
                    let mut geometry = geometry.unwrap();
                    let (_, size_hints) = self.conn.get_icccm_window_size_hints(
                        window,
                        Some(MIN_WINDOW_DIM),
                        &client.size_hints(),
                    );

                    geometry = if size_hints.is_some() {
                        geometry.with_size_hints(&size_hints)
                    } else {
                        geometry.with_minimum_dim(&MIN_WINDOW_DIM)
                    };

                    geometry.pos = client.free_region().pos;
                    geometry.dim.w += frame_extents.left + frame_extents.right;
                    geometry.dim.h += frame_extents.top + frame_extents.bottom;

                    let client = self.client_any_mut(window).unwrap();
                    client.set_size_hints(size_hints);
                    client.set_free_region(&geometry);

                    if client.is_managed() && workspace == self.active_workspace() {
                        self.apply_layout(workspace, true);
                    }
                }
            },
            PropertyKind::Strut => {
                if let Some(struts) = self.conn.get_window_strut(window) {
                    // TODO: screen of window
                    let screen = self.active_screen_mut();

                    screen.remove_window_strut(window);
                    screen.add_struts(struts);
                    screen.compute_placeable_region();

                    self.apply_layout(self.active_workspace(), true);
                }
            },
        }
    }

    fn handle_frame_extents_request(
        &self,
        window: Window,
        _on_root: bool,
    ) {
        debug!("FRAME_EXTENTS_REQUEST for window {:#0x}", window);

        self.conn.set_window_frame_extents(
            window,
            if let Some(client) = self.client_any(window) {
                client.frame_extents()
            } else {
                if self.conn.must_manage_window(window) {
                    FREE_DECORATION.extents()
                } else {
                    NO_DECORATION.extents()
                }
            },
        );
    }

    fn handle_mapping(
        &mut self,
        request: u8,
    ) {
        debug!("MAPPING with request {}", request);
        if self.conn.is_mapping_request(request) {} // TODO
    }

    fn handle_screen_change(&mut self) {
        debug!("SCREEN_CHANGE");

        let workspace = self.partitions.active_element().unwrap().screen().number();
        self.workspaces.activate_for(&Selector::AtIndex(workspace));
    }

    fn handle_randr(&mut self) {
        debug!("RANDR");
        self.acquire_partitions();
    }

    pub fn exit(&mut self) {
        info!("exit called, shutting down window manager");

        for index in 0..self.workspaces.len() {
            self.deiconify_all(index);
        }

        for (window, client) in self.client_map.drain() {
            self.conn.unparent_window(window, client.free_region().pos);
        }

        self.running = false;

        self.conn.cleanup();
        self.conn.flush();
    }
}
