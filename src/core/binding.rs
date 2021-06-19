use crate::model::Model;

use winsys::input::KeyCode;
use winsys::input::MouseEventKey;
use winsys::input::MouseShortcut;
use winsys::window::Window;

use std::collections::HashMap;

pub type KeyAction = Box<dyn FnMut(&mut Model<'_>)>;
pub type MouseAction = Box<dyn FnMut(&mut Model<'_>, Option<Window>)>;
pub type KeyBindings = HashMap<KeyCode, KeyAction>;
pub type MouseBindings = HashMap<(MouseEventKey, MouseShortcut), (MouseAction, bool)>;
