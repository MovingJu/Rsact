#![doc = include_str!("../README.md")]

pub mod buffer;
pub mod cell;
pub mod diff;
pub mod input;
pub mod renderer;
pub mod term;

#[cfg(unix)]
mod term_unix;
#[cfg(windows)]
mod term_windows;
