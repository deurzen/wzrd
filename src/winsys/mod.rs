#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

#[macro_use]
mod macros;

pub type Result<T> = anyhow::Result<T>;

pub mod connection;
pub mod event;
pub mod geometry;
pub mod hints;
pub mod input;
pub mod screen;
pub mod window;
pub mod xdata;
