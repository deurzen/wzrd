use super::super::input::Result;
pub use super::super::input::*;

use crate::geometry::Pos;
use crate::window::Window;

use std::collections::HashMap;
use std::convert::TryFrom;
use std::vec::Vec;

use anyhow::anyhow;
use strum::EnumIter;
use strum::IntoEnumIterator;

use x11rb::protocol::xproto::ButtonPressEvent;
use x11rb::protocol::xproto::ButtonReleaseEvent;
use x11rb::protocol::xproto::KeyPressEvent;
use x11rb::protocol::xproto::ModMask;
use x11rb::protocol::xproto::MotionNotifyEvent;

impl KeyCode {
    pub fn from_press_event(event: &KeyPressEvent) -> Self {
        Self {
            mask: event.state,
            code: event.detail,
        }
    }

    pub fn without_mask(
        &self,
        mask: ModMask,
    ) -> Self {
        Self {
            mask: self.mask & !(u16::from(mask)),
            code: self.code,
        }
    }
}

impl From<Button> for u8 {
    fn from(button: Button) -> u8 {
        match button {
            Button::Left => 1,
            Button::Middle => 2,
            Button::Right => 3,
            Button::ScrollUp => 4,
            Button::ScrollDown => 5,
            Button::Backward => 8,
            Button::Forward => 9,
        }
    }
}

impl TryFrom<u8> for Button {
    type Error = anyhow::Error;

    fn try_from(val: u8) -> Result<Self> {
        match val {
            1 => Ok(Self::Left),
            2 => Ok(Self::Middle),
            3 => Ok(Self::Right),
            4 => Ok(Self::ScrollUp),
            5 => Ok(Self::ScrollDown),
            8 => Ok(Self::Backward),
            9 => Ok(Self::Forward),
            _ => Err(anyhow!("no matching button for value {}", val)),
        }
    }
}

impl Modifier {
    pub fn was_held(
        &self,
        mask: u16,
    ) -> bool {
        mask & u16::from(*self) > 0
    }
}

impl From<Modifier> for u16 {
    fn from(modifier: Modifier) -> u16 {
        u16::from(match modifier {
            Modifier::Ctrl => ModMask::CONTROL,
            Modifier::Shift => ModMask::SHIFT,
            Modifier::Alt => ModMask::M1,
            Modifier::Super => ModMask::M4,
            Modifier::AltGr => ModMask::M3,
            Modifier::NumLock => ModMask::M2,
            Modifier::ScrollLock => ModMask::M5,
        })
    }
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

impl MouseShortcut {
    pub fn from_event(
        detail: u8,
        state: u16,
    ) -> Result<Self> {
        Ok(Self {
            button: Button::try_from(detail)?,
            modifiers: Modifier::iter().filter(|m| m.was_held(state)).collect(),
        })
    }

    pub fn mask(&self) -> u16 {
        self.modifiers
            .iter()
            .fold(0, |acc, &val| acc | u16::from(val))
    }

    pub fn button(&self) -> u8 {
        self.button.into()
    }
}

impl MouseEvent {
    pub fn from_press_event(
        event: &ButtonPressEvent,
        root: Window,
    ) -> Result<Self> {
        Ok(Self::new(
            MouseEventKind::Press,
            event.event,
            if event.child != x11rb::NONE {
                Some(event.child)
            } else {
                None
            },
            root,
            event.root_x,
            event.root_y,
            event.event_x,
            event.event_y,
            MouseShortcut::from_event(event.detail, event.state)?,
        ))
    }

    pub fn from_release_event(
        event: &ButtonReleaseEvent,
        root: Window,
    ) -> Result<Self> {
        Ok(Self::new(
            MouseEventKind::Release,
            event.event,
            if event.child != x11rb::NONE {
                Some(event.child)
            } else {
                None
            },
            root,
            event.root_x,
            event.root_y,
            event.event_x,
            event.event_y,
            MouseShortcut::from_event(event.detail, event.state)?,
        ))
    }

    pub fn from_motion_event(
        event: &MotionNotifyEvent,
        root: Window,
    ) -> Result<Self> {
        Ok(Self::new(
            MouseEventKind::Motion,
            event.event,
            if event.child != x11rb::NONE {
                Some(event.child)
            } else {
                None
            },
            root,
            event.root_x,
            event.root_y,
            event.event_x,
            event.event_y,
            MouseShortcut::from_event(1, event.state)?,
        ))
    }
}
