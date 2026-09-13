#![no_std]

mod process;
mod raw;

pub mod console;
pub use console::{Error, Stderr, Stdin, Stdout};
pub use process::exit;
