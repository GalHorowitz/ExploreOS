//! A library to hold common structure definition for the bootloader and kernel for passing during
//! the initial boot process

#![no_std]

use range_set::RangeSet;
use serial::SerialPort;
use page_tables::PhysAddr;

/// The size in bytes of the kernel stack
pub const KERNEL_STACK_SIZE: u32 = 0x1000;
/// The virtual address of the base of the kernel stack
pub const KERNEL_STACK_BASE_VADDR: u32 = LAST_PAGE_TABLE_VADDR - KERNEL_STACK_SIZE;

/// The virtual address of the base of kernel virtual allocations
pub const KERNEL_ALLOCATIONS_BASE_VADDR: u32 = 0xC4000000;

/// The virtual address where the page table containing the last page is mapped
pub const LAST_PAGE_TABLE_VADDR: u32 = 0xFFFFE000;

/// A structure to hold data the bootloader wants to pass to the kernel
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BootArgs {
    /// All memory ranges which are avaiable for use
    pub free_memory: RangeSet,
    /// The serial ports available for use
    pub serial_port: SerialPort,
    /// The physical address of the page table containing the last page. The kernel needs this
    /// information to access physical memory
    pub last_page_table_paddr: PhysAddr,

    /// The physical address of the linear frame buffer
    pub frame_buffer_paddr: PhysAddr,
    /// The width of frame buffer
    pub frame_buffer_width: u16,
    /// The height of the frame buffer
    pub frame_buffer_height: u16,
}