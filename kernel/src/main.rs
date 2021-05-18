//! Kernel entry point

#![feature(panic_info_message, default_alloc_error_handler, naked_functions, asm, const_panic)]
#![no_std]
#![no_main]

extern crate compiler_reqs;
extern crate alloc;

use boot_args::BootArgs;
use page_tables::{PhysAddr, VirtAddr};
use serial::println;

mod panic;
mod memory_manager;
mod tss;
mod gdt;
mod interrupts;
mod screen;
mod keyboard;
mod mouse;
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

    // Initialize the GDT and the TSS
    unsafe { gdt::init(); }

    // Initialize the IDT, PIC and PIT and enable interrupts
    interrupts::init();
    println!("Enabled interrupts");

    // Initialize the PS/2 controller (which will in turn initialize a keyboard driver if a PS/2
    // keyboard is connected)
    ps2::controller::init();

    // Initialize and clear the screen
    screen::init();
    screen::print("> ");

    // Test syscall TODO: REMOVE
    unsafe {
        asm!("
            int 0x67
        ");
    }

    // Test heap allocator TODO: REMOVE
    {
        let start = cpu::serializing_rdtsc();
        let mut vec: alloc::vec::Vec<u8> = alloc::vec::Vec::with_capacity(1024 * 1024);
        vec.push(4u8);
        vec.push(6u8);
        vec.push(8u8);
        vec.push(13u8);
        let elapsed = cpu::serializing_rdtsc() - start;
        println!("Took {} cycles to allocate {} bytes {:?}", elapsed, vec.capacity(), &vec[..3]);
    }

    const USER_CODE_VADDR:  u32 = 0x1000_0000;
    const USER_STACK_VADDR: u32 = 0x0FFF_F000;
    const USER_STACK_SIZE: u32 = 0x1000;
    const KERNEL_INTR_STACK_VADDR: u32 = 0xFFFFB000;
    const KERNEL_INTR_STACK_SIZE: u32 = 0x1000;

    let func_vaddr = {
        let mut pmem = memory_manager::PHYS_MEM.lock();
        let (phys_mem, page_dir) = pmem.as_mut().unwrap();

        page_dir.map(phys_mem, VirtAddr(KERNEL_INTR_STACK_VADDR), KERNEL_INTR_STACK_SIZE, true,
            false).unwrap();

        let func_phys_addr = page_dir.translate_virt(phys_mem, VirtAddr(test_user as u32)).unwrap();
        let page_phys_addr = PhysAddr(func_phys_addr.0 & !0xFFF);
        page_dir.map_to_phys_page(phys_mem, VirtAddr(USER_CODE_VADDR), page_phys_addr, true, true,
            false, true).unwrap();

        page_dir.map(phys_mem, VirtAddr(USER_STACK_VADDR), USER_STACK_SIZE, true, true).unwrap();

        USER_CODE_VADDR + (func_phys_addr.0 & 0xFFF)
    };

    const USER_EFLAGS: u32 = 0b0000000000_000000_0_0_00_0_0_10_00_0_0_0_0_1_0;

    println!("func_vaddr={:#X}", func_vaddr);

    tss::set_kernel_esp(KERNEL_INTR_STACK_VADDR + KERNEL_INTR_STACK_SIZE);
    unsafe {
        cpu::jump_to_ring0(func_vaddr, gdt::USER_CS_SELECTOR | 3, USER_EFLAGS,
            USER_STACK_VADDR + USER_STACK_SIZE, gdt::USER_DS_SELECTOR | 3);
    }
}

extern fn test_user() -> ! {
    unsafe {
        asm!("int 0x67");
    }
    loop {}
}