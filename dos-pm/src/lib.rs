#![no_std]

mod process;
mod psp;
mod raw;

pub mod console;
pub mod env;
pub use console::{Error as ConsoleError, Stderr, Stdin, Stdout};
pub use env::{ArgIter, Args, ArgsError, args_bytes};
pub use process::exit;

#[cfg(feature = "heap")]
pub mod heap;
