use crate::model::Model;

use winsys::input::KeyCode;
use winsys::input::MouseEventKey;
use winsys::input::MouseShortcut;
use winsys::window::Window;

use std::collections::HashMap;

pub type Action = Box<dyn FnMut(&mut Model)>;
pub type MouseEvents = Box<dyn FnMut(&mut Model, Option<Window>)>;
pub type KeyEvents = Box<dyn FnMut(&mut Model)>;
pub type KeyBindings = HashMap<KeyCode, KeyEvents>;
pub type MouseBindings = HashMap<(MouseEventKey, MouseShortcut), (MouseEvents, bool)>;
