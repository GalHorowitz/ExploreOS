//! Kernel entry point

#![feature(panic_info_message, default_alloc_error_handler, naked_functions, asm)]
#![no_std]
#![no_main]

extern crate compiler_reqs;
extern crate alloc;

use boot_args::BootArgs;
use serial::println;

mod panic;
mod memory_manager;
mod gdt;
mod interrupts;
mod screen;
mod keyboard;
mod ps2;

/// Entry point of the kernel. `boot_args_ptr` is a a physical address below 1MiB which points to a
/// `BootArgs` structure.
#[no_mangle]
pub extern fn entry(boot_args_ptr: *const BootArgs) -> ! {
    // The first thing we do is copy the boot args to this memory space because we will be unmapping
    // the temporary 1MiB identity map soon
    let boot_args = unsafe {
        *boot_args_ptr
    };
    
    // Setup serial logging with the ports already initialized by the bootloader
    serial::init_with_ports(boot_args.serial_port);    

    println!(" === Kernel running!");

    // Initializes the memory manager, which also unmaps the temp identity map
    memory_manager::init(&boot_args);

    println!("Initialized memory manager");

    screen::init();
    screen::print("Hello from the kernel!\n");
    screen::print_with_attributes("\t\t\tZoop :)\n", 0x04);

    gdt::init();

    interrupts::init();
    println!("Enabled interrupts");
    ps2::controller::init();

    // unsafe {
    //     asm!("int 0x67");
    // }

    let start = cpu::serializing_rdtsc();
    let mut vec: alloc::vec::Vec<u8> = alloc::vec::Vec::with_capacity(1 * 1024 * 1024);
    vec.push(4u8);
    let elapsed = cpu::serializing_rdtsc() - start;
    println!("Took {} cycles to allocate {} bytes", elapsed, vec.capacity());

    cpu::halt_and_service_interrupts();
}