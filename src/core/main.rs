#![warn(clippy::all)]
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(missing_debug_implementations)]
#![warn(rust_2018_idioms)]
#![warn(trivial_numeric_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unsafe_code)]
#![warn(unused_extern_crates)]
#![warn(unused_import_braces)]
#![warn(unused_qualifications)]
#![allow(dead_code)]
#![recursion_limit = "256"]

#[macro_use]
extern crate log;

#[allow(unused_imports)]
use simplelog::LevelFilter;
#[allow(unused_imports)]
use simplelog::SimpleLogger;

use winsys::geometry::Edge;
use winsys::xdata::xconnection::XConnection;
pub use winsys::Result;

use std::collections::HashMap;
use std::collections::HashSet;

#[macro_use]
mod macros;

#[macro_use]
mod defaults;

mod binding;
mod change;
mod client;
mod compare;
mod consume;
mod cycle;
mod decoration;
mod error;
mod identify;
mod jump;
mod layout;
mod model;
mod partition;
mod placement;
mod rule;
mod stack;
mod util;
mod workspace;
mod zone;

use binding::KeyBindings;
use binding::MouseBindings;
use change::Change;
use change::Direction;
use change::Toggle;
use compare::MatchMethod;
use jump::JumpCriterium;
use layout::LayoutKind;
use model::Model;
use winsys::input::Button;
use winsys::input::Key;
use winsys::input::KeyInput;
use winsys::input::Modifier;
use winsys::input::MouseEventKind;
use winsys::input::MouseInput;
use winsys::input::MouseInputTarget;
use winsys::window::Window;
use workspace::ClientSelector;

pub fn main() -> Result<()> {
    #[cfg(debug_assertions)]
    SimpleLogger::init(LevelFilter::Debug, simplelog::Config::default())?;

    let (conn, screen_num) = x11rb::connect(None)?;
    let (mouse_bindings, key_bindings) = init_bindings();

    Model::new(
        &mut XConnection::new(&conn, screen_num)?,
        &key_bindings,
        &mouse_bindings,
    )
    .run(key_bindings, mouse_bindings);

    Ok(())
}

fn init_bindings() -> (MouseBindings, KeyBindings) {
    let mut mouse_bindings = MouseBindings::new();
    let mut key_bindings = KeyBindings::new();

    mouse_bindings.insert(
        MouseInput {
            target: MouseInputTarget::Client,
            button: Button::Right,
            modifiers: hashset!(Modifier::Alt, Modifier::Ctrl),
        },
        |model: &mut Model<'_>, window: Option<Window>| -> bool {
            if let Some(window) = window {
                model.set_floating_window(window, Toggle::Reverse);
            }

            true
        },
    );

    mouse_bindings.insert(
        MouseInput {
            target: MouseInputTarget::Client,
            button: Button::Middle,
            modifiers: hashset!(Modifier::Alt, Modifier::Ctrl, Modifier::Shift),
        },
        |model: &mut Model<'_>, window: Option<Window>| -> bool {
            if let Some(window) = window {
                model.set_fullscreen_window(window, Toggle::Reverse);
            }

            true
        },
    );

    mouse_bindings.insert(
        MouseInput {
            target: MouseInputTarget::Client,
            button: Button::Middle,
            modifiers: hashset!(Modifier::Alt),
        },
        |model: &mut Model<'_>, window: Option<Window>| -> bool {
            if let Some(window) = window {
                model.center_window(window);
            }

            true
        },
    );

    mouse_bindings.insert(
        MouseInput {
            target: MouseInputTarget::Client,
            button: Button::Left,
            modifiers: hashset!(Modifier::Alt),
        },
        |model: &mut Model<'_>, window: Option<Window>| -> bool {
            if let Some(window) = window {
                model.start_moving(window);
            }

            true
        },
    );

    mouse_bindings.insert(
        MouseInput {
            target: MouseInputTarget::Client,
            button: Button::Right,
            modifiers: hashset!(Modifier::Alt),
        },
        |model: &mut Model<'_>, window: Option<Window>| -> bool {
            if let Some(window) = window {
                model.start_resizing(window);
            }

            true
        },
    );

    mouse_bindings.insert(
        MouseInput {
            target: MouseInputTarget::Client,
            button: Button::ScrollDown,
            modifiers: hashset!(Modifier::Alt, Modifier::Ctrl, Modifier::Shift),
        },
        |model: &mut Model<'_>, window: Option<Window>| -> bool {
            if let Some(window) = window {
                model.grow_ratio_window(window, -15);
            }

            true
        },
    );

    mouse_bindings.insert(
        MouseInput {
            target: MouseInputTarget::Client,
            button: Button::ScrollUp,
            modifiers: hashset!(Modifier::Alt, Modifier::Ctrl, Modifier::Shift),
        },
        |model: &mut Model<'_>, window: Option<Window>| -> bool {
            if let Some(window) = window {
                model.grow_ratio_window(window, 15);
            }

            true
        },
    );

    mouse_bindings.insert(
        MouseInput {
            target: MouseInputTarget::Client,
            button: Button::ScrollUp,
            modifiers: hashset!(Modifier::Alt),
        },
        |model: &mut Model<'_>, _: Option<Window>| -> bool {
            model.cycle_focus(Direction::Backward);
            false
        }
    );

    mouse_bindings.insert(
        MouseInput {
            target: MouseInputTarget::Client,
            button: Button::ScrollDown,
            modifiers: hashset!(Modifier::Alt),
        },
        |model: &mut Model<'_>, _: Option<Window>| -> bool {
            model.cycle_focus(Direction::Forward);
            false
        }
    );

    mouse_bindings.insert(
        MouseInput {
            target: MouseInputTarget::Global,
            button: Button::ScrollUp,
            modifiers: hashset!(Modifier::Alt, Modifier::Shift),
        },
        |model: &mut Model<'_>, _: Option<Window>| -> bool {
            model.activate_next_workspace(Direction::Backward);
            false
        }
    );

    mouse_bindings.insert(
        MouseInput {
            target: MouseInputTarget::Global,
            button: Button::ScrollDown,
            modifiers: hashset!(Modifier::Alt, Modifier::Shift),
        },
        |model: &mut Model<'_>, _: Option<Window>| -> bool {
            model.activate_next_workspace(Direction::Forward);
            false
        }
    );

    mouse_bindings.insert(
        MouseInput {
            target: MouseInputTarget::Client,
            button: Button::Backward,
            modifiers: hashset!(Modifier::Alt),
        },
        |model: &mut Model<'_>, window: Option<Window>| -> bool {
            if let Some(window) = window {
                model.move_window_to_next_workspace(window, Direction::Backward);
            }

            false
        }
    );

    mouse_bindings.insert(
        MouseInput {
            target: MouseInputTarget::Client,
            button: Button::Forward,
            modifiers: hashset!(Modifier::Alt),
        },
        |model: &mut Model<'_>, window: Option<Window>| -> bool {
            if let Some(window) = window {
                model.move_window_to_next_workspace(window, Direction::Forward);
            }

            false
        }
    );

    key_bindings.insert(
        KeyInput {
            key: Key::Escape,
            modifiers: hashset!(Modifier::Alt, Modifier::Ctrl, Modifier::Shift),
        },
        |model: &mut Model<'_>| {
            model.exit();
        }
    );

    key_bindings.insert(
        KeyInput {
            key: Key::J,
            modifiers: hashset!(Modifier::Alt),
        },
        |model: &mut Model<'_>| {
            model.cycle_focus(Direction::Forward);
        }
    );

    key_bindings.insert(
        KeyInput {
            key: Key::K,
            modifiers: hashset!(Modifier::Alt),
        },
        |model: &mut Model<'_>| {
            model.cycle_focus(Direction::Backward);
        }
    );

    // // (kind, target, focus): "[modifiers]-button" => action
    // let mouse_bindings = build_mouse_bindings!(
    //     }),

    // // "[modifiers]-key" => action
    // let key_bindings = build_key_bindings!(
    //     "1-C-S-q" => do_internal!(exit),

    //     // client state modifiers
    //     "1-c" => do_internal!(kill_focus),
    //     "1-S-space" => do_internal!(set_floating_focus, Toggle::Reverse),
    //     "1-f" => do_internal!(set_fullscreen_focus, Toggle::Reverse),
    //     "1-x" => do_internal!(set_stick_focus, Toggle::Reverse),
    //     "1-2-C-f" => do_internal!(set_contained_focus, Toggle::Reverse),
    //     "1-2-C-i" => do_internal!(set_invincible_focus, Toggle::Reverse),
    //     "1-2-C-p" => do_internal!(set_producing_focus, Toggle::Reverse),
    //     "1-2-C-y" => do_internal!(set_iconifyable_focus, Toggle::Reverse),
    //     "1-y" => do_internal!(set_iconify_focus, Toggle::On),
    //     "1-u" => do_internal!(pop_deiconify),
    //     "1-2-u" => do_internal_block!(model, {
    //         model.deiconify_all(model.active_workspace());
    //     }),

    //     // free client arrangers
    //     "1-C-space" => do_internal!(center_focus),
    //     "1-C-h" => do_internal!(nudge_focus, Edge::Left, 15),
    //     "1-C-j" => do_internal!(nudge_focus, Edge::Bottom, 15),
    //     "1-C-k" => do_internal!(nudge_focus, Edge::Top, 15),
    //     "1-C-l" => do_internal!(nudge_focus, Edge::Right, 15),
    //     "1-C-S-h" => do_internal!(stretch_focus, Edge::Left, 15),
    //     "1-C-S-j" => do_internal!(stretch_focus, Edge::Bottom, 15),
    //     "1-C-S-k" => do_internal!(stretch_focus, Edge::Top, 15),
    //     "1-C-S-l" => do_internal!(stretch_focus, Edge::Right, 15),
    //     "1-C-S-y" => do_internal!(stretch_focus, Edge::Left, -15),
    //     "1-C-S-u" => do_internal!(stretch_focus, Edge::Bottom, -15),
    //     "1-C-S-i" => do_internal!(stretch_focus, Edge::Top, -15),
    //     "1-C-S-o" => do_internal!(stretch_focus, Edge::Right, -15),
    //     "1-C-Left" => do_internal!(snap_focus, Edge::Left),
    //     "1-C-Down" => do_internal!(snap_focus, Edge::Bottom),
    //     "1-C-Up" => do_internal!(snap_focus, Edge::Top),
    //     "1-C-Right" => do_internal!(snap_focus, Edge::Right),

    //     // client order modifiers
    //     "1-j" => do_internal!(cycle_focus, Direction::Forward),
    //     "1-k" => do_internal!(cycle_focus, Direction::Backward),
    //     "1-S-j" => do_internal!(drag_focus, Direction::Forward),
    //     "1-S-k" => do_internal!(drag_focus, Direction::Backward),
    //     "1-S-semicolon" => do_internal!(rotate_clients, Direction::Forward),
    //     "1-S-comma" => do_internal!(rotate_clients, Direction::Backward),

    //     // zone creators
    //     "1-C-Return" => do_internal!(create_layout_zone),
    //     "1-C-S-Return" => do_internal!(create_tab_zone),
    //     "1-C-C" => do_internal!(delete_zone),

    //     // zone order modifiers
    //     // "1-C-j" => do_internal!(cycle_zones, Direction::Forward),
    //     // "1-C-k" => do_internal!(cycle_zones, Direction::Backward),

    //     // active workspace layout modifiers
    //     "1-S-f" => do_internal!(set_layout, LayoutKind::Float),
    //     "1-S-l" => do_internal!(set_layout, LayoutKind::BLFloat),
    //     "1-z" => do_internal!(set_layout, LayoutKind::SingleFloat),
    //     "1-S-z" => do_internal!(set_layout, LayoutKind::BLSingleFloat),
    //     "1-m" => do_internal!(set_layout, LayoutKind::Monocle),
    //     "1-g" => do_internal!(set_layout, LayoutKind::Center),
    //     "1-t" => do_internal!(set_layout, LayoutKind::Stack),
    //     "1-S-t" => do_internal!(set_layout, LayoutKind::SStack),
    //     "1-C-S-p" => do_internal!(set_layout, LayoutKind::Paper),
    //     "1-2-C-S-p" => do_internal!(set_layout, LayoutKind::SPaper),
    //     "1-C-S-b" => do_internal!(set_layout, LayoutKind::BStack),
    //     "1-2-C-S-b" => do_internal!(set_layout, LayoutKind::SBStack),
    //     "1-S-y" => do_internal!(set_layout, LayoutKind::Horz),
    //     "1-C-y" => do_internal!(set_layout, LayoutKind::SHorz),
    //     "1-S-v" => do_internal!(set_layout, LayoutKind::Vert),
    //     "1-C-v" => do_internal!(set_layout, LayoutKind::SVert),
    //     "1-C-S-f" => do_internal!(apply_float_retain_region),
    //     "1-space" => do_internal!(toggle_layout),

    //     // active workspace layout data modifiers
    //     "1-plus" => do_internal!(change_gap_size, Change::Inc(5u32)),
    //     "1-minus" => do_internal!(change_gap_size, Change::Dec(5u32)),
    //     "1-S-equal" => do_internal!(reset_gap_size),
    //     "1-i" => do_internal!(change_main_count, Change::Inc(1u32)),
    //     "1-d" => do_internal!(change_main_count, Change::Dec(1u32)),
    //     "1-l" => do_internal!(change_main_factor, Change::Inc(0.05f32)),
    //     "1-h" => do_internal!(change_main_factor, Change::Dec(0.05f32)),
    //     "1-S-Left" => do_internal!(change_margin, Edge::Left, Change::Inc(5i32)),
    //     "1-C-S-Left" => do_internal!(change_margin, Edge::Left, Change::Dec(5i32)),
    //     "1-S-Up" => do_internal!(change_margin, Edge::Top, Change::Inc(5i32)),
    //     "1-C-S-Up" => do_internal!(change_margin, Edge::Top, Change::Dec(5i32)),
    //     "1-S-Down" => do_internal!(change_margin, Edge::Bottom, Change::Inc(5i32)),
    //     "1-C-S-Down" => do_internal!(change_margin, Edge::Bottom, Change::Dec(5i32)),
    //     "1-S-Right" => do_internal!(change_margin, Edge::Right, Change::Inc(5i32)),
    //     "1-C-S-Right" => do_internal!(change_margin, Edge::Right, Change::Dec(5i32)),
    //     "1-C-S-equal" => do_internal!(reset_margin),
    //     "1-2-C-S-l" => do_internal!(copy_prev_layout_data),
    //     "1-2-C-S-equal" => do_internal!(reset_layout_data),

    //     // workspace activators
    //     "1-Escape" => do_internal!(toggle_workspace),
    //     "1-bracketleft" => do_internal!(activate_next_workspace, Direction::Backward),
    //     "1-bracketright" => do_internal!(activate_next_workspace, Direction::Forward),
    //     "1-1" => do_internal!(activate_workspace, 0),
    //     "1-2" => do_internal!(activate_workspace, 1),
    //     "1-3" => do_internal!(activate_workspace, 2),
    //     "1-4" => do_internal!(activate_workspace, 3),
    //     "1-5" => do_internal!(activate_workspace, 4),
    //     "1-6" => do_internal!(activate_workspace, 5),
    //     "1-7" => do_internal!(activate_workspace, 6),
    //     "1-8" => do_internal!(activate_workspace, 7),
    //     "1-9" => do_internal!(activate_workspace, 8),
    //     "1-0" => do_internal!(activate_workspace, 9),

    //     // workspace client movers
    //     "1-S-bracketleft" => do_internal!(move_focus_to_next_workspace, Direction::Backward),
    //     "1-S-bracketright" => do_internal!(move_focus_to_next_workspace, Direction::Forward),
    //     "1-S-1" => do_internal!(move_focus_to_workspace, 0),
    //     "1-S-2" => do_internal!(move_focus_to_workspace, 1),
    //     "1-S-3" => do_internal!(move_focus_to_workspace, 2),
    //     "1-S-4" => do_internal!(move_focus_to_workspace, 3),
    //     "1-S-5" => do_internal!(move_focus_to_workspace, 4),
    //     "1-S-6" => do_internal!(move_focus_to_workspace, 5),
    //     "1-S-7" => do_internal!(move_focus_to_workspace, 6),
    //     "1-S-8" => do_internal!(move_focus_to_workspace, 7),
    //     "1-S-9" => do_internal!(move_focus_to_workspace, 8),
    //     "1-S-0" => do_internal!(move_focus_to_workspace, 9),

    //     // placeable region modifiers
    //     "1-v" => do_internal!(toggle_screen_struts),

    //     // client jump criteria
    //     "1-b" => do_internal!(jump_client,
    //         JumpCriterium::ByClass(MatchMethod::Equals("qutebrowser"))
    //     ),
    //     "1-S-b" => do_internal!(jump_client,
    //         JumpCriterium::ByClass(MatchMethod::Equals("Firefox"))
    //     ),
    //     "1-C-b" => do_internal!(jump_client,
    //         JumpCriterium::ByClass(MatchMethod::Equals("Chromium"))
    //     ),
    //     "1-2-space" => do_internal!(jump_client,
    //         JumpCriterium::ByClass(MatchMethod::Equals("Spotify"))
    //     ),
    //     "1-e" => do_internal_block!(model, {
    //         model.jump_client(JumpCriterium::ByName(
    //             MatchMethod::Contains("[vim]"),
    //         ));
    //     }),
    //     "1-slash" => do_internal_block!(model, {
    //         model.jump_client(JumpCriterium::OnWorkspaceBySelector(
    //             model.active_workspace(),
    //             &ClientSelector::Last,
    //         ));
    //     }),
    //     "1-period" => do_internal_block!(model, {
    //         model.jump_client(JumpCriterium::OnWorkspaceBySelector(
    //             model.active_workspace(),
    //             &ClientSelector::AtMaster,
    //         ));
    //     }),
    //     "1-comma" => do_internal_block!(model, {
    //         model.jump_client(JumpCriterium::OnWorkspaceBySelector(
    //             model.active_workspace(),
    //             &ClientSelector::First,
    //         ));
    //     }),

    //     // external spawn commands
    //     "XF86AudioPlay", "1-2-p" => spawn_external!("playerctl play-pause"),
    //     "XF86AudioPrev", "1-2-k" => spawn_external!("playerctl previous"),
    //     "XF86AudioNext", "1-2-j" => spawn_external!("playerctl next"),
    //     "1-2-BackSpace" => spawn_external!("playerctl stop"),
    //     "XF86AudioMute" => spawn_external!("amixer -D pulse sset Master toggle"),
    //     "XF86AudioLowerVolume" => spawn_external!("amixer -D pulse sset Master 5%-"),
    //     "XF86AudioRaiseVolume" => spawn_external!("amixer -D pulse sset Master 5%+"),

    //     "1-Return" => spawn_external!("st"),
    //     "1-S-Return" => spawn_external!(concat!("st -n ", WM_NAME!(), ":cf")),

    //     "1-p" => spawn_external!("dmenu_run"),
    //     "1-q" => spawn_external!("qutebrowser"),
    //     "1-S-q" => spawn_external!("firefox"),
    //     "1-C-q" => spawn_external!("chromium"),

    //     "1-C-e" => spawn_external!("st -g 140x42 -e zsh -i -c neomutt"),
    //     "1-C-s" => spawn_external!("st -g 80x42 -e zsh -i -c sncli"),
    //     "1-C-i" => spawn_external!("st -g 80x42 -e zsh -i -c irssi"),

    //     "S-XF86AudioMute" => spawn_external!("amixer -D pulse sset Capture toggle"),
    //     "XF86AudioMicMute" => spawn_external!("amixer -D pulse sset Capture toggle"),

    //     // external shell commands
    //     "1-S-p" => spawn_from_shell!("$HOME/bin/dmenupass"),
    //     "1-C-p" => spawn_from_shell!("$HOME/bin/dmenupass --copy"),
    //     "1-S-o" => spawn_from_shell!("$HOME/bin/dmenunotify"),
    //     "Print", "1-2-slash" => spawn_from_shell!(
    //         "maim -u -m 1 -s \
    //         $(date +$HOME/screenshots/scrots/SS_%Y-%h-%d_%H-%M-%S.png)"
    //     ),
    //     "S-Print", "1-2-S-slash" => spawn_from_shell!(
    //         "maim -u -m 1 \
    //         $(date +$HOME/screenshots/scrots/SS_%Y-%h-%d_%H-%M-%S.png)"
    //     ),
    // );

    (mouse_bindings, key_bindings)
}
