//! The entry point for the Rust bootloader

#![no_std]
#![no_main]
#![feature(panic_info_message)]

use core::panic::PanicInfo;
use serial::{print, println};

mod compiler_reqs;
mod screen;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    print!("[PANIC!]");

    if let Some(location) = info.location() {
        print!(" {}:{}:{}", location.file(), location.line(), location.column());
    }

    if let Some(msg) = info.message() {
        print!(" {}", msg);
    }
    print!("\n");    

    cpu::halt();
}

/// x86 register state for invoking interrupts
#[derive(Default)]
#[repr(C)]
struct RegisterState {
    eax: u32,
    ecx: u32,
    edx: u32,
    ebx: u32,
    ebp: u32,
    esi: u32,
    edi: u32,
    eflags: u32,

    ds: u16,
    es: u16,
    fs: u16,
    gs: u16,
    ss: u16
}

extern {
    /// Invokes the interrupt `interrupt_num` in real mode with the specified context `regs`.
    /// The function overwrites `regs` with the state of the interrupt result.
    /// The eflags and the segment registers passed in are ignored.
    fn invoke_realmode_interrupt(interrupt_num: u8, regs: *mut RegisterState);
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
    screen::print_with_attributes("This is colored!", 0xB4);

    unsafe {
        invoke_realmode_interrupt(0x10, &mut RegisterState {
            eax: 0x00_03,
            ..Default::default()
        });
    }
    println!("We returned from `invoke_realmode_interrupt`!");
    
    cpu::halt();
}
