//! Basic VGA text-mode print functions

use page_tables::PhysAddr;

use crate::real_mode::{RegisterState, invoke_realmode_interrupt};

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

/// Returns a slice to the screen buffer
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

#[repr(C, packed)]
struct VBEInfoBlock {
    signature: [u8; 4],
    version: u16,
    oem_string_ptr: u32,
    capabilities: u32,
    video_mode_ptr: u32,
    total_memory: u16,
    oem_software_revision: u16,
    oem_vendor_name_ptr: u32,
    oem_product_name_ptr: u32,
    oem_product_rev_ptr: u32,
    reserved: [u8; 222],
    oem_data: [u8; 256],
}

#[repr(C, packed)]
struct ModeInfoBlock {
    mode_attributes: u16,
    window_a_attributes: u8,
    window_b_attributes: u8,
    window_granularity: u16,
    window_size: u16,
    window_a_start_segment: u16,
    window_b_start_segment: u16,
    window_function_ptr: u32,
    bytes_per_scanline: u16,

    x_resolution: u16,
    y_resolution: u16,
    x_char_size: u8,
    y_char_size: u8,
    number_of_planes: u8,
    bits_per_pixel: u8,
    number_of_banks: u8,
    memory_model: u8,
    bank_size: u8,
    number_of_image_pages: u8,
    reserved_1: u8, // Always 1

    // Masks are specified by a (size, position) pair which specificy how many bits and the
    // lsb of the mask
    red_mask_size: u8,
    red_field_position: u8,
    green_mask_size: u8,
    green_field_position: u8,
    blue_mask_size: u8,
    blue_field_position: u8,
    reserved_mask_size: u8,
    reserved_field_position: u8,
    direct_color_mode_attributes: u8,

    phys_frame_buffer_ptr: u32,
    reserved_2: u32,
    reserved_3: u16,

    linear_bytes_per_scanline: u16,
    banked_number_of_image_pages: u8,
    linear_number_of_image_pages: u8,
    linear_red_mask_size: u8,
    linear_red_field_position: u8,
    linear_green_mask_size: u8,
    linear_green_field_position: u8,
    linear_blue_mask_size: u8,
    linear_blue_field_position: u8,
    linear_reserved_mask_size: u8,
    linear_reserved_field_position: u8,
    max_pixel_clock: u32,
    reserved_4: [u8; 189],
    unknown: u8, // VBE3 Spec says the structure is 256 bytes long, but specifies only the previous
                 // fields which add to 255 bytes...
}


/// Sets up graphics mode using the BIOS VESA interface. Returns the physical address of the frame
/// buffer, and its width and height
pub fn setup_vesa() -> (PhysAddr, u16, u16) {
    let mut info_block = VBEInfoBlock {
        signature: [0x56, 0x42, 0x45, 0x32], // Pre-setting "VBE2" as the signature signifies we
                                             // we want VESA3.0
        version: 0,
        oem_string_ptr: 0,
        capabilities: 0,
        video_mode_ptr: 0,
        total_memory: 0,
        oem_software_revision: 0,
        oem_vendor_name_ptr: 0,
        oem_product_name_ptr: 0,
        oem_product_rev_ptr: 0,
        reserved: [0; 222],
        oem_data: [0; 256],
    };
    assert!(core::mem::size_of::<VBEInfoBlock>() == 512);

    let mut register_context = RegisterState {
        eax: 0x4F00, // Return VBE Controller Information
        edi: &mut info_block as *mut VBEInfoBlock as u32,
        ..Default::default()
    };

    unsafe { invoke_realmode_interrupt(0x10, &mut register_context); }

    if register_context.eax != 0x4F {
        panic!("Failed to get VBE controller info");
    }
    assert!(info_block.version == 0x0300);

    // Convert from seg:off 16bit pointer to 32-bit pointer
    let real_mode_ptr = |ptr: u32| ((ptr & 0xFFFF0000) >> 12) + (ptr & 0xFFFF);
    let mode_list_ptr = real_mode_ptr(info_block.video_mode_ptr) as *const u16;

    // We iterate over all available modes, searching for a 1440x900 32 bits/pixel graphics mode.
    // This is obviously not a final solution, we should find the best mode available and inform
    // the kernel about the result
    let mut mode_to_set = None;
    let mut i = 0;
    loop {
        let mode = unsafe { *mode_list_ptr.offset(i) };
        i += 1;

        if mode == 0xFFFF {
            break;
        }

        let mut mode_info = ModeInfoBlock {
            mode_attributes: 0,
            window_a_attributes: 0,
            window_b_attributes: 0,
            window_granularity: 0,
            window_size: 0,
            window_a_start_segment: 0,
            window_b_start_segment: 0,
            window_function_ptr: 0,
            bytes_per_scanline: 0,
            x_resolution: 0,
            y_resolution: 0,
            x_char_size: 0,
            y_char_size: 0,
            number_of_planes: 0,
            bits_per_pixel: 0,
            number_of_banks: 0,
            memory_model: 0,
            bank_size: 0,
            number_of_image_pages: 0,
            reserved_1: 1,
            red_mask_size: 0,
            red_field_position: 0,
            green_mask_size: 0,
            green_field_position: 0,
            blue_mask_size: 0,
            blue_field_position: 0,
            reserved_mask_size: 0,
            reserved_field_position: 0,
            direct_color_mode_attributes: 0,
            phys_frame_buffer_ptr: 0,
            reserved_2: 0,
            reserved_3: 0,
            linear_bytes_per_scanline: 0,
            banked_number_of_image_pages: 0,
            linear_number_of_image_pages: 0,
            linear_red_mask_size: 0,
            linear_red_field_position: 0,
            linear_green_mask_size: 0,
            linear_green_field_position: 0,
            linear_blue_mask_size: 0,
            linear_blue_field_position: 0,
            linear_reserved_mask_size: 0,
            linear_reserved_field_position: 0,
            max_pixel_clock: 0,
            reserved_4: [0; 189],
            unknown: 0
        };
        assert!(core::mem::size_of::<ModeInfoBlock>() == 256);

        let mut register_context = RegisterState {
            eax: 0x4F01, // Return VBE Mode Information
            ecx: mode as u32,
            edi: &mut mode_info as *mut ModeInfoBlock as u32,
            ..Default::default()
        };
    
        unsafe { invoke_realmode_interrupt(0x10, &mut register_context); }
    
        if register_context.eax != 0x4F {
            panic!("Failed to get VBE mode info");
        }

        let mode_supported = (mode_info.mode_attributes & 1) != 0;
        let color_mode = (mode_info.mode_attributes & 8) != 0;
        let graphics_mode = (mode_info.mode_attributes & 16) != 0;
        let linear_frame_buffer = (mode_info.mode_attributes & 128) != 0;

        if !mode_supported || !linear_frame_buffer || !graphics_mode || !color_mode {
            continue;
        }

        if mode_info.memory_model != 6 {
            continue;
        }

        if mode_info.linear_red_mask_size != 8 || mode_info.linear_blue_mask_size != 8
            || mode_info.linear_green_mask_size != 8 {
            continue;
        }

        if mode_info.x_resolution != 1440 || mode_info.y_resolution != 900 || mode_info.bits_per_pixel != 32 {
            continue;
        }

        assert!(mode_info.linear_blue_field_position == 0);
        assert!(mode_info.linear_green_field_position == 8);
        assert!(mode_info.linear_red_field_position == 16);

        mode_to_set = Some((mode, mode_info.phys_frame_buffer_ptr));
        break;
    }

    let (mode_to_set, framebuffer_addr) = mode_to_set.expect("No support for 1440x900 32 bits/pixel");

    let mut register_context = RegisterState {
        eax: 0x4F02, // Set VBE Mode
        ebx: (mode_to_set as u32) | (1 << 14), // Bit 14 signifies we want a linear frame buffer
        ..Default::default()
    };

    unsafe { invoke_realmode_interrupt(0x10, &mut register_context); }

    if register_context.eax != 0x4F {
        panic!("Failed to set VBE mode");
    }

    (PhysAddr(framebuffer_addr), 1440, 900)
}