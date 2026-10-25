#![no_std]
#![no_main]

use embedded_io::Write;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    park()
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let mut stdout = dos_pm::Stdout;
    stdout.write_all(b"hello, dos\r\n").unwrap();
    dos_pm::exit(0)
}

fn park() -> ! {
    loop {
        core::hint::spin_loop();
    }
}
