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

impl TryFrom<ModMask> for Modifier {
    type Error = anyhow::Error;

    fn try_from(val: ModMask) -> Result<Self> {
        match val {
            ModMask::CONTROL => Ok(Modifier::Ctrl),
            ModMask::SHIFT => Ok(Modifier::Shift),
            ModMask::M1 => Ok(Modifier::Alt),
            ModMask::M4 => Ok(Modifier::Super),
            ModMask::M3 => Ok(Modifier::AltGr),
            ModMask::M2 => Ok(Modifier::NumLock),
            ModMask::M5 => Ok(Modifier::ScrollLock),
            _ => Err(anyhow!("no matching modifier for value {}", u16::from(val))),
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
