#![deny(clippy::all)]
#![allow(dead_code)]

#[macro_use]
extern crate log;

#[allow(unused_imports)]
use simplelog::LevelFilter;
#[allow(unused_imports)]
use simplelog::SimpleLogger;

use winsys::common::Edge;
use winsys::xdata::xconnection::XConnection;
pub use winsys::Result;

#[macro_use]
mod macros;

#[macro_use]
mod common;

mod binding;
mod client;
mod consume;
mod cycle;
mod jump;
mod layout;
mod model;
mod partition;
mod rule;
mod stack;
mod util;
mod workspace;
mod zone;

use binding::KeyBindings;
use binding::MouseBindings;
use common::Change;
use common::Direction;
use jump::JumpCriterium;
use jump::MatchMethod;
use layout::LayoutKind;
use model::Model;
use workspace::ClientSelector;

pub fn main() -> Result<()> {
    #[cfg(debug_assertions)]
    SimpleLogger::init(LevelFilter::Debug, simplelog::Config::default())?;

    let (conn, screen_num) = x11rb::connect(None)?;
    let mut xconn = XConnection::new(&conn, screen_num)?;

    let (mouse_bindings, key_bindings) = init_bindings();

    Model::new(&mut xconn, &key_bindings, &mouse_bindings)
        .run(key_bindings, mouse_bindings);

    Ok(())
}

fn init_bindings() -> (MouseBindings, KeyBindings) {
    // (kind, target, focus): "[modifiers]-button" => action
    let mouse_bindings = build_mouse_bindings!(
        // client state modifiers
        (Press, Client, true):
        "1-C-Right" => do_internal_mouse_block!(model, window, {
            if let Some(window) = window {
                model.toggle_float_client(window);
            }
        }),
        (Press, Client, true):
        "1-C-S-Middle" => do_internal_mouse_block!(model, window, {
            if let Some(window) = window {
                model.toggle_fullscreen_client(window);
            }
        }),

        // free client arrangers
        (Press, Client, true):
        "1-Middle" => do_internal_mouse_block!(model, window, {
            if let Some(window) = window {
                model.center_client(window);
            }
        }),
        (Press, Client, true):
        "1-Left" => do_internal_mouse_block!(model, window, {
            if let Some(window) = window {
                model.start_moving(window);
            }
        }),
        (Press, Client, true):
        "1-Right" => do_internal_mouse_block!(model, window, {
            if let Some(window) = window {
                model.start_resizing(window);
            }
        }),
        (Press, Client, false):
        "1-C-S-ScrollDown" => do_internal_mouse_block!(model, window, {
            if let Some(window) = window {
                model.grow_ratio_client(window, -15);
            }
        }),
        (Press, Client, false):
        "1-C-S-ScrollUp" => do_internal_mouse_block!(model, window, {
            if let Some(window) = window {
                model.grow_ratio_client(window, 15);
            }
        }),

        // client order modifiers
        (Press, Global, false):
        "1-ScrollDown" => do_internal_mouse!(cycle_focus, Direction::Forward),
        (Press, Global, false):
        "1-ScrollUp" => do_internal_mouse!(cycle_focus, Direction::Backward),

        // workspace activators
        (Press, Global, false):
        "1-S-ScrollDown" => do_internal_mouse!(activate_next_workspace),
        (Press, Global, false):
        "1-S-ScrollUp" => do_internal_mouse!(activate_prev_workspace),

        // workspace client movement
        (Press, Client, false):
        "1-Forward" => do_internal_mouse_block!(model, window, {
            if let Some(window) = window {
                model.move_client_to_next_workspace(window);
            }
        }),
        (Press, Client, false):
        "1-Backward" => do_internal_mouse_block!(model, window, {
            if let Some(window) = window {
                model.move_client_to_prev_workspace(window);
            }
        }),

        // NOPs
        (Release, Global, false):
        "1-ScrollDown" => do_nothing!(),
        (Release, Global, false):
        "1-ScrollUp" => do_nothing!(),
    );

    // "[modifiers]-key" => action
    let key_bindings = build_key_bindings!(
        "1-C-S-q" => do_internal!(exit),

        // client state modifiers
        "1-c" => do_internal!(kill_focus),
        "1-S-space" => do_internal!(toggle_float_focus),
        "1-f" => do_internal!(toggle_fullscreen_focus),
        "1-x" => do_internal!(toggle_stick_focus),
        "1-2-C-f" => do_internal!(toggle_in_window_focus),
        "1-2-C-i" => do_internal!(toggle_invincible_focus),
        "1-2-C-p" => do_internal!(toggle_producing_focus),
        "1-y" => do_internal!(iconify_focus),
        "1-u" => do_internal!(pop_deiconify),
        "1-2-u" => do_internal_block!(model, {
            let workspace_index = model.active_workspace();
            model.deiconify_all(workspace_index);
        }),

        // free client arrangers
        "1-C-space" => do_internal!(center_focus),
        "1-C-h" => do_internal!(nudge_focus, Edge::Left, 15),
        "1-C-j" => do_internal!(nudge_focus, Edge::Bottom, 15),
        "1-C-k" => do_internal!(nudge_focus, Edge::Top, 15),
        "1-C-l" => do_internal!(nudge_focus, Edge::Right, 15),
        "1-C-S-h" => do_internal!(stretch_focus, Edge::Left, 15),
        "1-C-S-j" => do_internal!(stretch_focus, Edge::Bottom, 15),
        "1-C-S-k" => do_internal!(stretch_focus, Edge::Top, 15),
        "1-C-S-l" => do_internal!(stretch_focus, Edge::Right, 15),
        "1-C-S-y" => do_internal!(stretch_focus, Edge::Left, -15),
        "1-C-S-u" => do_internal!(stretch_focus, Edge::Bottom, -15),
        "1-C-S-i" => do_internal!(stretch_focus, Edge::Top, -15),
        "1-C-S-o" => do_internal!(stretch_focus, Edge::Right, -15),
        "1-C-Left" => do_internal!(snap_focus, Edge::Left),
        "1-C-Down" => do_internal!(snap_focus, Edge::Bottom),
        "1-C-Up" => do_internal!(snap_focus, Edge::Top),
        "1-C-Right" => do_internal!(snap_focus, Edge::Right),

        // client order modifiers
        "1-j" => do_internal!(cycle_focus, Direction::Forward),
        "1-k" => do_internal!(cycle_focus, Direction::Backward),
        "1-S-j" => do_internal!(drag_focus, Direction::Forward),
        "1-S-k" => do_internal!(drag_focus, Direction::Backward),
        "1-S-semicolon" => do_internal!(rotate_clients, Direction::Forward),
        "1-S-comma" => do_internal!(rotate_clients, Direction::Backward),

        // active workspace layout setters
        "1-m" => do_internal!(set_layout, LayoutKind::Monocle),
        "1-t" => do_internal!(set_layout, LayoutKind::Stack),
        "1-g" => do_internal!(set_layout, LayoutKind::Center),
        "1-z" => do_internal!(set_layout, LayoutKind::SingleFloat),
        "1-S-f" => do_internal!(set_layout, LayoutKind::Float),
        "1-C-S-f" => do_internal!(apply_float_retain_region),
        "1-S-t" => do_internal!(set_layout, LayoutKind::SStack),
        "1-C-S-p" => do_internal!(set_layout, LayoutKind::Paper),
        "1-space" => do_internal!(toggle_layout),

        // active workspace layout-data modifiers
        "1-plus" => do_internal!(change_gap_size, Change::Inc),
        "1-minus" => do_internal!(change_gap_size, Change::Dec),
        "1-S-equal" => do_internal!(reset_gap_size),
        "1-i" => do_internal!(change_main_count, Change::Inc),
        "1-d" => do_internal!(change_main_count, Change::Dec),
        "1-l" => do_internal!(change_main_factor, Change::Inc),
        "1-h" => do_internal!(change_main_factor, Change::Dec),
        "1-S-Left" => do_internal!(change_margin, Edge::Left, Change::Inc),
        "1-C-S-Left" => do_internal!(change_margin, Edge::Left, Change::Dec),
        "1-S-Up" => do_internal!(change_margin, Edge::Top, Change::Inc),
        "1-C-S-Up" => do_internal!(change_margin, Edge::Top, Change::Dec),
        "1-S-Down" => do_internal!(change_margin, Edge::Bottom, Change::Inc),
        "1-C-S-Down" => do_internal!(change_margin, Edge::Bottom, Change::Dec),
        "1-S-Right" => do_internal!(change_margin, Edge::Right, Change::Inc),
        "1-C-S-Right" => do_internal!(change_margin, Edge::Right, Change::Dec),
        "1-C-S-equal" => do_internal!(reset_margin),
        "1-2-C-S-equal" => do_internal!(reset_layout),

        // workspace activators
        "1-Escape" => do_internal!(toggle_workspace),
        "1-bracketleft" => do_internal!(activate_prev_workspace),
        "1-bracketright" => do_internal!(activate_next_workspace),
        "1-1" => do_internal!(activate_workspace, 0),
        "1-2" => do_internal!(activate_workspace, 1),
        "1-3" => do_internal!(activate_workspace, 2),
        "1-4" => do_internal!(activate_workspace, 3),
        "1-5" => do_internal!(activate_workspace, 4),
        "1-6" => do_internal!(activate_workspace, 5),
        "1-7" => do_internal!(activate_workspace, 6),
        "1-8" => do_internal!(activate_workspace, 7),
        "1-9" => do_internal!(activate_workspace, 8),
        "1-0" => do_internal!(activate_workspace, 9),

        // workspace client movement
        "1-S-bracketleft" => do_internal!(move_focus_to_prev_workspace),
        "1-S-bracketright" => do_internal!(move_focus_to_next_workspace),
        "1-S-1" => do_internal!(move_focus_to_workspace, 0),
        "1-S-2" => do_internal!(move_focus_to_workspace, 1),
        "1-S-3" => do_internal!(move_focus_to_workspace, 2),
        "1-S-4" => do_internal!(move_focus_to_workspace, 3),
        "1-S-5" => do_internal!(move_focus_to_workspace, 4),
        "1-S-6" => do_internal!(move_focus_to_workspace, 5),
        "1-S-7" => do_internal!(move_focus_to_workspace, 6),
        "1-S-8" => do_internal!(move_focus_to_workspace, 7),
        "1-S-9" => do_internal!(move_focus_to_workspace, 8),
        "1-S-0" => do_internal!(move_focus_to_workspace, 9),

        // placeable region modifiers
        "1-v" => do_internal!(toggle_screen_struts),

        // client jump criteria
        "1-b" => do_internal!(jump_client,
            &JumpCriterium::ByClass("qutebrowser", MatchMethod::Equals)
        ),
        "1-S-b" => do_internal!(jump_client,
            &JumpCriterium::ByClass("Firefox", MatchMethod::Equals)
        ),
        "1-C-b" => do_internal!(jump_client,
            &JumpCriterium::ByClass("Chromium", MatchMethod::Equals)
        ),
        "1-2-space" => do_internal!(jump_client,
            &JumpCriterium::ByClass("Spotify", MatchMethod::Equals)
        ),
        "1-e" => do_internal_block!(model, {
            model.jump_client(&JumpCriterium::ByName(
                "[vim]",
                MatchMethod::Contains,
            ));
        }),
        "1-slash" => do_internal_block!(model, {
            let workspace_index = model.active_workspace();

            model.jump_client(&JumpCriterium::OnWorkspaceBySelector(
                workspace_index,
                &ClientSelector::Last,
            ));
        }),
        "1-period" => do_internal_block!(model, {
            let workspace_index = model.active_workspace();

            model.jump_client(&JumpCriterium::OnWorkspaceBySelector(
                workspace_index,
                &ClientSelector::AtMaster,
            ));
        }),
        "1-comma" => do_internal_block!(model, {
            let workspace_index = model.active_workspace();

            model.jump_client(&JumpCriterium::OnWorkspaceBySelector(
                workspace_index,
                &ClientSelector::First,
            ));
        }),

        // external spawn commands
        "XF86AudioPlay", "1-2-p" => spawn_external!("playerctl play-pause"),
        "XF86AudioPrev", "1-2-k" => spawn_external!("playerctl previous"),
        "XF86AudioNext", "1-2-j" => spawn_external!("playerctl next"),
        "1-2-BackSpace" => spawn_external!("playerctl stop"),
        "XF86AudioMute" => spawn_external!("amixer -D pulse sset Master toggle"),
        "XF86AudioLowerVolume" => spawn_external!("amixer -D pulse sset Master 5%-"),
        "XF86AudioRaiseVolume" => spawn_external!("amixer -D pulse sset Master 5%+"),

        "1-Return" => spawn_external!("st"),
        "1-S-Return" => spawn_external!(concat!("st -n ", WM_NAME!(), ":cf")),

        "1-p" => spawn_external!("dmenu_run"),
        "1-q" => spawn_external!("qutebrowser"),
        "1-S-q" => spawn_external!("firefox"),
        "1-C-q" => spawn_external!("chromium"),

        "1-C-e" => spawn_external!("st -g 140x42 -e zsh -i -c neomutt"),
        "1-C-s" => spawn_external!("st -g 80x42 -e zsh -i -c sncli"),
        "1-C-i" => spawn_external!("st -g 80x42 -e zsh -i -c irssi"),

        "S-XF86AudioMute" => spawn_external!("amixer -D pulse sset Capture toggle"),
        "XF86AudioMicMute" => spawn_external!("amixer -D pulse sset Capture toggle"),

        // external shell commands
        "1-S-p" => spawn_from_shell!("$HOME/bin/dmenupass"),
        "1-C-p" => spawn_from_shell!("$HOME/bin/dmenupass --copy"),
        "1-S-o" => spawn_from_shell!("$HOME/bin/dmenunotify"),
        "Print", "1-2-slash" => spawn_from_shell!(
            "maim -u -m 1 -s \
            $(date +$HOME/screenshots/scrots/SS_%Y-%h-%d_%H-%M-%S.png)"
        ),
        "S-Print", "1-2-S-slash" => spawn_from_shell!(
            "maim -u -m 1 \
            $(date +$HOME/screenshots/scrots/SS_%Y-%h-%d_%H-%M-%S.png)"
        ),
    );

    (mouse_bindings, key_bindings)
}
