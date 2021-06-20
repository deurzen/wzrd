pub use crate::Result;

use crate::geometry::Corner;
use crate::geometry::Edge;
use crate::geometry::Pos;
use crate::window::Window;

use std::collections::HashMap;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::hash::Hash;
use std::hash::Hasher;
use std::vec::Vec;

use anyhow::anyhow;
use strum::EnumIter;
use strum::IntoEnumIterator;

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum Grip {
    Edge(Edge),
    Corner(Corner),
}

impl Grip {
    pub fn is_top_grip(&self) -> bool {
        *self == Grip::Edge(Edge::Top)
            || *self == Grip::Corner(Corner::TopLeft)
            || *self == Grip::Corner(Corner::TopRight)
    }

    pub fn is_left_grip(&self) -> bool {
        *self == Grip::Edge(Edge::Left)
            || *self == Grip::Corner(Corner::TopLeft)
            || *self == Grip::Corner(Corner::BottomLeft)
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, EnumIter, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub enum Modifier {
    Ctrl = 1 << 0,
    Shift = 1 << 1,
    Alt = 1 << 2,
    AltGr = 1 << 3,
    Super = 1 << 4,
    NumLock = 1 << 5,
    ScrollLock = 1 << 6,
}

impl TryFrom<&str> for Modifier {
    type Error = anyhow::Error;

    fn try_from(val: &str) -> Result<Self> {
        match val {
            "C" => Ok(Self::Ctrl),
            "A" => Ok(Self::Alt),
            "S" => Ok(Self::Shift),
            "M" => Ok(Self::Super),
            "AltGr" => Ok(Self::Alt),
            "Num" => Ok(Self::NumLock),
            "Scroll" => Ok(Self::ScrollLock),
            _ => Err(anyhow!("unable to resolve \"{}\" to modifier", val)),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Button {
    Left,
    Middle,
    Right,
    ScrollUp,
    ScrollDown,
    Backward,
    Forward,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum MouseEventKind {
    Press,
    Release,
    Motion,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum MouseInputTarget {
    Global,
    Root,
    Client,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MouseInput {
    pub target: MouseInputTarget,
    pub button: Button,
    pub modifiers: HashSet<Modifier>,
}

impl Hash for MouseInput {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        self.button.hash(state);
        self.modifiers
            .iter()
            .fold(0u8, |acc, &m| acc | m as u8)
            .hash(state);
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub input: MouseInput,
    pub window: Option<Window>,
    pub root_rpos: Pos,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Key {
    Any,
    Backspace,
    Tab,
    Clear,
    Return,
    Shift,
    Control,
    Alt,
    Super,
    Menu,
    Pause,
    CapsLock,
    Escape,
    Space,
    ExclamationMark,
    QuotationMark,
    QuestionMark,
    NumberSign,
    DollarSign,
    PercentSign,
    AtSign,
    Ampersand,
    Apostrophe,
    LeftParenthesis,
    RightParenthesis,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    Underscore,
    Grave,
    Bar,
    Tilde,
    QuoteLeft,
    Asterisk,
    Plus,
    Comma,
    Minus,
    Period,
    Slash,
    BackSlash,
    Colon,
    SemiColon,
    Less,
    Equal,
    Greater,
    PageUp,
    PageDown,
    End,
    Home,
    Left,
    Up,
    Right,
    Down,
    Select,
    Print,
    Execute,
    PrintScreen,
    Insert,
    Delete,
    Help,
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    NumPad0,
    NumPad1,
    NumPad2,
    NumPad3,
    NumPad4,
    NumPad5,
    NumPad6,
    NumPad7,
    NumPad8,
    NumPad9,
    Multiply,
    Add,
    Seperator,
    Subtract,
    Decimal,
    Divide,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    Numlock,
    ScrollLock,
    LeftShift,
    RightShift,
    LeftControl,
    RightContol,
    LeftAlt,
    RightAlt,
    LeftSuper,
    RightSuper,
    BrowserBack,
    BrowserForward,
    BrowserRefresh,
    BrowserStop,
    BrowserSearch,
    BrowserFavorites,
    BrowserHome,
    VolumeMute,
    VolumeDown,
    VolumeUp,
    NextTrack,
    PreviousTrack,
    StopMedia,
    PlayPause,
    LaunchMail,
    SelectMedia,
    LaunchAppA,
    LaunchAppB,
    LaunchAppC,
    LaunchAppD,
    LaunchAppE,
    LaunchAppF,
    LaunchApp0,
    LaunchApp1,
    LaunchApp2,
    LaunchApp3,
    LaunchApp4,
    LaunchApp5,
    LaunchApp6,
    LaunchApp7,
    LaunchApp8,
    LaunchApp9,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyInput {
    pub key: Key,
    pub modifiers: HashSet<Modifier>,
}

impl Hash for KeyInput {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        self.key.hash(state);
        self.modifiers
            .iter()
            .fold(0u8, |acc, &m| acc | m as u8)
            .hash(state);
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct KeyEvent {
    pub input: KeyInput,
    pub window: Option<Window>,
}
