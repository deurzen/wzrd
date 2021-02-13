use crate::common::Dim;
use crate::common::Edge;
use crate::common::Pos;
use crate::common::Region;
use crate::common::Strut;
use crate::common::Window;

use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
    vec::Vec,
};

#[derive(Debug, Clone)]
pub struct Screen {
    number: usize,
    full_region: Region,
    placeable_region: Region,
    windows: HashMap<Window, Vec<Edge>>,
    struts: HashMap<Edge, Vec<Strut>>,
    showing_struts: bool,
}

impl std::cmp::PartialEq<Self> for Screen {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.number == other.number
    }
}

impl Screen {
    pub fn new(
        region: Region,
        number: usize,
    ) -> Self {
        Screen::init(Self {
            number,
            full_region: region,
            placeable_region: region,
            windows: HashMap::new(),
            struts: HashMap::with_capacity(4),
            showing_struts: true,
        })
    }

    fn init(mut self) -> Self {
        self.struts.insert(Edge::Left, Vec::with_capacity(1));
        self.struts.insert(Edge::Right, Vec::with_capacity(1));
        self.struts.insert(Edge::Top, Vec::with_capacity(1));
        self.struts.insert(Edge::Bottom, Vec::with_capacity(1));

        self
    }

    pub fn showing_struts(&self) -> bool {
        self.showing_struts
    }

    pub fn number(&self) -> usize {
        self.number
    }

    pub fn set_number(
        &mut self,
        number: usize,
    ) {
        self.number = number
    }

    pub fn show_and_yield_struts(&mut self) -> Vec<Window> {
        self.showing_struts = true;
        self.compute_placeable_region();

        self.windows.keys().cloned().collect()
    }

    pub fn hide_and_yield_struts(&mut self) -> Vec<Window> {
        self.showing_struts = false;
        self.compute_placeable_region();

        self.windows.keys().cloned().collect()
    }

    pub fn full_region(&self) -> Region {
        self.full_region
    }

    pub fn placeable_region(&self) -> Region {
        self.placeable_region
    }

    pub fn contains_window(
        &self,
        window: Window,
    ) -> bool {
        self.windows.contains_key(&window)
    }

    pub fn compute_placeable_region(&mut self) {
        let mut region = self.full_region;

        if self.showing_struts {
            if let Some(strut) = self.struts.get(&Edge::Left).unwrap().last() {
                region.pos.x += strut.width as i32;
                region.dim.w -= strut.width;
            }

            if let Some(strut) = self.struts.get(&Edge::Right).unwrap().last() {
                region.dim.w -= strut.width;
            }

            if let Some(strut) = self.struts.get(&Edge::Top).unwrap().last() {
                region.pos.y += strut.width as i32;
                region.dim.h -= strut.width;
            }

            if let Some(strut) = self.struts.get(&Edge::Bottom).unwrap().last()
            {
                region.dim.h -= strut.width;
            }
        }

        self.placeable_region = region;
    }

    pub fn add_strut(
        &mut self,
        edge: Edge,
        window: Window,
        width: u32,
    ) {
        let strut = self.struts.get_mut(&edge).unwrap();
        let index = strut.binary_search_by(|s| s.width.cmp(&width));

        strut.insert(index.unwrap_or_else(|e| e), Strut::new(window, width));
        self.windows.entry(window).or_insert(vec![edge]).push(edge);
    }

    pub fn add_struts(
        &mut self,
        struts: Vec<Option<Strut>>,
    ) {
        if let Some(left_strut) = struts[0] {
            self.add_strut(Edge::Left, left_strut.window, left_strut.width);
        }

        if let Some(right_strut) = struts[1] {
            self.add_strut(Edge::Right, right_strut.window, right_strut.width);
        }

        if let Some(top_strut) = struts[2] {
            self.add_strut(Edge::Top, top_strut.window, top_strut.width);
        }

        if let Some(bottom_strut) = struts[3] {
            self.add_strut(
                Edge::Bottom,
                bottom_strut.window,
                bottom_strut.width,
            );
        }
    }

    pub fn remove_window_strut(
        &mut self,
        window: Window,
    ) {
        for (_, struts) in &mut self.struts {
            // a window may have strut at multiple screen edges
            struts.retain(|s| s.window != window);
        }

        self.windows.remove(&window);
    }

    pub fn update_strut(
        &mut self,
        edge: Edge,
        window: Window,
        width: u32,
    ) {
        self.remove_window_strut(window);
        self.add_strut(edge, window, width);
    }

    pub fn max_strut_val(
        &self,
        edge: Edge,
    ) -> Option<u32> {
        match edge {
            Edge::Left => {
                if let Some(strut) =
                    self.struts.get(&Edge::Left).unwrap().last()
                {
                    return Some(strut.width);
                }
            },
            Edge::Right => {
                if let Some(strut) =
                    self.struts.get(&Edge::Right).unwrap().last()
                {
                    return Some(strut.width);
                }
            },
            Edge::Top => {
                if let Some(strut) = self.struts.get(&Edge::Top).unwrap().last()
                {
                    return Some(strut.width);
                }
            },
            Edge::Bottom => {
                if let Some(strut) =
                    self.struts.get(&Edge::Bottom).unwrap().last()
                {
                    return Some(strut.width);
                }
            },
        };

        None
    }

    pub fn has_strut_window(
        &self,
        window: Window,
    ) -> bool {
        self.windows.contains_key(&window)
    }

    pub fn full_encompasses(
        &self,
        pos: Pos,
    ) -> bool {
        self.full_region.encompasses(pos)
    }

    pub fn placeable_encompasses(
        &self,
        pos: Pos,
    ) -> bool {
        self.placeable_region.encompasses(pos)
    }

    pub fn full_contains(
        &self,
        region: Region,
    ) -> bool {
        self.full_region.contains(region)
    }

    pub fn placeable_contains(
        &self,
        region: Region,
    ) -> bool {
        self.placeable_region.contains(region)
    }

    pub fn full_occludes(
        &self,
        region: Region,
    ) -> bool {
        self.full_region.occludes(region)
    }

    pub fn placeable_occludes(
        &self,
        region: Region,
    ) -> bool {
        self.placeable_region.occludes(region)
    }
}
