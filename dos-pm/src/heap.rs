#![allow(unsafe_code)]
//! DOS x86 `critical-section` impl for the `embedded-alloc` heap bundle.
//!
//! # Consumer recipe
//!
//! ```no_run
//! use embedded_alloc::LlffHeap as Heap;
//!
//! #[global_allocator]
//! static HEAP: Heap = Heap::empty();
//!
//! fn main() {
//!     const SIZE: usize = 64 * 1024;
//!     unsafe { embedded_alloc::init!(HEAP, SIZE) }
//! }
//! ```
//!
//! `init!` is one-shot: panics on a second call or `size == 0`.
//! Do not allocate before `init!`.
//!
//! # Safety
//!
//! No-op critical-section: correct under single-threaded, no-ISR execution.
//! Must be upgraded to `cli`/`sti` if preemption is introduced.

#[cfg(target_arch = "x86")]
struct CriticalSectionImpl;

#[cfg(target_arch = "x86")]
critical_section::set_impl!(CriticalSectionImpl);

#[cfg(target_arch = "x86")]
unsafe impl critical_section::Impl for CriticalSectionImpl {
    unsafe fn acquire() -> critical_section::RawRestoreState {
        ()
    }

    unsafe fn release(_restore_state: critical_section::RawRestoreState) {}
}
