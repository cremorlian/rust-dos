#![no_std]
#![no_main]

use core::arch::asm;

const COM1_DATA: u16 = 0x3F8;
const COM1_STATUS: u16 = 0x3FD;

// core::ptr::write_volatile is a plain volatile store on this target, not an I/O
// port write, so the port instructions have to be spelled out.
#[inline]
fn outb(port: u16, value: u8) {
    unsafe { asm!("out dx, al", in("dx") port, in("al") value, options(nostack, preserves_flags)) }
}

#[inline]
fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe { asm!("in al, dx", out("al") value, in("dx") port, options(nostack, preserves_flags)) };
    value
}

fn putc(byte: u8) {
    while inb(COM1_STATUS) & 0x20 == 0 {}
    outb(COM1_DATA, byte);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    park()
}

static MARKER: &str = "hello, dos\r\n";

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // black_box stops the compiler inlining the static's address into a
    // memory load, which would read the string's bytes as a pointer.
    let m = core::hint::black_box(MARKER);
    // Raw pointers, not slice indexing: indexing pulls in memcpy/memset,
    // which libc-less targets do not have.
    let mut ptr = m.as_ptr();
    let end = unsafe { ptr.add(m.len()) };
    while ptr < end {
        putc(unsafe { *ptr });
        ptr = unsafe { ptr.add(1) };
    }
    park()
}

fn park() -> ! {
    loop {
        core::hint::spin_loop();
    }
}
