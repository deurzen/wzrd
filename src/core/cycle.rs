use crate::change::Direction;
use crate::identify::Ident;
use crate::identify::Identify;
use crate::identify::Index;
use crate::util::BuildIdHasher;
use crate::util::Util;

use std::cell::Cell;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::VecDeque;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum InsertPos {
    BeforeActive,
    AfterActive,
    BeforeIndex(Index),
    AfterIndex(Index),
    BeforeIdent(Ident),
    AfterIdent(Ident),
    Front,
    Back,
}

#[derive(Clone, Copy)]
pub enum Selector<'a, T> {
    AtActive,
    AtIndex(Index),
    AtIdent(Ident),
    First,
    Last,
    ForCond(&'a dyn Fn(&T) -> bool),
}

#[derive(Clone, Copy, PartialEq)]
enum StackAction {
    Insert,
    Remove,
}

#[derive(Debug, Clone, PartialEq)]
struct HistoryStack {
    stack: VecDeque<Ident>,
}

impl HistoryStack {
    fn new() -> Self {
        HistoryStack {
            stack: VecDeque::with_capacity(30),
        }
    }

    fn clear(&mut self) {
        self.stack.clear();
    }

    fn push_back(
        &mut self,
        id: Ident,
    ) {
        self.stack.push_back(id);
    }

    fn pop_back(&mut self) -> Option<Ident> {
        self.stack.pop_back()
    }

    fn remove_id(
        &mut self,
        id: Ident,
    ) {
        if let Some(index) = self.stack.iter().rposition(|&i| i == id) {
            self.stack.remove(index);
        }
    }

    fn as_vecdeque(&self) -> VecDeque<Ident> {
        self.stack.clone()
    }

    fn as_vec(&self) -> Vec<Ident> {
        self.stack.iter().cloned().collect::<Vec<Ident>>()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cycle<T>
where
    T: Identify + std::fmt::Debug,
{
    index: Cell<Index>,

    elements: VecDeque<T>,
    indices: HashMap<Ident, Index, BuildIdHasher>,

    unwindable: bool,
    stack: RefCell<HistoryStack>,
}

impl<T> Cycle<T>
where
    T: Identify + std::fmt::Debug,
{
    pub fn new(
        elements: Vec<T>,
        unwindable: bool,
    ) -> Self {
        Self {
            indices: elements
                .iter()
                .enumerate()
                .map(|(i, e)| (e.id(), i))
                .collect(),

            index: Cell::new(Util::last_index(elements.iter())),
            elements: elements.into(),

            unwindable,
            stack: RefCell::new(HistoryStack::new()),
        }
    }

    #[inline]
    fn index(&self) -> Option<Index> {
        if self.index.get() < self.elements.len() {
            Some(self.index.get())
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.index.set(0);
        self.elements.clear();
        self.indices.clear();
        self.stack.borrow_mut().clear();
    }

    #[inline]
    pub fn next_will_wrap(
        &self,
        dir: Direction,
    ) -> bool {
        self.index.get() == Util::last_index(self.elements.iter()) && dir == Direction::Forward
            || self.index.get() == 0 && dir == Direction::Backward
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    #[inline]
    pub fn contains(
        &self,
        element: &T,
    ) -> bool {
        self.elements.contains(element)
    }

    #[inline]
    pub fn active_index(&self) -> Index {
        self.index.get()
    }

    #[inline]
    pub fn next_index(
        &self,
        dir: Direction,
    ) -> Index {
        self.next_index_from(self.index.get(), dir)
    }

    #[inline]
    pub fn next_element(
        &self,
        dir: Direction,
    ) -> Option<&T> {
        let next_index = self.next_index(dir);
        self.get_for(&Selector::AtIndex(next_index))
    }

    pub fn cycle_active(
        &self,
        dir: Direction,
    ) -> Option<&T> {
        self.push_active_to_stack();
        self.index.set(self.next_index(dir));
        self.active_element()
    }

    pub fn index_for(
        &self,
        sel: &Selector<T>,
    ) -> Option<Index> {
        match sel {
            Selector::AtActive => Some(self.index.get()),
            Selector::AtIndex(index) => {
                if *index < self.len() {
                    return Some(*index);
                }

                None
            },
            Selector::AtIdent(id) => {
                if let Some(index) = self.id_to_index(*id) {
                    return self.index_for(&Selector::AtIndex(index));
                }

                None
            },
            Selector::First => Some(0),
            Selector::Last => Some(self.elements.len() - 1),
            Selector::ForCond(f) => self.by(f).map(|(i, _)| i),
        }
    }

    #[inline]
    pub fn active_element(&self) -> Option<&T> {
        self.elements.get(self.index.get())
    }

    #[inline]
    pub fn active_element_mut(&mut self) -> Option<&mut T> {
        self.elements.get_mut(self.index.get())
    }

    pub fn rotate(
        &mut self,
        dir: Direction,
    ) {
        if !self.elements.is_empty() {
            match dir {
                Direction::Forward => self.elements.rotate_right(1),
                Direction::Backward => self.elements.rotate_left(1),
            };

            self.indices.clear();
            for (i, id) in self.elements.iter().enumerate().map(|(i, e)| (i, e.id())) {
                self.indices.insert(id as Ident, i as Index);
            }
        }
    }

    pub fn drag_active(
        &mut self,
        dir: Direction,
    ) -> Option<&T> {
        match (self.index.get(), self.next_index(dir), dir) {
            (0, _, Direction::Backward) => self.rotate(dir),
            (_, 0, Direction::Forward) => self.rotate(dir),
            (active, next, _) => {
                let active_id = self.elements.get(active).unwrap().id();
                let next_id = self.elements.get(next).unwrap().id();

                self.elements.swap(active, next);

                *self.indices.get_mut(&active_id).unwrap() = next;
                *self.indices.get_mut(&next_id).unwrap() = active;
            },
        };

        self.cycle_active(dir)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn insert_at(
        &mut self,
        insert_pos: &InsertPos,
        element: T,
    ) {
        match insert_pos {
            InsertPos::BeforeActive => self.insert(self.index.get(), element),
            InsertPos::AfterActive => {
                self.insert_at(&InsertPos::AfterIndex(self.index.get()), element)
            },
            InsertPos::BeforeIndex(index) => self.insert(*index, element),
            InsertPos::Front => self.push_front(element),
            InsertPos::Back => self.push_back(element),
            InsertPos::AfterIndex(index) => {
                let next_index = index + 1;

                if next_index > self.elements.len() {
                    self.push_back(element)
                } else {
                    self.insert(next_index, element)
                }
            },
            InsertPos::BeforeIdent(id) => {
                if let Some(index) = self.id_to_index(*id) {
                    self.insert_at(&InsertPos::BeforeIndex(index), element);
                }
            },
            InsertPos::AfterIdent(id) => {
                if let Some(index) = self.id_to_index(*id) {
                    self.insert_at(&InsertPos::AfterIndex(index), element);
                }
            },
        }
    }

    pub fn insert(
        &mut self,
        index: Index,
        element: T,
    ) {
        self.push_active_to_stack();
        self.sync_indices(index, StackAction::Insert);
        self.indices.insert((&element).id(), index);
        self.elements.insert(index, element);
        self.index.set(index);
    }

    pub fn push_front(
        &mut self,
        element: T,
    ) {
        self.push_active_to_stack();
        self.sync_indices(0, StackAction::Insert);
        self.indices.insert((&element).id(), 0);
        self.elements.push_front(element);
        self.index.set(0);
    }

    pub fn push_back(
        &mut self,
        element: T,
    ) {
        let end = self.elements.len();

        self.push_active_to_stack();
        self.indices.insert((&element).id(), end);
        self.elements.push_back(element);
        self.index.set(end);
    }

    #[inline]
    pub fn iter(&self) -> std::collections::vec_deque::Iter<T> {
        self.elements.iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> std::collections::vec_deque::IterMut<T> {
        self.elements.iter_mut()
    }

    #[inline]
    pub fn get(
        &self,
        index: Index,
    ) -> Option<&T> {
        self.elements.get(index)
    }

    #[inline]
    pub fn get_mut(
        &mut self,
        index: Index,
    ) -> Option<&mut T> {
        self.elements.get_mut(index)
    }

    pub fn get_for(
        &self,
        sel: &Selector<T>,
    ) -> Option<&T> {
        match sel {
            Selector::AtActive => self.active_element(),
            Selector::AtIndex(index) => self.elements.get(*index),
            Selector::First => self.elements.get(0),
            Selector::Last => self.elements.get(Util::last_index(self.elements.iter())),
            Selector::ForCond(f) => self.by(f).map(|(_, e)| e),
            Selector::AtIdent(id) => {
                if let Some(index) = self.id_to_index(*id) {
                    return self.get_for(&Selector::AtIndex(index));
                }

                None
            },
        }
    }

    pub fn get_for_mut(
        &mut self,
        sel: &Selector<T>,
    ) -> Option<&mut T> {
        match sel {
            Selector::AtActive => self.active_element_mut(),
            Selector::AtIndex(index) => self.elements.get_mut(*index),
            Selector::First => self.elements.get_mut(0),
            Selector::Last => self
                .elements
                .get_mut(Util::last_index(self.elements.iter())),
            Selector::ForCond(f) => self.by_mut(f).map(|(_, e)| e),
            Selector::AtIdent(id) => {
                if let Some(index) = self.id_to_index(*id) {
                    return self.get_for_mut(&Selector::AtIndex(index));
                }

                None
            },
        }
    }

    pub fn get_all_for(
        &self,
        sel: &Selector<T>,
    ) -> Vec<&T> {
        match sel {
            Selector::AtActive => self.active_element().into_iter().collect(),
            Selector::AtIndex(index) => self.elements.get(*index).into_iter().collect(),
            Selector::First => self.elements.get(0).into_iter().collect(),
            Selector::Last => self
                .elements
                .get(Util::last_index(self.elements.iter()))
                .into_iter()
                .collect(),
            Selector::ForCond(f) => self.elements.iter().filter(|e| f(*e)).collect(),
            Selector::AtIdent(id) => {
                if let Some(index) = self.id_to_index(*id) {
                    return self.get_all_for(&Selector::AtIndex(index));
                }

                vec![]
            },
        }
    }

    pub fn get_all_for_mut(
        &mut self,
        sel: &Selector<T>,
    ) -> Vec<&mut T> {
        match sel {
            Selector::AtActive => self.active_element_mut().into_iter().collect(),
            Selector::AtIndex(index) => self.elements.get_mut(*index).into_iter().collect(),
            Selector::First => self.elements.get_mut(0).into_iter().collect(),
            Selector::Last => self
                .elements
                .get_mut(Util::last_index(self.elements.iter()))
                .into_iter()
                .collect(),
            Selector::ForCond(f) => self.elements.iter_mut().filter(|e| f(*e)).collect(),
            Selector::AtIdent(id) => {
                if let Some(index) = self.id_to_index(*id) {
                    return self.get_all_for_mut(&Selector::AtIndex(index));
                }

                vec![]
            },
        }
    }

    pub fn on_active<F: Fn(&T)>(
        &self,
        f: F,
    ) {
        if let Some(element) = self.active_element() {
            f(element);
        }
    }

    pub fn on_active_mut<F: FnMut(&mut T)>(
        &mut self,
        mut f: F,
    ) {
        if let Some(element) = self.active_element_mut() {
            f(element);
        }
    }

    pub fn on_all<F: Fn(&T)>(
        &self,
        f: F,
    ) {
        for element in self.elements.iter() {
            f(element);
        }
    }

    pub fn on_all_mut<F: FnMut(&mut T)>(
        &mut self,
        mut f: F,
    ) {
        for element in self.elements.iter_mut() {
            f(element);
        }
    }

    pub fn on_all_for<F: Fn(&T)>(
        &self,
        f: F,
        sel: &Selector<T>,
    ) {
        for element in self.get_all_for(sel) {
            f(element);
        }
    }

    pub fn on_all_for_mut<F: FnMut(&mut T)>(
        &mut self,
        mut f: F,
        sel: &Selector<T>,
    ) {
        for element in self.get_all_for_mut(sel) {
            f(element);
        }
    }

    pub fn activate_for(
        &self,
        sel: &Selector<T>,
    ) -> Option<&T> {
        match sel {
            Selector::AtActive => self.active_element(),
            Selector::AtIndex(index) => {
                self.push_active_to_stack();
                self.index.set(*index);
                self.active_element()
            },
            Selector::AtIdent(id) => {
                if let Some(index) = self.id_to_index(*id) {
                    return self.activate_for(&Selector::AtIndex(index));
                }

                None
            },
            Selector::First => {
                self.push_active_to_stack();
                self.index.set(0);
                self.active_element()
            },
            Selector::Last => {
                self.push_active_to_stack();
                self.index.set(Util::last_index(self.elements.iter()));
                self.active_element()
            },
            Selector::ForCond(f) => {
                if let Some((index, _)) = self.by(f) {
                    self.push_active_to_stack();
                    self.index.set(index);
                    Some(&self.elements[index])
                } else {
                    None
                }
            },
        }
    }

    pub fn remove_for(
        &mut self,
        sel: &Selector<T>,
    ) -> Option<T> {
        let (index, element) = match sel {
            Selector::AtActive => (self.index.get(), self.elements.remove(self.index.get())),
            Selector::AtIndex(index) => (*index, self.elements.remove(*index)),
            Selector::AtIdent(id) => {
                if let Some(index) = self.id_to_index(*id) {
                    return self.remove_for(&Selector::AtIndex(index));
                }

                return None;
            },
            Selector::First => (0, self.elements.remove(0)),
            Selector::Last => {
                let end = Util::last_index(self.elements.iter());
                (end, self.elements.remove(end))
            },
            Selector::ForCond(f) => {
                if let Some((index, _)) = self.by(f) {
                    (index, self.elements.remove(index))
                } else {
                    return None;
                }
            },
        };

        self.remove_element(index, &element);
        element
    }

    pub fn swap(
        &mut self,
        sel1: &Selector<T>,
        sel2: &Selector<T>,
    ) {
        let index1 = self.index_for(sel1);

        if let Some(index1) = index1 {
            let index2 = self.index_for(sel2);

            if let Some(index2) = index2 {
                self.elements.swap(index1, index2);
            }
        }
    }

    fn next_index_from(
        &self,
        index: Index,
        dir: Direction,
    ) -> Index {
        let end = Util::last_index(self.elements.iter());

        match dir {
            Direction::Forward => {
                if index == end {
                    0
                } else {
                    index + 1
                }
            },
            Direction::Backward => {
                if index == 0 {
                    end
                } else {
                    index - 1
                }
            },
        }
    }

    fn sync_indices(
        &mut self,
        pivot_index: Index,
        action: StackAction,
    ) {
        for index in pivot_index..self.elements.len() {
            let id = self.elements.get(index).unwrap().id();

            match action {
                StackAction::Remove => *self.indices.get_mut(&id).unwrap() -= 1,
                StackAction::Insert => *self.indices.get_mut(&id).unwrap() += 1,
            }
        }

        if action == StackAction::Remove {
            match pivot_index.cmp(&self.index.get()) {
                Ordering::Equal => {
                    if let Some(id) = self.pop_from_stack() {
                        if let Some(index) = self.id_to_index(id) {
                            self.index.set(index);
                            return;
                        }
                    }

                    self.index.set(Util::last_index(self.elements.iter()));
                },
                Ordering::Less => {
                    let index = self.index.get();

                    if index > 0 {
                        self.index.set(index - 1);
                    }
                },
                Ordering::Greater => {},
            }
        }
    }

    fn by(
        &self,
        cond: impl Fn(&T) -> bool,
    ) -> Option<(Index, &T)> {
        self.elements.iter().enumerate().find(|(_, e)| cond(*e))
    }

    fn by_mut(
        &mut self,
        cond: impl Fn(&T) -> bool,
    ) -> Option<(Index, &mut T)> {
        self.elements.iter_mut().enumerate().find(|(_, e)| cond(*e))
    }

    fn index_of(
        &self,
        element: T,
    ) -> Option<Index> {
        self.id_to_index(element.id())
    }

    fn index_to_id(
        &self,
        index: Index,
    ) -> Option<Ident> {
        if let Some(element) = self.elements.get(index) {
            return Some(element.id());
        }

        None
    }

    fn id_to_index(
        &self,
        id: Ident,
    ) -> Option<Index> {
        if let Some(index) = self.indices.get(&id) {
            return Some(*index);
        }

        None
    }

    pub fn stack(&self) -> VecDeque<Ident> {
        self.stack.borrow().as_vecdeque()
    }

    pub fn stack_after_focus(&self) -> Vec<Ident> {
        let mut stack: Vec<Ident> = self.stack.borrow().as_vec();

        if let Some(index) = self.index() {
            if let Some(id) = self.index_to_id(index) {
                if let Some(found_index) = stack.iter().rposition(|i| *i == id) {
                    stack.remove(found_index);
                }

                stack.push(id);
            }
        }

        stack
    }

    fn push_index_to_stack(
        &self,
        index: Option<Index>,
    ) {
        if !self.unwindable {
            return;
        }

        if let Some(index) = index {
            if let Some(id) = self.index_to_id(index) {
                let mut stack = self.stack.borrow_mut();
                stack.remove_id(id);
                stack.push_back(id);
            }
        }
    }

    #[inline]
    fn push_active_to_stack(&self) {
        if !self.unwindable {
            return;
        }

        self.push_index_to_stack(self.index());
    }

    fn remove_element(
        &mut self,
        index: Index,
        element: &Option<T>,
    ) {
        if let Some(element) = element {
            let id = element.id();

            self.indices.remove(&id);
            self.remove_from_stack(id);
            self.sync_indices(index, StackAction::Remove);
        }
    }

    #[inline]
    fn remove_from_stack(
        &self,
        id: Ident,
    ) {
        if !self.unwindable {
            return;
        }

        self.stack.borrow_mut().remove_id(id);
    }

    #[inline]
    fn pop_from_stack(&self) -> Option<Ident> {
        if !self.unwindable {
            return None;
        }

        self.stack.borrow_mut().pop_back()
    }
}

impl<T: PartialEq + Identify + std::fmt::Debug> Cycle<T> {
    pub fn equivalent_selectors(
        &self,
        sel1: &Selector<T>,
        sel2: &Selector<T>,
    ) -> bool {
        match (self.index_for(&sel1), self.index_for(&sel2)) {
            (Some(e), Some(f)) => e == f,
            _ => false,
        }
    }
}

impl<T: Clone + Identify + std::fmt::Debug> Cycle<T> {
    #[allow(dead_code)]
    pub fn as_vec(&self) -> Vec<T> {
        self.iter().cloned().collect()
    }
}

impl<T: Identify + std::fmt::Debug> std::ops::Index<Index> for Cycle<T> {
    type Output = T;

    fn index(
        &self,
        index: Index,
    ) -> &Self::Output {
        &self.elements[index]
    }
}

impl<T: Identify + std::fmt::Debug> std::ops::IndexMut<Index> for Cycle<T> {
    fn index_mut(
        &mut self,
        index: Index,
    ) -> &mut Self::Output {
        &mut self.elements[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod i32 {
        impl super::Identify for i32 {
            fn id(&self) -> super::Ident {
                *self as super::Ident
            }
        }
    }

    #[test]
    fn removing_element_before_focus() {
        let mut cycle = Cycle::new(vec![0, 10, 20, 30, 40, 50, 60], false);

        assert_eq!(cycle.index.get(), 6);

        cycle.remove_for(&Selector::AtIndex(2));

        assert_eq!(cycle.index.get(), 5);
        assert_eq!(cycle.indices.get(&0), Some(&0));
        assert_eq!(cycle.indices.get(&10), Some(&1));
        assert_eq!(cycle.indices.get(&20), None);
        assert_eq!(cycle.indices.get(&30), Some(&2));
        assert_eq!(cycle.indices.get(&40), Some(&3));
        assert_eq!(cycle.indices.get(&50), Some(&4));
        assert_eq!(cycle.indices.get(&60), Some(&5));

        cycle.remove_for(&Selector::AtIndex(2));

        assert_eq!(cycle.index.get(), 4);
        assert_eq!(cycle.indices.get(&0), Some(&0));
        assert_eq!(cycle.indices.get(&10), Some(&1));
        assert_eq!(cycle.indices.get(&20), None);
        assert_eq!(cycle.indices.get(&30), None);
        assert_eq!(cycle.indices.get(&40), Some(&2));
        assert_eq!(cycle.indices.get(&50), Some(&3));
        assert_eq!(cycle.indices.get(&60), Some(&4));

        cycle.remove_for(&Selector::AtIndex(2));

        assert_eq!(cycle.index.get(), 3);
        assert_eq!(cycle.indices.get(&0), Some(&0));
        assert_eq!(cycle.indices.get(&10), Some(&1));
        assert_eq!(cycle.indices.get(&20), None);
        assert_eq!(cycle.indices.get(&30), None);
        assert_eq!(cycle.indices.get(&40), None);
        assert_eq!(cycle.indices.get(&50), Some(&2));
        assert_eq!(cycle.indices.get(&60), Some(&3));

        cycle.remove_for(&Selector::AtIndex(2));

        assert_eq!(cycle.index.get(), 2);
        assert_eq!(cycle.indices.get(&0), Some(&0));
        assert_eq!(cycle.indices.get(&10), Some(&1));
        assert_eq!(cycle.indices.get(&20), None);
        assert_eq!(cycle.indices.get(&30), None);
        assert_eq!(cycle.indices.get(&40), None);
        assert_eq!(cycle.indices.get(&50), None);
        assert_eq!(cycle.indices.get(&60), Some(&2));

        cycle.remove_for(&Selector::AtIndex(2));

        assert_eq!(cycle.index.get(), 1);
        assert_eq!(cycle.indices.get(&0), Some(&0));
        assert_eq!(cycle.indices.get(&10), Some(&1));
        assert_eq!(cycle.indices.get(&20), None);
        assert_eq!(cycle.indices.get(&30), None);
        assert_eq!(cycle.indices.get(&40), None);
        assert_eq!(cycle.indices.get(&50), None);
        assert_eq!(cycle.indices.get(&60), None);

        cycle.remove_for(&Selector::AtIndex(2));

        assert_eq!(cycle.index.get(), 1);
        assert_eq!(cycle.indices.get(&0), Some(&0));
        assert_eq!(cycle.indices.get(&10), Some(&1));
        assert_eq!(cycle.indices.get(&20), None);
        assert_eq!(cycle.indices.get(&30), None);
        assert_eq!(cycle.indices.get(&40), None);
        assert_eq!(cycle.indices.get(&50), None);
        assert_eq!(cycle.indices.get(&60), None);

        cycle.remove_for(&Selector::AtIndex(1));

        assert_eq!(cycle.index.get(), 0);
        assert_eq!(cycle.indices.get(&0), Some(&0));
        assert_eq!(cycle.indices.get(&10), None);
        assert_eq!(cycle.indices.get(&20), None);
        assert_eq!(cycle.indices.get(&30), None);
        assert_eq!(cycle.indices.get(&40), None);
        assert_eq!(cycle.indices.get(&50), None);
        assert_eq!(cycle.indices.get(&60), None);

        cycle.remove_for(&Selector::AtIndex(0));

        assert_eq!(cycle.index.get(), 0);
        assert_eq!(cycle.indices.get(&0), None);
        assert_eq!(cycle.indices.get(&10), None);
        assert_eq!(cycle.indices.get(&20), None);
        assert_eq!(cycle.indices.get(&30), None);
        assert_eq!(cycle.indices.get(&40), None);
        assert_eq!(cycle.indices.get(&50), None);
        assert_eq!(cycle.indices.get(&60), None);
    }

    #[test]
    fn removing_last_element_at_focus() {
        let mut cycle = Cycle::new(vec![0, 10, 20, 30, 40, 50, 60], false);

        assert_eq!(cycle.index.get(), 6);

        cycle.remove_for(&Selector::AtIndex(6));

        assert_eq!(cycle.index.get(), 5);
        assert_eq!(cycle.indices.get(&0), Some(&0));
        assert_eq!(cycle.indices.get(&10), Some(&1));
        assert_eq!(cycle.indices.get(&20), Some(&2));
        assert_eq!(cycle.indices.get(&30), Some(&3));
        assert_eq!(cycle.indices.get(&40), Some(&4));
        assert_eq!(cycle.indices.get(&50), Some(&5));
        assert_eq!(cycle.indices.get(&60), None);

        cycle.remove_for(&Selector::AtIndex(6));

        assert_eq!(cycle.index.get(), 5);
        assert_eq!(cycle.indices.get(&0), Some(&0));
        assert_eq!(cycle.indices.get(&10), Some(&1));
        assert_eq!(cycle.indices.get(&20), Some(&2));
        assert_eq!(cycle.indices.get(&30), Some(&3));
        assert_eq!(cycle.indices.get(&40), Some(&4));
        assert_eq!(cycle.indices.get(&50), Some(&5));
        assert_eq!(cycle.indices.get(&60), None);

        cycle.remove_for(&Selector::AtIndex(5));

        assert_eq!(cycle.index.get(), 4);
        assert_eq!(cycle.indices.get(&0), Some(&0));
        assert_eq!(cycle.indices.get(&10), Some(&1));
        assert_eq!(cycle.indices.get(&20), Some(&2));
        assert_eq!(cycle.indices.get(&30), Some(&3));
        assert_eq!(cycle.indices.get(&40), Some(&4));
        assert_eq!(cycle.indices.get(&50), None);
        assert_eq!(cycle.indices.get(&60), None);

        cycle.remove_for(&Selector::AtIndex(4));

        assert_eq!(cycle.index.get(), 3);
        assert_eq!(cycle.indices.get(&0), Some(&0));
        assert_eq!(cycle.indices.get(&10), Some(&1));
        assert_eq!(cycle.indices.get(&20), Some(&2));
        assert_eq!(cycle.indices.get(&30), Some(&3));
        assert_eq!(cycle.indices.get(&40), None);
        assert_eq!(cycle.indices.get(&50), None);
        assert_eq!(cycle.indices.get(&60), None);

        cycle.remove_for(&Selector::AtIndex(3));

        assert_eq!(cycle.index.get(), 2);
        assert_eq!(cycle.indices.get(&0), Some(&0));
        assert_eq!(cycle.indices.get(&10), Some(&1));
        assert_eq!(cycle.indices.get(&20), Some(&2));
        assert_eq!(cycle.indices.get(&30), None);
        assert_eq!(cycle.indices.get(&40), None);
        assert_eq!(cycle.indices.get(&50), None);
        assert_eq!(cycle.indices.get(&60), None);

        cycle.remove_for(&Selector::AtIndex(2));

        assert_eq!(cycle.index.get(), 1);
        assert_eq!(cycle.indices.get(&0), Some(&0));
        assert_eq!(cycle.indices.get(&10), Some(&1));
        assert_eq!(cycle.indices.get(&20), None);
        assert_eq!(cycle.indices.get(&30), None);
        assert_eq!(cycle.indices.get(&40), None);
        assert_eq!(cycle.indices.get(&50), None);
        assert_eq!(cycle.indices.get(&60), None);

        cycle.remove_for(&Selector::AtIndex(1));

        assert_eq!(cycle.index.get(), 0);
        assert_eq!(cycle.indices.get(&0), Some(&0));
        assert_eq!(cycle.indices.get(&10), None);
        assert_eq!(cycle.indices.get(&20), None);
        assert_eq!(cycle.indices.get(&30), None);
        assert_eq!(cycle.indices.get(&40), None);
        assert_eq!(cycle.indices.get(&50), None);
        assert_eq!(cycle.indices.get(&60), None);

        cycle.remove_for(&Selector::AtIndex(0));

        assert_eq!(cycle.index.get(), 0);
        assert_eq!(cycle.indices.get(&0), None);
        assert_eq!(cycle.indices.get(&10), None);
        assert_eq!(cycle.indices.get(&20), None);
        assert_eq!(cycle.indices.get(&30), None);
        assert_eq!(cycle.indices.get(&40), None);
        assert_eq!(cycle.indices.get(&50), None);
        assert_eq!(cycle.indices.get(&60), None);

        cycle.remove_for(&Selector::AtIndex(0));

        assert_eq!(cycle.index.get(), 0);
        assert_eq!(cycle.indices.get(&0), None);
        assert_eq!(cycle.indices.get(&10), None);
        assert_eq!(cycle.indices.get(&20), None);
        assert_eq!(cycle.indices.get(&30), None);
        assert_eq!(cycle.indices.get(&40), None);
        assert_eq!(cycle.indices.get(&50), None);
        assert_eq!(cycle.indices.get(&60), None);
    }

    #[test]
    fn removing_first_element_at_focus() {
        let mut cycle = Cycle::new(vec![0, 10, 20, 30, 40, 50, 60], false);

        assert_eq!(cycle.index.get(), 6);
        cycle.activate_for(&Selector::AtIndex(0));
        assert_eq!(cycle.index.get(), 0);

        cycle.remove_for(&Selector::AtIndex(0));

        assert_eq!(cycle.index.get(), 5);
        assert_eq!(cycle.indices.get(&0), None);
        assert_eq!(cycle.indices.get(&10), Some(&0));
        assert_eq!(cycle.indices.get(&20), Some(&1));
        assert_eq!(cycle.indices.get(&30), Some(&2));
        assert_eq!(cycle.indices.get(&40), Some(&3));
        assert_eq!(cycle.indices.get(&50), Some(&4));
        assert_eq!(cycle.indices.get(&60), Some(&5));

        cycle.activate_for(&Selector::AtIndex(0));
        assert_eq!(cycle.index.get(), 0);

        cycle.remove_for(&Selector::AtIndex(0));

        assert_eq!(cycle.index.get(), 4);
        assert_eq!(cycle.indices.get(&0), None);
        assert_eq!(cycle.indices.get(&10), None);
        assert_eq!(cycle.indices.get(&20), Some(&0));
        assert_eq!(cycle.indices.get(&30), Some(&1));
        assert_eq!(cycle.indices.get(&40), Some(&2));
        assert_eq!(cycle.indices.get(&50), Some(&3));
        assert_eq!(cycle.indices.get(&60), Some(&4));

        cycle.activate_for(&Selector::AtIndex(0));
        assert_eq!(cycle.index.get(), 0);

        cycle.remove_for(&Selector::AtIndex(0));

        assert_eq!(cycle.index.get(), 3);
        assert_eq!(cycle.indices.get(&0), None);
        assert_eq!(cycle.indices.get(&10), None);
        assert_eq!(cycle.indices.get(&20), None);
        assert_eq!(cycle.indices.get(&30), Some(&0));
        assert_eq!(cycle.indices.get(&40), Some(&1));
        assert_eq!(cycle.indices.get(&50), Some(&2));
        assert_eq!(cycle.indices.get(&60), Some(&3));

        cycle.activate_for(&Selector::AtIndex(0));
        assert_eq!(cycle.index.get(), 0);

        cycle.remove_for(&Selector::AtIndex(0));

        assert_eq!(cycle.index.get(), 2);
        assert_eq!(cycle.indices.get(&0), None);
        assert_eq!(cycle.indices.get(&10), None);
        assert_eq!(cycle.indices.get(&20), None);
        assert_eq!(cycle.indices.get(&30), None);
        assert_eq!(cycle.indices.get(&40), Some(&0));
        assert_eq!(cycle.indices.get(&50), Some(&1));
        assert_eq!(cycle.indices.get(&60), Some(&2));

        cycle.activate_for(&Selector::AtIndex(0));
        assert_eq!(cycle.index.get(), 0);

        cycle.remove_for(&Selector::AtIndex(0));

        assert_eq!(cycle.index.get(), 1);
        assert_eq!(cycle.indices.get(&0), None);
        assert_eq!(cycle.indices.get(&10), None);
        assert_eq!(cycle.indices.get(&20), None);
        assert_eq!(cycle.indices.get(&30), None);
        assert_eq!(cycle.indices.get(&40), None);
        assert_eq!(cycle.indices.get(&50), Some(&0));
        assert_eq!(cycle.indices.get(&60), Some(&1));

        cycle.activate_for(&Selector::AtIndex(0));
        assert_eq!(cycle.index.get(), 0);

        cycle.remove_for(&Selector::AtIndex(0));

        assert_eq!(cycle.index.get(), 0);
        assert_eq!(cycle.indices.get(&0), None);
        assert_eq!(cycle.indices.get(&10), None);
        assert_eq!(cycle.indices.get(&20), None);
        assert_eq!(cycle.indices.get(&30), None);
        assert_eq!(cycle.indices.get(&40), None);
        assert_eq!(cycle.indices.get(&50), None);
        assert_eq!(cycle.indices.get(&60), Some(&0));

        cycle.activate_for(&Selector::AtIndex(0));
        assert_eq!(cycle.index.get(), 0);

        cycle.remove_for(&Selector::AtIndex(0));

        assert_eq!(cycle.index.get(), 0);
        assert_eq!(cycle.indices.get(&0), None);
        assert_eq!(cycle.indices.get(&10), None);
        assert_eq!(cycle.indices.get(&20), None);
        assert_eq!(cycle.indices.get(&30), None);
        assert_eq!(cycle.indices.get(&40), None);
        assert_eq!(cycle.indices.get(&50), None);
        assert_eq!(cycle.indices.get(&60), None);

        cycle.activate_for(&Selector::AtIndex(0));
        assert_eq!(cycle.index.get(), 0);

        cycle.remove_for(&Selector::AtIndex(0));

        assert_eq!(cycle.index.get(), 0);
        assert_eq!(cycle.indices.get(&0), None);
        assert_eq!(cycle.indices.get(&10), None);
        assert_eq!(cycle.indices.get(&20), None);
        assert_eq!(cycle.indices.get(&30), None);
        assert_eq!(cycle.indices.get(&40), None);
        assert_eq!(cycle.indices.get(&50), None);
        assert_eq!(cycle.indices.get(&60), None);
    }
}
