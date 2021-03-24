use crate::compare::MatchMethod;
use crate::decoration::Decoration;
use crate::identify::Ident;
use crate::identify::Identify;
use crate::placement::PlacementClass;
use crate::zone::ZoneId;

use winsys::connection::Pid;
use winsys::geometry::Extents;
use winsys::geometry::Pos;
use winsys::geometry::Region;
use winsys::hints::SizeHints;
use winsys::window::Window;
use winsys::window::WindowType;

use std::cell::Cell;
use std::cell::RefCell;
use std::time::SystemTime;

pub struct Client {
    zone: ZoneId,
    window: Window,
    frame: Window,
    name: RefCell<String>,
    class: RefCell<String>,
    instance: RefCell<String>,
    context: Cell<usize>,
    workspace: Cell<usize>,
    window_type: WindowType,
    active_region: Cell<Region>,
    previous_region: Cell<Region>,
    inner_region: Cell<Region>,
    free_region: Cell<Region>,
    tile_region: Cell<Region>,
    decoration: Cell<Decoration>,
    size_hints: Cell<Option<SizeHints>>,
    warp_pos: Cell<Option<Pos>>,
    parent: Option<Window>,
    children: RefCell<Vec<Window>>,
    leader: Option<Window>,
    producer: Option<Window>,
    consumers: RefCell<Vec<Window>>,
    focused: Cell<bool>,
    mapped: Cell<bool>,
    managed: Cell<bool>,
    in_window: Cell<bool>,
    floating: Cell<bool>,
    fullscreen: Cell<bool>,
    iconified: Cell<bool>,
    disowned: Cell<bool>,
    sticky: Cell<bool>,
    invincible: Cell<bool>,
    urgent: Cell<bool>,
    consuming: Cell<bool>,
    producing: Cell<bool>,
    pid: Option<Pid>,
    ppid: Option<Pid>,
    last_focused: Cell<SystemTime>,
    managed_since: SystemTime,
    expected_unmap_count: Cell<u8>,
}

impl Identify for Client {
    fn id(&self) -> Ident {
        self.window as Ident
    }
}

impl Client {
    pub fn new(
        zone: ZoneId,
        window: Window,
        frame: Window,
        name: impl Into<String>,
        class: impl Into<String>,
        instance: impl Into<String>,
        window_type: WindowType,
        pid: Option<Pid>,
        ppid: Option<Pid>,
    ) -> Self {
        Self {
            zone,
            window,
            frame,
            name: RefCell::new(name.into()),
            class: RefCell::new(class.into()),
            instance: RefCell::new(instance.into()),
            context: Cell::new(0),
            workspace: Cell::new(0),
            window_type,
            active_region: Cell::new(Default::default()),
            previous_region: Cell::new(Default::default()),
            inner_region: Cell::new(Default::default()),
            free_region: Cell::new(Default::default()),
            tile_region: Cell::new(Default::default()),
            decoration: Cell::new(Default::default()),
            size_hints: Cell::new(None),
            warp_pos: Cell::new(None),
            parent: None,
            children: RefCell::new(Vec::new()),
            leader: None,
            producer: None,
            consumers: RefCell::new(Vec::new()),
            focused: Cell::new(false),
            mapped: Cell::new(false),
            managed: Cell::new(true),
            in_window: Cell::new(false),
            floating: Cell::new(false),
            fullscreen: Cell::new(false),
            iconified: Cell::new(false),
            disowned: Cell::new(false),
            sticky: Cell::new(false),
            invincible: Cell::new(false),
            urgent: Cell::new(false),
            consuming: Cell::new(false),
            producing: Cell::new(true),
            pid,
            ppid,
            last_focused: Cell::new(SystemTime::now()),
            managed_since: SystemTime::now(),
            expected_unmap_count: Cell::new(0),
        }
    }

    #[inline]
    pub fn zone(&self) -> ZoneId {
        self.zone
    }

    #[inline]
    pub fn windows(&self) -> (Window, Window) {
        (self.window, self.frame)
    }

    #[inline]
    pub fn window(&self) -> Window {
        self.window
    }

    #[inline]
    pub fn frame(&self) -> Window {
        self.frame
    }

    #[inline]
    pub fn set_name(
        &self,
        name: impl Into<String>,
    ) {
        self.name.replace(name.into());
    }

    #[inline]
    pub fn name(&self) -> String {
        self.name.borrow().to_owned()
    }

    #[inline]
    pub fn name_matches(
        &self,
        match_method: MatchMethod<&'static str>,
    ) -> bool {
        match match_method {
            MatchMethod::Equals(comp) => &*self.name.borrow() == comp,
            MatchMethod::Contains(comp) => (&*self.name.borrow()).contains(comp),
        }
    }

    #[inline]
    pub fn set_class(
        &self,
        class: impl Into<String>,
    ) {
        self.class.replace(class.into());
    }

    #[inline]
    pub fn class(&self) -> String {
        self.class.borrow().to_owned()
    }

    #[inline]
    pub fn class_matches(
        &self,
        match_method: MatchMethod<&'static str>,
    ) -> bool {
        match match_method {
            MatchMethod::Equals(comp) => &*self.class.borrow() == comp,
            MatchMethod::Contains(comp) => (&*self.class.borrow()).contains(comp),
        }
    }

    #[inline]
    pub fn set_instance(
        &self,
        instance: impl Into<String>,
    ) {
        self.instance.replace(instance.into());
    }

    #[inline]
    pub fn instance(&self) -> String {
        self.instance.borrow().to_owned()
    }

    #[inline]
    pub fn instance_matches(
        &self,
        match_method: MatchMethod<&'static str>,
    ) -> bool {
        match match_method {
            MatchMethod::Equals(comp) => &*self.instance.borrow() == comp,
            MatchMethod::Contains(comp) => (&*self.instance.borrow()).contains(comp),
        }
    }

    #[inline]
    pub fn set_context(
        &self,
        context: usize,
    ) {
        self.context.replace(context);
    }

    #[inline]
    pub fn context(&self) -> usize {
        self.context.get()
    }

    #[inline]
    pub fn set_workspace(
        &self,
        workspace: usize,
    ) {
        self.workspace.replace(workspace);
    }

    #[inline]
    pub fn workspace(&self) -> usize {
        self.workspace.get()
    }

    #[inline]
    pub fn window_type(&self) -> WindowType {
        self.window_type
    }

    #[inline]
    fn set_active_region(
        &self,
        active_region: Region,
    ) {
        self.set_inner_region(active_region);
        let active_region = self.active_region.replace(active_region);
        self.previous_region.replace(active_region);
    }

    #[inline]
    pub fn active_region(&self) -> Region {
        self.active_region.get()
    }

    #[inline]
    pub fn previous_region(&self) -> Region {
        self.previous_region.get()
    }

    #[inline]
    fn set_inner_region(
        &self,
        active_region: Region,
    ) {
        self.inner_region
            .replace(if let Some(frame) = self.decoration.get().frame {
                let mut inner_region = active_region - frame.extents;

                inner_region.pos.x = frame.extents.left;
                inner_region.pos.y = frame.extents.top;

                inner_region.dim.w = active_region.dim.w - frame.extents.left - frame.extents.right;
                inner_region.dim.h = active_region.dim.h - frame.extents.top - frame.extents.bottom;

                inner_region
            } else {
                let mut inner_region = active_region;

                inner_region.pos.x = 0;
                inner_region.pos.y = 0;

                inner_region
            });
    }

    #[inline]
    pub fn set_region(
        &self,
        region: PlacementClass<Region>,
    ) {
        match region {
            PlacementClass::Free(region) => {
                self.free_region.replace(region);
                self.set_active_region(region);
            },
            PlacementClass::Tile(region) => {
                self.tile_region.replace(region);
                self.set_active_region(region);
            },
        }
    }

    #[inline]
    pub fn free_region(&self) -> Region {
        self.free_region.get()
    }

    #[inline]
    pub fn tile_region(&self) -> Region {
        self.tile_region.get()
    }

    #[inline]
    pub fn inner_region(&self) -> Region {
        self.inner_region.get()
    }

    #[inline]
    pub fn set_decoration(
        &self,
        decoration: Decoration,
    ) {
        self.decoration.replace(decoration);
    }

    #[inline]
    pub fn decoration(&self) -> Decoration {
        self.decoration.get().to_owned()
    }

    #[inline]
    pub fn frame_extents(&self) -> Extents {
        Extents {
            left: 0,
            right: 0,
            top: 0,
            bottom: 0,
        } + self.decoration.get().to_owned()
    }

    #[inline]
    pub fn set_size_hints(
        &self,
        size_hints: Option<SizeHints>,
    ) {
        self.size_hints.set(size_hints);
    }

    #[inline]
    pub fn size_hints(&self) -> Option<SizeHints> {
        self.size_hints.get()
    }

    #[inline]
    pub fn set_warp_pos(
        &self,
        pointer_pos: Pos,
    ) {
        self.warp_pos.replace(Some(pointer_pos));
    }

    #[inline]
    pub fn unset_warp_pos(&self) {
        self.warp_pos.replace(None);
    }

    #[inline]
    pub fn warp_pos(&self) -> Option<Pos> {
        self.warp_pos.get().to_owned()
    }

    #[inline]
    pub fn set_parent(
        &mut self,
        parent: Window,
    ) {
        self.parent = Some(parent);
    }

    #[inline]
    pub fn parent(&self) -> Option<Window> {
        self.parent
    }

    #[inline]
    pub fn add_child(
        &self,
        child: Window,
    ) {
        self.children.borrow_mut().push(child);
    }

    #[inline]
    pub fn remove_child(
        &self,
        child: Window,
    ) {
        let mut children = self.children.borrow_mut();
        if let Some(index) = children.iter().rposition(|&c| c == child) {
            children.remove(index);
        }
    }

    #[inline]
    pub fn set_leader(
        &mut self,
        leader: Window,
    ) {
        self.leader = Some(leader);
    }

    #[inline]
    pub fn leader(&self) -> Option<Window> {
        self.leader
    }

    #[inline]
    pub fn set_producer(
        &mut self,
        producer: Window,
    ) {
        self.producer = Some(producer);
    }

    #[inline]
    pub fn unset_producer(&mut self) {
        self.producer = None;
    }

    #[inline]
    pub fn producer(&self) -> Option<Window> {
        self.producer
    }

    #[inline]
    pub fn add_consumer(
        &self,
        consumer: Window,
    ) {
        self.consumers.borrow_mut().push(consumer);
    }

    #[inline]
    pub fn remove_consumer(
        &self,
        consumer: Window,
    ) {
        let mut consumers = self.consumers.borrow_mut();
        if let Some(index) = consumers.iter().rposition(|&c| c == consumer) {
            consumers.remove(index);
        }
    }

    #[inline]
    pub fn consumer_len(&self) -> usize {
        self.consumers.borrow().len()
    }

    #[inline]
    pub fn is_free(&self) -> bool {
        self.floating.get() || self.disowned.get() || !self.managed.get()
    }

    #[inline]
    pub fn set_focused(
        &self,
        focused: bool,
    ) {
        self.focused.set(focused);
    }

    #[inline]
    pub fn is_focused(&self) -> bool {
        self.focused.get()
    }

    #[inline]
    pub fn set_mapped(
        &self,
        mapped: bool,
    ) {
        self.mapped.set(mapped);
    }

    #[inline]
    pub fn is_mapped(&self) -> bool {
        self.mapped.get()
    }

    #[inline]
    pub fn set_managed(
        &self,
        managed: bool,
    ) {
        self.managed.set(managed);
    }

    #[inline]
    pub fn is_managed(&self) -> bool {
        self.managed.get()
    }

    #[inline]
    pub fn set_in_window(
        &self,
        in_window: bool,
    ) {
        self.in_window.set(in_window);
    }

    #[inline]
    pub fn is_in_window(&self) -> bool {
        self.in_window.get()
    }

    #[inline]
    pub fn set_floating(
        &self,
        floating: bool,
    ) {
        self.floating.replace(floating);
    }

    #[inline]
    pub fn is_floating(&self) -> bool {
        self.floating.get()
    }

    #[inline]
    pub fn set_fullscreen(
        &self,
        fullscreen: bool,
    ) {
        self.fullscreen.replace(fullscreen);
    }

    #[inline]
    pub fn is_fullscreen(&self) -> bool {
        self.fullscreen.get()
    }

    #[inline]
    pub fn set_iconified(
        &self,
        iconified: bool,
    ) {
        self.iconified.replace(iconified);
    }

    #[inline]
    pub fn is_iconified(&self) -> bool {
        self.iconified.get()
    }

    #[inline]
    pub fn set_disowned(
        &self,
        disowned: bool,
    ) {
        self.disowned.replace(disowned);
    }

    #[inline]
    pub fn is_disowned(&self) -> bool {
        self.disowned.get()
    }

    #[inline]
    pub fn set_sticky(
        &self,
        sticky: bool,
    ) {
        self.sticky.replace(sticky);
    }

    #[inline]
    pub fn is_sticky(&self) -> bool {
        self.sticky.get()
    }

    #[inline]
    pub fn set_invincible(
        &self,
        invincible: bool,
    ) {
        self.invincible.replace(invincible);
    }

    #[inline]
    pub fn is_invincible(&self) -> bool {
        self.invincible.get()
    }

    #[inline]
    pub fn set_urgent(
        &self,
        urgent: bool,
    ) {
        self.urgent.replace(urgent);
    }

    #[inline]
    pub fn is_urgent(&self) -> bool {
        self.urgent.get()
    }

    #[inline]
    pub fn set_consuming(
        &self,
        consuming: bool,
    ) {
        self.consuming.replace(consuming);
    }

    #[inline]
    pub fn is_consuming(&self) -> bool {
        self.consuming.get()
    }

    #[inline]
    pub fn set_producing(
        &self,
        producing: bool,
    ) {
        self.producing.replace(producing);
    }

    #[inline]
    pub fn is_producing(&self) -> bool {
        self.producing.get()
    }

    #[inline]
    pub fn pid(&self) -> Option<Pid> {
        self.pid
    }

    #[inline]
    pub fn ppid(&self) -> Option<Pid> {
        self.ppid
    }

    #[inline]
    pub fn last_focused(&self) -> SystemTime {
        self.last_focused.get()
    }

    #[inline]
    pub fn managed_since(&self) -> SystemTime {
        self.managed_since
    }

    #[inline]
    pub fn expect_unmap(&self) {
        self.expected_unmap_count
            .replace(self.expected_unmap_count.get() + 1);
    }

    #[inline]
    pub fn consume_unmap_if_expecting(&self) -> bool {
        let expected_unmap_count = self.expected_unmap_count.get();
        let expecting = expected_unmap_count > 0;

        if expecting {
            self.expected_unmap_count.replace(expected_unmap_count - 1);
        }

        expecting
    }

    #[inline]
    pub fn is_expecting_unmap(&self) -> bool {
        self.expected_unmap_count.get() > 0
    }
}

impl PartialEq for Client {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.window == other.window
    }
}

pub struct Hex32(pub u32);

impl std::fmt::Debug for Hex32 {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{:#0x}", &self.0)
    }
}

impl std::fmt::Debug for Client {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("window", &Hex32(self.window))
            .field("frame", &Hex32(self.frame))
            .field("name", &self.name)
            .field("class", &self.class)
            .field("instance", &self.instance)
            .field("context", &self.context)
            .field("workspace", &self.workspace)
            .field("window_type", &self.window_type)
            .field("active_region", &self.active_region)
            .field("previous_region", &self.previous_region)
            .field("inner_region", &self.inner_region)
            .field("free_region", &self.free_region)
            .field("tile_region", &self.tile_region)
            .field("decoration", &self.decoration)
            .field("size_hints", &self.size_hints)
            .field("warp_pos", &self.warp_pos)
            .field("parent", &self.parent.map(|parent| Hex32(parent)))
            .field(
                "children",
                &self
                    .children
                    .borrow()
                    .iter()
                    .map(|&child| Hex32(child))
                    .collect::<Vec<Hex32>>(),
            )
            .field("leader", &self.leader)
            .field("producer", &self.producer)
            .field("consumers", &self.consumers)
            .field("focused", &self.focused)
            .field("mapped", &self.mapped)
            .field("managed", &self.managed)
            .field("in_window", &self.in_window)
            .field("floating", &self.floating)
            .field("fullscreen", &self.fullscreen)
            .field("iconified", &self.iconified)
            .field("disowned", &self.disowned)
            .field("sticky", &self.sticky)
            .field("invincible", &self.invincible)
            .field("urgent", &self.urgent)
            .field("consuming", &self.consuming)
            .field("pid", &self.pid)
            .field("ppid", &self.ppid)
            .field("last_focused", &self.last_focused)
            .field("managed_since", &self.managed_since)
            .field("expected_unmap_count", &self.expected_unmap_count)
            .finish()
    }
}
