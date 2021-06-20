use crate::connection::Connection;
use crate::connection::Pid;
use crate::event::Event;
use crate::event::PropertyKind;
use crate::event::StackMode;
use crate::event::ToggleAction;
use crate::geometry::Corner;
use crate::geometry::Dim;
use crate::geometry::Edge;
use crate::geometry::Extents;
use crate::geometry::Pos;
use crate::geometry::Ratio;
use crate::geometry::Region;
use crate::geometry::Strut;
use crate::hints::Hints;
use crate::hints::SizeHints;
use crate::input::Button;
use crate::input::Modifier;
use crate::input::Grip;
use crate::input::Key;
use crate::input::KeyEvent;
use crate::input::MouseEventKind;
use crate::input::MouseInputTarget;
use crate::input::KeyInput;
use crate::input::MouseEvent;
use crate::input::MouseInput;
use crate::screen::Screen;
use crate::window::IcccmWindowState;
use crate::window::Window;
use crate::window::WindowState;
use crate::window::WindowType;
use crate::Result;

use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::str::FromStr;

use x11rb::connection;
use x11rb::cursor::Handle as CursorHandle;
use x11rb::errors::ReplyError;
use x11rb::properties;
use x11rb::protocol;
use x11rb::protocol::randr;
use x11rb::protocol::xproto;
use x11rb::protocol::xproto::ConnectionExt;
use x11rb::protocol::xproto::EventMask;
use x11rb::protocol::xproto::ModMask;
use x11rb::protocol::xproto::CLIENT_MESSAGE_EVENT;
use x11rb::protocol::ErrorKind;
use x11rb::protocol::Event as XEvent;
use x11rb::resource_manager::Database;
use x11rb::wrapper::ConnectionExt as _;

use anyhow::anyhow;
use strum::*;

type Atom = u32;

x11rb::atom_manager! {
    pub Atoms: AtomsCookie {
        Any,
        ATOM,
        CARDINAL,
        WINDOW,
        STRING,
        UTF8_STRING,

        // ICCCM client properties
        WM_NAME,
        WM_CLASS,
        WM_CLIENT_MACHINE,
        WM_PROTOCOLS,
        WM_NORMAL_HINTS,
        WM_DELETE_WINDOW,
        WM_WINDOW_ROLE,
        WM_CLIENT_LEADER,
        WM_TRANSIENT_FOR,
        WM_TAKE_FOCUS,

        // ICCCM window manager properties
        WM_STATE,
        WM_ICON_SIZE,

        // EWMH root properties
        _NET_SUPPORTED,
        _NET_CLIENT_LIST,
        _NET_CLIENT_LIST_STACKING,
        _NET_NUMBER_OF_DESKTOPS,
        _NET_DESKTOP_GEOMETRY,
        _NET_DESKTOP_VIEWPORT,
        _NET_CURRENT_DESKTOP,
        _NET_DESKTOP_NAMES,
        _NET_ACTIVE_WINDOW,
        _NET_WORKAREA,
        _NET_SUPPORTING_WM_CHECK,
        _NET_VIRTUAL_ROOTS,
        _NET_DESKTOP_LAYOUT,
        _NET_SHOWING_DESKTOP,

        // EWMH root messages
        _NET_CLOSE_WINDOW,
        _NET_MOVERESIZE_WINDOW,
        _NET_WM_MOVERESIZE,
        _NET_REQUEST_FRAME_EXTENTS,

        // EWMH application properties
        _NET_WM_NAME,
        _NET_WM_VISIBLE_NAME,
        _NET_WM_ICON_NAME,
        _NET_WM_VISIBLE_ICON_NAME,
        _NET_WM_DESKTOP,
        _NET_WM_WINDOW_TYPE,
        _NET_WM_STATE,
        _NET_WM_ALLOWED_ACTIONS,
        _NET_WM_STRUT,
        _NET_WM_STRUT_PARTIAL,
        _NET_WM_ICON_GEOMETRY,
        _NET_WM_ICON,
        _NET_WM_PID,
        _NET_WM_HANDLED_ICONS,
        _NET_WM_USER_TIME,
        _NET_WM_USER_TIME_WINDOW,
        _NET_FRAME_EXTENTS,
        _NET_WM_OPAQUE_REGION,
        _NET_WM_BYPASS_COMPOSITOR,

        // EWMH window states
        _NET_WM_STATE_MODAL,
        _NET_WM_STATE_STICKY,
        _NET_WM_STATE_MAXIMIZED_VERT,
        _NET_WM_STATE_MAXIMIZED_HORZ,
        _NET_WM_STATE_SHADED,
        _NET_WM_STATE_SKIP_TASKBAR,
        _NET_WM_STATE_SKIP_PAGER,
        _NET_WM_STATE_HIDDEN,
        _NET_WM_STATE_FULLSCREEN,
        _NET_WM_STATE_ABOVE,
        _NET_WM_STATE_BELOW,
        _NET_WM_STATE_DEMANDS_ATTENTION,
        _NET_WM_STATE_FOCUSED,

        // EWMH window types
        _NET_WM_WINDOW_TYPE_DESKTOP,
        _NET_WM_WINDOW_TYPE_DOCK,
        _NET_WM_WINDOW_TYPE_TOOLBAR,
        _NET_WM_WINDOW_TYPE_MENU,
        _NET_WM_WINDOW_TYPE_UTILITY,
        _NET_WM_WINDOW_TYPE_SPLASH,
        _NET_WM_WINDOW_TYPE_DIALOG,
        _NET_WM_WINDOW_TYPE_DROPDOWN_MENU,
        _NET_WM_WINDOW_TYPE_POPUP_MENU,
        _NET_WM_WINDOW_TYPE_TOOLTIP,
        _NET_WM_WINDOW_TYPE_NOTIFICATION,
        _NET_WM_WINDOW_TYPE_COMBO,
        _NET_WM_WINDOW_TYPE_DND,
        _NET_WM_WINDOW_TYPE_NORMAL,

        // EWMH protocols
        _NET_WM_PING,
        _NET_WM_SYNC_REQUEST,
        _NET_WM_FULLSCREEN_MONITORS,

        // System tray protocols
        _NET_SYSTEM_TRAY_ORIENTATION,
        _NET_SYSTEM_TRAY_OPCODE,
        _NET_SYSTEM_TRAY_ORIENTATION_HORZ,
        _NET_SYSTEM_TRAY_S0,
        _XEMBED,
        _XEMBED_INFO,
    }
}

pub struct XConnection<'conn, Conn: connection::Connection> {
    conn: &'conn Conn,
    atoms: Atoms,
    type_map: HashMap<Atom, WindowType>,
    state_map: HashMap<Atom, WindowState>,
    screen: xproto::Screen,
    check_window: Window,
    background_gc: xproto::Gcontext,
    database: Option<Database>,
    confined_to: Cell<Option<Window>>,
    keys: RefCell<HashMap<u8, Key>>,
    keycodes: RefCell<HashMap<Key, u8>>,
    root_event_mask: EventMask,
    window_event_mask: EventMask,
    frame_event_mask: EventMask,
    mouse_event_mask: EventMask,
    regrab_event_mask: EventMask,
}

impl<'conn, Conn: connection::Connection> XConnection<'conn, Conn> {
    pub fn new(
        conn: &'conn Conn,
        screen_num: usize,
    ) -> Result<Self> {
        let screen = conn.setup().roots[screen_num].clone();
        let root = screen.root;

        let aux = xproto::ChangeWindowAttributesAux::default()
            .event_mask(EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY);

        let res = conn.change_window_attributes(screen.root, &aux)?.check();
        if let Err(ReplyError::X11Error(err)) = res {
            if err.error_kind == ErrorKind::Access {
                return Err(anyhow!("another window manager is already running"));
            } else {
                return Err(anyhow!("unable to set up window manager"));
            }
        }

        let atoms = Atoms::new(conn)?.reply()?;
        let check_window = conn.generate_id()?;

        let type_map: HashMap<Atom, WindowType> = map!(
            atoms._NET_WM_WINDOW_TYPE_DESKTOP => WindowType::Desktop,
            atoms._NET_WM_WINDOW_TYPE_DOCK => WindowType::Dock,
            atoms._NET_WM_WINDOW_TYPE_TOOLBAR => WindowType::Toolbar,
            atoms._NET_WM_WINDOW_TYPE_MENU => WindowType::Menu,
            atoms._NET_WM_WINDOW_TYPE_UTILITY => WindowType::Utility,
            atoms._NET_WM_WINDOW_TYPE_SPLASH => WindowType::Splash,
            atoms._NET_WM_WINDOW_TYPE_DIALOG => WindowType::Dialog,
            atoms._NET_WM_WINDOW_TYPE_DROPDOWN_MENU => WindowType::DropdownMenu,
            atoms._NET_WM_WINDOW_TYPE_POPUP_MENU => WindowType::PopupMenu,
            atoms._NET_WM_WINDOW_TYPE_TOOLTIP => WindowType::Tooltip,
            atoms._NET_WM_WINDOW_TYPE_NOTIFICATION => WindowType::Notification,
            atoms._NET_WM_WINDOW_TYPE_COMBO => WindowType::Combo,
            atoms._NET_WM_WINDOW_TYPE_DND => WindowType::Dnd,
            atoms._NET_WM_WINDOW_TYPE_NORMAL => WindowType::Normal,
        );

        let state_map: HashMap<Atom, WindowState> = map!(
            atoms._NET_WM_STATE_MODAL => WindowState::Modal,
            atoms._NET_WM_STATE_STICKY => WindowState::Sticky,
            atoms._NET_WM_STATE_MAXIMIZED_VERT => WindowState::MaximizedVert,
            atoms._NET_WM_STATE_MAXIMIZED_HORZ => WindowState::MaximizedHorz,
            atoms._NET_WM_STATE_SHADED => WindowState::Shaded,
            atoms._NET_WM_STATE_SKIP_TASKBAR => WindowState::SkipTaskbar,
            atoms._NET_WM_STATE_SKIP_PAGER => WindowState::SkipPager,
            atoms._NET_WM_STATE_HIDDEN => WindowState::Hidden,
            atoms._NET_WM_STATE_FULLSCREEN => WindowState::Fullscreen,
            atoms._NET_WM_STATE_ABOVE => WindowState::Above,
            atoms._NET_WM_STATE_BELOW => WindowState::Below,
            atoms._NET_WM_STATE_DEMANDS_ATTENTION => WindowState::DemandsAttention,
        );

        conn.create_window(
            x11rb::COPY_DEPTH_FROM_PARENT,
            check_window,
            root,
            -1,
            -1,
            1,
            1,
            0,
            xproto::WindowClass::INPUT_ONLY,
            x11rb::COPY_FROM_PARENT,
            &xproto::CreateWindowAux::default().override_redirect(1),
        )?;

        conn.map_window(check_window)?;
        conn.configure_window(
            check_window,
            &xproto::ConfigureWindowAux::default().stack_mode(xproto::StackMode::BELOW),
        )?;

        randr::select_input(
            conn,
            check_window,
            randr::NotifyMask::OUTPUT_CHANGE
                | randr::NotifyMask::CRTC_CHANGE
                | randr::NotifyMask::SCREEN_CHANGE,
        )?;

        let background_gc = conn.generate_id()?;
        conn.create_gc(background_gc, screen.root, &xproto::CreateGCAux::default())?;

        let database = Database::new_from_default(conn).ok();
        if let Some(ref database) = database {
            drop(
                CursorHandle::new(conn, screen_num, &database).map(|cookie| {
                    cookie.reply().map(|reply| {
                        let aux = xproto::ChangeWindowAttributesAux::default()
                            .cursor(reply.load_cursor(conn, "left_ptr").ok());

                        drop(conn.change_window_attributes(screen.root, &aux));
                    })
                }),
            );
        }

        let keys = RefCell::new(HashMap::new());
        let keycodes = RefCell::new(HashMap::new());

        let root_event_mask: EventMask = EventMask::PROPERTY_CHANGE
            | EventMask::SUBSTRUCTURE_REDIRECT
            | EventMask::STRUCTURE_NOTIFY
            | EventMask::BUTTON_PRESS
            | EventMask::POINTER_MOTION
            | EventMask::FOCUS_CHANGE;

        let window_event_mask: EventMask =
            EventMask::PROPERTY_CHANGE | EventMask::STRUCTURE_NOTIFY | EventMask::FOCUS_CHANGE;

        let frame_event_mask: EventMask = EventMask::STRUCTURE_NOTIFY
            | EventMask::SUBSTRUCTURE_REDIRECT
            | EventMask::SUBSTRUCTURE_NOTIFY
            | EventMask::BUTTON_PRESS
            | EventMask::BUTTON_RELEASE
            | EventMask::POINTER_MOTION;

        let mouse_event_mask: EventMask =
            EventMask::BUTTON_PRESS | EventMask::BUTTON_RELEASE | EventMask::BUTTON_MOTION;

        let regrab_event_mask: EventMask = EventMask::BUTTON_PRESS | EventMask::BUTTON_RELEASE;

        Self::init(Self {
            conn,
            atoms,
            type_map,
            state_map,
            screen,
            check_window,
            background_gc,
            database,
            confined_to: Cell::new(None),
            keys,
            keycodes,
            root_event_mask,
            window_event_mask,
            frame_event_mask,
            mouse_event_mask,
            regrab_event_mask,
        })
    }

    #[inline]
    fn init(connection: Self) -> Result<Self> {
        Ok(connection)
    }

    pub fn window_is_any_of_types(
        &self,
        window: Window,
        types: &[Atom],
    ) -> bool {
        self.conn
            .get_property(
                false,
                window,
                self.atoms._NET_WM_WINDOW_TYPE,
                self.atoms.ATOM,
                0,
                std::u32::MAX,
            )
            .map_or(false, |cookie| {
                cookie.reply().map_or(false, |reply| {
                    reply.value32().map_or(false, |mut window_types| {
                        window_types.any(|type_| types.contains(&type_))
                    })
                })
            })
    }

    pub fn window_is_any_of_states(
        &self,
        window: Window,
        states: &[Atom],
    ) -> bool {
        self.conn
            .get_property(
                false,
                window,
                self.atoms._NET_WM_STATE,
                self.atoms.ATOM,
                0,
                std::u32::MAX,
            )
            .map_or(false, |cookie| {
                cookie.reply().map_or(false, |reply| {
                    reply.value32().map_or(false, |mut window_states| {
                        window_states.any(|state| states.contains(&state))
                    })
                })
            })
    }

    pub fn window_has_any_of_protocols(
        &self,
        window: Window,
        protocols: &[Atom],
    ) -> bool {
        self.conn
            .get_property(
                false,
                window,
                self.atoms.WM_PROTOCOLS,
                self.atoms.ATOM,
                0,
                std::u32::MAX,
            )
            .map_or(false, |cookie| {
                cookie.reply().map_or(false, |reply| {
                    reply.value32().map_or(false, |mut window_protocols| {
                        window_protocols.any(|protocol| protocols.contains(&protocol))
                    })
                })
            })
    }

    #[inline]
    fn send_client_message(
        &self,
        window: Window,
        atom: Atom,
        type_: Atom,
    ) -> Result<()> {
        let data = [atom, x11rb::CURRENT_TIME, 0, 0, 0];

        let event = xproto::ClientMessageEvent {
            response_type: CLIENT_MESSAGE_EVENT,
            format: 32,
            sequence: 0,
            window,
            type_,
            data: data.into(),
        };

        self.conn
            .send_event(false, window, EventMask::NO_EVENT, &event)?;

        Ok(())
    }

    #[inline]
    fn send_protocol_client_message(
        &self,
        window: Window,
        atom: Atom,
    ) -> Result<()> {
        self.send_client_message(window, atom, self.atoms.WM_PROTOCOLS)
    }

    #[inline]
    fn get_window_state_from_atom(
        &self,
        atom: Atom,
    ) -> Option<WindowState> {
        self.state_map.get(&atom).copied()
    }

    #[inline]
    fn get_atom_from_window_state(
        &self,
        state: WindowState,
    ) -> Atom {
        match state {
            WindowState::Modal => self.atoms._NET_WM_STATE_MODAL,
            WindowState::Sticky => self.atoms._NET_WM_STATE_STICKY,
            WindowState::MaximizedVert => self.atoms._NET_WM_STATE_MAXIMIZED_VERT,
            WindowState::MaximizedHorz => self.atoms._NET_WM_STATE_MAXIMIZED_HORZ,
            WindowState::Shaded => self.atoms._NET_WM_STATE_SHADED,
            WindowState::SkipTaskbar => self.atoms._NET_WM_STATE_SKIP_TASKBAR,
            WindowState::SkipPager => self.atoms._NET_WM_STATE_SKIP_PAGER,
            WindowState::Hidden => self.atoms._NET_WM_STATE_HIDDEN,
            WindowState::Fullscreen => self.atoms._NET_WM_STATE_FULLSCREEN,
            WindowState::Above => self.atoms._NET_WM_STATE_ABOVE,
            WindowState::Below => self.atoms._NET_WM_STATE_BELOW,
            WindowState::DemandsAttention => self.atoms._NET_WM_STATE_DEMANDS_ATTENTION,
        }
    }

    #[inline]
    fn get_window_type_from_atom(
        &self,
        atom: Atom,
    ) -> Option<WindowType> {
        self.type_map.get(&atom).copied()
    }

    #[inline]
    fn get_atom_from_window_type(
        &self,
        type_: WindowType,
    ) -> Atom {
        match type_ {
            WindowType::Desktop => self.atoms._NET_WM_WINDOW_TYPE_DESKTOP,
            WindowType::Dock => self.atoms._NET_WM_WINDOW_TYPE_DOCK,
            WindowType::Toolbar => self.atoms._NET_WM_WINDOW_TYPE_TOOLBAR,
            WindowType::Menu => self.atoms._NET_WM_WINDOW_TYPE_MENU,
            WindowType::Utility => self.atoms._NET_WM_WINDOW_TYPE_UTILITY,
            WindowType::Splash => self.atoms._NET_WM_WINDOW_TYPE_SPLASH,
            WindowType::Dialog => self.atoms._NET_WM_WINDOW_TYPE_DIALOG,
            WindowType::DropdownMenu => self.atoms._NET_WM_WINDOW_TYPE_DROPDOWN_MENU,
            WindowType::PopupMenu => self.atoms._NET_WM_WINDOW_TYPE_POPUP_MENU,
            WindowType::Tooltip => self.atoms._NET_WM_WINDOW_TYPE_TOOLTIP,
            WindowType::Notification => self.atoms._NET_WM_WINDOW_TYPE_NOTIFICATION,
            WindowType::Combo => self.atoms._NET_WM_WINDOW_TYPE_COMBO,
            WindowType::Dnd => self.atoms._NET_WM_WINDOW_TYPE_DND,
            WindowType::Normal => self.atoms._NET_WM_WINDOW_TYPE_NORMAL,
        }
    }

    fn get_key(
        &self,
        keycode: u8,
    ) -> Key {
        if let Some(&key) = self.keys.borrow().get(&keycode) {
            return key;
        }

        let key = match keycode {
            9 => Key::Escape,
            10 => Key::One,
            11 => Key::Two,
            12 => Key::Three,
            13 => Key::Four,
            14 => Key::Five,
            15 => Key::Six,
            16 => Key::Seven,
            17 => Key::Eight,
            18 => Key::Nine,
            19 => Key::Zero,
            20 => Key::Minus,
            21 => Key::Equal,
            22 => Key::Backspace,
            23 => Key::Tab,
            24 => Key::Q,
            25 => Key::W,
            26 => Key::E,
            27 => Key::R,
            28 => Key::T,
            29 => Key::Y,
            30 => Key::U,
            31 => Key::I,
            32 => Key::O,
            33 => Key::P,
            34 => Key::LeftBracket,
            35 => Key::RightBracket,
            36 => Key::Return,
            37 => Key::Control,
            38 => Key::A,
            39 => Key::S,
            40 => Key::D,
            41 => Key::F,
            42 => Key::G,
            43 => Key::H,
            44 => Key::J,
            45 => Key::K,
            46 => Key::L,
            47 => Key::SemiColon,
            48 => Key::Apostrophe,
            49 => Key::Tilde,
            50 => Key::Shift,
            51 => Key::BackSlash,
            52 => Key::Z,
            53 => Key::X,
            54 => Key::C,
            55 => Key::V,
            56 => Key::B,
            57 => Key::N,
            58 => Key::M,
            59 => Key::Comma,
            60 => Key::Period,
            61 => Key::Slash,
            62 => Key::RightShift,
            63 => Key::Multiply,
            64 => Key::Alt,
            65 => Key::Space,
            66 => Key::CapsLock,
            67 => Key::F1,
            68 => Key::F2,
            69 => Key::F3,
            70 => Key::F4,
            71 => Key::F5,
            72 => Key::F6,
            73 => Key::F7,
            74 => Key::F8,
            75 => Key::F9,
            76 => Key::F10,
            77 => Key::Numlock,
            78 => Key::ScrollLock,
            79 => Key::NumPad7,
            80 => Key::NumPad8,
            81 => Key::NumPad9,
            82 => Key::Subtract,
            83 => Key::NumPad4,
            84 => Key::NumPad5,
            85 => Key::NumPad6,
            86 => Key::Add,
            87 => Key::NumPad1,
            88 => Key::NumPad2,
            89 => Key::NumPad3,
            90 => Key::NumPad0,
            94 => Key::Less,
            95 => Key::F11,
            96 => Key::F12,
            105 => Key::RightContol,
            106 => Key::Divide,
            107 => Key::PrintScreen,
            108 => Key::RightAlt,
            110 => Key::Home,
            111 => Key::Up,
            112 => Key::PageUp,
            113 => Key::Left,
            114 => Key::Right,
            115 => Key::End,
            116 => Key::Down,
            117 => Key::PageDown,
            118 => Key::Insert,
            119 => Key::Delete,
            121 => Key::VolumeMute,
            122 => Key::VolumeDown,
            123 => Key::VolumeUp,
            127 => Key::Pause,
            128 => Key::LaunchAppA,
            129 => Key::Decimal,
            133 => Key::Super,
            134 => Key::RightSuper,
            135 => Key::Menu,
            146 => Key::Help,
            156 => Key::LaunchApp1,
            157 => Key::LaunchApp2,
            163 => Key::LaunchMail,
            164 => Key::BrowserFavorites,
            166 => Key::BrowserBack,
            167 => Key::BrowserForward,
            171 => Key::NextTrack,
            172 => Key::PlayPause,
            173 => Key::PreviousTrack,
            174 => Key::StopMedia,
            180 => Key::BrowserHome,
            182 => Key::BrowserStop,
            187 => Key::LeftParenthesis,
            188 => Key::RightParenthesis,
            192 => Key::LaunchApp5,
            193 => Key::LaunchApp6,
            194 => Key::LaunchApp7,
            195 => Key::LaunchApp8,
            196 => Key::LaunchApp9,
            210 => Key::LaunchApp3,
            211 => Key::LaunchApp4,
            212 => Key::LaunchAppB,
            225 => Key::BrowserSearch,
            234 => Key::SelectMedia,
            _ => return Key::Any,
        };

        self.keys.borrow_mut().insert(keycode, key);
        self.keycodes.borrow_mut().insert(key, keycode);

        key
    }

    fn get_keycode(
        &self,
        key: Key,
    ) -> u8 {
        if let Some(&keycode) = self.keycodes.borrow().get(&key) {
            return keycode;
        }

        let keycode = match key {
            Key::Backspace => 22,
            Key::Tab => 23,
            Key::Return => 36,
            Key::Shift => 50,
            Key::Control => 37,
            Key::Alt => 64,
            Key::Super => 133,
            Key::Menu => 135,
            Key::Pause => 127,
            Key::CapsLock => 66,
            Key::Escape => 9,
            Key::Space => 65,
            Key::ExclamationMark => 10,
            Key::QuotationMark => 48,
            Key::QuestionMark => 61,
            Key::NumberSign => 12,
            Key::DollarSign => 13,
            Key::PercentSign => 14,
            Key::AtSign => 11,
            Key::Ampersand => 16,
            Key::Apostrophe => 48,
            Key::LeftParenthesis => 187,
            Key::RightParenthesis => 188,
            Key::LeftBracket => 34,
            Key::RightBracket => 35,
            Key::LeftBrace => 34,
            Key::RightBrace => 35,
            Key::Underscore => 20,
            Key::Grave => 49,
            Key::Bar => 51,
            Key::Tilde => 49,
            Key::QuoteLeft => 49,
            Key::Asterisk => 17,
            Key::Plus => 21,
            Key::Comma => 59,
            Key::Minus => 20,
            Key::Period => 60,
            Key::Slash => 61,
            Key::BackSlash => 51,
            Key::Colon => 47,
            Key::SemiColon => 47,
            Key::Less => 94,
            Key::Equal => 21,
            Key::Greater => 60,
            Key::PageUp => 112,
            Key::PageDown => 117,
            Key::End => 115,
            Key::Home => 110,
            Key::Left => 113,
            Key::Up => 111,
            Key::Right => 114,
            Key::Down => 116,
            Key::Print => 107,
            Key::PrintScreen => 107,
            Key::Insert => 118,
            Key::Delete => 119,
            Key::Help => 146,
            Key::Zero => 19,
            Key::One => 10,
            Key::Two => 11,
            Key::Three => 12,
            Key::Four => 13,
            Key::Five => 14,
            Key::Six => 15,
            Key::Seven => 16,
            Key::Eight => 17,
            Key::Nine => 18,
            Key::A => 38,
            Key::B => 56,
            Key::C => 54,
            Key::D => 40,
            Key::E => 26,
            Key::F => 41,
            Key::G => 42,
            Key::H => 43,
            Key::I => 31,
            Key::J => 44,
            Key::K => 45,
            Key::L => 46,
            Key::M => 58,
            Key::N => 57,
            Key::O => 32,
            Key::P => 33,
            Key::Q => 24,
            Key::R => 27,
            Key::S => 39,
            Key::T => 28,
            Key::U => 30,
            Key::V => 55,
            Key::W => 25,
            Key::X => 53,
            Key::Y => 29,
            Key::Z => 52,
            Key::NumPad0 => 90,
            Key::NumPad1 => 87,
            Key::NumPad2 => 88,
            Key::NumPad3 => 89,
            Key::NumPad4 => 83,
            Key::NumPad5 => 84,
            Key::NumPad6 => 85,
            Key::NumPad7 => 79,
            Key::NumPad8 => 80,
            Key::NumPad9 => 81,
            Key::Multiply => 63,
            Key::Add => 86,
            Key::Subtract => 82,
            Key::Decimal => 129,
            Key::Divide => 106,
            Key::F1 => 67,
            Key::F2 => 68,
            Key::F3 => 69,
            Key::F4 => 70,
            Key::F5 => 71,
            Key::F6 => 72,
            Key::F7 => 73,
            Key::F8 => 74,
            Key::F9 => 75,
            Key::F10 => 76,
            Key::F11 => 95,
            Key::F12 => 96,
            Key::Numlock => 77,
            Key::ScrollLock => 78,
            Key::LeftShift => 50,
            Key::RightShift => 62,
            Key::LeftControl => 37,
            Key::RightContol => 105,
            Key::LeftAlt => 64,
            Key::RightAlt => 108,
            Key::LeftSuper => 133,
            Key::RightSuper => 134,
            Key::BrowserBack => 166,
            Key::BrowserForward => 167,
            Key::BrowserStop => 182,
            Key::BrowserSearch => 225,
            Key::BrowserFavorites => 164,
            Key::BrowserHome => 180,
            Key::VolumeMute => 121,
            Key::VolumeDown => 122,
            Key::VolumeUp => 123,
            Key::NextTrack => 171,
            Key::PreviousTrack => 173,
            Key::StopMedia => 174,
            Key::PlayPause => 172,
            Key::LaunchMail => 163,
            Key::SelectMedia => 234,
            Key::LaunchAppA => 128,
            Key::LaunchAppB => 212,
            Key::LaunchApp1 => 156,
            Key::LaunchApp2 => 157,
            Key::LaunchApp3 => 210,
            Key::LaunchApp4 => 211,
            Key::LaunchApp5 => 192,
            Key::LaunchApp6 => 193,
            Key::LaunchApp7 => 194,
            Key::LaunchApp8 => 195,
            Key::LaunchApp9 => 196,
            _ => return 0,
        };

        self.keys.borrow_mut().insert(keycode, key);
        self.keycodes.borrow_mut().insert(key, keycode);

        keycode
    }

    fn set_window_state_atom(
        &self,
        window: Window,
        state_atom: Atom,
        on: bool,
    ) {
        if on {
            if self.window_is_any_of_states(window, &[state_atom]) {
                return;
            }

            drop(self.conn.change_property32(
                xproto::PropMode::APPEND,
                window,
                self.atoms._NET_WM_STATE,
                xproto::AtomEnum::ATOM,
                &[state_atom],
            ));
        } else {
            let mut states = self
                .conn
                .get_property(
                    false,
                    window,
                    self.atoms._NET_WM_STATE,
                    self.atoms.ATOM,
                    0,
                    std::u32::MAX,
                )
                .map_or(Vec::with_capacity(0), |cookie| {
                    cookie.reply().map_or(Vec::with_capacity(0), |reply| {
                        reply
                            .value32()
                            .map_or(Vec::with_capacity(0), |window_states| {
                                window_states.collect()
                            })
                    })
                });

            states.retain(|&state| state != state_atom);

            drop(self.conn.change_property32(
                xproto::PropMode::REPLACE,
                window,
                self.atoms._NET_WM_STATE,
                xproto::AtomEnum::ATOM,
                &states,
            ));
        }
    }

    #[inline]
    fn on_button_press(
        &self,
        event: &xproto::ButtonPressEvent,
    ) -> Option<Event> {
        let window = event.event;
        let window = if window == self.screen.root || window == x11rb::NONE {
            if event.child == x11rb::NONE {
                None
            } else {
                Some(event.child)
            }
        } else {
            Some(window)
        };

        Some(Event::Mouse {
            event: MouseEvent {
                kind: MouseEventKind::Press,
                input: MouseInput {
                    target: MouseInputTarget::Global,
                    button: {
                        if let Ok(button) = Button::try_from(event.detail) {
                            button
                        } else {
                            return None;
                        }
                    },
                    modifiers: {
                        let mut modifiers = HashSet::new();

                        if event.state & u16::from(xproto::ModMask::CONTROL) > 0 {
                            modifiers.insert(Modifier::Ctrl);
                        }

                        if event.state & u16::from(xproto::ModMask::SHIFT) > 0 {
                            modifiers.insert(Modifier::Shift);
                        }

                        if event.state & u16::from(xproto::ModMask::M1) > 0 {
                            modifiers.insert(Modifier::Alt);
                        }

                        if event.state & u16::from(xproto::ModMask::M4) > 0 {
                            modifiers.insert(Modifier::Super);
                        }

                        modifiers
                    },
                },
                window: {
                    if window == Some(self.screen.root) {
                        None
                    } else {
                        window
                    }
                },
                root_rpos: Pos {
                    x: event.root_x as i32,
                    y: event.root_y as i32,
                },
            },
            on_root: window == Some(self.screen.root),
        })
    }

    #[inline]
    fn on_button_release(
        &self,
        event: &xproto::ButtonReleaseEvent,
    ) -> Option<Event> {
        let window = event.event;
        let window = if window == self.screen.root || window == x11rb::NONE {
            if event.child == x11rb::NONE {
                None
            } else {
                Some(event.child)
            }
        } else {
            None
        };

        Some(Event::Mouse {
            event: MouseEvent {
                kind: MouseEventKind::Release,
                input: MouseInput {
                    target: MouseInputTarget::Global,
                    button: {
                        if let Ok(button) = Button::try_from(event.detail) {
                            button
                        } else {
                            return None;
                        }
                    },
                    modifiers: {
                        let mut modifiers = HashSet::new();

                        if event.state & u16::from(xproto::ModMask::CONTROL) > 0 {
                            modifiers.insert(Modifier::Ctrl);
                        }

                        if event.state & u16::from(xproto::ModMask::SHIFT) > 0 {
                            modifiers.insert(Modifier::Shift);
                        }

                        if event.state & u16::from(xproto::ModMask::M1) > 0 {
                            modifiers.insert(Modifier::Alt);
                        }

                        if event.state & u16::from(xproto::ModMask::M4) > 0 {
                            modifiers.insert(Modifier::Super);
                        }

                        modifiers
                    },
                },
                window: {
                    if window == Some(self.screen.root) {
                        None
                    } else {
                        window
                    }
                },
                root_rpos: Pos {
                    x: event.root_x as i32,
                    y: event.root_y as i32,
                },
            },
            on_root: window == Some(self.screen.root),
        })
    }

    #[inline]
    fn on_motion_notify(
        &self,
        event: &xproto::MotionNotifyEvent,
    ) -> Option<Event> {
        let window = event.event;
        let window = if window == self.screen.root || window == x11rb::NONE {
            if event.child == x11rb::NONE {
                None
            } else {
                Some(event.child)
            }
        } else {
            None
        };

        Some(Event::Mouse {
            event: MouseEvent {
                kind: MouseEventKind::Motion,
                input: MouseInput {
                    target: MouseInputTarget::Global,
                    button: Button::Left,
                    modifiers: {
                        let mut modifiers = HashSet::new();

                        if event.state & u16::from(xproto::ModMask::CONTROL) > 0 {
                            modifiers.insert(Modifier::Ctrl);
                        }

                        if event.state & u16::from(xproto::ModMask::SHIFT) > 0 {
                            modifiers.insert(Modifier::Shift);
                        }

                        if event.state & u16::from(xproto::ModMask::M1) > 0 {
                            modifiers.insert(Modifier::Alt);
                        }

                        if event.state & u16::from(xproto::ModMask::M4) > 0 {
                            modifiers.insert(Modifier::Super);
                        }

                        modifiers
                    },
                },
                window: {
                    if window == Some(self.screen.root) {
                        None
                    } else {
                        window
                    }
                },
                root_rpos: Pos {
                    x: event.root_x as i32,
                    y: event.root_y as i32,
                },
            },
            on_root: window == Some(self.screen.root),
        })
    }

    #[inline]
    fn on_key_press(
        &self,
        event: &xproto::KeyPressEvent,
    ) -> Option<Event> {
        Some(Event::Key {
            event: KeyEvent {
                input: KeyInput {
                    key: self.get_key(event.detail),
                    modifiers: {
                        let mut modifiers = HashSet::new();

                        if event.state & u16::from(xproto::ModMask::CONTROL) > 0 {
                            modifiers.insert(Modifier::Ctrl);
                        }

                        if event.state & u16::from(xproto::ModMask::SHIFT) > 0 {
                            modifiers.insert(Modifier::Shift);
                        }

                        if event.state & u16::from(xproto::ModMask::M1) > 0 {
                            modifiers.insert(Modifier::Alt);
                        }

                        if event.state & u16::from(xproto::ModMask::M4) > 0 {
                            modifiers.insert(Modifier::Super);
                        }

                        modifiers
                    },
                },
                window: {
                    let window = event.event;

                    if window == self.screen.root || window == x11rb::NONE {
                        Some(event.child)
                    } else {
                        None
                    }
                },
            }
        })
    }

    #[inline]
    fn on_map_request(
        &self,
        event: &xproto::MapRequestEvent,
    ) -> Option<Event> {
        Some(Event::MapRequest {
            window: event.window,
            ignore: !self.must_manage_window(event.window),
        })
    }

    #[inline]
    fn on_map_notify(
        &self,
        event: &xproto::MapNotifyEvent,
    ) -> Option<Event> {
        Some(Event::Map {
            window: event.window,
            ignore: !self.must_manage_window(event.window),
        })
    }

    #[inline]
    fn on_enter_notify(
        &self,
        event: &xproto::EnterNotifyEvent,
    ) -> Option<Event> {
        Some(Event::Enter {
            window: event.event,
            root_rpos: Pos {
                x: event.root_x as i32,
                y: event.root_y as i32,
            },
            window_rpos: Pos {
                x: event.event_x as i32,
                y: event.event_y as i32,
            },
        })
    }

    #[inline]
    fn on_leave_notify(
        &self,
        event: &xproto::LeaveNotifyEvent,
    ) -> Option<Event> {
        Some(Event::Leave {
            window: event.event,
            root_rpos: Pos {
                x: event.root_x as i32,
                y: event.root_y as i32,
            },
            window_rpos: Pos {
                x: event.event_x as i32,
                y: event.event_y as i32,
            },
        })
    }

    #[inline]
    fn on_destroy_notify(
        &self,
        event: &xproto::DestroyNotifyEvent,
    ) -> Option<Event> {
        Some(Event::Destroy {
            window: event.window,
        })
    }

    #[inline]
    fn on_expose(
        &self,
        event: &xproto::ExposeEvent,
    ) -> Option<Event> {
        Some(Event::Expose {
            window: event.window,
        })
    }

    #[inline]
    fn on_unmap_notify(
        &self,
        event: &xproto::UnmapNotifyEvent,
    ) -> Option<Event> {
        self.conn
            .get_window_attributes(event.window)
            .ok()
            .map(|cookie| Event::Unmap {
                window: event.window,
                ignore: cookie
                    .reply()
                    .map_or(false, |reply| reply.override_redirect),
            })
    }

    #[inline]
    fn on_configure_request(
        &self,
        event: &xproto::ConfigureRequestEvent,
    ) -> Option<Event> {
        let geometry = self.get_window_geometry(event.window).ok()?;

        let mut x = None;
        let mut y = None;
        let mut w = None;
        let mut h = None;

        if event.value_mask & u16::from(xproto::ConfigWindow::X) != 0 {
            x = Some(event.x as i32);
        }

        if event.value_mask & u16::from(xproto::ConfigWindow::Y) != 0 {
            y = Some(event.y as i32);
        }

        if event.value_mask & u16::from(xproto::ConfigWindow::WIDTH) != 0 {
            w = Some(event.width as i32);
        }

        if event.value_mask & u16::from(xproto::ConfigWindow::HEIGHT) != 0 {
            h = Some(event.height as i32);
        }

        let pos = match (x, y) {
            (Some(x), Some(y)) => Some(Pos {
                x,
                y,
            }),
            (None, Some(y)) => Some(Pos {
                x: geometry.pos.x,
                y,
            }),
            (Some(x), None) => Some(Pos {
                x,
                y: geometry.pos.y,
            }),
            _ => None,
        };

        let dim = match (w, h) {
            (Some(w), Some(h)) => Some(Dim {
                w,
                h,
            }),
            (None, Some(h)) => Some(Dim {
                w: geometry.dim.w,
                h,
            }),
            (Some(w), None) => Some(Dim {
                w,
                h: geometry.dim.h,
            }),
            _ => None,
        };

        if pos.is_some() || dim.is_some() {
            return Some(Event::PlacementRequest {
                window: event.window,
                pos,
                dim,
                on_root: event.window == self.screen.root,
            });
        }

        if event.value_mask & u16::from(xproto::ConfigWindow::STACK_MODE) != 0 {
            if event.sibling != x11rb::NONE {
                match event.stack_mode {
                    // window is placed above sibling
                    xproto::StackMode::ABOVE => {
                        return Some(Event::RestackRequest {
                            window: event.window,
                            sibling: event.sibling,
                            mode: StackMode::Above,
                            on_root: event.window == self.screen.root,
                        });
                    },
                    // sibling is placed above window
                    xproto::StackMode::BELOW => {
                        return Some(Event::RestackRequest {
                            window: event.window,
                            sibling: event.sibling,
                            mode: StackMode::Below,
                            on_root: event.window == self.screen.root,
                        });
                    },
                    _ => {},
                }
            }
        }

        None
    }

    #[inline]
    fn on_configure_notify(
        &self,
        event: &xproto::ConfigureNotifyEvent,
    ) -> Option<Event> {
        Some(Event::Configure {
            window: event.window,
            region: Region::new(
                event.x as i32,
                event.y as i32,
                event.width as i32,
                event.height as i32,
            ),
            on_root: event.window == self.screen.root,
        })
    }

    #[inline]
    fn on_property_notify(
        &self,
        event: &xproto::PropertyNotifyEvent,
    ) -> Option<Event> {
        if event.state == xproto::Property::NEW_VALUE {
            if event.atom == self.atoms.WM_NAME || event.atom == self.atoms._NET_WM_NAME {
                return Some(Event::Property {
                    window: event.window,
                    kind: PropertyKind::Name,
                    on_root: event.window == self.screen.root,
                });
            }

            if event.atom == self.atoms.WM_CLASS {
                return Some(Event::Property {
                    window: event.window,
                    kind: PropertyKind::Class,
                    on_root: event.window == self.screen.root,
                });
            }

            if event.atom == self.atoms.WM_NORMAL_HINTS {
                return Some(Event::Property {
                    window: event.window,
                    kind: PropertyKind::Size,
                    on_root: event.window == self.screen.root,
                });
            }
        }

        if event.atom == self.atoms._NET_WM_STRUT || event.atom == self.atoms._NET_WM_STRUT_PARTIAL
        {
            return Some(Event::Property {
                window: event.window,
                kind: PropertyKind::Strut,
                on_root: event.window == self.screen.root,
            });
        }

        None
    }

    #[inline]
    fn on_client_message(
        &self,
        event: &xproto::ClientMessageEvent,
    ) -> Option<Event> {
        let data = match event.format {
            8 => event.data.as_data8().iter().map(|&i| i as usize).collect(),
            16 => event.data.as_data16().iter().map(|&i| i as usize).collect(),
            32 => event.data.as_data32().iter().map(|&i| i as usize).collect(),
            _ => Vec::with_capacity(0),
        };

        if event.type_ == self.atoms._NET_WM_STATE {
            if event.format != 32 || data.len() < 3 {
                return None;
            }

            let mut state = None;

            for i in 1..=2 {
                if state.is_none() {
                    if data[i] != 0 {
                        state = self.get_window_state_from_atom(data[i] as Atom);
                    }
                }
            }

            if let Some(state) = state {
                return Some(Event::StateRequest {
                    window: event.window,
                    state,
                    action: match data[0] {
                        0 => ToggleAction::Remove,
                        1 => ToggleAction::Add,
                        2 => ToggleAction::Toggle,
                        _ => return None,
                    },
                    on_root: event.window == self.screen.root,
                });
            }
        } else if event.type_ == self.atoms._NET_MOVERESIZE_WINDOW {
            // TODO: handle gravity
            let x = data.get(1)?.clone();
            let y = data.get(2)?.clone();
            let width = data.get(3)?.clone();
            let height = data.get(4)?.clone();

            return Some(Event::PlacementRequest {
                window: event.window,
                pos: Some(Pos {
                    x: x as i32,
                    y: y as i32,
                }),
                dim: Some(Dim {
                    w: width as i32,
                    h: height as i32,
                }),
                on_root: event.window == self.screen.root,
            });
        } else if event.type_ == self.atoms._NET_WM_MOVERESIZE {
            let x_root = data.get(0)?.clone();
            let y_root = data.get(1)?.clone();
            let direction = data.get(2)?.clone();

            return Some(Event::GripRequest {
                window: event.window,
                pos: Pos {
                    x: x_root as i32,
                    y: y_root as i32,
                },
                grip: match direction {
                    0 => Some(Grip::Corner(Corner::TopLeft)),
                    1 => Some(Grip::Edge(Edge::Top)),
                    2 => Some(Grip::Corner(Corner::TopRight)),
                    3 => Some(Grip::Edge(Edge::Right)),
                    4 => Some(Grip::Corner(Corner::BottomRight)),
                    5 => Some(Grip::Edge(Edge::Bottom)),
                    6 => Some(Grip::Corner(Corner::BottomLeft)),
                    7 => Some(Grip::Edge(Edge::Left)),
                    8 => None,
                    _ => return None,
                },
                on_root: event.window == self.screen.root,
            });
        } else if event.type_ == self.atoms._NET_REQUEST_FRAME_EXTENTS {
            return Some(Event::FrameExtentsRequest {
                window: event.window,
                on_root: event.window == self.screen.root,
            });
        } else if event.type_ == self.atoms._NET_CURRENT_DESKTOP {
            return Some(Event::WorkspaceRequest {
                window: None,
                index: data.get(0)?.clone(),
                on_root: event.window == self.screen.root,
            });
        } else if event.type_ == self.atoms._NET_CLOSE_WINDOW {
            return Some(Event::CloseRequest {
                window: event.window,
                on_root: event.window == self.screen.root,
            });
        } else if event.type_ == self.atoms._NET_ACTIVE_WINDOW {
            if let Some(&source) = data.get(0) {
                if source <= 2 {
                    return Some(Event::FocusRequest {
                        window: event.window,
                        on_root: event.window == self.screen.root,
                    });
                }
            }
        }

        None
    }

    #[inline]
    fn on_mapping_notify(
        &self,
        event: &xproto::MappingNotifyEvent,
    ) -> Option<Event> {
        None
    }

    #[inline]
    fn on_randr_notify(
        &self,
        _event: &randr::NotifyEvent,
    ) -> Option<Event> {
        Some(Event::ScreenChange)
    }
}

impl<'conn, Conn: connection::Connection> Connection for XConnection<'conn, Conn> {
    #[inline]
    fn flush(&self) -> bool {
        self.conn.flush().is_ok()
    }

    #[inline]
    fn step(&self) -> Option<Event> {
        self.conn
            .wait_for_event()
            .ok()
            .and_then(|event| match event {
                XEvent::ButtonPress(e) => self.on_button_press(&e),
                XEvent::ButtonRelease(e) => self.on_button_release(&e),
                XEvent::MotionNotify(e) => self.on_motion_notify(&e),
                XEvent::KeyPress(e) => self.on_key_press(&e),
                XEvent::MapRequest(e) => self.on_map_request(&e),
                XEvent::MapNotify(e) => self.on_map_notify(&e),
                XEvent::EnterNotify(e) => self.on_enter_notify(&e),
                XEvent::LeaveNotify(e) => self.on_leave_notify(&e),
                XEvent::DestroyNotify(e) => self.on_destroy_notify(&e),
                XEvent::Expose(e) => self.on_expose(&e),
                XEvent::UnmapNotify(e) => self.on_unmap_notify(&e),
                XEvent::ConfigureRequest(e) => self.on_configure_request(&e),
                XEvent::ConfigureNotify(e) => self.on_configure_notify(&e),
                XEvent::PropertyNotify(e) => self.on_property_notify(&e),
                XEvent::ClientMessage(e) => self.on_client_message(&e),
                XEvent::MappingNotify(e) => self.on_mapping_notify(&e),
                XEvent::RandrNotify(e) => self.on_randr_notify(&e),
                _ => None,
            })
    }

    fn connected_outputs(&self) -> Vec<Screen> {
        let resources = randr::get_screen_resources(self.conn, self.check_window);

        if let Ok(resources) = resources {
            if let Ok(reply) = resources.reply() {
                return reply
                    .crtcs
                    .into_iter()
                    .flat_map(|crtc| {
                        randr::get_crtc_info(self.conn, crtc, 0).map(|cookie| cookie.reply())
                    })
                    .enumerate()
                    .map(|(i, reply)| {
                        let reply = reply.unwrap();
                        let region = Region {
                            pos: Pos {
                                x: reply.x as i32,
                                y: reply.y as i32,
                            },
                            dim: Dim {
                                w: reply.width as i32,
                                h: reply.height as i32,
                            },
                        };

                        Screen::new(region, i)
                    })
                    .filter(|screen| {
                        let region = screen.full_region();
                        region.dim.w > 0 && region.dim.h > 0
                    })
                    .collect();
            }
        }

        panic!("could not obtain screen resources")
    }

    fn top_level_windows(&self) -> Vec<Window> {
        self.conn
            .query_tree(self.screen.root)
            .map_or(Vec::with_capacity(0), |cookie| {
                cookie.reply().map_or(Vec::with_capacity(0), |reply| {
                    reply
                        .children
                        .into_iter()
                        .filter(|&w| self.must_manage_window(w))
                        .collect()
                })
            })
    }

    #[inline]
    fn get_pointer_position(&self) -> Pos {
        self.conn
            .query_pointer(self.screen.root)
            .map_or(Pos::default(), |cookie| {
                cookie.reply().map_or(Pos::default(), |reply| Pos {
                    x: reply.root_x as i32,
                    y: reply.root_y as i32,
                })
            })
    }

    #[inline]
    fn warp_pointer_center_of_window_or_root(
        &self,
        window: Option<Window>,
        screen: &Screen,
    ) {
        let pos = Pos::from_center_of_dim(
            window
                .and_then(|window| self.get_window_geometry(window).ok())
                .unwrap_or(screen.placeable_region())
                .dim,
        );

        drop(self.conn.warp_pointer(
            x11rb::NONE,
            window.unwrap_or(self.screen.root),
            0,
            0,
            0,
            0,
            pos.x as i16,
            pos.y as i16,
        ));
    }

    #[inline]
    fn warp_pointer(
        &self,
        pos: Pos,
    ) {
        drop(self.conn.warp_pointer(
            x11rb::NONE,
            self.screen.root,
            0,
            0,
            0,
            0,
            pos.x as i16,
            pos.y as i16,
        ));
    }

    fn warp_pointer_rpos(
        &self,
        window: Window,
        pos: Pos,
    ) {
        drop(
            self.conn
                .warp_pointer(x11rb::NONE, window, 0, 0, 0, 0, pos.x as i16, pos.y as i16),
        );
    }

    #[inline]
    fn confine_pointer(
        &self,
        window: Window,
    ) {
        if self.confined_to.get().is_none() {
            if self
                .conn
                .grab_pointer(
                    false,
                    self.screen.root,
                    u32::from(EventMask::POINTER_MOTION | EventMask::BUTTON_RELEASE) as u16,
                    xproto::GrabMode::ASYNC,
                    xproto::GrabMode::ASYNC,
                    self.screen.root,
                    x11rb::NONE,
                    x11rb::CURRENT_TIME,
                )
                .is_ok()
            {
                drop(self.conn.grab_keyboard(
                    false,
                    self.screen.root,
                    x11rb::CURRENT_TIME,
                    xproto::GrabMode::ASYNC,
                    xproto::GrabMode::ASYNC,
                ));

                self.confined_to.set(Some(window));
            }
        }
    }

    #[inline]
    fn release_pointer(&self) {
        if self.confined_to.get().is_some() {
            drop(self.conn.ungrab_pointer(x11rb::CURRENT_TIME));
            drop(self.conn.ungrab_keyboard(x11rb::CURRENT_TIME));

            self.confined_to.set(None);
        }
    }

    fn cleanup(&self) {
        drop(
            self.conn
                .ungrab_key(xproto::Grab::ANY, self.screen.root, xproto::ModMask::ANY),
        );

        drop(self.conn.destroy_window(self.check_window));

        drop(
            self.conn
                .delete_property(self.screen.root, self.atoms._NET_ACTIVE_WINDOW),
        );

        drop(
            self.conn
                .delete_property(self.screen.root, self.atoms._NET_SUPPORTING_WM_CHECK),
        );

        drop(
            self.conn
                .delete_property(self.screen.root, self.atoms._NET_WM_NAME),
        );

        drop(
            self.conn
                .delete_property(self.screen.root, self.atoms.WM_CLASS),
        );

        drop(
            self.conn
                .delete_property(self.screen.root, self.atoms._NET_SUPPORTED),
        );

        drop(
            self.conn
                .delete_property(self.screen.root, self.atoms._NET_WM_PID),
        );

        drop(
            self.conn
                .delete_property(self.screen.root, self.atoms._NET_CLIENT_LIST),
        );

        drop(self.conn);
    }

    #[inline]
    fn create_frame(
        &self,
        region: Region,
    ) -> Window {
        const ERR: &str = "unable to create frame";

        let frame = self.conn.generate_id().expect(ERR);
        let aux = xproto::CreateWindowAux::new()
            .backing_store(Some(xproto::BackingStore::ALWAYS))
            .event_mask(EventMask::EXPOSURE | EventMask::KEY_PRESS);

        self.conn
            .create_window(
                x11rb::COPY_DEPTH_FROM_PARENT,
                frame,
                self.screen.root,
                region.pos.x as i16,
                region.pos.y as i16,
                region.dim.w as u16,
                region.dim.h as u16,
                0,
                xproto::WindowClass::INPUT_OUTPUT,
                0,
                &aux,
            )
            .expect(ERR);

        self.flush();

        frame
    }

    #[inline]
    fn create_handle(&self) -> Window {
        const ERR: &str = "unable to create handle";

        let handle = self.conn.generate_id().expect(ERR);
        let aux = xproto::CreateWindowAux::new().override_redirect(1);

        self.conn
            .create_window(
                x11rb::COPY_DEPTH_FROM_PARENT,
                handle,
                self.screen.root,
                -2,
                -2,
                1,
                1,
                0,
                xproto::WindowClass::INPUT_ONLY,
                0,
                &aux,
            )
            .expect(ERR);

        self.flush();

        handle
    }

    #[inline]
    fn init_window(
        &self,
        window: Window,
        focus_follows_mouse: bool,
    ) {
        drop(self.conn.change_window_attributes(
            window,
            &xproto::ChangeWindowAttributesAux::default().event_mask(self.window_event_mask),
        ));
    }

    #[inline]
    fn init_frame(
        &self,
        window: Window,
        focus_follows_mouse: bool,
    ) {
        drop(self.conn.change_window_attributes(
            window,
            &xproto::ChangeWindowAttributesAux::default().event_mask(
                self.frame_event_mask
                    | if focus_follows_mouse {
                        EventMask::ENTER_WINDOW
                    } else {
                        EventMask::NO_EVENT
                    },
            ),
        ));
    }

    #[inline]
    fn init_unmanaged(
        &self,
        window: Window,
    ) {
        drop(self.conn.change_window_attributes(
            window,
            &xproto::ChangeWindowAttributesAux::default().event_mask(EventMask::STRUCTURE_NOTIFY),
        ));
    }

    #[inline]
    fn cleanup_window(
        &self,
        window: Window,
    ) {
        drop(self.conn.delete_property(window, self.atoms._NET_WM_STATE));
        drop(
            self.conn
                .delete_property(window, self.atoms._NET_WM_DESKTOP),
        );
    }

    #[inline]
    fn map_window(
        &self,
        window: Window,
    ) {
        drop(self.conn.map_window(window));
    }

    #[inline]
    fn unmap_window(
        &self,
        window: Window,
    ) {
        drop(self.conn.unmap_window(window));
    }

    #[inline]
    fn reparent_window(
        &self,
        window: Window,
        parent: Window,
        pos: Pos,
    ) {
        drop(
            self.conn
                .reparent_window(window, parent, pos.x as i16, pos.y as i16),
        );
    }

    #[inline]
    fn unparent_window(
        &self,
        window: Window,
        pos: Pos,
    ) {
        drop(
            self.conn
                .reparent_window(window, self.screen.root, pos.x as i16, pos.y as i16),
        );
    }

    #[inline]
    fn destroy_window(
        &self,
        window: Window,
    ) {
        drop(self.conn.destroy_window(window));
    }

    #[inline]
    fn close_window(
        &self,
        window: Window,
    ) -> bool {
        self.send_protocol_client_message(window, self.atoms.WM_DELETE_WINDOW)
            .map_or(false, |_| self.flush())
    }

    #[inline]
    fn kill_window(
        &self,
        window: Window,
    ) -> bool {
        let protocols = &[self.atoms.WM_DELETE_WINDOW];

        if self.window_has_any_of_protocols(window, protocols) {
            self.close_window(window)
        } else {
            self.conn
                .kill_client(window)
                .map_or(false, |_| self.flush())
        }
    }

    #[inline]
    fn place_window(
        &self,
        window: Window,
        region: &Region,
    ) {
        drop(
            self.conn.configure_window(
                window,
                &xproto::ConfigureWindowAux::default()
                    .x(region.pos.x as i32)
                    .y(region.pos.y as i32)
                    .width(region.dim.w as u32)
                    .height(region.dim.h as u32),
            ),
        );
    }

    #[inline]
    fn move_window(
        &self,
        window: Window,
        pos: Pos,
    ) {
        drop(
            self.conn.configure_window(
                window,
                &xproto::ConfigureWindowAux::default()
                    .x(pos.x as i32)
                    .y(pos.y as i32),
            ),
        );
    }

    #[inline]
    fn resize_window(
        &self,
        window: Window,
        dim: Dim,
    ) {
        drop(
            self.conn.configure_window(
                window,
                &xproto::ConfigureWindowAux::default()
                    .width(dim.w as u32)
                    .height(dim.h as u32),
            ),
        );
    }

    #[inline]
    fn focus_window(
        &self,
        window: Window,
    ) {
        drop(
            self.conn
                .set_input_focus(xproto::InputFocus::PARENT, window, x11rb::CURRENT_TIME),
        );

        drop(self.conn.change_property32(
            xproto::PropMode::REPLACE,
            self.screen.root,
            self.atoms._NET_ACTIVE_WINDOW,
            xproto::AtomEnum::WINDOW,
            &[window],
        ));
    }

    #[inline]
    fn stack_window_above(
        &self,
        window: Window,
        sibling: Option<Window>,
    ) {
        let mut aux = xproto::ConfigureWindowAux::default().stack_mode(xproto::StackMode::ABOVE);

        if let Some(sibling) = sibling {
            aux = aux.sibling(sibling);
        }

        drop(self.conn.configure_window(window, &aux));
    }

    #[inline]
    fn stack_window_below(
        &self,
        window: Window,
        sibling: Option<Window>,
    ) {
        let mut aux = xproto::ConfigureWindowAux::default().stack_mode(xproto::StackMode::BELOW);

        if let Some(sibling) = sibling {
            aux = aux.sibling(sibling);
        }

        drop(self.conn.configure_window(window, &aux));
    }

    #[inline]
    fn insert_window_in_save_set(
        &self,
        window: Window,
    ) {
        drop(self.conn.change_save_set(xproto::SetMode::INSERT, window));
    }

    fn grab_bindings(
        &self,
        key_inputs: &[&KeyInput],
        mouse_inputs: &[&MouseInput],
    ) {
        for &m in &[0, u16::from(ModMask::M2), u16::from(ModMask::M5)] {
            for key_input in key_inputs {
                drop(
                    self.conn.grab_key(
                        false,
                        self.screen.root,
                        key_input
                            .modifiers
                            .iter()
                            .fold(0u16, |acc, &m| acc | {
                                u16::from(match m {
                                    Modifier::Ctrl => xproto::ModMask::CONTROL,
                                    Modifier::Shift => xproto::ModMask::SHIFT,
                                    Modifier::Alt => xproto::ModMask::M1,
                                    Modifier::Super => xproto::ModMask::M4,
                                    Modifier::NumLock => xproto::ModMask::M2,
                                    Modifier::ScrollLock => xproto::ModMask::M5,
                                    _ => xproto::ModMask::ANY,
                                })
                            })
                            | u16::from(m),
                        self.get_keycode(key_input.key),
                        xproto::GrabMode::ASYNC,
                        xproto::GrabMode::ASYNC,
                    ),
                );
            }

            for mouse_input in mouse_inputs {
                drop(
                    self.conn.grab_button(
                        false,
                        self.screen.root,
                        u32::from(self.mouse_event_mask) as u16,
                        xproto::GrabMode::ASYNC,
                        xproto::GrabMode::ASYNC,
                        x11rb::NONE,
                        x11rb::NONE,
                        xproto::ButtonIndex::try_from(mouse_input.button as u8).unwrap(),
                        mouse_input
                            .modifiers
                            .iter()
                            .fold(0u16, |acc, &m| acc | {
                                u16::from(match m {
                                    Modifier::Ctrl => xproto::ModMask::CONTROL,
                                    Modifier::Shift => xproto::ModMask::SHIFT,
                                    Modifier::Alt => xproto::ModMask::M1,
                                    Modifier::Super => xproto::ModMask::M4,
                                    Modifier::NumLock => xproto::ModMask::M2,
                                    Modifier::ScrollLock => xproto::ModMask::M5,
                                    _ => xproto::ModMask::ANY,
                                })
                            })
                            | u16::from(m),
                    ),
                );
            }
        }

        drop(self.conn.change_window_attributes(
            self.screen.root,
            &xproto::ChangeWindowAttributesAux::default().event_mask(self.root_event_mask),
        ));

        self.flush();
    }

    #[inline]
    fn regrab_buttons(
        &self,
        window: Window,
    ) {
        drop(self.conn.grab_button(
            true,
            window,
            u32::from(self.regrab_event_mask) as u16,
            xproto::GrabMode::ASYNC,
            xproto::GrabMode::ASYNC,
            x11rb::NONE,
            x11rb::NONE,
            xproto::ButtonIndex::ANY,
            xproto::ModMask::ANY,
        ));
    }

    #[inline]
    fn ungrab_buttons(
        &self,
        window: Window,
    ) {
        drop(
            self.conn
                .ungrab_button(xproto::ButtonIndex::ANY, window, xproto::ModMask::ANY),
        );
    }

    #[inline]
    fn unfocus(&self) {
        drop(self.conn.set_input_focus(
            xproto::InputFocus::PARENT,
            self.check_window,
            x11rb::CURRENT_TIME,
        ));

        drop(
            self.conn
                .delete_property(self.screen.root, self.atoms._NET_ACTIVE_WINDOW),
        );
    }

    #[inline]
    fn set_window_border_width(
        &self,
        window: Window,
        width: u32,
    ) {
        drop(self.conn.configure_window(
            window,
            &xproto::ConfigureWindowAux::default().border_width(width),
        ));
    }

    #[inline]
    fn set_window_border_color(
        &self,
        window: Window,
        color: u32,
    ) {
        drop(self.conn.change_window_attributes(
            window,
            &xproto::ChangeWindowAttributesAux::default().border_pixel(color),
        ));
    }

    #[inline]
    fn set_window_background_color(
        &self,
        window: Window,
        color: u32,
    ) {
        if let Ok(geometry) = self.get_window_geometry(window) {
            drop(self.conn.change_gc(
                self.background_gc,
                &xproto::ChangeGCAux::new().foreground(color),
            ));

            drop(
                self.conn
                    .poly_fill_rectangle(window, self.background_gc, &[xproto::Rectangle {
                        x: 0,
                        y: 0,
                        width: geometry.dim.w as u16,
                        height: geometry.dim.h as u16,
                    }]),
            );
        }
    }

    #[inline]
    fn update_window_offset(
        &self,
        window: Window,
        frame: Window,
    ) {
        if let Ok(frame_geometry) = self.get_window_geometry(frame) {
            if let Ok(window_geometry) = self.get_window_geometry(window) {
                let event = xproto::ConfigureNotifyEvent {
                    response_type: xproto::CONFIGURE_NOTIFY_EVENT,
                    sequence: 0,
                    event: window,
                    window,
                    above_sibling: x11rb::NONE,
                    x: (frame_geometry.pos.x + window_geometry.pos.x) as i16,
                    y: (frame_geometry.pos.y + window_geometry.pos.y) as i16,
                    width: window_geometry.dim.w as u16,
                    height: window_geometry.dim.h as u16,
                    border_width: 0,
                    override_redirect: false,
                };

                drop(
                    self.conn
                        .send_event(false, window, EventMask::STRUCTURE_NOTIFY, &event),
                );
            }
        }
    }

    #[inline]
    fn get_focused_window(&self) -> Window {
        self.conn
            .get_input_focus()
            .map_or(self.screen.root, |cookie| {
                cookie.reply().map_or(self.screen.root, |reply| reply.focus)
            })
    }

    #[inline]
    fn get_window_geometry(
        &self,
        window: Window,
    ) -> Result<Region> {
        Ok(self.conn.get_geometry(window)?.reply().map(|reply| {
            Region::new(
                reply.x as i32,
                reply.y as i32,
                reply.width as i32,
                reply.height as i32,
            )
        })?)
    }

    #[inline]
    fn get_window_pid(
        &self,
        window: Window,
    ) -> Option<Pid> {
        let id_spec = protocol::res::ClientIdSpec {
            client: window,
            mask: u8::from(protocol::res::ClientIdMask::LOCAL_CLIENT_PID) as u32,
        };

        protocol::res::query_client_ids(self.conn, &[id_spec])
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .and_then(|reply| {
                for i in reply.ids {
                    if (i.spec.mask
                        & (u8::from(protocol::res::ClientIdMask::LOCAL_CLIENT_PID)) as u32)
                        != 0
                    {
                        if i.value.len() > 0 && i.value[0] != 0 {
                            return Some(i.value[0] as Pid);
                        }
                    }
                }

                None
            })
    }

    #[inline]
    fn must_manage_window(
        &self,
        window: Window,
    ) -> bool {
        let do_not_manage = self
            .conn
            .get_window_attributes(window)
            .map_or(false, |cookie| {
                cookie.reply().map_or(false, |reply| {
                    reply.override_redirect || reply.class == xproto::WindowClass::INPUT_ONLY
                })
            });

        if do_not_manage {
            return false;
        }

        let to_exclude = &[
            self.atoms._NET_WM_WINDOW_TYPE_DOCK,
            self.atoms._NET_WM_WINDOW_TYPE_TOOLBAR,
        ];

        !self.window_is_any_of_types(window, to_exclude)
    }

    #[inline]
    fn must_free_window(
        &self,
        window: Window,
    ) -> bool {
        if self.get_window_desktop(window) == Some(0xFFFFFFFF)
            || self.window_is_any_of_states(window, &[self.atoms._NET_WM_STATE_MODAL])
            || self.window_is_any_of_types(window, &[
                self.atoms._NET_WM_WINDOW_TYPE_DIALOG,
                self.atoms._NET_WM_WINDOW_TYPE_UTILITY,
                self.atoms._NET_WM_WINDOW_TYPE_TOOLBAR,
                self.atoms._NET_WM_WINDOW_TYPE_SPLASH,
            ])
        {
            return true;
        }

        self.get_window_geometry(window).map_or(false, |geometry| {
            let (_, sh) = self.get_icccm_window_size_hints(window, None, &None);

            sh.map_or(false, |sh| {
                match (sh.min_width, sh.min_height, sh.max_width, sh.max_height) {
                    (Some(miw), Some(mih), Some(maw), Some(mah)) => {
                        maw > 0 && mah > 0 && maw == miw && mah == mih
                    },
                    _ => false,
                }
            })
        })
    }

    fn window_is_mappable(
        &self,
        window: Window,
    ) -> bool {
        self.conn
            .get_window_attributes(window)
            .map_or(false, |cookie| {
                cookie.reply().map_or(false, |reply| {
                    let default_state = properties::WmHintsState::Normal;
                    let initial_state = properties::WmHints::get(self.conn, window).ok().map_or(
                        default_state,
                        |cookie| {
                            cookie.reply().map_or(default_state, |reply| {
                                reply.initial_state.unwrap_or(default_state)
                            })
                        },
                    );

                    reply.class != xproto::WindowClass::INPUT_ONLY
                        && !self.window_is_any_of_states(window, &[self.atoms._NET_WM_STATE_HIDDEN])
                        && match initial_state {
                            properties::WmHintsState::Normal => true,
                            _ => false,
                        }
                })
            })
    }

    #[inline]
    fn set_icccm_window_state(
        &self,
        window: Window,
        state: IcccmWindowState,
    ) {
        drop(self.conn.change_property32(
            xproto::PropMode::REPLACE,
            window,
            self.atoms.WM_STATE,
            self.atoms.CARDINAL,
            &[
                match state {
                    IcccmWindowState::Withdrawn => 0,
                    IcccmWindowState::Normal => 1,
                    IcccmWindowState::Iconic => 3,
                },
                0,
            ],
        ));
    }

    #[inline]
    fn set_icccm_window_hints(
        &self,
        window: Window,
        hints: Hints,
    ) {
        let wm_hints = properties::WmHints {
            input: hints.input,
            initial_state: match hints.initial_state {
                Some(IcccmWindowState::Normal) => Some(properties::WmHintsState::Normal),
                Some(IcccmWindowState::Iconic) => Some(properties::WmHintsState::Iconic),
                _ => None,
            },
            icon_pixmap: None,
            icon_window: None,
            icon_position: None,
            icon_mask: None,
            window_group: hints.group,
            urgent: hints.urgent,
        };

        drop(wm_hints.set(self.conn, window));
    }

    #[inline]
    fn get_icccm_window_name(
        &self,
        window: Window,
    ) -> String {
        static NO_NAME: &str = "n/a";

        self.conn
            .get_property(
                false,
                window,
                self.atoms.WM_NAME,
                self.atoms.UTF8_STRING,
                0,
                std::u32::MAX,
            )
            .map_or(NO_NAME.to_owned(), |cookie| {
                cookie.reply().map_or(NO_NAME.to_owned(), |reply| {
                    std::str::from_utf8(
                        &reply
                            .value8()
                            .map_or(Vec::with_capacity(0), |value| value.collect::<Vec<u8>>()),
                    )
                    .map_or(NO_NAME.to_owned(), |name| name.to_owned())
                })
            })
    }

    #[inline]
    fn get_icccm_window_class(
        &self,
        window: Window,
    ) -> String {
        static NO_CLASS: &str = "n/a";

        properties::WmClass::get(self.conn, window).map_or(NO_CLASS.to_owned(), |cookie| {
            cookie.reply().map_or(NO_CLASS.to_owned(), |reply| {
                std::str::from_utf8(reply.class())
                    .map_or(NO_CLASS.to_owned(), |class| class.to_owned())
            })
        })
    }

    #[inline]
    fn get_icccm_window_instance(
        &self,
        window: Window,
    ) -> String {
        static NO_INSTANCE: &str = "n/a";

        properties::WmClass::get(self.conn, window).map_or(NO_INSTANCE.to_owned(), |cookie| {
            cookie.reply().map_or(NO_INSTANCE.to_owned(), |reply| {
                std::str::from_utf8(reply.instance())
                    .map_or(NO_INSTANCE.to_owned(), |instance| instance.to_owned())
            })
        })
    }

    #[inline]
    fn get_icccm_window_transient_for(
        &self,
        window: Window,
    ) -> Option<Window> {
        self.conn
            .get_property(
                false,
                window,
                self.atoms.WM_TRANSIENT_FOR,
                self.atoms.WINDOW,
                0,
                std::u32::MAX,
            )
            .ok()?
            .reply()
            .ok()
            .and_then(|transient_for| {
                let transient_for: Vec<u32> = transient_for.value32()?.collect();

                if transient_for.is_empty() {
                    None
                } else {
                    Some(transient_for[0])
                }
            })
    }

    #[inline]
    fn get_icccm_window_client_leader(
        &self,
        window: Window,
    ) -> Option<Window> {
        self.conn
            .get_property(
                false,
                window,
                self.atoms.WM_CLIENT_LEADER,
                self.atoms.WINDOW,
                0,
                std::u32::MAX,
            )
            .ok()?
            .reply()
            .ok()
            .and_then(|client_leader| {
                let client_leader: Vec<u32> = client_leader.value32()?.collect();

                if client_leader.is_empty() {
                    None
                } else {
                    Some(client_leader[0])
                }
            })
    }

    #[inline]
    fn get_icccm_window_hints(
        &self,
        window: Window,
    ) -> Option<Hints> {
        let hints = properties::WmHints::get(self.conn, window)
            .ok()?
            .reply()
            .ok()?;

        let urgent = hints.urgent;
        let input = hints.input;
        let group = hints.window_group;

        let initial_state = hints.initial_state.map(|state| match state {
            properties::WmHintsState::Normal => IcccmWindowState::Normal,
            properties::WmHintsState::Iconic => IcccmWindowState::Iconic,
        });

        Some(Hints {
            urgent,
            input,
            initial_state,
            group,
        })
    }

    #[inline]
    fn get_icccm_window_size_hints(
        &self,
        window: Window,
        min_window_dim: Option<Dim>,
        current_size_hints: &Option<SizeHints>,
    ) -> (bool, Option<SizeHints>) {
        let size_hints = properties::WmSizeHints::get_normal_hints(self.conn, window)
            .ok()
            .and_then(|cookie| cookie.reply().ok());

        if size_hints.is_none() {
            return (!current_size_hints.is_none(), None);
        }

        let size_hints = size_hints.unwrap();

        let (by_user, pos) = size_hints.position.map_or((false, None), |(spec, x, y)| {
            (
                match spec {
                    properties::WmSizeHintsSpecification::UserSpecified => true,
                    _ => false,
                },
                if x > 0 || y > 0 {
                    Some(Pos {
                        x,
                        y,
                    })
                } else {
                    None
                },
            )
        });

        let (sh_min_width, sh_min_height) =
            size_hints.min_size.map_or((None, None), |(width, height)| {
                (
                    if width > 0 { Some(width) } else { None },
                    if height > 0 {
                        Some(height as i32)
                    } else {
                        None
                    },
                )
            });

        let (sh_base_width, sh_base_height) =
            size_hints
                .base_size
                .map_or((None, None), |(width, height)| {
                    (
                        if width > 0 { Some(width) } else { None },
                        if height > 0 { Some(height) } else { None },
                    )
                });

        let (max_width, max_height) =
            size_hints.max_size.map_or((None, None), |(width, height)| {
                (
                    if width > 0 { Some(width) } else { None },
                    if height > 0 { Some(height) } else { None },
                )
            });

        let min_width = if sh_min_width.is_some() {
            sh_min_width
        } else {
            sh_base_width
        };
        let min_height = if sh_min_height.is_some() {
            sh_min_height
        } else {
            sh_base_height
        };

        let base_width = if sh_base_width.is_some() {
            sh_base_width
        } else {
            sh_min_width
        };
        let base_height = if sh_base_height.is_some() {
            sh_base_height
        } else {
            sh_min_height
        };

        let min_width = if let Some(min_width) = min_width {
            if let Some(min_window_dim) = min_window_dim {
                if min_width >= min_window_dim.w {
                    Some(min_width)
                } else {
                    Some(min_window_dim.w)
                }
            } else if min_width > 0 {
                Some(min_width)
            } else {
                None
            }
        } else {
            None
        };

        let min_height = if let Some(min_height) = min_height {
            if let Some(min_window_dim) = min_window_dim {
                if min_height >= min_window_dim.h {
                    Some(min_height)
                } else {
                    Some(min_window_dim.h)
                }
            } else if min_height > 0 {
                Some(min_height)
            } else {
                None
            }
        } else {
            None
        };

        let (inc_width, inc_height) =
            size_hints
                .size_increment
                .map_or((None, None), |(inc_width, inc_height)| {
                    (
                        if inc_width > 0 && inc_width < 0xFFFF {
                            Some(inc_width)
                        } else {
                            None
                        },
                        if inc_height > 0 && inc_height < 0xFFFF {
                            Some(inc_height)
                        } else {
                            None
                        },
                    )
                });

        let ((min_ratio, max_ratio), (min_ratio_vulgar, max_ratio_vulgar)) = size_hints
            .aspect
            .map_or(((None, None), (None, None)), |(min_ratio, max_ratio)| {
                (
                    (
                        if min_ratio.numerator > 0 && min_ratio.denominator > 0 {
                            Some(min_ratio.numerator as f64 / min_ratio.denominator as f64)
                        } else {
                            None
                        },
                        if max_ratio.numerator > 0 && max_ratio.denominator > 0 {
                            Some(max_ratio.numerator as f64 / max_ratio.denominator as f64)
                        } else {
                            None
                        },
                    ),
                    (
                        Some(Ratio {
                            numerator: min_ratio.numerator as i32,
                            denominator: min_ratio.denominator as i32,
                        }),
                        Some(Ratio {
                            numerator: max_ratio.numerator as i32,
                            denominator: max_ratio.denominator as i32,
                        }),
                    ),
                )
            });

        let size_hints = Some(SizeHints {
            by_user,
            pos,
            min_width,
            min_height,
            max_width,
            max_height,
            base_width,
            base_height,
            inc_width,
            inc_height,
            min_ratio,
            max_ratio,
            min_ratio_vulgar,
            max_ratio_vulgar,
        });

        (*current_size_hints != size_hints, size_hints)
    }

    fn init_wm_properties(
        &self,
        wm_name: &str,
        desktop_names: &[&str],
    ) {
        let wm_instance_class_names = &[wm_name, wm_name];
        let wm_class = wm_instance_class_names.join("\0");

        drop(self.conn.change_property8(
            xproto::PropMode::REPLACE,
            self.check_window,
            self.atoms._NET_WM_NAME,
            self.atoms.UTF8_STRING,
            wm_name.as_bytes(),
        ));

        drop(self.conn.change_property8(
            xproto::PropMode::REPLACE,
            self.check_window,
            self.atoms.WM_CLASS,
            self.atoms.UTF8_STRING,
            wm_class.as_bytes(),
        ));

        drop(self.conn.change_property32(
            xproto::PropMode::REPLACE,
            self.check_window,
            self.atoms._NET_WM_PID,
            self.atoms.CARDINAL,
            &[std::process::id() as u32],
        ));

        drop(self.conn.change_property32(
            xproto::PropMode::REPLACE,
            self.screen.root,
            self.atoms._NET_SUPPORTING_WM_CHECK,
            self.atoms.WINDOW,
            &[self.check_window],
        ));

        drop(self.conn.change_property8(
            xproto::PropMode::REPLACE,
            self.screen.root,
            self.atoms._NET_WM_NAME,
            self.atoms.UTF8_STRING,
            wm_name.as_bytes(),
        ));

        drop(self.conn.change_property8(
            xproto::PropMode::REPLACE,
            self.screen.root,
            self.atoms.WM_CLASS,
            self.atoms.UTF8_STRING,
            wm_class.as_bytes(),
        ));

        drop(self.conn.change_property32(
            xproto::PropMode::REPLACE,
            self.check_window,
            self.atoms._NET_SUPPORTING_WM_CHECK,
            self.atoms.WINDOW,
            &[self.check_window],
        ));

        drop(self.conn.change_property32(
            xproto::PropMode::REPLACE,
            self.screen.root,
            self.atoms._NET_SUPPORTED,
            self.atoms.ATOM,
            &[
                self.atoms._NET_ACTIVE_WINDOW,
                self.atoms._NET_CLIENT_LIST,
                self.atoms._NET_CLIENT_LIST_STACKING,
                self.atoms._NET_CLOSE_WINDOW,
                self.atoms._NET_CURRENT_DESKTOP,
                self.atoms._NET_DESKTOP_NAMES,
                self.atoms._NET_DESKTOP_VIEWPORT,
                self.atoms._NET_MOVERESIZE_WINDOW,
                self.atoms._NET_NUMBER_OF_DESKTOPS,
                self.atoms._NET_SUPPORTED,
                self.atoms._NET_SUPPORTING_WM_CHECK,
                self.atoms._NET_WM_DESKTOP,
                self.atoms._NET_MOVERESIZE_WINDOW,
                self.atoms._NET_WM_MOVERESIZE,
                self.atoms._NET_WM_NAME,
                self.atoms._NET_WM_STATE,
                self.atoms._NET_WM_STATE_DEMANDS_ATTENTION,
                self.atoms._NET_WM_STATE_FOCUSED,
                self.atoms._NET_WM_STATE_FULLSCREEN,
                self.atoms._NET_WM_STATE_HIDDEN,
                self.atoms._NET_WM_STATE_MODAL,
                self.atoms._NET_WM_STATE_STICKY,
                self.atoms._NET_WM_STRUT_PARTIAL,
                self.atoms._NET_WM_VISIBLE_NAME,
                self.atoms._NET_WM_WINDOW_TYPE,
                self.atoms._NET_WM_WINDOW_TYPE_DIALOG,
                self.atoms._NET_WM_WINDOW_TYPE_DOCK,
                self.atoms._NET_WM_WINDOW_TYPE_DROPDOWN_MENU,
                self.atoms._NET_WM_WINDOW_TYPE_MENU,
                self.atoms._NET_WM_WINDOW_TYPE_NORMAL,
                self.atoms._NET_WM_WINDOW_TYPE_NOTIFICATION,
                self.atoms._NET_WM_WINDOW_TYPE_POPUP_MENU,
                self.atoms._NET_WM_WINDOW_TYPE_SPLASH,
                self.atoms._NET_WM_WINDOW_TYPE_TOOLBAR,
                self.atoms._NET_WM_WINDOW_TYPE_TOOLTIP,
                self.atoms._NET_WM_WINDOW_TYPE_UTILITY,
            ],
        ));

        drop(self.conn.change_property32(
            xproto::PropMode::REPLACE,
            self.screen.root,
            self.atoms._NET_WM_PID,
            self.atoms.CARDINAL,
            &[std::process::id() as u32],
        ));

        drop(
            self.conn
                .delete_property(self.screen.root, self.atoms._NET_CLIENT_LIST),
        );

        self.update_desktops(desktop_names);
    }

    #[inline]
    fn set_current_desktop(
        &self,
        index: usize,
    ) {
        drop(self.conn.change_property32(
            xproto::PropMode::REPLACE,
            self.screen.root,
            self.atoms._NET_CURRENT_DESKTOP,
            self.atoms.CARDINAL,
            &[index as u32],
        ));
    }

    #[inline]
    fn set_root_window_name(
        &self,
        name: &str,
    ) {
        drop(self.conn.change_property8(
            xproto::PropMode::REPLACE,
            self.screen.root,
            self.atoms.WM_NAME,
            self.atoms.UTF8_STRING,
            name.as_bytes(),
        ));
    }

    #[inline]
    fn set_window_desktop(
        &self,
        window: Window,
        index: usize,
    ) {
        drop(self.conn.change_property32(
            xproto::PropMode::REPLACE,
            window,
            self.atoms._NET_WM_DESKTOP,
            self.atoms.CARDINAL,
            &[index as u32],
        ));
    }

    #[inline]
    fn set_window_state(
        &self,
        window: Window,
        state: WindowState,
        on: bool,
    ) {
        self.set_window_state_atom(
            window,
            match state {
                WindowState::Modal => self.atoms._NET_WM_STATE_MODAL,
                WindowState::Sticky => self.atoms._NET_WM_STATE_STICKY,
                WindowState::MaximizedVert => self.atoms._NET_WM_STATE_MAXIMIZED_VERT,
                WindowState::MaximizedHorz => self.atoms._NET_WM_STATE_MAXIMIZED_HORZ,
                WindowState::Shaded => self.atoms._NET_WM_STATE_SHADED,
                WindowState::SkipTaskbar => self.atoms._NET_WM_STATE_SKIP_TASKBAR,
                WindowState::SkipPager => self.atoms._NET_WM_STATE_SKIP_PAGER,
                WindowState::Hidden => self.atoms._NET_WM_STATE_HIDDEN,
                WindowState::Fullscreen => self.atoms._NET_WM_STATE_FULLSCREEN,
                WindowState::Above => self.atoms._NET_WM_STATE_ABOVE,
                WindowState::Below => self.atoms._NET_WM_STATE_BELOW,
                WindowState::DemandsAttention => self.atoms._NET_WM_STATE_DEMANDS_ATTENTION,
            },
            on,
        );
    }

    #[inline]
    fn set_window_frame_extents(
        &self,
        window: Window,
        extents: Extents,
    ) {
        let mut frame_extents: Vec<u32> = Vec::with_capacity(4);

        frame_extents.push(extents.left as u32);
        frame_extents.push(extents.right as u32);
        frame_extents.push(extents.top as u32);
        frame_extents.push(extents.bottom as u32);

        drop(self.conn.change_property32(
            xproto::PropMode::REPLACE,
            window,
            self.atoms._NET_FRAME_EXTENTS,
            self.atoms.CARDINAL,
            &frame_extents[..],
        ));
    }

    #[inline]
    fn set_desktop_geometry(
        &self,
        geometries: &[&Region],
    ) {
        let mut areas = Vec::with_capacity(geometries.len());

        geometries.iter().for_each(|geometry| {
            areas.push(geometry.pos.x as u32);
            areas.push(geometry.pos.y as u32);
            areas.push(geometry.dim.w as u32);
            areas.push(geometry.dim.h as u32);
        });

        drop(self.conn.change_property32(
            xproto::PropMode::REPLACE,
            self.screen.root,
            self.atoms._NET_DESKTOP_GEOMETRY,
            self.atoms.CARDINAL,
            &areas[..],
        ));
    }

    #[inline]
    fn set_desktop_viewport(
        &self,
        viewports: &[&Region],
    ) {
        let mut areas = Vec::with_capacity(viewports.len());

        viewports.iter().for_each(|viewport| {
            areas.push(viewport.pos.x as u32);
            areas.push(viewport.pos.y as u32);
            areas.push(viewport.dim.w as u32);
            areas.push(viewport.dim.h as u32);
        });

        drop(self.conn.change_property32(
            xproto::PropMode::REPLACE,
            self.screen.root,
            self.atoms._NET_DESKTOP_VIEWPORT,
            self.atoms.CARDINAL,
            &areas[..],
        ));
    }

    #[inline]
    fn set_workarea(
        &self,
        workareas: &[&Region],
    ) {
        let mut areas = Vec::with_capacity(workareas.len());

        workareas.iter().for_each(|workarea| {
            areas.push(workarea.pos.x as u32);
            areas.push(workarea.pos.y as u32);
            areas.push(workarea.dim.w as u32);
            areas.push(workarea.dim.h as u32);
        });

        drop(self.conn.change_property32(
            xproto::PropMode::REPLACE,
            self.screen.root,
            self.atoms._NET_WORKAREA,
            self.atoms.CARDINAL,
            &areas[..],
        ));
    }

    #[inline]
    fn update_desktops(
        &self,
        desktop_names: &[&str],
    ) {
        drop(self.conn.change_property32(
            xproto::PropMode::REPLACE,
            self.screen.root,
            self.atoms._NET_NUMBER_OF_DESKTOPS,
            self.atoms.CARDINAL,
            &[desktop_names.len() as u32],
        ));

        drop(self.conn.change_property8(
            xproto::PropMode::REPLACE,
            self.screen.root,
            self.atoms._NET_DESKTOP_NAMES,
            self.atoms.UTF8_STRING,
            desktop_names.join("\0").as_bytes(),
        ));
    }

    #[inline]
    fn update_client_list(
        &self,
        clients: &[Window],
    ) {
        drop(self.conn.change_property32(
            xproto::PropMode::REPLACE,
            self.screen.root,
            self.atoms._NET_CLIENT_LIST,
            self.atoms.WINDOW,
            clients,
        ));
    }

    #[inline]
    fn update_client_list_stacking(
        &self,
        clients: &[Window],
    ) {
        drop(self.conn.change_property32(
            xproto::PropMode::REPLACE,
            self.screen.root,
            self.atoms._NET_CLIENT_LIST_STACKING,
            self.atoms.WINDOW,
            clients,
        ));
    }

    #[inline]
    fn get_window_strut(
        &self,
        window: Window,
    ) -> Option<Vec<Option<Strut>>> {
        if let Some(strut) = self.get_window_strut_partial(window) {
            return Some(strut);
        }

        self.conn
            .get_property(
                false,
                window,
                self.atoms._NET_WM_STRUT,
                self.atoms.CARDINAL,
                0,
                std::u32::MAX,
            )
            .ok()?
            .reply()
            .ok()
            .and_then(|strut| {
                let widths: Vec<u32> = strut.value32()?.collect();

                if widths.is_empty() {
                    return None;
                }

                let mut struts = Vec::with_capacity(4);

                for (i, &width) in widths.iter().enumerate() {
                    if i == 4 {
                        break;
                    }

                    struts.push(if width != 0 {
                        Some(Strut {
                            window,
                            width,
                        })
                    } else {
                        None
                    });
                }

                Some(struts)
            })
    }

    #[inline]
    fn get_window_strut_partial(
        &self,
        window: Window,
    ) -> Option<Vec<Option<Strut>>> {
        self.conn
            .get_property(
                false,
                window,
                self.atoms._NET_WM_STRUT_PARTIAL,
                self.atoms.CARDINAL,
                0,
                std::u32::MAX,
            )
            .ok()?
            .reply()
            .ok()
            .and_then(|strut_partial| {
                let widths: Vec<u32> = strut_partial.value32()?.collect();

                if widths.is_empty() {
                    return None;
                }

                let mut struts = Vec::with_capacity(1);

                for (i, &width) in widths.iter().enumerate() {
                    if i == 4 {
                        break;
                    }

                    struts.push(if width != 0 {
                        Some(Strut {
                            window,
                            width,
                        })
                    } else {
                        None
                    });
                }

                Some(struts)
            })
    }

    #[inline]
    fn get_window_desktop(
        &self,
        window: Window,
    ) -> Option<usize> {
        self.conn
            .get_property(
                false,
                window,
                self.atoms._NET_WM_DESKTOP,
                self.atoms.CARDINAL,
                0,
                std::u32::MAX,
            )
            .ok()?
            .reply()
            .ok()
            .and_then(|desktop| {
                let desktop: Vec<u32> = desktop.value32()?.collect();

                if desktop.is_empty() {
                    None
                } else {
                    Some(desktop[0] as usize)
                }
            })
    }

    #[inline]
    fn get_window_preferred_type(
        &self,
        window: Window,
    ) -> WindowType {
        self.get_window_types(window)
            .get(0)
            .map_or(WindowType::Normal, |&type_| type_)
    }

    fn get_window_types(
        &self,
        window: Window,
    ) -> Vec<WindowType> {
        let mut window_types = Vec::new();

        drop(
            self.conn
                .get_property(
                    false,
                    window,
                    self.atoms._NET_WM_WINDOW_TYPE,
                    self.atoms.ATOM,
                    0,
                    std::u32::MAX,
                )
                .ok()
                .and_then(|cookie| cookie.reply().ok())
                .map(|types| {
                    let types: Vec<u32> = types
                        .value32()
                        .map_or(Vec::with_capacity(0), |value| value.collect());

                    for type_ in types {
                        if let Some(type_) = self.get_window_type_from_atom(type_) {
                            window_types.push(type_);
                        }
                    }
                }),
        );

        window_types
    }

    #[inline]
    fn get_window_preferred_state(
        &self,
        window: Window,
    ) -> Option<WindowState> {
        self.get_window_states(window).get(0).copied()
    }

    fn get_window_states(
        &self,
        window: Window,
    ) -> Vec<WindowState> {
        let mut window_states = Vec::new();

        drop(
            self.conn
                .get_property(
                    false,
                    window,
                    self.atoms._NET_WM_STATE,
                    self.atoms.ATOM,
                    0,
                    std::u32::MAX,
                )
                .ok()
                .and_then(|cookie| cookie.reply().ok())
                .map(|states| {
                    let states: Vec<u32> = states
                        .value32()
                        .map_or(Vec::with_capacity(0), |value| value.collect());

                    for state in states {
                        if let Some(state) = self.get_window_state_from_atom(state) {
                            window_states.push(state);
                        }
                    }
                }),
        );

        window_states
    }

    #[inline]
    fn window_is_fullscreen(
        &self,
        window: Window,
    ) -> bool {
        self.window_is_any_of_states(window, &[self.atoms._NET_WM_STATE_FULLSCREEN])
    }

    #[inline]
    fn window_is_above(
        &self,
        window: Window,
    ) -> bool {
        self.window_is_any_of_states(window, &[self.atoms._NET_WM_STATE_ABOVE])
    }

    #[inline]
    fn window_is_below(
        &self,
        window: Window,
    ) -> bool {
        self.window_is_any_of_states(window, &[self.atoms._NET_WM_STATE_BELOW])
    }

    #[inline]
    fn window_is_sticky(
        &self,
        window: Window,
    ) -> bool {
        self.window_is_any_of_states(window, &[self.atoms._NET_WM_STATE_STICKY])
            || self.get_window_desktop(window) == Some(0xFFFFFFFF)
    }
}
