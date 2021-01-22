//! The entry point for the Rust bootloader

#![feature(panic_info_message, default_alloc_error_handler)]
#![no_std]
#![no_main]

extern crate alloc;

mod compiler_reqs;
mod panic;
mod real_mode;
mod memory_manager;
mod screen;
mod disk_reading;

use serial::println;

#[no_mangle]
pub extern fn entry(boot_disk_id: u8, bootloader_size: u32) -> ! {
    serial::init();
    memory_manager::init(bootloader_size);

    println!("Bootloader running!");

    screen::reset();
    screen::print("Welcome to the bootloader! Loading kernel from disk...\n");

    let kernel_image = disk_reading::read_kernel(boot_disk_id, bootloader_size);
    if kernel_image.is_none() {
        screen::print("Failed to read kernel from disk :(");
        cpu::halt();
    }
    let kernel_image = kernel_image.unwrap();
    screen::print(&alloc::format!("Read {} bytes from disk!", kernel_image.len()));

    cpu::halt();
}
