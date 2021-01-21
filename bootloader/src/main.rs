//! The entry point for the Rust bootloader

#![no_std]
#![no_main]
#![feature(panic_info_message, default_alloc_error_handler)]

extern crate alloc;

use serial::{print, println};

mod compiler_reqs;
mod panic;
mod real_mode;
mod memory_manager;
mod screen;

#[no_mangle]
pub extern fn entry(bootloader_size: u32) -> ! {
    serial::init();

    print!("This is a print macro! ");
    println!("This is a print macro with a new line!");
    
    memory_manager::init(bootloader_size);

    let mut foo = alloc::vec::Vec::new();
    foo.push(5);
    foo.push(5);
    foo.push(6);
    foo.push(-5555);
    foo.push(2);
    foo.remove(1);
    println!("{:?}", foo);

    let bar = alloc::format!("apples {}", foo[2]);
    println!("{}", bar);

    screen::reset();
    for _ in 0..10 {
        screen::print("Hello!\n");
    }
    screen::print_with_attributes("This is colored!", 0xB4);
    

    cpu::halt();
}
