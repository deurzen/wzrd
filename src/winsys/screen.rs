use crate::geometry::Dim;
use crate::geometry::Edge;
use crate::geometry::Pos;
use crate::geometry::Region;
use crate::geometry::Strut;
use crate::window::Window;

use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::vec::Vec;

#[derive(Debug, Clone)]
pub struct Screen {
    number: Cell<usize>,
    full_region: Cell<Region>,
    placeable_region: Cell<Region>,
    windows: RefCell<HashMap<Window, Vec<Edge>>>,
    struts: RefCell<HashMap<Edge, Vec<Strut>>>,
    showing_struts: Cell<bool>,
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
            number: Cell::new(number),
            full_region: Cell::new(region),
            placeable_region: Cell::new(region),
            windows: RefCell::new(HashMap::new()),
            struts: RefCell::new(HashMap::with_capacity(4)),
            showing_struts: Cell::new(true),
        })
    }

    fn init(self) -> Self {
        let mut struts = self.struts.borrow_mut();

        struts.insert(Edge::Left, Vec::with_capacity(1));
        struts.insert(Edge::Right, Vec::with_capacity(1));
        struts.insert(Edge::Top, Vec::with_capacity(1));
        struts.insert(Edge::Bottom, Vec::with_capacity(1));

        drop(struts);
        self
    }

    #[inline]
    pub fn showing_struts(&self) -> bool {
        self.showing_struts.get()
    }

    #[inline]
    pub fn number(&self) -> usize {
        self.number.get()
    }

    #[inline]
    pub fn set_number(
        &self,
        number: usize,
    ) {
        self.number.set(number)
    }

    #[inline]
    pub fn show_and_yield_struts(
        &self,
        show: bool,
    ) -> Vec<Window> {
        self.showing_struts.set(show);
        self.compute_placeable_region();

        self.windows.borrow().keys().cloned().collect()
    }

    #[inline]
    pub fn full_region(&self) -> Region {
        self.full_region.get()
    }

    #[inline]
    pub fn placeable_region(&self) -> Region {
        self.placeable_region.get()
    }

    #[inline]
    pub fn contains_window(
        &self,
        window: Window,
    ) -> bool {
        self.windows.borrow().contains_key(&window)
    }

    #[inline]
    pub fn compute_placeable_region(&self) {
        let mut region = self.full_region.get();

        if self.showing_struts.get() {
            let struts = self.struts.borrow();

            if let Some(strut) = struts.get(&Edge::Left).unwrap().last() {
                region.pos.x += strut.width as i32;
                region.dim.w -= strut.width as i32;
            }

            if let Some(strut) = struts.get(&Edge::Right).unwrap().last() {
                region.dim.w -= strut.width as i32;
            }

            if let Some(strut) = struts.get(&Edge::Top).unwrap().last() {
                region.pos.y += strut.width as i32;
                region.dim.h -= strut.width as i32;
            }

            if let Some(strut) = struts.get(&Edge::Bottom).unwrap().last() {
                region.dim.h -= strut.width as i32;
            }
        }

        self.placeable_region.set(region);
    }

    #[inline]
    pub fn add_strut(
        &self,
        edge: Edge,
        window: Window,
        width: u32,
    ) {
        let mut struts = self.struts.borrow_mut();
        let strut = struts.get_mut(&edge).unwrap();

        let index = strut.binary_search_by(|s| s.width.cmp(&width));
        strut.insert(index.unwrap_or_else(|e| e), Strut::new(window, width));

        self.windows
            .borrow_mut()
            .entry(window)
            .or_insert(vec![edge])
            .push(edge);
    }

    #[inline]
    pub fn add_struts(
        &self,
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
            self.add_strut(Edge::Bottom, bottom_strut.window, bottom_strut.width);
        }
    }

    #[inline]
    pub fn remove_window_strut(
        &self,
        window: Window,
    ) {
        self.struts.borrow_mut().iter_mut().for_each(|(_, struts)| {
            // a window may have strut at multiple screen edges
            struts.retain(|s| s.window != window);
        });

        self.windows.borrow_mut().remove(&window);
    }

    #[inline]
    pub fn update_strut(
        &self,
        edge: Edge,
        window: Window,
        width: u32,
    ) {
        self.remove_window_strut(window);
        self.add_strut(edge, window, width);
    }

    #[inline]
    pub fn max_strut_val(
        &self,
        edge: Edge,
    ) -> Option<u32> {
        self.struts
            .borrow()
            .get(&edge)
            .unwrap()
            .last()
            .map(|strut| strut.width)
    }

    #[inline]
    pub fn has_strut_window(
        &self,
        window: Window,
    ) -> bool {
        self.windows.borrow().contains_key(&window)
    }

    #[inline]
    pub fn full_encompasses(
        &self,
        pos: Pos,
    ) -> bool {
        self.full_region.get().encompasses(pos)
    }

    #[inline]
    pub fn placeable_encompasses(
        &self,
        pos: Pos,
    ) -> bool {
        self.placeable_region.get().encompasses(pos)
    }

    #[inline]
    pub fn full_contains(
        &self,
        region: Region,
    ) -> bool {
        self.full_region.get().contains(region)
    }

    #[inline]
    pub fn placeable_contains(
        &self,
        region: Region,
    ) -> bool {
        self.placeable_region.get().contains(region)
    }

    #[inline]
    pub fn full_occludes(
        &self,
        region: Region,
    ) -> bool {
        self.full_region.get().occludes(region)
    }

    #[inline]
    pub fn placeable_occludes(
        &self,
        region: Region,
    ) -> bool {
        self.placeable_region.get().occludes(region)
    }
}
