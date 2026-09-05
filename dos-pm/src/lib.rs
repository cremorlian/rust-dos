//! DOS protected-mode (DPMI) runtime.
//!
//! A host-stable, library-first, `#![no_std]` crate for 32-bit protected-mode
//! DOS programs (DPMI clients): `embedded_io` console output, process exit, and
//! heap allocation over DPMI `INT 31h`.

#![no_std]

pub mod console;
