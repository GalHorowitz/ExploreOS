//! Basic VGA text-mode print functions

// For future reference:
// http://web.stanford.edu/class/cs140/projects/pintos/specs/freevga/vga/vga.htm#register

// FIXME: NOT THREAD SAFE

use page_tables::{VirtAddr, PhysAddr};

const SCREEN_BUFFER_PADDR: u32 = 0xB8000;
const SCREEN_BUFFER_VADDR: u32 = 0xB8000000;
const SCREEN_HEIGHT: usize = 25;
const SCREEN_WIDTH: usize = 80;
pub const ATTR_WHITE_ON_BLACK: u8 = 0x0f;

const REG_SCREEN_CTRL_PORT: u16 = 0x3D4;
const REG_SCREEN_DATA_PORT: u16 = 0x3D5;
const CURSOR_START_REG_INDEX: u8 = 10;
const CURSOR_END_REG_INDEX: u8 = 11;
const CURSOR_HIGH_REG_INDEX: u8 = 14;
const CURSOR_LOW_REG_INDEX: u8 = 15;

/// Initializes the screen
pub fn init() {
    let mut pmem = crate::memory_manager::PHYS_MEM.lock();
    let phys_mem = pmem.as_mut().unwrap();
    
    let mut pages = crate::memory_manager::PAGES.lock();
    let page_dir = pages.as_mut().unwrap();

    // Map the screen buffer so we can write to it
    page_dir.map_to_phys_page(phys_mem, VirtAddr(SCREEN_BUFFER_VADDR),
        PhysAddr(SCREEN_BUFFER_PADDR), true, false, true, false)
        .expect("Failed to map screen buffer");
    
    // Reset the screen
    clear_screen();
    // Reset the cursor position
    set_cursor_offset(0);
    // Reset the cursor shape
    enable_cursor(13, 14);
}

/// Returns a slice to the screen buffer
fn get_screen_buffer() -> &'static mut [u16] {
    unsafe {
        core::slice::from_raw_parts_mut(SCREEN_BUFFER_VADDR as *mut u16, SCREEN_WIDTH*SCREEN_HEIGHT)
    }
}


/// Prints `message` on screen at the cursor
pub fn print(message: &str) {
    for &ch in message.as_bytes() {
        print_char(ch, ATTR_WHITE_ON_BLACK);
    }
}

/// Prints `message` on screen at the cursor with the specified `attributes`
pub fn print_with_attributes(message: &str, attributes: u8) {
    for &ch in message.as_bytes() {
        print_char(ch, attributes);
    }
}

/// Prints one `character` to the screen with the specified `attributes` at the cursor, and then
/// advances the cursor. Also handles new lines.
pub fn print_char(character: u8, attributes: u8) {
    let screen_buffer = get_screen_buffer();

    let cursor_offset = get_cursor_offset();

    // Check if we got a new line
    if character == b'\n' {
        // Get the actual row
        let cursor_row = cursor_offset / SCREEN_WIDTH;

        // If we get a new line at the last row we need to scroll the screen
        if cursor_row == SCREEN_HEIGHT - 1 {
            scroll_one_line();
            // Actually set the cursor offset to the start of this row
            set_cursor_offset(cursor_row * SCREEN_WIDTH);
        } else {
            // Set the cursor offset to the start of the next row
            set_cursor_offset((cursor_row + 1) * SCREEN_WIDTH);
        }
    } else {
        // Combine the character and attribute
        let char_and_attr = ((attributes as u16) << 8) | (character as u16);
        screen_buffer[cursor_offset] = char_and_attr;

        // If we just set the last character of the screen we need to scroll
        if cursor_offset == (SCREEN_WIDTH * SCREEN_HEIGHT) - 1 {
            scroll_one_line();
            // Set the cursor offset to the start of the last row
            set_cursor_offset((SCREEN_HEIGHT - 1) * SCREEN_WIDTH);
        } else {
            // Advance the cursor
            set_cursor_offset(cursor_offset + 1);
        }
    }
}

/// Clears the entire screen
pub fn clear_screen() {
    // We must include an attribute or else the cursor won't show up
    get_screen_buffer().fill((ATTR_WHITE_ON_BLACK as u16) << 8);
}

/// Sets the character cursor offset of the VGA device.
pub fn set_cursor_offset(offset: usize) {
    assert!(offset < SCREEN_WIDTH*SCREEN_HEIGHT);
    unsafe {
        // The control port is used as an index into the registers
        // Index 14 is the high byte of the cursor offset
        cpu::out8(REG_SCREEN_CTRL_PORT, CURSOR_HIGH_REG_INDEX);
        cpu::out8(REG_SCREEN_DATA_PORT, (offset >> 8) as u8);

        // Index 15 is the low byte of the cursor offset
        cpu::out8(REG_SCREEN_CTRL_PORT, CURSOR_LOW_REG_INDEX);
        cpu::out8(REG_SCREEN_DATA_PORT, (offset & 0xFF) as u8);
    }
}

/// Scrolls the screen one line by memmoving the rows up one row, and clearing the last row
pub fn scroll_one_line() {
    let screen_buffer = get_screen_buffer();

    // We get a reference to the rows following the first row, this is the source of the copy
    let second_row_onward = &screen_buffer[SCREEN_WIDTH..];

    // Calculate how many u16s we need to copy for the entire screen except for one row
    let num_elements = SCREEN_WIDTH * (SCREEN_HEIGHT - 1);

    unsafe {
        core::ptr::copy(second_row_onward.as_ptr(), screen_buffer.as_mut_ptr(), num_elements);
    }

    // Clear the last row (We must include an attribute or else the cursor won't show up)
    screen_buffer[num_elements..].fill((ATTR_WHITE_ON_BLACK as u16) << 8);
}

/// Retrieves the character cursor offset from the VGA device.
pub fn get_cursor_offset() -> usize {
    // TODO: We are the only one controlling the screen, we can just save the cursor location
    // instead of accessing the ports which is slow
    unsafe {
        // The control port is used as an index into the registers
        // Index 14 is the high byte of the cursor offset
        cpu::out8(REG_SCREEN_CTRL_PORT, CURSOR_HIGH_REG_INDEX);
        let offset_high = cpu::in8(REG_SCREEN_DATA_PORT) as u16;

        // Index 15 is the low byte of the cursor offset
        cpu::out8(REG_SCREEN_CTRL_PORT, CURSOR_LOW_REG_INDEX);
        let offset_low = cpu::in8(REG_SCREEN_DATA_PORT) as u16;

        ((offset_high << 8) | offset_low) as usize
    }
}

pub fn enable_cursor(cursor_start: u8, cursor_end: u8) {
    assert!(cursor_start < 32);
    assert!(cursor_end < 32);
    unsafe {
        // Bits 0-4 control the cursor start, bit 5 is the cursor disable bit, and bits 6-7 are reserved
        cpu::out8(REG_SCREEN_CTRL_PORT, CURSOR_START_REG_INDEX);
        cpu::out8(REG_SCREEN_DATA_PORT, cursor_start | (cpu::in8(REG_SCREEN_DATA_PORT)&0xc0));
        
        cpu::out8(REG_SCREEN_CTRL_PORT, CURSOR_END_REG_INDEX);
        cpu::out8(REG_SCREEN_DATA_PORT, cursor_end | (cpu::in8(REG_SCREEN_DATA_PORT)&0xe0));
    }
}

pub fn disable_cursor() {
    unsafe {
        cpu::out8(REG_SCREEN_CTRL_PORT, CURSOR_START_REG_INDEX);
        cpu::out8(REG_SCREEN_DATA_PORT, 0b00100000);
    }
}