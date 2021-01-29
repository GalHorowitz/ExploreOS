//! A library to hold common structure definition for the bootloader and kernel for passing during
//! the initial boot process

#![no_std]

use range_set::RangeSet;
use serial::SerialPort;

/// A structure to hold data the bootloader wants to pass to the kernel
#[repr(C)]
pub struct BootArgs {
    /// All memory ranges which are avaiable for use
    pub free_memory: RangeSet,
    // The serial ports available for use
    pub serial_port: SerialPort
}