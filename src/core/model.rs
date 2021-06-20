#[allow(unused_imports)]
use crate::util::Util;

use crate::binding::KeyBindings;
use crate::binding::MouseBindings;
use crate::change::Change;
use crate::change::Direction;
use crate::change::Toggle;
use crate::client::Client;
use crate::consume::get_spawner_pid;
use crate::cycle::Cycle;
use crate::cycle::InsertPos;
use crate::cycle::Selector;
use crate::decoration::Decoration;
use crate::defaults;
use crate::error::StateChangeError;
use crate::identify::Ident;
use crate::identify::Index;
use crate::jump::JumpCriterium;
use crate::layout::Layout;
use crate::layout::LayoutKind;
use crate::partition::Partition;
use crate::placement::Placement;
use crate::placement::PlacementClass;
use crate::placement::PlacementMethod;
use crate::placement::PlacementRegion;
use crate::placement::PlacementTarget;
use crate::rule::Rules;
use crate::stack::StackLayer;
use crate::stack::StackManager;
use crate::util::BuildIdHasher;
use crate::workspace::Buffer;
use crate::workspace::BufferKind;
use crate::workspace::Workspace;
use crate::zone::ZoneContent;
use crate::zone::ZoneManager;

use winsys::connection::Connection;
use winsys::connection::Pid;
use winsys::event::Event;
use winsys::event::PropertyKind;
use winsys::event::StackMode;
use winsys::event::ToggleAction;
use winsys::geometry::Corner;
use winsys::geometry::Dim;
use winsys::geometry::Edge;
use winsys::geometry::Pos;
use winsys::geometry::Region;
use winsys::hints::Hints;
use winsys::input::Grip;
use winsys::input::KeyEvent;
use winsys::input::KeyInput;
use winsys::input::MouseEvent;
use winsys::input::MouseEventKind;
use winsys::input::MouseInput;
use winsys::input::MouseInputTarget;
use winsys::screen::Screen;
use winsys::window::IcccmWindowState;
use winsys::window::Window;
use winsys::window::WindowState;
use winsys::window::WindowType;

use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;

pub struct Model<'model> {
    conn: &'model mut dyn Connection,
    zone_manager: ZoneManager,
    stack_manager: StackManager,
    stacking_order: RefCell<Vec<Window>>,
    pid_map: HashMap<Pid, Window>,
    client_map: HashMap<Window, Client, BuildIdHasher>,
    window_map: HashMap<Window, Window, BuildIdHasher>,
    frame_map: HashMap<Window, Window, BuildIdHasher>,
    sticky_clients: RefCell<HashSet<Window, BuildIdHasher>>,
    unmanaged_windows: RefCell<HashSet<Window, BuildIdHasher>>,
    fullscreen_regions: RefCell<HashMap<Window, Region, BuildIdHasher>>,
    partitions: Cycle<Partition>,
    workspaces: Cycle<Workspace>,
    move_buffer: Buffer,
    resize_buffer: Buffer,
    prev_partition: Cell<Index>,
    prev_workspace: Cell<Index>,
    running: bool,
    focus: Cell<Option<Window>>,
    jumped_from: Cell<Option<Window>>,
}

impl<'model> Model<'model> {
    pub fn new(
        conn: &'model mut dyn Connection,
        key_bindings: &KeyBindings,
        mouse_bindings: &MouseBindings,
    ) -> Self {
        Self::init(
            Self {
                zone_manager: ZoneManager::new(),
                stack_manager: StackManager::new(),
                stacking_order: RefCell::new(Vec::with_capacity(200)),
                pid_map: HashMap::new(),
                client_map: HashMap::with_hasher(BuildIdHasher),
                window_map: HashMap::with_hasher(BuildIdHasher),
                frame_map: HashMap::with_hasher(BuildIdHasher),
                sticky_clients: RefCell::new(HashSet::with_hasher(BuildIdHasher)),
                unmanaged_windows: RefCell::new(HashSet::with_hasher(BuildIdHasher)),
                fullscreen_regions: RefCell::new(HashMap::with_hasher(BuildIdHasher)),
                partitions: Cycle::new(Vec::new(), false),
                workspaces: Cycle::new(Vec::with_capacity(defaults::WORKSPACE_NAMES.len()), false),
                move_buffer: Buffer::new(BufferKind::Move, conn.create_handle()),
                resize_buffer: Buffer::new(BufferKind::Resize, conn.create_handle()),
                prev_partition: Cell::new(0),
                prev_workspace: Cell::new(0),
                running: true,
                focus: Cell::new(None),
                jumped_from: Cell::new(None),
                conn,
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
        let screen_region = model
            .partitions
            .active_element()
            .expect("no screen region found")
            .placeable_region();

        defaults::WORKSPACE_NAMES
            .iter()
            .enumerate()
            .for_each(|(i, &workspace_name)| {
                let root_id = model.zone_manager.new_zone(
                    None,
                    ZoneContent::Layout(Layout::new(), Cycle::new(Vec::new(), true)),
                );

                model
                    .workspaces
                    .push_back(Workspace::new(workspace_name, i as Ident, root_id));

                model
                    .zone_manager
                    .zone_mut(root_id)
                    .set_region(screen_region);
            });

        model.workspaces.activate_for(&Selector::AtIndex(0));
        model.conn.set_current_desktop(0);

        model
            .conn
            .init_wm_properties(WM_NAME!(), &defaults::WORKSPACE_NAMES);

        model.conn.grab_bindings(
            &key_bindings.keys().into_iter().collect::<Vec<&KeyInput>>(),
            &mouse_bindings
                .keys()
                .into_iter()
                .collect::<Vec<&MouseInput>>(),
        );

        model
            .conn
            .top_level_windows()
            .into_iter()
            .for_each(|window| {
                model.manage(window, !model.conn.must_manage_window(window));
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

    #[inline(always)]
    fn window(
        &self,
        window: Window,
    ) -> Option<Window> {
        if self.window_map.contains_key(&window) {
            return Some(window);
        }

        self.frame_map.get(&window).map(|window| window.to_owned())
    }

    #[inline(always)]
    fn window_unchecked(
        &self,
        window: Window,
    ) -> Window {
        self.window(window).unwrap()
    }

    #[inline(always)]
    fn frame(
        &self,
        window: Window,
    ) -> Option<Window> {
        if self.frame_map.contains_key(&window) {
            return Some(window);
        }

        self.window_map.get(&window).map(|window| window.to_owned())
    }

    #[inline(always)]
    fn frame_unchecked(
        &self,
        window: Window,
    ) -> Window {
        self.frame(window).unwrap()
    }

    #[inline(always)]
    fn client_any(
        &self,
        mut window: Window,
    ) -> Option<&Client> {
        if let Some(&inside) = self.frame_map.get(&window) {
            window = inside;
        }

        self.client_map.get(&window)
    }

    #[inline(always)]
    fn client_any_unchecked(
        &self,
        mut window: Window,
    ) -> &Client {
        if let Some(&inside) = self.frame_map.get(&window) {
            window = inside;
        }

        &self.client_map[&window]
    }

    #[inline(always)]
    fn client(
        &self,
        window: Window,
    ) -> Option<&Client> {
        self.client_any(window).filter(|client| client.is_managed())
    }

    #[inline(always)]
    fn client_unchecked(
        &self,
        window: Window,
    ) -> &Client {
        self.client_any(window)
            .filter(|&client| client.is_managed())
            .unwrap()
    }

    #[inline(always)]
    fn active_partition(&self) -> usize {
        self.partitions.active_index()
    }

    #[inline(always)]
    fn active_screen(&self) -> &Screen {
        self.partitions.active_element().unwrap().screen()
    }

    #[inline(always)]
    pub fn active_workspace(&self) -> usize {
        self.workspaces.active_index()
    }

    #[inline(always)]
    fn focused_client(&self) -> Option<&Client> {
        self.focus
            .get()
            .or_else(|| self.workspace(self.active_workspace()).focused_client())
            .and_then(|focus| self.client_map.get(&focus))
    }

    #[inline(always)]
    fn workspace(
        &self,
        index: Index,
    ) -> &Workspace {
        &self.workspaces[index]
    }

    fn acquire_partitions(&mut self) {
        let partitions: Vec<Partition> = self
            .conn
            .connected_outputs()
            .into_iter()
            .enumerate()
            .map(|(i, screen)| {
                screen.compute_placeable_region();
                Partition::new(screen, i)
            })
            .collect();

        if partitions.is_empty() {
            error!("no screen resources found, keeping old partitions");
        } else {
            info!("acquired partitions: {:#?}", partitions);
            self.partitions = Cycle::new(partitions, false);
        }
    }

    #[inline]
    pub fn toggle_screen_struts(&self) {
        let screen = self.active_screen();
        let show = !screen.showing_struts();

        screen
            .show_and_yield_struts(show)
            .iter()
            .for_each(|&strut| {
                if show {
                    self.conn.map_window(strut);
                } else {
                    self.conn.unmap_window(strut);
                }
            });

        self.apply_layout(self.active_workspace());
    }

    fn apply_layout(
        &self,
        index: Index,
    ) {
        let workspace = match self.workspaces.get(index) {
            Some(workspace) if index == self.active_workspace() => workspace,
            _ => return,
        };

        info!("applying layout on workspace {}", index);

        let (show, hide): (Vec<Placement>, Vec<Placement>) = workspace
            .arrange(
                &self.zone_manager,
                &self.client_map,
                self.partitions.active_element().unwrap().placeable_region(),
                |client| !Self::is_applyable(client) || client.is_iconified(),
            )
            .into_iter()
            .partition(|placement| placement.region != PlacementRegion::NoRegion);

        show.into_iter().for_each(|placement| {
            match placement.kind {
                PlacementTarget::Client(window) => {
                    let client = &self.client_map[&window];

                    self.update_client_placement(client, &placement);
                    self.place_client(client, placement.method);
                    self.map_client(client);
                },
                PlacementTarget::Tab(_) => {},
                PlacementTarget::Layout => {},
            };
        });

        hide.into_iter().for_each(|placement| {
            match placement.kind {
                PlacementTarget::Client(window) => {
                    self.unmap_client(&self.client_map[&window]);
                },
                PlacementTarget::Tab(_) => {},
                PlacementTarget::Layout => {},
            };
        });
    }

    fn apply_stack(
        &self,
        index: Index,
    ) {
        let workspace = match self.workspaces.get(index) {
            Some(workspace) if index == self.active_workspace() => workspace,
            _ => return,
        };

        info!("applying stack on workspace {}", index);

        let desktop = self.stack_manager.layer_windows(StackLayer::Desktop);
        let below = self.stack_manager.layer_windows(StackLayer::Below);
        let dock = self.stack_manager.layer_windows(StackLayer::Dock);
        let above = self.stack_manager.layer_windows(StackLayer::Above);
        let notification = self.stack_manager.layer_windows(StackLayer::Notification);

        let stack = workspace
            .stack_after_focus()
            .into_iter()
            .map(|window| self.frame_unchecked(window))
            .collect::<Vec<Window>>();

        let (regular, fullscreen): (Vec<_>, Vec<_>) = stack.iter().partition(|&&window| {
            let client = self.client_unchecked(window);
            !client.is_fullscreen() || client.is_contained()
        });

        let (free, regular): (Vec<_>, Vec<_>) = regular
            .into_iter()
            .partition(|&window| self.is_free(self.client_unchecked(window)));

        let mut windows = desktop
            .into_iter()
            .chain(below.into_iter())
            .chain(dock.into_iter())
            .chain(regular.into_iter())
            .chain(fullscreen.into_iter())
            .chain(free.into_iter())
            .chain(above.into_iter())
            .chain(notification)
            .into_iter()
            .collect::<Vec<Window>>();

        // handle {above,below}-other relationships
        self.stack_manager
            .above_other()
            .keys()
            .chain(self.stack_manager.below_other().keys())
            .for_each(|&window| {
                if let Some(index) = windows.iter().position(|&candidate| candidate == window) {
                    windows.remove(index);
                }
            });

        self.stack_manager
            .above_other()
            .iter()
            .for_each(|(&window, &sibling)| {
                if let Some(index) = windows.iter().position(|&candidate| candidate == sibling) {
                    if index < windows.len() {
                        windows.insert(index + 1, window);
                    }
                }
            });

        self.stack_manager
            .below_other()
            .iter()
            .for_each(|(&window, &sibling)| {
                if let Some(index) = windows.iter().position(|&candidate| candidate == sibling) {
                    windows.insert(index, window);
                }
            });

        let mut stack_iter = windows.iter();
        let mut prev_window = stack_iter.next().cloned();
        let mut order_changed = false;
        let stacking_order = self.stacking_order.borrow();

        stack_iter.enumerate().for_each(|(i, &window)| {
            order_changed |= stacking_order.get(i + 1) != Some(&window);

            if order_changed {
                self.conn.stack_window_above(window, prev_window);
            }

            prev_window = Some(window);
        });

        if !order_changed {
            return;
        }

        drop(stacking_order);
        self.stacking_order.replace(windows);

        let mut client_list = self.client_map.values().collect::<Vec<&Client>>();
        client_list.sort_by_key(|&a| a.managed_since());

        let client_list = client_list
            .into_iter()
            .map(|client| client.window())
            .collect::<Vec<Window>>();

        self.conn.update_client_list(&client_list);

        let mut client_list_stacking = client_list;
        let stack_windows = stack
            .into_iter()
            .map(|window| self.window_unchecked(window))
            .collect::<Vec<_>>();

        client_list_stacking.retain(|&window| !stack_windows.contains(&window));
        client_list_stacking = client_list_stacking
            .iter()
            .chain(stack_windows.iter())
            .copied()
            .collect();

        self.conn.update_client_list_stacking(&client_list_stacking);
    }

    #[inline]
    fn detect_rules(
        &self,
        instance: &str,
    ) -> Rules {
        const PREFIX: &str = &concat!(WM_NAME!(), ":");
        const PREFIX_LEN: usize = PREFIX.len();

        let mut rules: Rules = Default::default();

        match (instance.get(..PREFIX_LEN), instance.get(PREFIX_LEN..)) {
            (Some(PREFIX), Some(flags)) if !flags.is_empty() => {
                let mut invert = false;
                let mut workspace = false;

                for i in 0..flags.len() {
                    let flag = &flags[i..=i];

                    match flag {
                        "!" => {
                            invert = true;
                            continue;
                        },
                        "w" => {
                            workspace = true;
                            continue;
                        },
                        number if workspace => {
                            if let Ok(number) = number.parse::<usize>() {
                                if number < self.workspaces.len() {
                                    rules.workspace = Some(number);
                                }
                            }
                        },
                        "f" => rules.float = Some(!invert),
                        "F" => rules.fullscreen = Some(!invert),
                        "c" => rules.center = Some(!invert),
                        _ => {},
                    }

                    invert = false;
                    workspace = false;
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
            self.unmanaged_windows.borrow_mut().insert(window);

            return;
        }

        let pid = self.conn.get_window_pid(window);
        let ppid = pid.and_then(|pid| {
            get_spawner_pid(pid, std::process::id(), &self.pid_map, &self.client_map)
        });

        let name = self.conn.get_icccm_window_name(window);
        let class = self.conn.get_icccm_window_class(window);
        let instance = self.conn.get_icccm_window_instance(window);

        let preferred_state = self.conn.get_window_preferred_state(window);
        let preferred_type = self.conn.get_window_preferred_type(window);

        let mut geometry = match self.conn.get_window_geometry(window) {
            Ok(geometry) => geometry,
            Err(_) => return,
        };

        self.stop_moving();
        self.stop_resizing();

        let at_origin = geometry.pos.is_origin();
        let frame = self.conn.create_frame(geometry);
        let rules = self.detect_rules(&instance);
        let hints = self.conn.get_icccm_window_hints(window);
        let size_hints = self
            .conn
            .get_icccm_window_size_hints(window, Some(Client::MIN_CLIENT_DIM), &None)
            .1;

        geometry = match size_hints {
            Some(size_hints) => geometry
                .with_size_hints(&Some(size_hints))
                .with_extents(Decoration::FREE_DECORATION.extents()),
            None => geometry
                .with_minimum_dim(&Client::MIN_CLIENT_DIM)
                .with_extents(Decoration::FREE_DECORATION.extents()),
        };

        let parent = self.conn.get_icccm_window_transient_for(window);
        let screen = self.active_screen();
        let context = 0;
        let workspace = rules.workspace.unwrap_or_else(|| {
            self.conn
                .get_window_desktop(window)
                .filter(|&workspace| workspace < self.workspaces.len())
                .unwrap_or_else(|| self.active_workspace())
        });

        if rules.center() || size_hints.map_or(true, |size_hints| !size_hints.by_user) && at_origin
        {
            geometry = screen
                .full_region()
                .from_absolute_inner_center(geometry.dim);
        }

        let parent_zone = self.workspaces[workspace]
            .active_spawn_zone()
            .map(|id| self.zone_manager.nearest_cycle(id));

        let zone = self
            .zone_manager
            .new_zone(parent_zone, ZoneContent::Client(window));

        let mut client = Client::new(
            zone,
            window,
            frame,
            name,
            class,
            instance,
            preferred_type,
            pid,
            ppid,
        );

        let mut floating = self.conn.must_free_window(window) | rules.float();
        let fullscreen = self.conn.window_is_fullscreen(window) | rules.fullscreen();
        let sticky = self.conn.window_is_sticky(window);

        if let Some(parent) = parent {
            floating = true;
            client.set_parent(parent);
        }

        let leader = self
            .conn
            .get_icccm_window_client_leader(window)
            .and_then(|leader| self.client_any(leader));

        if let Some(leader) = leader {
            let leader_window = leader.window();

            if leader_window != window {
                floating = true;
                client.set_leader(leader_window);
            }
        }

        if let Some(hints) = hints {
            client.set_urgent(Toggle::from(hints.urgent));
        }

        client.set_floating(Toggle::from(floating));
        client.set_region(PlacementClass::Free(geometry));
        client.set_size_hints(size_hints);
        client.set_context(context);
        client.set_workspace(workspace);

        self.conn.reparent_window(window, frame, {
            let extents = Decoration::FREE_DECORATION.extents();

            Pos {
                x: extents.left,
                y: extents.top,
            }
        });

        if let Some(parent) = parent.and_then(|parent| self.client_any(parent)) {
            let parent_frame = parent.frame();

            parent.add_child(window);
            self.stack_manager.add_above_other(frame, parent_frame);
        }

        if let Some(pid) = pid {
            self.pid_map.insert(pid, window);
        }

        self.workspaces[workspace].add_client(window, &InsertPos::AfterActive);
        self.client_map.insert(window, client);
        self.frame_map.insert(frame, window);
        self.window_map.insert(window, frame);

        self.conn.insert_window_in_save_set(window);
        self.conn.init_window(window, false);
        self.conn.init_frame(frame, false);
        self.conn.set_window_border_width(window, 0);
        self.conn.set_window_desktop(window, workspace);
        self.conn
            .set_icccm_window_state(window, IcccmWindowState::Normal);

        self.apply_layout(workspace);
        self.focus_window(window);

        if let Some(WindowState::DemandsAttention) = preferred_state {
            self.handle_state_request(
                window,
                WindowState::DemandsAttention,
                ToggleAction::Add,
                false,
            );
        }

        let client = &self.client_map[&window];

        if sticky {
            self.stick(client);
        }

        if fullscreen {
            self.fullscreen(client);
        }

        if let Some(&ppid_window) = ppid.and_then(|ppid| self.pid_map.get(&ppid)) {
            if let Some(ppid_client) = self.client(ppid_window) {
                if ppid_client.is_producing() {
                    self.consume_client(client, ppid_client);
                }
            }
        }

        if let Some(warp_pos) = client
            .active_region()
            .quadrant_center_from_pos(self.conn.get_pointer_position())
        {
            self.conn.warp_pointer(warp_pos);
        }

        info!("managing client {:#?}", client);
    }

    fn remanage(
        &self,
        client: &Client,
        must_alter_workspace: bool,
    ) {
        if client.is_managed() {
            return;
        }

        let window = client.window();
        info!("remanaging client with window {:#0x}", window);

        client.set_managed(Toggle::On);

        if must_alter_workspace {
            let workspace = client
                .leader()
                .and_then(|leader| self.client(leader))
                .map(|leader| {
                    let workspace = leader.workspace();

                    client.set_workspace(workspace);
                    self.workspace(leader.workspace())
                });

            if let Some(workspace) = workspace {
                if !workspace.contains(window) {
                    workspace.add_client(window, &InsertPos::Back);
                }
            }
        }

        if client.is_sticky() {
            client.set_sticky(Toggle::Off);
            self.stick(client);
            self.map_client(client);
        }

        self.focus(client);
    }

    fn unmanage(
        &self,
        client: &Client,
    ) {
        if !client.is_managed() {
            return;
        }

        let window = client.window();
        info!("unmanaging client with window {:#0x}", window);

        client.set_managed(Toggle::Off);

        if client.is_sticky() {
            self.unstick(client);
            client.set_sticky(Toggle::On);
        }

        self.unmap_client(client);
        self.workspace(client.workspace()).remove_client(window);
    }

    // TODO: zones
    pub fn create_layout_zone(&mut self) {
        let workspace_index = self.active_workspace();
        let workspace = self.workspace(workspace_index);

        let cycle = workspace.active_focus_zone().unwrap();
        let cycle = self.zone_manager.nearest_cycle(cycle);
        let id = self.zone_manager.new_zone(
            Some(cycle),
            ZoneContent::Layout(Layout::new(), Cycle::new(Vec::new(), true)),
        );

        let workspace = self.workspace(workspace_index);
        workspace.add_zone(id, &InsertPos::Back);
        self.apply_layout(workspace_index);
        self.apply_stack(workspace_index);
    }

    // TODO: zones
    pub fn create_tab_zone(&mut self) {
        let workspace_index = self.active_workspace();
        let workspace = self.workspace(workspace_index);

        let cycle = workspace.active_focus_zone().unwrap();
        let cycle = self.zone_manager.nearest_cycle(cycle);
        let id = self
            .zone_manager
            .new_zone(Some(cycle), ZoneContent::Tab(Cycle::new(Vec::new(), true)));

        let workspace = self.workspace(workspace_index);
        workspace.add_zone(id, &InsertPos::Back);
        self.apply_layout(workspace_index);
        self.apply_stack(workspace_index);
    }

    // TODO: zones
    pub fn delete_zone(&mut self) {
        let workspace_index = self.active_workspace();
        let workspace = self.workspace(workspace_index);

        let cycle = workspace.active_spawn_zone().unwrap();
        let cycle = self.zone_manager.nearest_cycle(cycle);

        if cycle == workspace.root_zone() {
            return;
        }

        self.zone_manager.remove_zone(cycle);

        let workspace = self.workspace(workspace_index);
        workspace.remove_zone(cycle);
    }

    #[inline]
    fn is_applyable(client: &Client) -> bool {
        !client.is_floating()
            && !client.is_disowned()
            && client.is_managed()
            && (!client.is_fullscreen() || client.is_contained())
    }

    #[inline]
    fn is_free(
        &self,
        client: &Client,
    ) -> bool {
        client.is_free() || self.zone_manager.zone(client.zone()).method() == PlacementMethod::Free
    }

    #[inline]
    fn is_focusable(
        &self,
        client: &Client,
    ) -> bool {
        !client.is_disowned() && !client.is_iconified()
    }

    fn remove_window(
        &mut self,
        window: Window,
    ) {
        let client = match self.client_any(window) {
            Some(client) if !client.consume_unmap_if_expecting() => client,
            _ => return,
        };

        let (window, frame) = client.windows();
        let workspace = client.workspace();

        info!("removing client with window {:#0x}", window);

        if !client.is_managed() {
            self.remanage(client, true);
        }

        if let Ok(geometry) = self.conn.get_window_geometry(frame) {
            self.conn.unparent_window(window, geometry.pos);
        }

        self.conn.cleanup_window(window);
        self.conn.destroy_window(frame);

        if client.is_sticky() {
            self.unstick(client);
        }

        if Some(window) == self.jumped_from.get() {
            self.jumped_from.set(None);
        }

        if client.producer().is_some() {
            self.unconsume_client(client);
        }

        if let Some(parent) = client.parent().and_then(|parent| self.client(parent)) {
            parent.remove_child(window);
        }

        let id = client.zone();
        self.zone_manager.remove_zone(id);

        {
            let workspace = self.workspace(workspace);
            workspace.remove_client(window);
            workspace.remove_icon(window);
        }

        self.stack_manager.remove_window(window);
        self.frame_map.remove(&frame);
        self.window_map.remove(&window);
        self.client_map.remove(&window);
        self.pid_map.remove(&window);
        self.fullscreen_regions.borrow_mut().remove(&window);

        self.sync_focus();
        self.apply_layout(workspace);
    }

    #[inline(always)]
    fn render_decoration(
        &self,
        client: &Client,
    ) {
        let (border, frame_color) = client.decoration_colors();

        if let Some((width, color)) = border {
            self.conn.set_window_border_width(client.frame(), width);
            self.conn.set_window_border_color(client.frame(), color);
        }

        if let Some(color) = frame_color {
            self.conn.set_window_background_color(client.frame(), color);
        }
    }

    #[inline(always)]
    fn update_client_placement(
        &self,
        client: &Client,
        placement: &Placement,
    ) {
        let region = match placement.region {
            PlacementRegion::FreeRegion => client.free_region(),
            PlacementRegion::NewRegion(region) => region,
            PlacementRegion::NoRegion => return,
        };

        let zone = self.zone_manager.zone(client.zone());
        zone.set_method(placement.method);

        client.set_decoration(placement.decoration);
        client.set_region(match placement.method {
            PlacementMethod::Free => {
                zone.set_region(region);
                PlacementClass::Free(region)
            },
            PlacementMethod::Tile => PlacementClass::Tile(region),
        });
    }

    #[inline(always)]
    fn place_client(
        &self,
        client: &Client,
        method: PlacementMethod,
    ) {
        let (window, frame) = client.windows();

        self.conn.place_window(window, &client.inner_region());
        self.conn.place_window(frame, &match method {
            PlacementMethod::Free => client.free_region(),
            PlacementMethod::Tile => client.tile_region(),
        });

        self.render_decoration(client);
        self.conn.update_window_offset(window, frame);
    }

    #[inline(always)]
    fn map_client(
        &self,
        client: &Client,
    ) {
        if !client.is_mapped() {
            let (window, frame) = client.windows();
            info!("mapping client with window {:#0x}", window);

            self.conn.map_window(window);
            self.conn.map_window(frame);
            self.render_decoration(client);
            client.set_mapped(Toggle::On);
        }
    }

    #[inline(always)]
    fn unmap_client(
        &self,
        client: &Client,
    ) {
        if client.is_mapped() {
            let (window, frame) = client.windows();
            info!("unmapping client with window {:#0x}", window);

            self.conn.unmap_window(frame);
            client.expect_unmap();
            client.set_mapped(Toggle::Off);
        }
    }

    fn consume_client(
        &self,
        consumer: &Client,
        producer: &Client,
    ) {
        if producer.is_iconified() || consumer.is_iconified() {
            return;
        }

        let (cwindow, pwindow) = (consumer.window(), producer.window());
        let (cworkspace, pworkspace) = (consumer.workspace(), producer.workspace());

        info!(
            "consuming client with window {:#0x} and producer window {:#0x}",
            cwindow, pwindow
        );

        consumer.set_producer(pwindow);

        if producer.consumer_len() == 0 {
            let workspace = self.workspace(pworkspace);

            if pworkspace == cworkspace {
                workspace.replace_client(pwindow, cwindow);
            } else {
                workspace.remove_client(pwindow);
            }

            self.apply_layout(cworkspace);
            self.apply_stack(cworkspace);
        }

        producer.add_consumer(cwindow);
        self.unmanage(producer);
    }

    fn unconsume_client(
        &self,
        consumer: &Client,
    ) {
        let producer = match consumer
            .producer()
            .and_then(|producer| self.client_any(producer))
        {
            Some(producer) => producer,
            None => return,
        };

        info!(
            "unconsuming client with window {:#0x} and producer window {:#0x}",
            consumer.window(),
            producer.window()
        );

        producer.remove_consumer(consumer.window());

        if producer.consumer_len() == 0 {
            let workspace = consumer.workspace();

            {
                let workspace = self.workspace(workspace);

                if workspace.contains(consumer.window()) {
                    workspace.replace_client(consumer.window(), producer.window());
                } else {
                    workspace.add_client(producer.window(), &InsertPos::Back);
                }
            }

            producer.set_workspace(workspace);
            self.remanage(producer, false);
            self.apply_layout(workspace);
            self.apply_stack(workspace);
        }

        consumer.unset_producer();
    }

    #[inline(always)]
    pub fn kill_focus(&self) {
        if let Some(focus) = self.focus.get() {
            self.kill_window(focus);
        }
    }

    #[inline]
    pub fn kill_window(
        &self,
        window: Window,
    ) {
        match self.client_any(window) {
            Some(client) if !client.is_invincible() => {
                info!("killing client with window {:#0x}", window);

                self.conn.kill_window(window);
                self.conn.flush();
            },
            _ => {},
        }
    }

    #[inline(always)]
    pub fn cycle_zones(
        &self,
        dir: Direction,
    ) {
        self.workspace(self.active_workspace())
            .cycle_zones(dir, &self.zone_manager);
    }

    #[inline(always)]
    pub fn cycle_focus(
        &self,
        dir: Direction,
    ) {
        if let Some((_, window)) = self.workspace(self.active_workspace()).cycle_focus(
            dir,
            &self.client_map,
            &self.zone_manager,
        ) {
            self.focus_window(window);
            self.sync_focus();
        }
    }

    #[inline(always)]
    pub fn drag_focus(
        &self,
        dir: Direction,
    ) {
        if let Some(focus) = self.focus.get() {
            let workspace = self.active_workspace();

            self.workspace(workspace).drag_focus(dir);
            self.apply_layout(workspace);
            self.focus_window(focus);
        }
    }

    #[inline(always)]
    pub fn rotate_clients(
        &self,
        dir: Direction,
    ) {
        let workspace = self.workspace(self.active_workspace());

        if let Some(next) = workspace.next_client(dir.rev()) {
            workspace.rotate_clients(dir);
            self.apply_layout(self.active_workspace());
            self.focus_window(next);
        }
    }

    #[inline]
    pub fn move_focus_to_next_workspace(
        &self,
        dir: Direction,
    ) {
        if let Some(focus) = self.focus.get() {
            self.move_window_to_next_workspace(focus, dir);
        }
    }

    #[inline]
    pub fn move_window_to_next_workspace(
        &self,
        window: Window,
        dir: Direction,
    ) {
        if let Some(client) = self.client(window) {
            self.move_client_to_next_workspace(client, dir);
        }
    }

    #[inline]
    pub fn move_client_to_next_workspace(
        &self,
        client: &Client,
        dir: Direction,
    ) {
        self.move_client_to_workspace(
            client,
            Util::next_index(self.workspaces.iter(), self.active_workspace(), dir),
        );
    }

    #[inline]
    pub fn move_focus_to_workspace(
        &self,
        to: Index,
    ) {
        if let Some(focus) = self.focus.get() {
            self.move_window_to_workspace(focus, to);
        }
    }

    #[inline]
    fn move_window_to_workspace(
        &self,
        window: Window,
        to: Index,
    ) {
        if let Some(client) = self.client(window) {
            self.move_client_to_workspace(client, to);
        }
    }

    fn move_client_to_workspace(
        &self,
        client: &Client,
        to: Index,
    ) {
        let (window, from) =
            if to != self.active_workspace() && to < self.workspaces.len() && !client.is_sticky() {
                (client.window(), client.workspace())
            } else {
                return;
            };

        info!(
            "moving client with window {:#0x} to workspace {}",
            window, to
        );

        client.set_workspace(to);
        self.unmap_client(client);

        self.workspace(to).add_client(window, &InsertPos::Back);
        self.apply_layout(to);
        self.apply_stack(to);

        self.workspace(from).remove_client(window);
        self.apply_layout(from);
        self.apply_stack(from);

        self.sync_focus();
    }

    #[inline(always)]
    pub fn toggle_workspace(&self) {
        self.activate_workspace(self.prev_workspace.get());
    }

    #[inline(always)]
    pub fn activate_next_workspace(
        &self,
        dir: Direction,
    ) {
        self.activate_workspace(Util::next_index(
            self.workspaces.iter(),
            self.active_workspace(),
            dir,
        ));
    }

    pub fn activate_workspace(
        &self,
        to: Index,
    ) {
        if to == self.active_workspace() || to >= self.workspaces.len() {
            return;
        }

        info!("activating workspace {}", to);

        self.stop_moving();
        self.stop_resizing();

        let from = self.workspaces.active_index();
        self.prev_workspace.set(from);

        self.workspace(to)
            .on_each_client(&self.client_map, |client| {
                if !client.is_mapped() {
                    self.map_client(client);
                }
            });

        self.workspace(from)
            .on_each_client(&self.client_map, |client| {
                if client.is_mapped() && !client.is_sticky() {
                    self.unmap_client(client);
                }
            });

        self.sticky_clients.borrow().iter().for_each(|&window| {
            self.client_unchecked(window).set_workspace(to);
        });

        self.conn.set_current_desktop(to);

        self.workspaces.activate_for(&Selector::AtIndex(to));
        self.apply_layout(to);
        self.apply_stack(to);

        self.sync_focus();
    }

    #[inline]
    pub fn change_gap_size(
        &mut self,
        change: Change<u32>,
    ) -> Result<(), StateChangeError> {
        let workspace = self.active_workspace();

        self.workspaces[workspace].change_gap_size(change, &mut self.zone_manager)?;
        self.apply_layout(workspace);
        self.apply_stack(workspace);

        Ok(())
    }

    #[inline]
    pub fn copy_prev_layout_data(&mut self) -> Result<(), StateChangeError> {
        let workspace = self.active_workspace();

        self.workspaces[workspace].copy_prev_layout_data(&mut self.zone_manager)?;
        self.apply_layout(workspace);
        self.apply_stack(workspace);

        Ok(())
    }

    #[inline]
    pub fn reset_layout_data(&mut self) -> Result<(), StateChangeError> {
        let workspace = self.active_workspace();

        self.workspaces[workspace].reset_layout_data(&mut self.zone_manager)?;
        self.apply_layout(workspace);
        self.apply_stack(workspace);

        Ok(())
    }

    #[inline]
    pub fn reset_gap_size(&mut self) -> Result<(), StateChangeError> {
        let workspace = self.active_workspace();

        self.workspaces[workspace].reset_gap_size(&mut self.zone_manager)?;
        self.apply_layout(workspace);
        self.apply_stack(workspace);

        Ok(())
    }

    #[inline]
    pub fn change_main_count(
        &mut self,
        change: Change<u32>,
    ) -> Result<(), StateChangeError> {
        let workspace = self.active_workspace();

        self.workspaces[workspace].change_main_count(change, &mut self.zone_manager)?;
        self.apply_layout(workspace);
        self.apply_stack(workspace);

        Ok(())
    }

    #[inline]
    pub fn change_main_factor(
        &mut self,
        change: Change<f32>,
    ) -> Result<(), StateChangeError> {
        let workspace = self.active_workspace();

        self.workspaces[workspace].change_main_factor(change, &mut self.zone_manager)?;
        self.apply_layout(workspace);
        self.apply_stack(workspace);

        Ok(())
    }

    #[inline]
    pub fn change_margin(
        &mut self,
        edge: Edge,
        change: Change<i32>,
    ) -> Result<(), StateChangeError> {
        let workspace = self.active_workspace();

        self.workspaces[workspace].change_margin(edge, change, &mut self.zone_manager)?;
        self.apply_layout(workspace);
        self.apply_stack(workspace);

        Ok(())
    }

    #[inline]
    pub fn reset_margin(&mut self) -> Result<(), StateChangeError> {
        let workspace = self.active_workspace();

        self.workspaces[workspace].reset_margin(&mut self.zone_manager)?;
        self.apply_layout(workspace);
        self.apply_stack(workspace);

        Ok(())
    }

    #[inline]
    pub fn set_layout(
        &mut self,
        kind: LayoutKind,
    ) -> Result<(), StateChangeError> {
        let workspace = self.active_workspace();

        if let Some(id) = self.workspaces[workspace].active_focus_zone() {
            info!("activating layout {:?} on workspace {}", kind, workspace);

            self.zone_manager.set_kind(id, kind)?;
            self.apply_layout(workspace);
            self.apply_stack(workspace);
        }

        Ok(())
    }

    #[inline]
    pub fn toggle_layout(&mut self) {
        let workspace = self.active_workspace();

        if let Some(id) = self.workspaces[workspace].active_focus_zone() {
            let prev_kind = self.zone_manager.set_prev_kind(id);

            info!(
                "activating layout {:?} on workspace {}",
                prev_kind, workspace
            );

            self.apply_layout(workspace);
            self.apply_stack(workspace);
        }
    }

    #[inline(always)]
    pub fn set_floating_focus(
        &self,
        toggle: Toggle,
    ) {
        if let Some(focus) = self.focus.get() {
            self.set_floating_window(focus, toggle);
        }
    }

    #[inline(always)]
    pub fn set_floating_window(
        &self,
        window: Window,
        toggle: Toggle,
    ) {
        if let Some(client) = self.client(window) {
            self.set_floating_client(client, toggle);
        }
    }

    #[inline]
    fn set_floating_client(
        &self,
        client: &Client,
        toggle: Toggle,
    ) {
        info!(
            "{}floating client with window {:#0x}",
            if toggle.eval(client.is_floating()) {
                ""
            } else {
                "un"
            },
            client.window()
        );

        let workspace = client.workspace();

        client.set_floating(toggle);
        self.apply_layout(workspace);
        self.apply_stack(workspace);
    }

    #[inline]
    pub fn set_fullscreen_focus(
        &self,
        toggle: Toggle,
    ) {
        if let Some(focus) = self.focus.get() {
            self.set_fullscreen_window(focus, toggle);
        }
    }

    #[inline]
    pub fn set_fullscreen_window(
        &self,
        window: Window,
        toggle: Toggle,
    ) {
        if let Some(client) = self.client(window) {
            self.set_fullscreen_client(client, toggle);
        }
    }

    fn set_fullscreen_client(
        &self,
        client: &Client,
        toggle: Toggle,
    ) {
        if toggle.eval(client.is_fullscreen()) {
            self.fullscreen(client);
        } else {
            self.unfullscreen(client);
        }
    }

    #[inline(always)]
    fn fullscreen(
        &self,
        client: &Client,
    ) {
        if client.is_fullscreen() {
            return;
        }

        let window = client.window();
        let workspace = client.workspace();
        info!("enabling fullscreen for client with window {:#0x}", window);

        self.conn
            .set_window_state(window, WindowState::Fullscreen, true);

        client.set_fullscreen(Toggle::On);
        self.apply_layout(workspace);
        self.apply_stack(workspace);

        self.fullscreen_regions
            .borrow_mut()
            .insert(window, client.free_region());
    }

    #[inline(always)]
    fn unfullscreen(
        &self,
        client: &Client,
    ) {
        if !client.is_fullscreen() {
            return;
        }

        let window = client.window();
        let workspace = client.workspace();
        info!("disabling fullscreen for client with window {:#0x}", window);

        if let Some(free_region) = self.fullscreen_regions.borrow().get(&window).cloned() {
            client.set_region(PlacementClass::Free(free_region));
        }

        self.conn
            .set_window_state(window, WindowState::Fullscreen, false);

        client.set_fullscreen(Toggle::Off);
        self.apply_layout(workspace);
        self.apply_stack(workspace);

        self.fullscreen_regions.borrow_mut().remove(&window);
    }

    #[inline(always)]
    pub fn set_contained_focus(
        &self,
        toggle: Toggle,
    ) {
        if let Some(focus) = self.focus.get() {
            self.set_contained_window(focus, toggle);
        }
    }

    #[inline(always)]
    pub fn set_contained_window(
        &self,
        window: Window,
        toggle: Toggle,
    ) {
        if let Some(client) = self.client(window) {
            self.set_contained_client(client, toggle);
        }
    }

    #[inline]
    fn set_contained_client(
        &self,
        client: &Client,
        toggle: Toggle,
    ) {
        client.set_contained(toggle);
        self.set_fullscreen_client(client, Toggle::from(!client.is_contained()));
    }

    #[inline(always)]
    pub fn set_invincible_focus(
        &self,
        toggle: Toggle,
    ) {
        if let Some(focus) = self.focus.get() {
            self.set_invincible_window(focus, toggle);
        }
    }

    #[inline(always)]
    pub fn set_invincible_window(
        &self,
        window: Window,
        toggle: Toggle,
    ) {
        if let Some(client) = self.client(window) {
            self.set_invincible_client(client, toggle);
        }
    }

    #[inline(always)]
    fn set_invincible_client(
        &self,
        client: &Client,
        toggle: Toggle,
    ) {
        client.set_invincible(toggle);
    }

    #[inline(always)]
    pub fn set_producing_focus(
        &self,
        toggle: Toggle,
    ) {
        if let Some(focus) = self.focus.get() {
            self.set_producing_window(focus, toggle);
        }
    }

    #[inline(always)]
    pub fn set_producing_window(
        &self,
        window: Window,
        toggle: Toggle,
    ) {
        if let Some(client) = self.client(window) {
            self.set_producing_client(client, toggle);
        }
    }

    #[inline(always)]
    fn set_producing_client(
        &self,
        client: &Client,
        toggle: Toggle,
    ) {
        client.set_producing(toggle);
    }

    #[inline(always)]
    pub fn set_iconifyable_focus(
        &self,
        toggle: Toggle,
    ) {
        if let Some(focus) = self.focus.get() {
            self.set_iconifyable_window(focus, toggle);
        }
    }

    #[inline(always)]
    pub fn set_iconifyable_window(
        &self,
        window: Window,
        toggle: Toggle,
    ) {
        if let Some(client) = self.client(window) {
            self.set_iconifyable_client(client, toggle);
        }
    }

    #[inline(always)]
    fn set_iconifyable_client(
        &self,
        client: &Client,
        toggle: Toggle,
    ) {
        client.set_iconifyable(toggle);
    }

    #[inline]
    pub fn set_iconify_focus(
        &self,
        toggle: Toggle,
    ) {
        if let Some(focus) = self.focus.get() {
            self.set_iconify_window(focus, toggle);
        }
    }

    #[inline]
    pub fn set_iconify_window(
        &self,
        window: Window,
        toggle: Toggle,
    ) {
        if let Some(client) = self.client(window) {
            self.set_iconify_client(client, toggle);
        }
    }

    fn set_iconify_client(
        &self,
        client: &Client,
        toggle: Toggle,
    ) {
        if toggle.eval(client.is_iconified()) {
            if client.is_iconifyable() {
                self.iconify(client);
            }
        } else {
            self.deiconify(client);
        }
    }

    #[inline]
    pub fn pop_deiconify(&self) {
        if let Some(icon) = self.workspaces[self.active_workspace()].focused_icon() {
            self.set_iconify_window(icon, Toggle::Off);
        }
    }

    #[inline]
    pub fn deiconify_all(
        &self,
        index: Index,
    ) {
        if index >= self.workspaces.len() {
            warn!(
                "attempting to deicony_all from nonexistent workspace {}",
                index
            );
            return;
        }

        let workspace = &self.workspaces[index];

        while let Some(icon) = workspace.focused_icon() {
            self.set_iconify_window(icon, Toggle::Off);
        }
    }

    #[inline(always)]
    fn iconify(
        &self,
        client: &Client,
    ) {
        if client.is_iconified() {
            return;
        }

        let window = client.window();
        let workspace = client.workspace();

        info!("iconifying client with window {:#0x}", window);

        self.workspaces[workspace].client_to_icon(window);

        self.conn
            .set_icccm_window_state(window, IcccmWindowState::Iconic);

        client.set_iconified(Toggle::On);
        self.unmap_client(client);

        self.apply_layout(workspace);
        self.apply_stack(workspace);

        self.sync_focus();
    }

    #[inline(always)]
    fn deiconify(
        &self,
        client: &Client,
    ) {
        if !client.is_iconified() {
            return;
        }

        let window = client.window();
        let workspace = client.workspace();

        info!("deiconifying client with window {:#0x}", window);

        self.workspaces[workspace].icon_to_client(window);

        self.conn
            .set_icccm_window_state(window, IcccmWindowState::Normal);

        client.set_iconified(Toggle::Off);
        self.map_client(client);

        self.apply_layout(workspace);
        self.apply_stack(workspace);

        self.sync_focus();
    }

    #[inline]
    pub fn set_stick_focus(
        &self,
        toggle: Toggle,
    ) {
        if let Some(focus) = self.focus.get() {
            self.set_stick_window(focus, toggle);
        }
    }

    #[inline]
    pub fn set_stick_window(
        &self,
        window: Window,
        toggle: Toggle,
    ) {
        if let Some(client) = self.client(window) {
            self.set_stick_client(client, toggle);
        }
    }

    fn set_stick_client(
        &self,
        client: &Client,
        toggle: Toggle,
    ) {
        if toggle.eval(client.is_sticky()) {
            self.stick(client);
        } else {
            self.unstick(client);
        }
    }

    #[inline(always)]
    fn stick(
        &self,
        client: &Client,
    ) {
        if client.is_sticky() {
            return;
        }

        let window = client.window();
        info!("sticking client with window {:#0x}", window);

        self.workspaces
            .iter()
            .filter(|workspace| workspace.number() as Index != client.workspace())
            .for_each(|workspace| workspace.add_client(window, &InsertPos::Back));

        self.conn
            .set_window_state(window, WindowState::Sticky, true);

        client.set_sticky(Toggle::On);
        self.render_decoration(client);

        self.sticky_clients.borrow_mut().insert(window);
    }

    #[inline(always)]
    fn unstick(
        &self,
        client: &Client,
    ) {
        if !client.is_sticky() {
            return;
        }

        let window = client.window();
        info!("unsticking client with window {:#0x}", window);

        self.workspaces
            .iter()
            .filter(|workspace| workspace.number() as Index != client.workspace())
            .for_each(|workspace| {
                workspace.remove_client(window);
                workspace.remove_icon(window);
            });

        self.conn
            .set_window_state(window, WindowState::Sticky, false);

        client.set_sticky(Toggle::Off);
        self.render_decoration(client);

        self.sticky_clients.borrow_mut().remove(&window);
    }

    #[inline(always)]
    fn focus_window(
        &self,
        window: Window,
    ) {
        if let Some(client) = self.client(window) {
            self.focus(client);
        }
    }

    fn focus(
        &self,
        client: &Client,
    ) {
        let (window, frame) = match client.windows() {
            windows if self.is_focusable(client) && Some(windows.0) != self.focus.get() => windows,
            _ => return,
        };

        info!("focusing client with window {:#0x}", window);

        if self.active_workspace() != client.workspace() {
            self.activate_workspace(client.workspace());
        }

        let workspace = client.workspace();

        if let Some(prev_focus) = self.focus.get() {
            self.unfocus_window(prev_focus);
        }

        self.conn.ungrab_buttons(frame);

        if let Some(client) = self.client(window) {
            client.set_focused(Toggle::On);
            client.set_urgent(Toggle::Off);
        }

        let id = client.zone();
        let cycle = self.zone_manager.nearest_cycle(id);
        self.zone_manager.activate_zone(id);

        {
            let workspace = self.workspace(workspace);
            workspace.activate_zone(cycle);
            workspace.focus_client(window);
        }

        if self.zone_manager.is_within_persisent(id) {
            self.apply_layout(workspace);
        }

        if self.conn.get_focused_window() != window {
            self.conn.focus_window(window);
        }

        self.focus.set(Some(window));
        self.render_decoration(client);
        self.apply_stack(workspace);
    }

    #[inline]
    fn unfocus_window(
        &self,
        window: Window,
    ) {
        if let Some(client) = self.client(window) {
            self.unfocus(client);
        }
    }

    #[inline]
    fn unfocus(
        &self,
        client: &Client,
    ) {
        let (window, frame) = client.windows();
        info!("unfocusing client with window {:#0x}", window);

        client.set_warp_pos(self.conn.get_pointer_position());
        client.set_focused(Toggle::Off);

        self.conn.regrab_buttons(frame);
        self.render_decoration(client);
    }

    #[inline(always)]
    fn sync_focus(&self) {
        let workspace = self.workspace(self.active_workspace());

        match workspace.focused_client() {
            Some(focus) if Some(focus) != self.focus.get() => {
                self.focus_window(focus);
            },
            _ if workspace.is_empty() => {
                self.conn.unfocus();
                self.focus.set(None);
            },
            _ => {},
        }
    }

    pub fn jump_client(
        &self,
        criterium: JumpCriterium,
    ) {
        let mut window = match criterium {
            JumpCriterium::OnWorkspaceBySelector(index, &sel) if index < self.workspaces.len() => {
                match self.workspaces[index].get_client_for(sel, &self.zone_manager) {
                    Some(window) => window,
                    _ => return,
                }
            },
            JumpCriterium::ByName(method) => {
                match self
                    .client_map
                    .values()
                    .filter(|&client| client.is_managed() && client.name_matches(method))
                    .max_by_key(|client| client.last_focused())
                {
                    Some(client) => client.window(),
                    None => return,
                }
            },
            JumpCriterium::ByClass(method) => {
                match self
                    .client_map
                    .values()
                    .filter(|&client| client.is_managed() && client.class_matches(method))
                    .max_by_key(|client| client.last_focused())
                {
                    Some(client) => client.window(),
                    None => return,
                }
            },
            JumpCriterium::ByInstance(method) => {
                match self
                    .client_map
                    .values()
                    .filter(|&client| client.is_managed() && client.instance_matches(method))
                    .max_by_key(|client| client.last_focused())
                {
                    Some(client) => client.window(),
                    None => return,
                }
            },
            JumpCriterium::ForCond(cond) => {
                match self
                    .client_map
                    .values()
                    .filter(|&client| client.is_managed() && cond(client))
                    .max_by_key(|client| client.last_focused())
                {
                    Some(client) => client.window(),
                    None => return,
                }
            },
            _ => return,
        };

        if let Some(focus) = self.focus.get() {
            if window == focus {
                match self.jumped_from.get() {
                    Some(jumped_from) if jumped_from != focus => {
                        window = jumped_from;
                    },
                    _ => {},
                }
            }

            self.jumped_from.set(Some(focus));
        }

        info!("jumping to client with window {:#0x}", window);
        self.focus_window(window);
    }

    #[inline(always)]
    pub fn center_focus(&self) {
        if let Some(focus) = self.focus.get() {
            self.center_window(focus);
        }
    }

    #[inline(always)]
    pub fn center_window(
        &self,
        window: Window,
    ) {
        if let Some(client) = self.client(window) {
            self.center_client(client);
        }
    }

    pub fn center_client(
        &self,
        client: &Client,
    ) {
        if !self.is_free(client) {
            return;
        }

        info!("centering client with window {:#0x}", client.window());

        let mut region = client.free_region();
        region.pos = self
            .active_screen()
            .full_region()
            .from_absolute_inner_center(region.dim)
            .pos;

        self.conn.move_window(client.frame(), region.pos);
        client.set_region(PlacementClass::Free(region));
    }

    pub fn apply_float_retain_region(&mut self) {
        let workspace = self.active_workspace();

        self.workspace(workspace)
            .clients()
            .into_iter()
            .map(|window| self.client_unchecked(window))
            .for_each(|client| client.set_region(PlacementClass::Free(client.active_region())));

        self.set_layout(LayoutKind::Float).ok();
        self.apply_layout(workspace);
    }

    #[inline(always)]
    pub fn snap_focus(
        &self,
        edge: Edge,
    ) {
        if let Some(focus) = self.focus.get() {
            self.snap_window(focus, edge);
        }
    }

    #[inline(always)]
    pub fn snap_window(
        &self,
        window: Window,
        edge: Edge,
    ) {
        if let Some(client) = self.client(window) {
            self.snap_client(client, edge);
        }
    }

    fn snap_client(
        &self,
        client: &Client,
        edge: Edge,
    ) {
        if !self.is_free(client) {
            return;
        }

        let window = client.window();

        info!(
            "snapping client with window {:#0x} to edge {:?}",
            window, edge
        );

        let placeable_region = self.active_screen().placeable_region();
        let mut region = client.free_region();

        match edge {
            Edge::Left => region.pos.x = placeable_region.pos.x,
            Edge::Right => {
                let x = placeable_region.dim.w + placeable_region.pos.x;
                region.pos.x = std::cmp::max(0, x - region.dim.w)
            },
            Edge::Top => region.pos.y = placeable_region.pos.y,
            Edge::Bottom => {
                let y = placeable_region.dim.h + placeable_region.pos.y;
                region.pos.y = std::cmp::max(0, y - region.dim.h)
            },
        }

        client.set_region(PlacementClass::Free(region));

        let placement = Placement {
            method: PlacementMethod::Free,
            kind: PlacementTarget::Client(window),
            zone: client.zone(),
            region: PlacementRegion::FreeRegion,
            decoration: client.decoration(),
        };

        self.update_client_placement(client, &placement);
        self.place_client(client, placement.method);
    }

    #[inline(always)]
    pub fn nudge_focus(
        &self,
        edge: Edge,
        step: i32,
    ) {
        if let Some(focus) = self.focus.get() {
            self.nudge_window(focus, edge, step);
        }
    }

    #[inline(always)]
    pub fn nudge_window(
        &self,
        window: Window,
        edge: Edge,
        step: i32,
    ) {
        if let Some(client) = self.client(window) {
            self.nudge_client(client, edge, step);
        }
    }

    fn nudge_client(
        &self,
        client: &Client,
        edge: Edge,
        step: i32,
    ) {
        if !self.is_free(client) {
            return;
        }

        let window = client.window();

        info!(
            "nudging client with window {:#0x} at the {:?} by {}",
            window, edge, step
        );

        let mut region = client.free_region();

        match edge {
            Edge::Left => region.pos.x -= step,
            Edge::Right => region.pos.x += step,
            Edge::Top => region.pos.y -= step,
            Edge::Bottom => region.pos.y += step,
        }

        client.set_region(PlacementClass::Free(region));

        let placement = Placement {
            method: PlacementMethod::Free,
            kind: PlacementTarget::Client(window),
            zone: client.zone(),
            region: PlacementRegion::FreeRegion,
            decoration: client.decoration(),
        };

        self.update_client_placement(client, &placement);
        self.place_client(client, placement.method);
    }

    #[inline(always)]
    pub fn grow_ratio_focus(
        &self,
        step: i32,
    ) {
        if let Some(focus) = self.focus.get() {
            self.grow_ratio_window(focus, step);
        }
    }

    #[inline(always)]
    pub fn grow_ratio_window(
        &self,
        window: Window,
        step: i32,
    ) {
        if let Some(client) = self.client(window) {
            self.grow_ratio_client(client, step);
        }
    }

    fn grow_ratio_client(
        &self,
        client: &Client,
        step: i32,
    ) {
        if !self.is_free(client) {
            return;
        }

        let frame_extents = client.frame_extents();

        let original_region = client.free_region();
        let region = original_region;
        let (width, height) = region.dim.values();

        let fraction = width as f64 / (width + height) as f64;
        let width_inc = fraction * step as f64;
        let height_inc = step as f64 - width_inc;
        let width_inc = width_inc.round() as i32;
        let height_inc = height_inc.round() as i32;

        let mut region = region.without_extents(frame_extents);

        if (width_inc.is_negative() && -width_inc >= region.dim.w)
            || (height_inc.is_negative() && -height_inc >= region.dim.h)
            || (region.dim.w + width_inc <= Client::MIN_CLIENT_DIM.w)
            || (region.dim.h + height_inc <= Client::MIN_CLIENT_DIM.h)
        {
            return;
        }

        let window = client.window();

        info!(
            "{} client with window {:#0x} by {}",
            if step >= 0 { "growing" } else { "shrinking" },
            window,
            step.abs()
        );

        region.dim.w += width_inc;
        region.dim.h += height_inc;

        region = region.with_extents(frame_extents);
        let dx = region.dim.w - original_region.dim.w;
        let dy = region.dim.h - original_region.dim.h;

        let width_shift = (dx as f64 / 2f64) as i32;
        let height_shift = (dy as f64 / 2f64) as i32;

        region.pos.x -= width_shift;
        region.pos.y -= height_shift;

        client.set_region(PlacementClass::Free(region));

        let placement = Placement {
            method: PlacementMethod::Free,
            kind: PlacementTarget::Client(window),
            zone: client.zone(),
            region: PlacementRegion::FreeRegion,
            decoration: client.decoration(),
        };

        self.update_client_placement(client, &placement);
        self.place_client(client, placement.method);
    }

    #[inline(always)]
    pub fn stretch_focus(
        &self,
        edge: Edge,
        step: i32,
    ) {
        if let Some(focus) = self.focus.get() {
            self.stretch_window(focus, edge, step);
        }
    }

    #[inline(always)]
    pub fn stretch_window(
        &self,
        window: Window,
        edge: Edge,
        step: i32,
    ) {
        if let Some(client) = self.client(window) {
            self.stretch_client(client, edge, step);
        }
    }

    fn stretch_client(
        &self,
        client: &Client,
        edge: Edge,
        step: i32,
    ) {
        if !self.is_free(client) {
            return;
        }

        let window = client.window();

        info!(
            "stretching client with window {:#0x} at the {:?} by {}",
            window, edge, step
        );

        let frame_extents = client.frame_extents();
        let mut region = client.free_region().without_extents(frame_extents);

        match edge {
            Edge::Left if !(step.is_negative() && -step >= region.dim.w) => {
                if region.dim.w + step <= Client::MIN_CLIENT_DIM.w {
                    region.pos.x -= Client::MIN_CLIENT_DIM.w - region.dim.w;
                    region.dim.w = Client::MIN_CLIENT_DIM.w;
                } else {
                    region.pos.x -= step;
                    region.dim.w += step;
                }
            },
            Edge::Right if !(step.is_negative() && -step >= region.dim.w) => {
                if region.dim.w + step <= Client::MIN_CLIENT_DIM.w {
                    region.dim.w = Client::MIN_CLIENT_DIM.w;
                } else {
                    region.dim.w += step;
                }
            },
            Edge::Top if !(step.is_negative() && -step >= region.dim.h) => {
                if region.dim.h + step <= Client::MIN_CLIENT_DIM.h {
                    region.pos.y -= Client::MIN_CLIENT_DIM.h - region.dim.h;
                    region.dim.h = Client::MIN_CLIENT_DIM.h;
                } else {
                    region.pos.y -= step;
                    region.dim.h += step;
                }
            },
            Edge::Bottom if (!step.is_negative() && -step >= region.dim.h) => {
                if region.dim.h + step <= Client::MIN_CLIENT_DIM.h {
                    region.dim.h = Client::MIN_CLIENT_DIM.h;
                } else {
                    region.dim.h += step;
                }
            },
            _ => return,
        }

        client.set_region(PlacementClass::Free(region.with_extents(frame_extents)));

        let placement = Placement {
            method: PlacementMethod::Free,
            kind: PlacementTarget::Client(window),
            zone: client.zone(),
            region: PlacementRegion::FreeRegion,
            decoration: client.decoration(),
        };

        self.update_client_placement(client, &placement);
        self.place_client(client, placement.method);
    }

    pub fn start_moving(
        &self,
        window: Window,
    ) {
        if self.move_buffer.is_occupied() || self.resize_buffer.is_occupied() {
            return;
        }

        if let Some(client) = self.client(window) {
            self.move_buffer.set(
                client.window(),
                Grip::Corner(Corner::TopLeft),
                self.conn.get_pointer_position(),
                client.free_region(),
            );

            self.conn.confine_pointer(self.move_buffer.handle());
        }
    }

    #[inline(always)]
    pub fn stop_moving(&self) {
        if self.move_buffer.is_occupied() {
            self.conn.release_pointer();
            self.move_buffer.unset();
        }
    }

    #[inline(always)]
    pub fn handle_move(
        &self,
        pos: &Pos,
    ) {
        if !self.move_buffer.is_occupied() {
            return;
        }

        let client = self
            .move_buffer
            .window()
            .and_then(|window| self.client(window))
            .unwrap();

        if !self.is_free(client) {
            return;
        }

        client.set_region(PlacementClass::Free(Region {
            pos: self.move_buffer.window_region().unwrap().pos
                + self.move_buffer.grip_pos().unwrap().dist(*pos),
            dim: client.free_region().dim,
        }));

        let placement = Placement {
            method: PlacementMethod::Free,
            kind: PlacementTarget::Client(client.window()),
            zone: client.zone(),
            region: PlacementRegion::FreeRegion,
            decoration: client.decoration(),
        };

        self.update_client_placement(client, &placement);
        self.place_client(client, placement.method);
    }

    pub fn start_resizing(
        &self,
        window: Window,
    ) {
        if self.move_buffer.is_occupied() || self.resize_buffer.is_occupied() {
            return;
        }

        if let Some(client) = self.client(window) {
            let pos = self.conn.get_pointer_position();

            self.resize_buffer.set(
                client.window(),
                Grip::Corner(client.free_region().nearest_corner(pos)),
                pos,
                client.free_region(),
            );

            self.conn.confine_pointer(self.resize_buffer.handle());
        }
    }

    pub fn stop_resizing(&self) {
        if self.resize_buffer.is_occupied() {
            self.conn.release_pointer();
            self.resize_buffer.unset();
        }
    }

    #[inline(always)]
    pub fn handle_resize(
        &self,
        pos: &Pos,
    ) {
        if !self.resize_buffer.is_occupied() {
            return;
        }

        let client = self
            .resize_buffer
            .window()
            .and_then(|window| self.client(window))
            .unwrap();

        if !self.is_free(client) {
            return;
        }

        let mut region = client.free_region().without_extents(client.frame_extents());

        let window_region = self.resize_buffer.window_region().unwrap();
        let grip = self.resize_buffer.grip().unwrap();

        let top_grip = grip.is_top_grip();
        let left_grip = grip.is_left_grip();
        let delta = self.resize_buffer.grip_pos().unwrap().dist(pos.to_owned());

        let dest_w = if left_grip {
            window_region.dim.w - delta.dx
        } else {
            window_region.dim.w + delta.dx
        };

        let dest_h = if top_grip {
            window_region.dim.h - delta.dy
        } else {
            window_region.dim.h + delta.dy
        };

        region.dim.w = std::cmp::max(0, dest_w);
        region.dim.h = std::cmp::max(0, dest_h);

        if let Some(size_hints) = client.size_hints() {
            size_hints.apply(&mut region.dim);
        }

        region = region.with_extents(client.frame_extents());

        if top_grip {
            region.pos.y = window_region.pos.y + (window_region.dim.h - region.dim.h);
        }

        if left_grip {
            region.pos.x = window_region.pos.x + (window_region.dim.w - region.dim.w);
        }

        if region == client.previous_region() {
            return;
        }

        let placement = Placement {
            method: PlacementMethod::Free,
            kind: PlacementTarget::Client(client.window()),
            zone: client.zone(),
            region: PlacementRegion::NewRegion(region),
            decoration: client.decoration(),
        };

        self.update_client_placement(client, &placement);
        self.place_client(client, placement.method);
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
                        on_root,
                    } => self.handle_mouse(event, on_root, &mut mouse_bindings),
                    Event::Key {
                        event,
                    } => self.handle_key(event, &mut key_bindings),
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
                    Event::ScreenChange => self.handle_screen_change(),
                }
            }

            self.conn.flush();
        }
    }

    #[inline(always)]
    fn handle_mouse(
        &mut self,
        event: MouseEvent,
        on_root: bool,
        mouse_bindings: &mut MouseBindings,
    ) {
        let mut input = event.input;
        let window = event.window;

        match event.kind {
            MouseEventKind::Release => {
                self.stop_moving();
                self.stop_resizing();

                return;
            },
            MouseEventKind::Motion => {
                self.handle_move(&event.root_rpos);
                self.handle_resize(&event.root_rpos);

                return;
            },
            _ => {},
        }

        {
            // handle global mouse bindings
            input.target = MouseInputTarget::Global;
            let binding = mouse_bindings.get_mut(&input);

            if let Some(action) = binding {
                if action(self, None) {
                    // TODO: config.focus_follows_mouse
                    if let Some(focus) = self.focus.get() {
                        if window.is_some() && window != Some(focus) {
                            self.focus_window(window.unwrap());
                        }
                    }
                }

                return;
            }
        }

        if on_root {
            // handle root-targeted mouse bindings
            input.target = MouseInputTarget::Root;
            let binding = mouse_bindings.get_mut(&input);

            if let Some(action) = binding {
                action(self, None);
                return;
            }
        }

        {
            // handle client-targeted mouse bindings
            input.target = MouseInputTarget::Client;
            let binding = mouse_bindings.get_mut(&input);

            if let Some(window) = event.window {
                if let Some(window) = self.window(window) {
                    if let Some(action) = binding {
                        if action(self, Some(window)) {
                            // TODO: config.focus_follows_mouse
                            if let Some(focus) = self.focus.get() {
                                if window != focus {
                                    self.focus_window(window);
                                }
                            }
                        }
                    } else {
                        if event.kind != MouseEventKind::Release {
                            if let Some(focus) = self.focus.get() {
                                if window != focus {
                                    self.focus_window(window);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[inline(always)]
    fn handle_key(
        &mut self,
        event: KeyEvent,
        key_bindings: &mut KeyBindings,
    ) {
        if let Some(action) = key_bindings.get_mut(&event.input) {
            debug!("processing key binding: {:?}", event.input);
            action(self);
        }
    }

    #[inline(always)]
    fn handle_map_request(
        &mut self,
        window: Window,
        ignore: bool,
    ) {
        debug!("MAP_REQUEST for window {:#0x}", window);

        let workspace = self.active_workspace();

        if ignore {
            if let Some(struts) = self.conn.get_window_strut(window) {
                let screen = self.active_screen();
                screen.add_struts(struts);

                if !screen.showing_struts() {
                    self.conn.unmap_window(window);
                } else {
                    screen.compute_placeable_region();
                    self.apply_layout(workspace);
                    self.apply_stack(workspace);
                }
            }

            let preferred_state = self.conn.get_window_preferred_state(window);
            let preferred_type = self.conn.get_window_preferred_type(window);
            let geometry = self.conn.get_window_geometry(window);

            if let Some(layer) = match (preferred_state, preferred_type) {
                (Some(WindowState::Below), _) => Some(StackLayer::Below),
                (_, WindowType::Desktop) => Some(StackLayer::Desktop),
                (_, WindowType::Dock) => {
                    if let Ok(geometry) = geometry {
                        let screen = self.active_screen();
                        let full_region = screen.full_region();

                        if !screen.contains_window(window) {
                            let strut = match (
                                (geometry.pos.x, geometry.pos.y),
                                (geometry.dim.w, geometry.dim.h),
                            ) {
                                ((0, 0), (w, h)) if w == full_region.dim.w => Some((Edge::Top, h)),
                                ((0, 0), (w, h)) if h == full_region.dim.h => Some((Edge::Left, w)),
                                ((0, 0), (w, h)) if w > h => Some((Edge::Top, h)),
                                ((0, 0), (w, h)) if w < h => Some((Edge::Left, w)),
                                ((_, y), (_, h)) if y == full_region.dim.h - h => {
                                    Some((Edge::Bottom, h))
                                },
                                ((x, _), (w, _)) if x == full_region.dim.w - w => {
                                    Some((Edge::Right, w))
                                },
                                _ => None,
                            };

                            if let Some((edge, width)) = strut {
                                screen.add_strut(edge, window, width as u32);

                                if !screen.showing_struts() {
                                    self.conn.unmap_window(window);
                                } else {
                                    screen.compute_placeable_region();
                                    self.apply_layout(workspace);
                                    self.apply_stack(workspace);
                                }
                            }
                        }
                    }

                    Some(StackLayer::Dock)
                },
                (_, WindowType::Notification) => Some(StackLayer::Notification),
                (Some(WindowState::Above), _) => Some(StackLayer::Above),
                (..) => None,
            } {
                self.stack_manager.add_window(window, layer)
            };

            self.apply_stack(self.active_workspace());
        }

        if self.client_map.contains_key(&window) {
            return;
        }

        self.manage(window, ignore);
    }

    #[inline]
    fn handle_map(
        &self,
        window: Window,
        _ignore: bool,
    ) {
        debug!("MAP for window {:#0x}", window);
    }

    #[inline]
    fn handle_enter(
        &self,
        window: Window,
        _root_rpos: Pos,
        _window_rpos: Pos,
    ) {
        debug!("ENTER for window {:#0x}", window);

        if let Some(client) = self.client(window) {
            if let Some(focus) = self.focus.get() {
                if client.window() != focus {
                    self.unfocus_window(focus);
                } else {
                    return;
                }
            }

            self.focus(client);
        }
    }

    #[inline]
    fn handle_leave(
        &self,
        window: Window,
        _root_rpos: Pos,
        _window_rpos: Pos,
    ) {
        debug!("LEAVE for window {:#0x}", window);
        self.unfocus_window(window);
    }

    #[inline]
    fn handle_destroy(
        &mut self,
        window: Window,
    ) {
        debug!("DESTROY for window {:#0x}", window);

        let screen = self.active_screen();

        if screen.has_strut_window(window) {
            screen.remove_window_strut(window);
            screen.compute_placeable_region();

            let workspace = self.active_workspace();
            self.apply_layout(workspace);
            self.apply_stack(workspace);
        }

        self.unmanaged_windows.borrow_mut().remove(&window);
        self.remove_window(window);
    }

    #[inline]
    fn handle_expose(
        &self,
        _window: Window,
    ) {
    }

    #[inline]
    fn handle_unmap(
        &mut self,
        window: Window,
        _ignore: bool,
    ) {
        debug!("UNMAP for window {:#0x}", window);

        if self.unmanaged_windows.borrow().contains(&window) {
            return;
        }

        self.handle_destroy(window);
    }

    #[inline]
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

    #[inline]
    fn handle_state_request(
        &self,
        window: Window,
        state: WindowState,
        action: ToggleAction,
        on_root: bool,
    ) {
        let client = match self.client_any(window) {
            Some(client) => client,
            _ => return,
        };

        debug!(
            "STATE_REQUEST for window {:#0x}, with state {:?} and action {:?}",
            window, state, action
        );

        match action {
            ToggleAction::Add => match state {
                WindowState::Fullscreen => self.fullscreen(client),
                WindowState::Sticky => self.stick(client),
                WindowState::DemandsAttention => {
                    self.conn.set_icccm_window_hints(window, Hints {
                        urgent: true,
                        input: None,
                        initial_state: None,
                        group: None,
                    });

                    if let Some(client) = self.client_any(window) {
                        client.set_urgent(Toggle::On);
                        self.render_decoration(client);
                    }
                },
                _ => {},
            },
            ToggleAction::Remove => match state {
                WindowState::Fullscreen => self.unfullscreen(client),
                WindowState::Sticky => self.unstick(client),
                WindowState::DemandsAttention => {
                    self.conn.set_icccm_window_hints(window, Hints {
                        urgent: false,
                        input: None,
                        initial_state: None,
                        group: None,
                    });

                    if let Some(client) = self.client_any(window) {
                        client.set_urgent(Toggle::Off);
                        self.render_decoration(client);
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

    #[inline]
    fn handle_focus_request(
        &self,
        window: Window,
        on_root: bool,
    ) {
        debug!("FOCUS_REQUEST for window {:#0x}", window);

        if !on_root {
            self.focus_window(window);
        }
    }

    #[inline]
    fn handle_close_request(
        &self,
        window: Window,
        on_root: bool,
    ) {
        debug!("CLOSE_REQUEST for window {:#0x}", window);

        if !on_root {
            self.conn.kill_window(window);
        }
    }

    #[inline]
    fn handle_workspace_request(
        &self,
        window: Option<Window>,
        index: usize,
        on_root: bool,
    ) {
        debug!(
            "WORKSPACE_REQUEST for workspace {} by window {:?}",
            index,
            window.map(|window| format!("{:#0x}", window))
        );

        if on_root {
            self.activate_workspace(index);
        }
    }

    #[inline]
    fn handle_placement_request(
        &self,
        window: Window,
        pos: Option<Pos>,
        dim: Option<Dim>,
        _on_root: bool,
    ) {
        if pos.is_none() && dim.is_none() {
            return;
        }

        debug!(
            "PLACEMENT_REQUEST for window {:#0x} with pos {:?} and dim {:?}",
            window, pos, dim
        );

        let client = match self.client(window) {
            Some(client) if self.is_free(client) => client,
            None => {
                if let Ok(mut geometry) = self.conn.get_window_geometry(window) {
                    if let Some(pos) = pos {
                        geometry.pos = pos;
                    }

                    if let Some(dim) = dim {
                        geometry.dim = dim;
                    }

                    self.conn.place_window(window, &geometry);
                }

                return;
            },
            _ => return,
        };

        let extents = client.frame_extents();
        let mut region = if window == client.window() {
            Region {
                pos: if let Some(pos) = pos {
                    Pos {
                        x: pos.x - extents.left,
                        y: pos.y - extents.top,
                    }
                } else {
                    client.free_region().pos
                },
                dim: if let Some(dim) = dim {
                    Dim {
                        w: dim.w + extents.left + extents.right,
                        h: dim.h + extents.top + extents.bottom,
                    }
                } else {
                    client.free_region().dim
                },
            }
        } else {
            Region {
                pos: pos.unwrap_or(client.free_region().pos),
                dim: dim.unwrap_or(client.free_region().dim),
            }
        };

        region = region
            .without_extents(extents)
            .with_size_hints(&client.size_hints())
            .with_minimum_dim(&Client::MIN_CLIENT_DIM)
            .with_extents(extents);

        client.set_region(PlacementClass::Free(region));

        let placement = Placement {
            method: PlacementMethod::Free,
            kind: PlacementTarget::Client(window),
            zone: client.zone(),
            region: PlacementRegion::FreeRegion,
            decoration: client.decoration(),
        };

        self.update_client_placement(client, &placement);
        self.place_client(client, placement.method);
    }

    #[inline]
    fn handle_grip_request(
        &self,
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
            self.move_buffer.unset();
            self.resize_buffer.unset();

            if let Some(client) = self.client(window) {
                self.resize_buffer.set(
                    client.window(),
                    grip,
                    self.conn.get_pointer_position(),
                    client.free_region(),
                );

                self.conn.confine_pointer(self.resize_buffer.handle());
            }
        } else {
            self.start_moving(window);
        }
    }

    #[inline]
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
            StackMode::Above => self.stack_manager.add_above_other(window, sibling),
            StackMode::Below => self.stack_manager.add_below_other(window, sibling),
        }

        self.apply_stack(self.active_workspace());
    }

    #[inline]
    fn handle_property(
        &self,
        window: Window,
        kind: PropertyKind,
        _on_root: bool,
    ) {
        debug!("PROPERTY for window {:#0x} of kind {:?}", window, kind);

        match kind {
            PropertyKind::Name => {
                if let Some(client) = self.client_any(window) {
                    client.set_name(self.conn.get_icccm_window_name(window));
                }
            },
            PropertyKind::Class => {
                if let Some(client) = self.client_any(window) {
                    client.set_class(self.conn.get_icccm_window_class(window));
                    client.set_instance(self.conn.get_icccm_window_instance(window));
                }
            },
            PropertyKind::Size => {
                if let Some(client) = self.client_any(window) {
                    let window = client.window();

                    let size_hints = self
                        .conn
                        .get_icccm_window_size_hints(
                            window,
                            Some(Client::MIN_CLIENT_DIM),
                            &client.size_hints(),
                        )
                        .1;

                    let mut geometry = match self.conn.get_window_geometry(window) {
                        Ok(geometry) => geometry,
                        Err(_) => return,
                    }
                    .with_size_hints(&size_hints)
                    .with_minimum_dim(&Client::MIN_CLIENT_DIM);

                    let extents = client.frame_extents();
                    geometry.pos = client.free_region().pos;
                    geometry.dim.w += extents.left + extents.right;
                    geometry.dim.h += extents.top + extents.bottom;

                    client.set_size_hints(size_hints);
                    client.set_region(PlacementClass::Free(geometry));

                    if client.is_managed() {
                        let workspace = client.workspace();
                        self.apply_layout(workspace);
                        self.apply_stack(workspace);
                    }
                }
            },
            PropertyKind::Strut => {
                if let Some(struts) = self.conn.get_window_strut(window) {
                    let screen = self.active_screen();
                    screen.remove_window_strut(window);
                    screen.add_struts(struts);
                    screen.compute_placeable_region();

                    let workspace = self.active_workspace();
                    self.apply_layout(workspace);
                    self.apply_stack(workspace);
                }
            },
        }
    }

    #[inline]
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
                Decoration::NO_DECORATION.extents()
            },
        );
    }

    #[cold]
    fn handle_screen_change(&mut self) {
        debug!("SCREEN_CHANGE");

        self.acquire_partitions();
        self.workspaces
            .activate_for(&Selector::AtIndex(self.active_screen().number()));
    }

    #[cold]
    pub fn exit(&mut self) {
        info!("exit called, shutting down {}", WM_NAME!());

        (0..self.workspaces.len()).for_each(|workspace| {
            self.deiconify_all(workspace);
        });

        self.client_map.iter().for_each(|(&window, client)| {
            self.conn.unparent_window(window, client.free_region().pos);
        });

        self.conn.cleanup();
        self.conn.flush();

        self.running = false;
    }
}
