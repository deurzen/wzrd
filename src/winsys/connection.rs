use crate::common::Dim;
use crate::common::Extents;
use crate::common::Hints;
use crate::common::IcccmWindowState;
use crate::common::Pid;
use crate::common::Pos;
use crate::common::Region;
use crate::common::SizeHints;
use crate::common::Strut;
use crate::common::Window;
use crate::common::WindowState;
use crate::common::WindowType;
use crate::event::Event;
use crate::input::*;
use crate::screen::Screen;
use crate::Result;

use std::collections::HashMap;

pub trait Connection {
    fn flush(&self) -> bool;
    fn step(&self) -> Option<Event>;
    fn connected_outputs(&self) -> Vec<Screen>;
    fn top_level_windows(&self) -> Vec<Window>;
    fn get_pointer_position(&self) -> Pos;
    fn warp_pointer_center_of_window_or_root(
        &self,
        window: Option<Window>,
        screen: &Screen,
    );
    fn warp_pointer(
        &self,
        pos: Pos,
    );
    fn warp_pointer_rpos(
        &self,
        window: Window,
        pos: Pos,
    );
    fn confine_pointer(
        &mut self,
        window: Window,
    );
    fn release_pointer(&mut self);
    fn is_mapping_request(
        &self,
        request: u8,
    ) -> bool;
    fn cleanup(&self);

    // Window manipulation
    fn create_frame(
        &self,
        region: Region,
    ) -> Window;
    fn create_handle(&self) -> Window;
    fn init_window(
        &self,
        window: Window,
        focus_follows_mouse: bool,
    );
    fn init_frame(
        &self,
        window: Window,
        focus_follows_mouse: bool,
    );
    fn init_unmanaged(
        &self,
        window: Window,
    );
    fn cleanup_window(
        &self,
        window: Window,
    );
    fn map_window(
        &self,
        window: Window,
    );
    fn unmap_window(
        &self,
        window: Window,
    );
    fn reparent_window(
        &self,
        window: Window,
        parent: Window,
        pos: Pos,
    );
    fn unparent_window(
        &self,
        window: Window,
        pos: Pos,
    );
    fn destroy_window(
        &self,
        window: Window,
    );
    fn close_window(
        &self,
        window: Window,
    ) -> bool;
    fn kill_window(
        &self,
        window: Window,
    ) -> bool;
    fn place_window(
        &self,
        window: Window,
        region: &Region,
    );
    fn move_window(
        &self,
        window: Window,
        pos: Pos,
    );
    fn resize_window(
        &self,
        window: Window,
        dim: Dim,
    );
    fn focus_window(
        &self,
        window: Window,
    );
    fn stack_window_above(
        &self,
        window: Window,
        sibling: Option<Window>,
    );
    fn stack_window_below(
        &self,
        window: Window,
        sibling: Option<Window>,
    );
    fn insert_window_in_save_set(
        &self,
        window: Window,
    );
    fn grab_bindings(
        &self,
        key_codes: &[KeyCode],
        mouse_bindings: &[&(MouseEventKey, MouseShortcut)],
    );
    fn regrab_buttons(
        &self,
        window: Window,
    );
    fn ungrab_buttons(
        &self,
        window: Window,
    );
    fn unfocus(&self);
    fn set_window_border_width(
        &self,
        window: Window,
        width: u32,
    );
    fn set_window_border_color(
        &self,
        window: Window,
        color: u32,
    );
    fn set_window_background_color(
        &self,
        window: Window,
        color: u32,
    );
    fn update_window_offset(
        &self,
        window: Window,
        frame: Window,
    );
    fn get_focused_window(&self) -> Window;
    fn get_window_geometry(
        &self,
        window: Window,
    ) -> Result<Region>;
    fn get_window_pid(
        &self,
        window: Window,
    ) -> Option<Pid>;
    fn must_manage_window(
        &self,
        window: Window,
    ) -> bool;
    fn must_free_window(
        &self,
        window: Window,
    ) -> bool;
    fn window_is_mappable(
        &self,
        window: Window,
    ) -> bool;

    // ICCCM
    fn set_icccm_window_state(
        &self,
        window: Window,
        state: IcccmWindowState,
    );
    fn set_icccm_window_hints(
        &self,
        window: Window,
        hints: Hints,
    );
    fn get_icccm_window_name(
        &self,
        window: Window,
    ) -> String;
    fn get_icccm_window_class(
        &self,
        window: Window,
    ) -> String;
    fn get_icccm_window_instance(
        &self,
        window: Window,
    ) -> String;
    fn get_icccm_window_transient_for(
        &self,
        window: Window,
    ) -> Option<Window>;
    fn get_icccm_window_client_leader(
        &self,
        window: Window,
    ) -> Option<Window>;
    fn get_icccm_window_hints(
        &self,
        window: Window,
    ) -> Option<Hints>;
    fn get_icccm_window_size_hints(
        &self,
        window: Window,
        min_window_dim: Option<Dim>,
        current_size_hints: &Option<SizeHints>,
    ) -> (bool, Option<SizeHints>);

    // EWMH
    fn init_wm_properties(
        &self,
        wm_name: &str,
        desktop_names: &[&str],
    );
    fn set_current_desktop(
        &self,
        index: usize,
    );
    fn set_root_window_name(
        &self,
        name: &str,
    );
    fn set_window_desktop(
        &self,
        window: Window,
        index: usize,
    );
    fn set_window_fullscreen(
        &self,
        window: Window,
        on: bool,
    );
    fn set_window_above(
        &self,
        window: Window,
        on: bool,
    );
    fn set_window_below(
        &self,
        window: Window,
        on: bool,
    );
    fn set_window_state(
        &self,
        window: Window,
        state: WindowState,
        on: bool,
    );
    fn set_window_frame_extents(
        &self,
        window: Window,
        extents: Extents,
    );
    fn set_desktop_geometry(
        &self,
        geometries: &[&Region],
    );
    fn set_desktop_viewport(
        &self,
        viewports: &[&Region],
    );
    fn set_workarea(
        &self,
        workareas: &[&Region],
    );
    fn update_desktops(
        &self,
        desktop_names: &[&str],
    );
    fn update_client_list(
        &self,
        clients: &[Window],
    );
    fn update_client_list_stacking(
        &self,
        clients: &[Window],
    );
    fn get_window_strut(
        &self,
        window: Window,
    ) -> Option<Vec<Option<Strut>>>;
    fn get_window_strut_partial(
        &self,
        window: Window,
    ) -> Option<Vec<Option<Strut>>>;
    fn get_window_desktop(
        &self,
        window: Window,
    ) -> Option<usize>;
    fn get_window_preferred_type(
        &self,
        window: Window,
    ) -> WindowType;
    fn get_window_types(
        &self,
        window: Window,
    ) -> Vec<WindowType>;
    fn get_window_preferred_state(
        &self,
        window: Window,
    ) -> Option<WindowState>;
    fn get_window_states(
        &self,
        window: Window,
    ) -> Vec<WindowState>;
    fn window_is_fullscreen(
        &self,
        window: Window,
    ) -> bool;
    fn window_is_above(
        &self,
        window: Window,
    ) -> bool;
    fn window_is_below(
        &self,
        window: Window,
    ) -> bool;
    fn window_is_sticky(
        &self,
        window: Window,
    ) -> bool;
}
