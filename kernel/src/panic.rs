//! Basic panic handler which prints the message to serial and halts

use core::panic::PanicInfo;
use serial::{print, println};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    print!("[KERNEL PANIC!]");
    
    if let Some(location) = info.location() {
        print!(" {}:{}:{}", location.file(), location.line(), location.column());
    }
        
    if let Some(msg) = info.message() {
        print!(" {}", msg);
    }
    println!();

	unsafe { cpu::halt(); }
}
