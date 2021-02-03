//! Kernel entry point

#![feature(panic_info_message, default_alloc_error_handler, naked_functions, asm)]
#![no_std]
#![no_main]

extern crate compiler_reqs;
extern crate alloc;

use boot_args::BootArgs;
use page_tables::VirtAddr;
use serial::println;

mod panic;
mod memory_manager;
mod gdt;
mod interrupts;

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

    gdt::init();

    interrupts::init();
    println!("Enabled interrupts");

    unsafe {
        asm!("int 0x67");
    }

    let start = cpu::rdtsc();
    let mut vec: alloc::vec::Vec<u8> = alloc::vec::Vec::with_capacity(512 * 1024 * 1024);
    vec.push(4u8);
    let elapsed = cpu::rdtsc() - start;
    println!("Took {} cycles to allocate {} bytes", elapsed, vec.capacity());

    let mut pmem = memory_manager::PHYS_MEM.lock();
    let phys_mem = pmem.as_mut().unwrap();
    let mut page_dir = memory_manager::PAGES.lock();
    let raw_table_entry = 0xb8000 | page_tables::PAGE_ENTRY_PRESENT | page_tables::PAGE_ENTRY_WRITE;
    unsafe { 
        page_dir.as_mut().unwrap().map_raw(phys_mem, VirtAddr(0xEEEE0000), raw_table_entry, false,
            true).expect("FAILED SCREEN MAP");
    }
    let screen = unsafe {
        core::slice::from_raw_parts_mut(0xEEEE0000 as *mut u16, 80*25)
    };
    screen.iter_mut().for_each(|x| *x = 0x0f38);
    
    cpu::halt();
}