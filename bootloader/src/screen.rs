//! Basic VGA text-mode print functions

const SCREEN_BUFFER_ADDRESS: usize = 0xb8000;
const SCREEN_HEIGHT: usize = 25;
const SCREEN_WIDTH: usize = 80;
const ATTR_WHITE_ON_BLACK: u8 = 0x0f;

const REG_SCREEN_CTRL_PORT: u16 = 0x3D4;
const REG_SCREEN_DATA_PORT: u16 = 0x3D5;
const CURSOR_HIGH_REG_INDEX: u8 = 14;
const CURSOR_LOW_REG_INDEX: u8 = 15;

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

/// Clears the screen and resets the cursor offset
pub fn reset() {
    clear_screen();
    set_cursor_offset(0);
}

/// Clears the entire screen
pub fn clear_screen() {
    get_screen_buffer().fill(0);
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
fn scroll_one_line() {
    let screen_buffer = get_screen_buffer();

    // We get a reference to the rows following the first row, this is the source of the copy
    let second_row_onward = &screen_buffer[SCREEN_WIDTH..];

    // Calculate how many u16s we need to copy for the entire screen except for one row
    let num_elements = SCREEN_WIDTH * (SCREEN_HEIGHT - 1);

    unsafe {
        core::ptr::copy(second_row_onward.as_ptr(), screen_buffer.as_mut_ptr(), num_elements);
    }

    // Clear the last row
    screen_buffer[num_elements..].fill(0);
}

/// Returns a slice to the screen buffer. Wraps unsafe code
fn get_screen_buffer() -> &'static mut [u16] {
    unsafe {
        core::slice::from_raw_parts_mut(SCREEN_BUFFER_ADDRESS as *mut u16, SCREEN_WIDTH*SCREEN_HEIGHT)
    }
}

/// Retrieves the character cursor offset from the VGA device.
fn get_cursor_offset() -> usize {
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