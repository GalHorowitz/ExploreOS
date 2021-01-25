#![feature(panic_info_message)]
#![no_std]
#![no_main]

extern crate compiler_reqs;

use range_set::RangeSet;
use page_tables::PageDirectory;

mod panic;

#[no_mangle]
pub extern fn entry(param: u32) -> ! {
    serial::init();
    serial::println!("Landed in kernel! {:#x}", param);

    let memory_map = unsafe {
        &*(param as *const RangeSet)
    };

    serial::println!("Free mem: {:#x}", memory_map.total_size().expect("Failed to get free mem"));

    let screen = unsafe {
        core::slice::from_raw_parts_mut(0xb8000 as *mut u16, 80*25)
    };
    screen.iter_mut().for_each(|x| *x = 0x0f39);

    cpu::halt();
}