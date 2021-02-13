#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

pub type Result<T> = anyhow::Result<T>;

pub mod common;
pub mod connection;
pub mod event;
pub mod input;
pub mod screen;
pub mod xdata;
