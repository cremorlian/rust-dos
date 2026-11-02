#![no_std]
#![no_main]

extern crate alloc;

use embedded_alloc::LlffHeap as Heap;
use embedded_io::{Read, Write as IoWrite};

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    park()
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let mut stdout = dos_pm::Stdout;
    stdout.write(b"hello from write\r\n").unwrap();
    stdout.write_all(b"hello from write_all\r\n").unwrap();
    dos_pm::print!("hello from print\r\n");
    dos_pm::println!("hello from println");

    let mut stdin = dos_pm::Stdin;
    let mut input = [0u8; 64];
    let mut input_len = 0;
    loop {
        let n = stdin.read(&mut input[input_len..]).unwrap();
        if n == 0 {
            break;
        }
        input_len += n;
    }
    stdout.write_all(b"hello from stdin: ").unwrap();
    stdout.write_all(&input[..input_len]).unwrap();
    stdout.write_all(b"\r\n").unwrap();

    unsafe {
        embedded_alloc::init!(HEAP, 4096);
    }
    let value = alloc::boxed::Box::new(42u8);
    if *value == 42 {
        stdout.write_all(b"hello from heap\r\n").unwrap();
    }
    drop(value);

    dos_pm::eprint!("hello from eprint\r\n");
    dos_pm::eprintln!("hello from eprintln");

    dos_pm::exit(0)
}

fn park() -> ! {
    loop {
        core::hint::spin_loop();
    }
}
