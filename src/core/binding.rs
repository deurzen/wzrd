use crate::model::Model;

use winsys::input::KeyInput;
use winsys::input::MouseInput;
use winsys::window::Window;

use std::collections::HashMap;

pub type KeyAction = fn(&mut Model<'_>);
pub type MouseAction = fn(&mut Model<'_>, Option<Window>) -> bool;
pub type KeyBindings = HashMap<KeyInput, KeyAction>;
pub type MouseBindings = HashMap<MouseInput, MouseAction>;
