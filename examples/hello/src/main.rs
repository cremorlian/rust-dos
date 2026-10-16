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

static GREETING: u32 = 0x1337;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    core::hint::black_box(GREETING);
    // Each marker byte is an immediate rather than a static, so writing the
    // marker does not exercise DS.
    putc(b'i');
    putc(b'm');
    putc(b'a');
    putc(b'g');
    putc(b'e');
    putc(b' ');
    putc(b'p');
    putc(b'm');
    putc(b'\r');
    putc(b'\n');
    park()
}

fn park() -> ! {
    loop {
        core::hint::spin_loop();
    }
}
