//! Kernel entry point

#![feature(asm_sym, asm_const, panic_info_message, default_alloc_error_handler, naked_functions, box_syntax)]
#![no_std]
#![no_main]

extern crate compiler_reqs;
extern crate alloc;

use alloc::vec;
use boot_args::BootArgs;
use page_tables::VirtAddr;
use serial::println;
use elf_parser::ElfParser;

use crate::process::{SCHEDULER_STATE, Process};

mod panic;
mod memory_manager;
mod tss;
mod gdt;
mod interrupts;
mod screen;
mod keyboard;
mod mouse;
mod ps2;
mod vfs;
mod userspace;
mod syscall;
mod process;
mod ext2;
mod time;

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

    // Initialize the GDT and the TSS
    unsafe { gdt::init(); }

    // Get current time
    time::init();

    // Initialize the IDT, PIC and PIT and enable interrupts
    interrupts::init();
    println!("Enabled interrupts");

    // Initialize the PS/2 controller (which will in turn initialize a keyboard driver if a PS/2
    // keyboard is connected)
    ps2::controller::init();

    // Initialize and clear the screen
    screen::init();

    // Test syscall TODO: REMOVE
    // unsafe {
    //     asm!("
    //         int 0x67
    //     ");
    // }
    
    // Test heap allocator TODO: REMOVE
    // {
    //     let start = cpu::serializing_rdtsc();
    //     let mut vec: alloc::vec::Vec<u8> = alloc::vec::Vec::with_capacity(1024 * 1024);
    //     vec.push(4u8);
    //     vec.push(6u8);
    //     vec.push(8u8);
    //     vec.push(13u8);
    //     let elapsed = cpu::serializing_rdtsc() - start;
    //     println!("Took {} cycles to allocate {} bytes {:?}", elapsed, vec.capacity(), &vec[..3]);
    // }

    ext2::init();

    let user_program = {
        let ext2_parser = ext2::EXT2_PARSER.lock();
        let ext2_parser = ext2_parser.as_ref().unwrap();
        let (user_program_inode, _) = ext2_parser.resolve_path_to_inode("/bin/shell", ext2_parser::ROOT_INODE).unwrap();
        let user_program_metadata = ext2_parser.get_inode(user_program_inode);
        let user_program_size = user_program_metadata.size_low as usize;
        let mut user_program = vec![0u8; user_program_size];
        assert!(ext2_parser.get_contents(user_program_inode, &mut user_program) == user_program_size);
        user_program
    };

    const KERNEL_INTR_STACK_VADDR: u32 = 0xFFFFB000;

    let elf_parser = ElfParser::parse(&user_program).unwrap();
    let proc = Process::new_from_elf(VirtAddr(KERNEL_INTR_STACK_VADDR), elf_parser);

    SCHEDULER_STATE.lock().processes[0] = Some(proc);
    process::switch_to_current_process();
}

// TODO: Package this better, currently changes in userland cause the kernel to re-compile
pub const RAM_EXT2_FS: &[u8] = include_bytes!("../../userland/test_ext2.fs");