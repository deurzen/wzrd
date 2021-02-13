use crate::model::Model;

use winsys::common::Window;
use winsys::input::KeyCode;
use winsys::input::MouseEvent;
use winsys::input::MouseEventKey;
use winsys::input::MouseShortcut;

use std::collections::HashMap;

pub type Action = Box<dyn FnMut(&mut Model)>;
pub type MouseEvents = Box<dyn FnMut(&mut Model, &MouseEvent, Option<Window>)>;
pub type KeyEvents = Box<dyn FnMut(&mut Model)>;
pub type KeyBindings = HashMap<KeyCode, KeyEvents>;
pub type MouseBindings =
    HashMap<(MouseEventKey, MouseShortcut), (MouseEvents, bool)>;
