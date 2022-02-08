#![no_std]
#![feature(maybe_uninit_uninit_array, maybe_uninit_slice, panic_info_message)]

extern crate compiler_reqs;

pub mod syscalls;

// TODO: Find a better place for these constants
pub const STDIN_FD: u32 = 0;
pub const STDOUT_FD: u32 = 1;

#[panic_handler]
pub fn panic(info: &core::panic::PanicInfo) -> ! {
	print!("[PANIC!]");
    
    if let Some(location) = info.location() {
        print!(" {}:{}:{}", location.file(), location.line(), location.column());
    }
        
    if let Some(msg) = info.message() {
        print!(" {}", msg);
    }
    println!();

	syscalls::exit(1);
}

/// Dummy struct to implement `core::fmt::Write` on
pub struct ScreenWriter;

impl core::fmt::Write for ScreenWriter {
    fn write_str(&mut self, msg: &str) -> core::fmt::Result {
		assert!(syscalls::write(STDOUT_FD, msg.as_bytes()).unwrap() as usize == msg.as_bytes().len());
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            let _ = write!($crate::ScreenWriter, $($arg)*);
        }
    };
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            let _ = writeln!($crate::ScreenWriter, $($arg)*);
        }
    };
}