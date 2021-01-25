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
use page_tables::{PageDirectory, VirtAddr};

extern {
    /// Calls the kernel `entry` with argument `param` after setting esp to `stack` and cr3 to
    /// `new_cr3`
    pub fn jump_to_kernel(entry: u32, stack: u32, param: u32, new_cr3: u32) -> !;
}

#[no_mangle]
pub extern fn entry(boot_disk_id: u8, bootloader_size: u32) -> ! {
    serial::init();
    memory_manager::init(bootloader_size);

    println!("Bootloader running!");

    screen::reset();
    screen::print("Welcome to the bootloader! Loading kernel from disk...\n");

    // Load the kernel
    let (kernel_entry, directory_base) = load_kernel(boot_disk_id, bootloader_size);

    // Get access to physical memory
    let mut pmem = memory_manager::PHYS_MEM.lock();
    let phys_mem = pmem.as_mut().expect("Physical memory is not initialized?!");
    // Get access to the page directory we created earlier when we loaded the kernel
    let mut directory = unsafe { PageDirectory::from_cr3(phys_mem, directory_base) };

    // Map the kernel stack
    const KERNEL_STACK_SIZE: u32 = 0x2000;
    const KERNEL_STACK_ADDR: u32 = 0xc0000000;
    unsafe {
        directory.map(VirtAddr(KERNEL_STACK_ADDR - KERNEL_STACK_SIZE), KERNEL_STACK_SIZE, true, false)
    }.expect("Failed to map kernel stack");

    // Temp identity map of the first 1MiB so we can continue executing after changing cr3
    unsafe {
        for paddr in (0..(1024*1024)).step_by(4096) {
            let raw_table_entry = 
                paddr | page_tables::PAGE_ENTRY_PRESENT | page_tables::PAGE_ENTRY_WRITE;
            directory.map_raw(VirtAddr(paddr), raw_table_entry, false).expect("Failed to identity map");
        }
    }

    println!("Kernel entry @{:#x}, Page directory @{:#x}", kernel_entry,
        directory.get_directory_addr().0);

    // The new CR3 is the physical address of the page directory
    let new_cr3 = directory.get_directory_addr().0;

    // We pass the physical address of the free memory range set to the kernel
    let memory_rangeset_addr = phys_mem as *const _ as u32;

    // Make the jump to the kernel
    unsafe {
        jump_to_kernel(kernel_entry, KERNEL_STACK_ADDR, memory_rangeset_addr, new_cr3);
    }
}

/// Reads the kernel from disk and maps into memory.
/// Returns (kernel entry vaddr, page directory paddr)
fn load_kernel(boot_disk_id: u8, bootloader_size: u32) -> (u32, u32) {
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
    kernel_elf.for_segment(|vaddr, size, init_bytes, flags| {
        println!("{:#09x} {:#09x} {:03b}", vaddr, size, flags);

        // Create a virtual mapping for the kernel segment
        unsafe {
            directory.map_init(VirtAddr(vaddr.try_into().ok()?), size.try_into().ok()?,
                flags & elf_parser::SEGMENT_FLAGS_PF_W != 0, false, |offset| {
                if offset < init_bytes.len() {
                    init_bytes[offset]
                } else {
                    0u8
                }
            })?;
        }

        Some(())
    }).expect("Failed to load and map kernel");

    let kernel_entry: u32 = kernel_elf.entry_point.try_into().ok()
        .expect("Kernel entry is outside of 32-bit address range");
    let directory_base = directory.get_directory_addr().0;

    (kernel_entry, directory_base)
}
