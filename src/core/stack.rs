use winsys::common::Window;

use std::collections::HashMap;
use std::vec::Vec;

#[derive(Debug)]
pub enum StackLayer {
    Desktop,
    Below,
    Dock,
    // Regular,
    // Free,
    // Transient,
    Above,
    // Fullscreen,
    Notification,
}

pub struct StackManager {
    window_layers: HashMap<Window, StackLayer>,

    desktop_windows: Vec<Window>,
    below_windows: Vec<Window>,
    dock_windows: Vec<Window>,
    above_windows: Vec<Window>,
    notification_windows: Vec<Window>,

    // which windows should be stacked directly {above,below} which other windows
    above_other: HashMap<Window, Window>,
    below_other: HashMap<Window, Window>,
}

impl StackManager {
    pub fn new() -> Self {
        Self {
            window_layers: HashMap::with_capacity(5),

            desktop_windows: Vec::with_capacity(10),
            below_windows: Vec::with_capacity(10),
            dock_windows: Vec::with_capacity(10),
            above_windows: Vec::with_capacity(10),
            notification_windows: Vec::with_capacity(10),

            above_other: HashMap::with_capacity(30),
            below_other: HashMap::with_capacity(30),
        }
    }

    pub fn add_window(
        &mut self,
        window: Window,
        layer: StackLayer,
    ) {
        if !self.window_layers.contains_key(&window) {
            let layer_windows = match layer {
                StackLayer::Desktop => &mut self.desktop_windows,
                StackLayer::Below => &mut self.below_windows,
                StackLayer::Dock => &mut self.dock_windows,
                StackLayer::Above => &mut self.above_windows,
                StackLayer::Notification => &mut self.notification_windows,
            };

            layer_windows.push(window);
            self.window_layers.insert(window, layer);
        }
    }

    pub fn add_above_other(
        &mut self,
        window: Window,
        sibling: Window,
    ) {
        if !self.above_other.contains_key(&window) {
            self.above_other.insert(window, sibling);
        }
    }

    pub fn add_below_other(
        &mut self,
        window: Window,
        sibling: Window,
    ) {
        if !self.below_other.contains_key(&window) {
            self.below_other.insert(window, sibling);
        }
    }

    pub fn remove_window(
        &mut self,
        window: Window,
    ) {
        if let Some(layer) = self.window_layers.get(&window) {
            let layer_windows = match layer {
                StackLayer::Desktop => &mut self.desktop_windows,
                StackLayer::Below => &mut self.below_windows,
                StackLayer::Dock => &mut self.dock_windows,
                StackLayer::Above => &mut self.above_windows,
                StackLayer::Notification => &mut self.notification_windows,
            };

            let index =
                layer_windows.iter().position(|&w| w == window).unwrap();

            layer_windows.remove(index);
            self.window_layers.remove(&window);
        }

        self.above_other.remove(&window);
        self.below_other.remove(&window);
    }

    pub fn relayer_window(
        &mut self,
        window: Window,
        layer: StackLayer,
    ) {
        self.remove_window(window);
        self.add_window(window, layer);
    }

    pub fn raise_window(
        &mut self,
        window: Window,
    ) {
        if let Some(layer) = self.window_layers.get(&window) {
            let layer_windows = match layer {
                StackLayer::Desktop => &mut self.desktop_windows,
                StackLayer::Below => &mut self.below_windows,
                StackLayer::Dock => &mut self.dock_windows,
                StackLayer::Above => &mut self.above_windows,
                StackLayer::Notification => &mut self.notification_windows,
            };

            let index =
                layer_windows.iter().position(|&w| w == window).unwrap();

            layer_windows.remove(index);
            layer_windows.push(window);
        }
    }

    pub fn layer_windows(
        &self,
        layer: StackLayer,
    ) -> Vec<Window> {
        match layer {
            StackLayer::Desktop => self.desktop_windows.to_owned(),
            StackLayer::Below => self.below_windows.to_owned(),
            StackLayer::Dock => self.dock_windows.to_owned(),
            StackLayer::Above => self.above_windows.to_owned(),
            StackLayer::Notification => self.notification_windows.to_owned(),
        }
    }

    pub fn above_other(&self) -> &HashMap<Window, Window> {
        &self.above_other
    }

    pub fn below_other(&self) -> &HashMap<Window, Window> {
        &self.below_other
    }
}
