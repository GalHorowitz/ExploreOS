//! The entry point for the Rust bootloader

#![no_std]
#![no_main]
#![feature(panic_info_message)]

use core::panic::PanicInfo;
use serial::{print, println};

mod compiler_reqs;
mod screen;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // if let Some(msg) = info.message() {
    //     screen::print_with_attributes(std::format!("{}", msg), 0x4);
    // }
    cpu::halt();
}


#[no_mangle]
pub extern fn entry() -> ! {
    serial::init();

    print!("This is a print macro! ");
    println!("This is a print macro with a new line!");

    screen::reset();
    for _ in 0..10 {
        screen::print("Hello!\n");
    }

    panic!("Hello there!");
}
