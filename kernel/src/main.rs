//! Kernel entry point

#![feature(panic_info_message)]
#![no_std]
#![no_main]

extern crate compiler_reqs;

use boot_args::BootArgs;
use serial::println;

mod panic;
mod memory_manager;

#[no_mangle]
pub extern fn entry(boot_args: *const BootArgs) -> ! {
    assert!(!boot_args.is_null());
    let boot_args = unsafe {
        & *boot_args
    };
    
    serial::init_with_ports(boot_args.serial_port);    

    println!(" === Kernel running!");

    memory_manager::init(boot_args);
    
    println!("Free mem: {:#x}", boot_args.free_memory.total_size().unwrap());

    let screen = unsafe {
        core::slice::from_raw_parts_mut(0xb8000 as *mut u16, 80*25)
    };
    screen.iter_mut().for_each(|x| *x = 0x0f39);

    // let page_directory = unsafe {
    //     PageDirectory::from_cr3(phys_mem, cpu::get_cr3() as u32)
    // };

    cpu::halt();
}