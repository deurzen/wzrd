pub use crate::Result;

use crate::geometry::Corner;
use crate::geometry::Edge;
use crate::geometry::Pos;
use crate::window::Window;

use std::collections::HashMap;
use std::convert::TryFrom;
use std::vec::Vec;

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

pub type CodeMap = HashMap<String, u8>;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct KeyCode {
    pub mask: u16,
    pub code: u8,
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

#[derive(Debug, PartialEq, EnumIter, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub enum Modifier {
    Ctrl,
    Shift,
    Alt,
    AltGr,
    Super,
    NumLock,
    ScrollLock,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct MouseShortcut {
    pub button: Button,
    pub modifiers: Vec<Modifier>,
}

impl MouseShortcut {
    pub fn new(
        button: Button,
        mut modifiers: Vec<Modifier>,
    ) -> Self {
        modifiers.sort();
        Self {
            button,
            modifiers,
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum MouseEventKind {
    Press,
    Release,
    Motion,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum EventTarget {
    Global,
    Root,
    Client,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct MouseEventKey {
    pub kind: MouseEventKind,
    pub target: EventTarget,
}

#[derive(Debug, Clone)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub window: Window,
    pub subwindow: Option<Window>,
    pub on_root: bool,
    pub root_rpos: Pos,
    pub window_rpos: Pos,
    pub shortcut: MouseShortcut,
}

impl MouseEvent {
    pub fn new(
        kind: MouseEventKind,
        window: Window,
        subwindow: Option<Window>,
        root: Window,
        root_rx: i16,
        root_ry: i16,
        window_rx: i16,
        window_ry: i16,
        shortcut: MouseShortcut,
    ) -> Self {
        Self {
            kind,
            window,
            subwindow,
            on_root: window == root,
            root_rpos: Pos {
                x: root_rx as i32,
                y: root_ry as i32,
            },
            window_rpos: Pos {
                x: window_rx as i32,
                y: window_ry as i32,
            },
            shortcut,
        }
    }
}
