#![doc = include_str!("../README.md")]

pub mod buffer;
pub mod cell;
pub mod component;
pub mod diff;
pub mod element;
pub mod input;
pub mod renderer;
pub mod term;
pub mod tree;
pub mod view;

#[cfg(unix)]
mod term_unix;
#[cfg(windows)]
mod term_windows;
