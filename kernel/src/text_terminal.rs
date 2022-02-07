//! Text-terminal emulation for basic printing in graphics mode

// FIXME: NOT THREAD SAFE

use exclusive_cell::ExclusiveCell;

const TERMINAL_COLS: usize = 120;
const TERMINAL_ROWS: usize = 50;

const FONT_DATA: &'static [u8] = include_bytes!("../../font/compact_font.bin");
const FONT_FIRST_CHAR: u8 = 32;
const FONT_LAST_CHAR: u8 = 126;
const FONT_WIDTH: usize = 12;
const FONT_HEIGHT: usize = 18;


struct TextTerminal {
	cursor_row: usize,
	cursor_col: usize,
	text: [u8; TERMINAL_COLS * TERMINAL_ROWS],
}

impl TextTerminal {
	const fn new() -> Self {
		Self {
			cursor_row: 0,
			cursor_col: 0,
			text: [0u8; TERMINAL_COLS * TERMINAL_ROWS],
		}
	}

	fn char_at(&mut self, col: usize, row: usize) -> &mut u8 {
		&mut self.text[(row * TERMINAL_COLS) + col]
	}

	/// Prints one `character` to the screen at the cursor, and then advances the cursor.
	/// Also handles new lines.
	fn print_char(&mut self, character: u8) {
		if character == b'\n' {	
			if self.cursor_row == TERMINAL_ROWS - 1 {
				// If we get a new line at the last row we need to scroll the screen
				self.scroll_one_line();
				// Actually set the cursor offset to the start of this row
				self.cursor_col = 0;
			} else {
				// Set the cursor offset to the start of the next row
				self.cursor_col = 0;
				self.cursor_row += 1;
			}
		} else if character == b'\r' {
			// If this is a carriage return, we move the cursor to the start of the row
			self.cursor_col = 0;
		} else if character == 8 {
			// If this is a backspace character, we clear the last character by setting it to zero
			
			// If we are not at the start of the screen, we move the cursor back
			if self.cursor_col != 0 || self.cursor_row != 0 {
				if self.cursor_col > 0 {
					self.cursor_col -= 1;
					*self.char_at(self.cursor_col, self.cursor_row) = 0;
				} else {
					self.cursor_row -= 1;
					// Find the last character in the previous line
					let mut last_char_col = 0;
					for i in (1..TERMINAL_COLS).rev() {
						if *self.char_at(i, self.cursor_row) != 0 {
							last_char_col = i;
							break;
						}
					}
					// We only remove a character if the line extended all the way to the end,
					// otherwise we treat the backspace as if it removed the 'newline'
					if last_char_col == TERMINAL_COLS - 1 {
						self.cursor_col = last_char_col;
						*self.char_at(self.cursor_col, self.cursor_row) = 0;
					} else {
						self.cursor_col = last_char_col + 1;
					}
				}
			}
		} else {
			*self.char_at(self.cursor_col, self.cursor_row) = character;

			// If this was the last character of the screen we need to scroll
			if self.cursor_row == TERMINAL_ROWS - 1 && self.cursor_col == TERMINAL_COLS - 1 {
				self.scroll_one_line();
				// Set the cursor offset to the start of the last row
				self.cursor_col = 0;
			} else {
				// Advance the cursor
				if self.cursor_col == TERMINAL_COLS - 1 {
					self.cursor_col = 0;
					self.cursor_row += 1;
				} else {
					self.cursor_col += 1;
				}
			}
		}
	}

	fn clear(&mut self) {
		self.text.fill(0);
	}

	/// Scrolls the screen one line by memmoving the rows up one row, and clearing the last row
	fn scroll_one_line(&mut self) {
		// We get a reference to the rows following the first row, this is the source of the copy
		let second_row_onward = &self.text[TERMINAL_COLS..];

		// Calculate how many u8s we need to copy for the entire screen except for one row
		let num_elements = TERMINAL_COLS * (TERMINAL_ROWS - 1);

		unsafe {
			core::ptr::copy(second_row_onward.as_ptr(), self.text.as_mut_ptr(), num_elements);
		}

		// Clear the last row
		self.text[num_elements..].fill(0);
	}
}

static TEXT_TERMINAL: ExclusiveCell<TextTerminal> = ExclusiveCell::new(TextTerminal::new());

pub fn redraw() {
	let terminal = TEXT_TERMINAL.acquire();

	let mut fb = crate::graphics_screen::FRAME_BUFFER.acquire();
	let frame_buffer = fb.as_mut().unwrap();

	for y in 0..TERMINAL_ROWS {
		for x in 0..TERMINAL_COLS {
			let mut chr = terminal.text[(y * TERMINAL_COLS) + x];
			if chr == 0 {
				chr = 32;
			}
			assert!(FONT_FIRST_CHAR <= chr && chr <= FONT_LAST_CHAR);

			for row in 0..FONT_HEIGHT {
				for col in 0..FONT_WIDTH {
					let char_off = (chr - FONT_FIRST_CHAR) as usize * FONT_WIDTH * FONT_HEIGHT;
					let font_off = char_off + (row * FONT_WIDTH) + col;

					let frame_x = (x * FONT_WIDTH) + col;
					let frame_y = (y * FONT_HEIGHT) + row;
					let frame_idx = (frame_y * frame_buffer.width) + frame_x;

					let mut gray_val = FONT_DATA[font_off] as u32;

					if terminal.cursor_col == x && terminal.cursor_row == y
						&& (row == 0 || row == FONT_HEIGHT - 1 || col == 0 || col == FONT_WIDTH - 1) {
						gray_val = 255;
					}

					let color_splat = gray_val | (gray_val << 8) | (gray_val << 16) | (gray_val << 24);
					frame_buffer.get_buffer()[frame_idx] = color_splat;
				}
			}
		}
	}
}

/// Prints `message` on screen at the cursor
pub fn print(message: &str) {
	{
		let mut terminal = TEXT_TERMINAL.acquire();
		for &ch in message.as_bytes() {
			terminal.print_char(ch);
		}
	}
	redraw();
}

pub fn debug_offset_cursor(mut off: isize) {
	off /= 10;
	{
		let mut terminal = TEXT_TERMINAL.acquire();
		terminal.cursor_col = (terminal.cursor_col as isize + off).min(TERMINAL_COLS as isize - 1).max(0) as usize;
	}
	if off != 0 {
		redraw();
	}
}