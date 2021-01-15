#![no_std]
#![no_main]
#![feature(asm)]

use core::panic::PanicInfo;

mod compiler_reqs;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern fn entry() -> ! {
    unsafe {
        core::ptr::write(0xb8000 as *mut u16, 0x0f53);
        asm!(r#"
            cli
            hlt
        "#, options(noreturn, nostack));
    }
}
