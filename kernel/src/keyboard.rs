//! General keyboard definitions and methods

use exclusive_cell::ExclusiveCell;

// The order of keys is generally from top to bottom, left to right, first the main keys, then the
// action keys, then arrows, and then the numpad and finally multimedia keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum KeyCode {
	Unknown = 0,
	KeyEscape,
	KeyF1,
	KeyF2,
	KeyF3,
	KeyF4,
	KeyF5,
	KeyF6,
	KeyF7,
	KeyF8,
	KeyF9,
	KeyF10,
	KeyF11,
	KeyF12,
	KeyBackTick,
	Key1,
	Key2,
	Key3,
	Key4,
	Key5,
	Key6,
	Key7,
	Key8,
	Key9,
	Key0,
	KeyMinus,
	KeyEquals,
	KeyBackspace,
	KeyTab,
	KeyQ,
	KeyW,
	KeyE,
	KeyR,
	KeyT,
	KeyY,
	KeyU,
	KeyI,
	KeyO,
	KeyP,
	KeyLeftSquareBracket,
	KeyRightSquareBracket,
	KeyEnter,
	KeyCapsLock,
	KeyA,
	KeyS,
	KeyD,
	KeyF,
	KeyG,
	KeyH,
	KeyJ,
	KeyK,
	KeyL,
	KeySemicolon,
	KeyApostrophe,
	KeyBackSlash,
	KeyLeftShift,
	KeyExtraBackSlash, // Present on some keyboards to the right of the left shift
	KeyZ,
	KeyX,
	KeyC,
	KeyV,
	KeyB,
	KeyN,
	KeyM,
	KeyComma,
	KeyPeriod,
	KeySlash,
	KeyRightShift,
	KeyLeftControl,
	KeyLeftLogo,
	KeyLeftAlt,
	KeySpace,
	KeyRightAlt,
	KeyRightLogo,
	KeyMenu,
	KeyRightControl,
	KeyPrintScreen,
	KeyScrollLock,
	KeyPause,
	KeyInsert,
	KeyHome,
	KeyPageUp,
	KeyDelete,
	KeyEnd,
	KeyPageDown,
	KeyUpArrow,
	KeyLeftArrow,
	KeyDownArrow,
	KeyRightArrow,
	KeyNumberLock,
	KeyNumpadSlash,
	KeyNumpadAsterisk,
	KeyNumpadMinus,
	KeyNumpad7,
	KeyNumpad8,
	KeyNumpad9,
	KeyNumpadPlus,
	KeyNumpad4,
	KeyNumpad5,
	KeyNumpad6,
	KeyNumpad1,
	KeyNumpad2,
	KeyNumpad3,
	KeyNumpadEnter,
	KeyNumpad0,
	KeyNumpadPeriod,
	KeyMultimediaWebBack,
	KeyMultimediaWebForward,
	KeyMultimediaPreviousTrack,
	KeyMultimediaStop,
	KeyMultimediaPlayPause,
	KeyMultimediaNextTrack,
	KeyMultimediaVolumeUp,
	KeyMultimediaMute,
	KeyMultimediaVolumeDown,
	KeyMultimediaWebHome,
	KeyMultimediaWebStop,
	KeyMultimediaSearch,
	KeyMultimediaEmail,
	KeyMultimediaCalculator,
	KeyMultimediaFavorites,
	KeyMultimediaRefresh,
	KeyMultimediaMyComputer,
	KeyMultimediaMediaSelect,
	KeyACPIPower,
	KeyACPISleep,
	KeyACPIWake,
	Count
}

impl KeyCode {
	/// Whether or not the key is in the numpad area
	fn is_in_numpad(&self) -> bool {
		match self {
			KeyCode::KeyNumpad0 | KeyCode::KeyNumpad1 | KeyCode::KeyNumpad2 | KeyCode::KeyNumpad3 |
			KeyCode::KeyNumpad4 | KeyCode::KeyNumpad5 | KeyCode::KeyNumpad6 | KeyCode::KeyNumpad7 |
			KeyCode::KeyNumpad8 | KeyCode::KeyNumpad9 | KeyCode::KeyNumberLock |
			KeyCode::KeyNumpadSlash | KeyCode::KeyNumpadAsterisk | KeyCode::KeyNumpadMinus |
			KeyCode::KeyNumpadPlus | KeyCode::KeyNumpadEnter | KeyCode::KeyNumpadPeriod => true,
			_ => false
		}
	}

	/// Whether or not this is a letter key
	fn is_letter(&self) -> bool {
		match self {
			KeyCode::KeyA | KeyCode::KeyB | KeyCode::KeyC | KeyCode::KeyD | KeyCode::KeyE |
			KeyCode::KeyF | KeyCode::KeyG | KeyCode::KeyH | KeyCode::KeyI | KeyCode::KeyJ |
			KeyCode::KeyK | KeyCode::KeyL | KeyCode::KeyM | KeyCode::KeyN | KeyCode::KeyO |
			KeyCode::KeyP | KeyCode::KeyQ | KeyCode::KeyR | KeyCode::KeyS | KeyCode::KeyT |
			KeyCode::KeyU | KeyCode::KeyV | KeyCode::KeyW | KeyCode::KeyX | KeyCode::KeyY |
			KeyCode::KeyZ => true,
			_ => false
		}
	}
}

pub enum KeyEventType {
	KeyDown,
	KeyUp,
}

pub struct KeyEvent {
	pub key_code: KeyCode,
	pub event_type: KeyEventType,
	
	// Modifiers
	pub shift_down: bool,
	pub ctrl_down: bool,
	pub alt_down: bool,
	pub logo_down: bool,
	pub caps_lock_enabled: bool,
	pub number_lock_enabled: bool,
}

impl KeyEvent {
	/// Returns the ASCII representation of the pressed key, modifier keys are respected. `None` is
	/// returned if the key press does not have an ASCII representation.
	fn as_ascii(&self) -> Option<u8> {
		// If Control/Alt/Logo is down, this is not a normal text key.
		if self.ctrl_down || self.alt_down || self.logo_down {
			return None;
		}

		let ascii_code = if self.key_code.is_in_numpad() {
			// If this is a numpad key, then the number lock has to be respected. If the number lock
			// is not enabled, or if shift down (even if number lock is enabled), the numbers act as
			// their action-counterpart, and not as text.
			if !self.number_lock_enabled || self.shift_down {
				match self.key_code {
					KeyCode::KeyNumpadSlash => b'/',
					KeyCode::KeyNumpadAsterisk => b'*',
					KeyCode::KeyNumpadMinus => b'-',
					KeyCode::KeyNumpadPlus => b'+',
					KeyCode::KeyNumpadEnter => b'\n',
					KeyCode::KeyNumpadPeriod => b'.',
					_ => 0
				}
			} else {
				match self.key_code {
					KeyCode::KeyNumpad0 => b'0',
					KeyCode::KeyNumpad1 => b'1',
					KeyCode::KeyNumpad2 => b'2',
					KeyCode::KeyNumpad3 => b'3',
					KeyCode::KeyNumpad4 => b'4',
					KeyCode::KeyNumpad5 => b'5',
					KeyCode::KeyNumpad6 => b'6',
					KeyCode::KeyNumpad7 => b'7',
					KeyCode::KeyNumpad8 => b'8',
					KeyCode::KeyNumpad9 => b'9',
					KeyCode::KeyNumpadSlash => b'/',
					KeyCode::KeyNumpadAsterisk => b'*',
					KeyCode::KeyNumpadMinus => b'-',
					KeyCode::KeyNumpadPlus => b'+',
					KeyCode::KeyNumpadEnter => b'\n',
					KeyCode::KeyNumpadPeriod => b'.',
					_ => 0
				}
			}
		} else if self.key_code.is_letter() {
			// If this is a letter key, caps lock has to be respected. Shift and caps lock both
			// switch from lower-case letters to upper-case letters, but if both caps lock is
			// enabled and shift down the effect is canceled and the letters are lower-case.
			if self.shift_down ^ self.caps_lock_enabled {
				match self.key_code {
					KeyCode::KeyA => b'A',
					KeyCode::KeyB => b'B',
					KeyCode::KeyC => b'C',
					KeyCode::KeyD => b'D',
					KeyCode::KeyE => b'E',
					KeyCode::KeyF => b'F',
					KeyCode::KeyG => b'G',
					KeyCode::KeyH => b'H',
					KeyCode::KeyI => b'I',
					KeyCode::KeyJ => b'J',
					KeyCode::KeyK => b'K',
					KeyCode::KeyL => b'L',
					KeyCode::KeyM => b'M',
					KeyCode::KeyN => b'N',
					KeyCode::KeyO => b'O',
					KeyCode::KeyP => b'P',
					KeyCode::KeyQ => b'Q',
					KeyCode::KeyR => b'R',
					KeyCode::KeyS => b'S',
					KeyCode::KeyT => b'T',
					KeyCode::KeyU => b'U',
					KeyCode::KeyV => b'V',
					KeyCode::KeyW => b'W',
					KeyCode::KeyX => b'X',
					KeyCode::KeyY => b'Y',
					KeyCode::KeyZ => b'Z',
					_ => 0
				}
			} else {
				match self.key_code {
					KeyCode::KeyA => b'a',
					KeyCode::KeyB => b'b',
					KeyCode::KeyC => b'c',
					KeyCode::KeyD => b'd',
					KeyCode::KeyE => b'e',
					KeyCode::KeyF => b'f',
					KeyCode::KeyG => b'g',
					KeyCode::KeyH => b'h',
					KeyCode::KeyI => b'i',
					KeyCode::KeyJ => b'j',
					KeyCode::KeyK => b'k',
					KeyCode::KeyL => b'l',
					KeyCode::KeyM => b'm',
					KeyCode::KeyN => b'n',
					KeyCode::KeyO => b'o',
					KeyCode::KeyP => b'p',
					KeyCode::KeyQ => b'q',
					KeyCode::KeyR => b'r',
					KeyCode::KeyS => b's',
					KeyCode::KeyT => b't',
					KeyCode::KeyU => b'u',
					KeyCode::KeyV => b'v',
					KeyCode::KeyW => b'w',
					KeyCode::KeyX => b'x',
					KeyCode::KeyY => b'y',
					KeyCode::KeyZ => b'z',
					_ => 0
				}
			}
		} else {
			// Keys have different meaning if the shift key is down
			if self.shift_down {
				match self.key_code {
					KeyCode::KeyBackTick => b'~',
					KeyCode::Key1 => b'!',
					KeyCode::Key2 => b'@',
					KeyCode::Key3 => b'#',
					KeyCode::Key4 => b'$',
					KeyCode::Key5 => b'%',
					KeyCode::Key6 => b'^',
					KeyCode::Key7 => b'&',
					KeyCode::Key8 => b'*',
					KeyCode::Key9 => b'(',
					KeyCode::Key0 => b')',
					KeyCode::KeyMinus => b'_',
					KeyCode::KeyEquals => b'+',
					KeyCode::KeyBackspace => 8, // TODO: Should I really do this?
					KeyCode::KeyTab => b'\t',
					KeyCode::KeyLeftSquareBracket => b'{',
					KeyCode::KeyRightSquareBracket => b'}',
					KeyCode::KeyEnter => b'\n',
					KeyCode::KeySemicolon => b':',
					KeyCode::KeyApostrophe => b'"',
					KeyCode::KeyBackSlash => b'|',
					KeyCode::KeyExtraBackSlash => b'|',
					KeyCode::KeyComma => b'<',
					KeyCode::KeyPeriod => b'>',
					KeyCode::KeySlash => b'?',
					KeyCode::KeySpace => b' ',
					_ => 0
				}
			} else {
				match self.key_code {
					KeyCode::KeyBackTick => b'`',
					KeyCode::Key1 => b'1',
					KeyCode::Key2 => b'2',
					KeyCode::Key3 => b'3',
					KeyCode::Key4 => b'4',
					KeyCode::Key5 => b'5',
					KeyCode::Key6 => b'6',
					KeyCode::Key7 => b'7',
					KeyCode::Key8 => b'8',
					KeyCode::Key9 => b'9',
					KeyCode::Key0 => b'0',
					KeyCode::KeyMinus => b'-',
					KeyCode::KeyEquals => b'=',
					KeyCode::KeyBackspace => 8, // TODO: Should I really do this?
					KeyCode::KeyTab => b'\t',
					KeyCode::KeyLeftSquareBracket => b'[',
					KeyCode::KeyRightSquareBracket => b']',
					KeyCode::KeyEnter => b'\n',
					KeyCode::KeySemicolon => b';',
					KeyCode::KeyApostrophe => b'\'',
					KeyCode::KeyBackSlash => b'\\',
					KeyCode::KeyExtraBackSlash => b'\\',
					KeyCode::KeyComma => b',',
					KeyCode::KeyPeriod => b'.',
					KeyCode::KeySlash => b'/',
					KeyCode::KeySpace => b' ',
					_ => 0
				}
			}
		};

		// We mark keys that are not representable in ASCII with a 0 (easier than writing Some(...)
		// everywhere)
		if ascii_code != 0 {
			Some(ascii_code)
		} else {
			None
		}
	}
}

struct KeyboardState {
	/// The state of each key in the keyboard. `true` signifies that the key is currently pressed
	key_state: [bool; KeyCode::Count as usize],

	number_lock_enabled: bool,
	caps_lock_enabled: bool,
	scroll_lock_enabled: bool,
}

impl KeyboardState {
	/// Constructs a keyboard state object that represents the state of the keyboard on start-up
	const fn new() -> Self {
		Self {
			key_state: [false; KeyCode::Count as usize],
			number_lock_enabled: true,
			caps_lock_enabled: false,
			scroll_lock_enabled: false,
		}
	}
}

/// The global keyboard state. Access should be exclusive: we do not expect to recieve two key
/// events simultaneously
static KEYBOARD_STATE: ExclusiveCell<KeyboardState> = ExclusiveCell::new(KeyboardState::new());

/// Updates the keyboard state given that the key with code `key_code` was pressed down
pub fn key_pressed_event(key_code: KeyCode) {
	// Acquire exclusive access to the keyboard state
	let mut keyboard_state = KEYBOARD_STATE.acquire();

	// Save the key as currently pressed
	keyboard_state.key_state[key_code as usize] = true;

	// Toggle the relevant lock state if the lock key is pressed
	if key_code == KeyCode::KeyCapsLock {
		keyboard_state.caps_lock_enabled = !keyboard_state.caps_lock_enabled;
	} else if key_code == KeyCode::KeyNumberLock {
		keyboard_state.number_lock_enabled = !keyboard_state.number_lock_enabled;
	} else if key_code == KeyCode::KeyScrollLock {
		keyboard_state.scroll_lock_enabled = !keyboard_state.scroll_lock_enabled;
	}

	// Calculate the modifier states by checking both left and right variants
	let shift_down = keyboard_state.key_state[KeyCode::KeyLeftShift as usize]
		|| keyboard_state.key_state[KeyCode::KeyRightShift as usize];
	let ctrl_down = keyboard_state.key_state[KeyCode::KeyLeftControl as usize]
		|| keyboard_state.key_state[KeyCode::KeyRightControl as usize];
	let alt_down = keyboard_state.key_state[KeyCode::KeyLeftAlt as usize]
		|| keyboard_state.key_state[KeyCode::KeyRightAlt as usize];
	let logo_down = keyboard_state.key_state[KeyCode::KeyLeftLogo as usize]
		|| keyboard_state.key_state[KeyCode::KeyRightLogo as usize];

	// Construct the KeyDown event
	let event = KeyEvent {
		key_code,
		event_type: KeyEventType::KeyDown,
		shift_down,
		ctrl_down,
		alt_down,
		logo_down,
		caps_lock_enabled: keyboard_state.caps_lock_enabled,
		number_lock_enabled: keyboard_state.number_lock_enabled,
	};

	// FIXME: REMOVE DEBUG
	if let Some(chr) = event.as_ascii() {
		crate::screen::print_char(chr, crate::screen::ATTR_WHITE_ON_BLACK);
		if chr == b'\n' {
			crate::screen::print("> ");
		}
	}
}

/// Updates the keyboard state given that the key with code `key_code` was released
pub fn key_released_event(key_code: KeyCode) {
	// Acquire exclusive rights to the keyboard state
	let mut keyboard_state = KEYBOARD_STATE.acquire();

	// Save the key as unpressed
	keyboard_state.key_state[key_code as usize] = false;

	// Calculate the modifier states by checking both left and right variants
	let shift_down = keyboard_state.key_state[KeyCode::KeyLeftShift as usize]
		|| keyboard_state.key_state[KeyCode::KeyRightShift as usize];
	let ctrl_down = keyboard_state.key_state[KeyCode::KeyLeftControl as usize]
		|| keyboard_state.key_state[KeyCode::KeyRightControl as usize];
	let alt_down = keyboard_state.key_state[KeyCode::KeyLeftAlt as usize]
		|| keyboard_state.key_state[KeyCode::KeyRightAlt as usize];
	let logo_down = keyboard_state.key_state[KeyCode::KeyLeftLogo as usize]
		|| keyboard_state.key_state[KeyCode::KeyRightLogo as usize];

	// Construct the KeyUp event
	let _event = KeyEvent {
		key_code,
		event_type: KeyEventType::KeyUp,
		shift_down,
		ctrl_down,
		alt_down,
		logo_down,
		caps_lock_enabled: keyboard_state.caps_lock_enabled,
		number_lock_enabled: keyboard_state.number_lock_enabled,
	};

	// TODO: Propagate this event somehow
}