#![no_std]

mod raw;

pub mod console;
pub use console::{Error, Stdin, Stdout};
