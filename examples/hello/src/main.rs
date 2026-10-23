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

static PREFIX: [u8; 5] = *b"hello";
static SUFFIX: [u8; 5] = *b"dos\r\n";

#[inline(never)]
fn emit(bytes: &[u8]) {
    // black_box keeps the stack pointer opaque, so the loop dereferences it
    // through a general register (DS) rather than through EBP/ESP (SS).
    let mut ptr = core::hint::black_box(bytes.as_ptr());
    let end = unsafe { ptr.add(bytes.len()) };
    while ptr < end {
        putc(unsafe { *ptr });
        ptr = unsafe { ptr.add(1) };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let prefix = core::hint::black_box(&PREFIX);
    let suffix = core::hint::black_box(&SUFFIX);

    let line = [
        prefix[0], prefix[1], prefix[2], prefix[3], prefix[4], b',', b' ', suffix[0],
        suffix[1], suffix[2], suffix[3], suffix[4],
    ];

    emit(&line);
    park()
}

fn park() -> ! {
    loop {
        core::hint::spin_loop();
    }
}
