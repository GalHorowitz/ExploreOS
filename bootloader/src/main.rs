//! The entry point for the Rust bootloader

#![feature(panic_info_message, default_alloc_error_handler)]
#![no_std]
#![no_main]

extern crate alloc;
extern crate compiler_reqs;

mod panic;
mod real_mode;
mod memory_manager;
mod screen;
mod disk;

use core::convert::TryInto;
use serial::println;
use elf_parser::ElfParser;
use page_tables::{PageDirectory, VirtAddr, PhysAddr};
use boot_args::{BootArgs, KERNEL_STACK_SIZE, KERNEL_STACK_BASE_VADDR, LAST_PAGE_TABLE_VADDR,
    KERNEL_ALLOCATIONS_BASE_VADDR};

/// Rust bootloader entry point
#[no_mangle]
pub extern fn entry(boot_disk_id: u8, bootloader_size: u32) -> ! {
    // Initialize serial ports for logging
    serial::init();

    println!(" === Bootloader running!");

    // Initialize the memory manager which handles physical allocations
    memory_manager::init(bootloader_size);

    // Clear the screen and display a message, because if the kernel is big this might take a couple
    // seconds
    screen::reset();
    screen::print("Welcome to the bootloader! Loading kernel from disk...\n");

    // Load and map the kernel
    let (kernel_entry, kernel_stack, new_cr3, last_page_table_paddr) =
        setup_kernel(boot_disk_id, bootloader_size);

    // Grab the lock of physical memory and serial ports so we can transfer them to the kernel
    let mut pmem = memory_manager::PHYS_MEM.lock();
    let mut serial = serial::SERIAL.lock();

    // Construct the boot args for the kernel
    let boot_args = BootArgs {
        free_memory: core::mem::replace(&mut *pmem, None).unwrap().0,
        serial_port: core::mem::replace(&mut *serial, None).unwrap(),
        last_page_table_paddr
    };

    // Release the locks because we will never return from the kernel so they would not be released
    // automatically
    core::mem::drop(serial);
    core::mem::drop(pmem);

    extern {
        /// Calls the kernel `entry` with argument `param` after setting esp to `stack` and cr3 to
        /// `new_cr3`
        pub fn jump_to_kernel(entry: u32, stack: u32, param: u32, new_cr3: u32) -> !;
    }

    // Make the jump to the kernel
    unsafe {
        jump_to_kernel(kernel_entry, kernel_stack, &boot_args as *const _ as u32, new_cr3);
    }
}

/// Reads the kernel from disk and maps it into memory. Also maps kernel stack and 1MiB identity.
/// Returns (kernel entry vaddr, kernel stack vaddr, new cr3, last page table vaddr)
fn setup_kernel(boot_disk_id: u8, bootloader_size: u32) -> (u32, u32, u32, PhysAddr) {
    // Read the kernel from disk
    let kernel_image = disk::read_kernel(boot_disk_id, bootloader_size);
    if kernel_image.is_none() { 
        screen::print_with_attributes("Failed to read kernel from disk.", 0xf4);
        panic!("Failed to read kernel from disk.");
    }
    let kernel_image = kernel_image.unwrap();
    screen::print(&alloc::format!("Read {} bytes from disk!", kernel_image.len()));

    // Parse the ELF of the kernel
    let kernel_elf = ElfParser::parse(&kernel_image);
    if kernel_elf.is_none() {
        screen::print_with_attributes("Failed to parse kernel ELF.", 0xf4);
        panic!("Failed to parse kernel ELF.");
    }
    let kernel_elf = kernel_elf.unwrap();

    // Get access to physical memory
    let mut pmem = memory_manager::PHYS_MEM.lock();
    let phys_mem = pmem.as_mut().expect("Physical memory is not initialized?!");

    // Create a new page directory
    let mut directory = PageDirectory::new(phys_mem).expect("Failed to create page directory");

    // Map the elf segments into pages
    kernel_elf.for_segment(|vaddr, size, init_bytes, read, write, exec| {
        let r = if read { 'R' } else { '_' };
        let w = if write { 'W' } else { '_' };
        let x = if exec { 'X' } else { '_' };
        println!("Mapping kernel segment {:#09x} {:#09x} [{}{}{}]", vaddr, size, r, w, x);

        // The kernel cannot extend beyond 0xC4000000 because that is where we place our kernel
        // allocs. TODO: Move this check out to a better place
        assert!(vaddr + (size - 1) < KERNEL_ALLOCATIONS_BASE_VADDR as usize);

        // Create a virtual mapping for the kernel segment
        directory.map_init(phys_mem, VirtAddr(vaddr.try_into().ok()?), size.try_into().ok()?,
            write, false, |offset| {
            if offset < init_bytes.len() {
                init_bytes[offset]
            } else {
                0u8
            }
        })?;

        Some(())
    }).expect("Failed to load and map kernel");

    let kernel_entry: u32 = kernel_elf.entry_point.try_into().ok()
        .expect("Kernel entry is outside of 32-bit address range");

    // Map the kernel stack
    directory.map(phys_mem, VirtAddr(KERNEL_STACK_BASE_VADDR), KERNEL_STACK_SIZE, true, false)
        .expect("Failed to map kernel stack");

    // Temp identity map of the first 1MiB so we can continue executing after changing cr3
    for paddr in (0..(1024*1024)).step_by(4096) {
        directory.map_to_phys_page(phys_mem, VirtAddr(paddr), PhysAddr(paddr), true, false, false,
            true).expect("Failed to map temp identity map");
    }

    // The new CR3 is the physical address of the page directory
    let new_cr3 = directory.get_directory_addr().0;


    // We permenantly map the page table containg the last page, which will be
    // used to map other page tables in and out of the last page so we can access them.
    let table_paddr = directory.get_page_table(phys_mem, VirtAddr(0xFFFFF000))
        .expect("Failed to get the phys addr of the last page table");
    directory.map_to_phys_page(phys_mem, VirtAddr(LAST_PAGE_TABLE_VADDR), table_paddr, true, false,
        false, true).expect("Failed to map page directory");
    
    println!("Kernel entry at {:#x}, Page directory at {:#x}", kernel_entry, new_cr3);

    (kernel_entry, KERNEL_STACK_BASE_VADDR + KERNEL_STACK_SIZE, new_cr3, table_paddr)
}
