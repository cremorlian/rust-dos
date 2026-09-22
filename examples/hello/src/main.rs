#![no_std]
#![no_main]

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    static GREETING: u32 = 0x1337;
    loop {
        core::hint::black_box(GREETING);
    }
}
