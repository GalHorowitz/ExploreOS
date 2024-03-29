//! PS/2 keyboard driver

use exclusive_cell::ExclusiveCell;
use crate::keyboard::KeyCode;
use crate::println;
use super::command_queue::{PS2Command, PS2CommandQueue};

/// Whether or not to print driver debug mesages
const PRINT_DEBUG_MESSAGES: bool = false;

/// The delay until the key starts repeating when a key is pressed down. Values ranges from 0 to 3
/// which maps to 250ms to 1000ms respectively
const TYPEMATIC_REPEAT_DELAY: u8 = 1;
/// The key repeat rate when a key is pressed down. Value ranges from 0 to 31 which maps to 30Hz to
/// 2Hz respectively
const TYPEMATIC_REPEAT_RATE: u8 = 20;

/// Command acknowledged keyboard response
const KEYBOARD_MSG_ACK: u8 = 0xFA;
//// Self-test passed keyboard response
const KEYBOARD_MSG_SELF_TEST_PASSED: u8 = 0xAA;
/// Message sent before a scan code to indicate the next key is an extended scan code
const KEYBOARD_MSG_EXTENDED_KEY: u8 = 0xE0;
/// Message sent before a scan code to indicate the next key is released (default is pressed)
const KEYBOARD_MSG_RELEASED_KEY: u8 = 0xF0;

/// The multi-byte scan code that represents a PrtScn press
const PRINT_SCREEN_PRESSED_MULTIBYTE_SCANCODE: [u8; 3] = [0x12, 0xE0, 0x7C];
/// The multi-byte scan code that represents a PrtScn release
const PRINT_SCREEN_RELEASED_MULTIBYTE_SCANCODE: [u8; 4] = [0x7C, 0xE0, 0xF0, 0x12];
/// The multi-byte scan code that represents a Pause press (and immediate release)
const PAUSE_PRESSED_MULTIBYTE_SCANCODE: [u8; 8] = [0xE1, 0x14, 0x77, 0xE1, 0xF0, 0x14, 0xF0, 0x77];

/// Supported keyboard commands
#[derive(Clone, Copy, Debug, PartialEq)]
enum PS2KeyboardCommand {
	IdentifyKeyboard,
	DisableScanning,
	EnableScanning,
	SetScanCodeSet(u8),
	SetLEDs { scroll_lock: bool, number_lock: bool, caps_lock: bool },
	SetTypematicByte{ delay: u8, rate: u8 },
}

impl From<PS2KeyboardCommand> for PS2Command {
	fn from(command: PS2KeyboardCommand) -> PS2Command {
        match command {
			PS2KeyboardCommand::IdentifyKeyboard =>	PS2Command { command: 0xF2, data: None },
			PS2KeyboardCommand::DisableScanning => PS2Command { command: 0xF5, data: None },
			PS2KeyboardCommand::EnableScanning => PS2Command { command: 0xF4, data: None },
			PS2KeyboardCommand::SetScanCodeSet(set) => PS2Command { command: 0xF0, data: Some(set) },
			PS2KeyboardCommand::SetTypematicByte { delay, rate } => PS2Command {
				command: 0xF3,
				data: Some((delay << 5) | rate)
			},
			PS2KeyboardCommand::SetLEDs { scroll_lock, number_lock, caps_lock } => PS2Command {
				command: 0xED,
				data: Some(((caps_lock as u8) << 2) | ((number_lock as u8) << 1) | (scroll_lock as u8))
			},
		}
    }
}

/// Keyboard driver state-machine states
#[derive(Debug)]
enum PS2KeyboardState {
	Uninitialized,
	SelfTest,
	Identifying,
	Initialized,
	ScanningKey,
	ScanningExtendedKey,
	ScanningReleasedKey,
	ScanningReleasedExtendedKey,
	ScanningPrintScreenPressedMultibyte(u8),
	ScanningPrintScreenReleasedMultibyte(u8),
	ScanningPausePressedMultibyte(u8),
}

/// A PS/2 keyboard driver
struct PS2KeyboardDriver {
	/// The current state of the keyboard
	state: PS2KeyboardState,
	/// Command queue for command sequences
	command_queue: PS2CommandQueue,
}

impl PS2KeyboardDriver {
	/// Construct an uninitialized keyboard driver
	const fn new() -> Self {
		PS2KeyboardDriver {
			state: PS2KeyboardState::Uninitialized,
			command_queue: PS2CommandQueue::new(false)
		}
	}

	/// Handle a keyboard IRQ
	pub fn handle_interrupt(&mut self, keyboard_message: u8) {
		// Get the keyboard message from the PS/2 controller
		if PRINT_DEBUG_MESSAGES {
			println!("[PS2Keyboard({:?})] recieved message: {:#X}", self.state, keyboard_message);
		}

		// We first check if this is a response to a command we queued, and handle the response if
		// it is
		let queue_empty = self.command_queue.handle_message(keyboard_message);

		// If there no commands on in the queue then we need to handle the message based on the
		// current state. On the other hand, if the receiving of this message acknowledged the last
		// command in the queue then we just finished a transition from one state to another and
		// need to take the relevant action for the new state.
		if queue_empty {
			match self.state {
				PS2KeyboardState::Uninitialized => {
					// We are in this state following a keyboard reset command which did not go
					// through the command queue, so we assert the reset command was acknoweldged
					// and then transition to the self-test state (which is done following a reset)
					assert!(keyboard_message == KEYBOARD_MSG_ACK);
					self.state = PS2KeyboardState::SelfTest;
				},
				PS2KeyboardState::SelfTest => {
					assert!(keyboard_message == KEYBOARD_MSG_SELF_TEST_PASSED);
					// We then begin identifying the keyboard by first disabling scanning so it
					// won't interfere with the identification result and sending the identify cmd
					self.command_queue.queue(PS2KeyboardCommand::DisableScanning);
					self.command_queue.queue(PS2KeyboardCommand::IdentifyKeyboard);
					self.state = PS2KeyboardState::Identifying;
				},
				PS2KeyboardState::Identifying => {
					// The keyboard type could be 0 bytes, 1 byte, or 2 bytes, and the only way to
					// know is by trying to get another byte and timing out.
					let mut keyboard_type = [0u8; 2];
					let mut type_length = 0;
					if let Some(type_a) = super::controller::recieve_data_with_timeout() {
						keyboard_type[0] = type_a;
						type_length = 1;
						if let Some(type_b) = super::controller::recieve_data_with_timeout() {
							keyboard_type[1] = type_b;
							type_length = 2;
						}
					}

					if PRINT_DEBUG_MESSAGES {
						println!("[PS2Keyboard({:?})] Type: {:X?}", self.state,
							&keyboard_type[..type_length]);
					}
					
					// We expect an MF2 keyboard with no translation
					assert!(type_length == 2 && keyboard_type == [0xAB, 0x83]);

					// We then initialize the keyboard, by first setting the scan code set to set 2
					self.command_queue.queue(PS2KeyboardCommand::SetScanCodeSet(2));
					// We then set the typematic byte to some defaults
					self.command_queue.queue(PS2KeyboardCommand::SetTypematicByte {
						delay: TYPEMATIC_REPEAT_DELAY,
						rate: TYPEMATIC_REPEAT_RATE
					});
					// We then set the keyboard LEDs to a known state
					self.command_queue.queue(PS2KeyboardCommand::SetLEDs {
						number_lock: true,
						caps_lock: false,
						scroll_lock: false,
					});
					// And finally we re-enable scanning
					self.command_queue.queue(PS2KeyboardCommand::EnableScanning);
					self.state = PS2KeyboardState::Initialized;
				},
				PS2KeyboardState::Initialized => {
					// When we get to this state all the initialization commands have been completed
					// and we can finally switch to the final `Scanning` state in which we wait for
					// scan codes.
					if PRINT_DEBUG_MESSAGES {
						println!("[PS2Keyboard({:?})] PS/2 Keyboard initialized!", self.state);
					}
					self.state = PS2KeyboardState::ScanningKey;
				},
				PS2KeyboardState::ScanningKey => {
					if keyboard_message == KEYBOARD_MSG_EXTENDED_KEY {
						self.state = PS2KeyboardState::ScanningExtendedKey;
					} else if keyboard_message == KEYBOARD_MSG_RELEASED_KEY {
						self.state = PS2KeyboardState::ScanningReleasedKey;
					} else if keyboard_message == PAUSE_PRESSED_MULTIBYTE_SCANCODE[0] {
						self.state = PS2KeyboardState::ScanningPausePressedMultibyte(1);
					} else {
						// If this is not a message with a special meaning, it is just a simple scan
						// code, so we translate it to a key code and notify the keyboard state 
						let key_code = simple_scancode_to_keycode(keyboard_message);
						crate::keyboard::key_pressed_event(key_code);
					}
				},
				PS2KeyboardState::ScanningExtendedKey => {
					if keyboard_message == KEYBOARD_MSG_RELEASED_KEY {
						self.state = PS2KeyboardState::ScanningReleasedExtendedKey;
					} else if keyboard_message == PRINT_SCREEN_PRESSED_MULTIBYTE_SCANCODE[0] {
						self.state = PS2KeyboardState::ScanningPrintScreenPressedMultibyte(1);
					} else {
						// If this is not a message with a special meaning, it is just an extended
						// scan code, so we translate it to a key code and notify the keyboard state
						let key_code = extended_scancode_to_keycode(keyboard_message);
						crate::keyboard::key_pressed_event(key_code);
						self.state = PS2KeyboardState::ScanningKey;
					}
				},
				PS2KeyboardState::ScanningReleasedKey => {
					// We translate the scan code and notify the keyboard of the release
					let key_code = simple_scancode_to_keycode(keyboard_message);
					crate::keyboard::key_released_event(key_code);
					self.state = PS2KeyboardState::ScanningKey;
				},
				PS2KeyboardState::ScanningReleasedExtendedKey => {
					if keyboard_message == PRINT_SCREEN_RELEASED_MULTIBYTE_SCANCODE[0] {
						self.state = PS2KeyboardState::ScanningPrintScreenReleasedMultibyte(1);
					} else {
						// If this is not a message with a special meaning, it is just an extended
						// scan code, so we translate it to a key code and notify the keyboard state 
						let key_code = extended_scancode_to_keycode(keyboard_message);
						crate::keyboard::key_released_event(key_code);
						self.state = PS2KeyboardState::ScanningKey;
					}
				},
				PS2KeyboardState::ScanningPrintScreenPressedMultibyte(byte_idx) => {
					let byte_idx = byte_idx as usize;
					// We make sure the keyboard message is the expected next byte in the sequence
					if keyboard_message == PRINT_SCREEN_PRESSED_MULTIBYTE_SCANCODE[byte_idx] {
						if byte_idx < PRINT_SCREEN_PRESSED_MULTIBYTE_SCANCODE.len() - 1 {
							// If there are still bytes left in the sequence, we transition to the
							// next step in the sequence and wait for the next message
							self.state = PS2KeyboardState::ScanningPrintScreenPressedMultibyte(
								(byte_idx + 1) as u8);
						} else {
							// If this was the last byte in the sequence, then we successfully
							// scanned the key, and we notify the keyboard state
							crate::keyboard::key_pressed_event(KeyCode::KeyPrintScreen);
							self.state = PS2KeyboardState::ScanningKey;
						}
					} else {
						// If we did not get the byte that we expected, this is an unknown scan code
						crate::keyboard::key_pressed_event(KeyCode::Unknown);
						self.state = PS2KeyboardState::ScanningKey;
					}
				},
				PS2KeyboardState::ScanningPrintScreenReleasedMultibyte(byte_idx) => {
					let byte_idx = byte_idx as usize;
					// We make sure the keyboard message is the expected next byte in the sequence
					if keyboard_message == PRINT_SCREEN_RELEASED_MULTIBYTE_SCANCODE[byte_idx] {
						if byte_idx < PRINT_SCREEN_RELEASED_MULTIBYTE_SCANCODE.len() - 1 {
							// If there are still bytes left in the sequence, we transition to the
							// next step in the sequence and wait for the next message
							self.state = PS2KeyboardState::ScanningPrintScreenReleasedMultibyte(
								(byte_idx + 1) as u8);
						} else {
							// If this was the last byte in the sequence, then we successfully
							// scanned the key, and we notify the keyboard state
							crate::keyboard::key_released_event(KeyCode::KeyPrintScreen);
							self.state = PS2KeyboardState::ScanningKey;
						}
					} else {
						// If we did not get the byte that we expected, this is an unknown scan code
						crate::keyboard::key_released_event(KeyCode::Unknown);
						self.state = PS2KeyboardState::ScanningKey;
					}
				},
				PS2KeyboardState::ScanningPausePressedMultibyte(byte_idx) => {
					let byte_idx = byte_idx as usize;
					// We make sure the keyboard message is the expected next byte in the sequence
					if keyboard_message == PAUSE_PRESSED_MULTIBYTE_SCANCODE[byte_idx] {
						if byte_idx < PAUSE_PRESSED_MULTIBYTE_SCANCODE.len() - 1 {
							self.state = PS2KeyboardState::ScanningPausePressedMultibyte(
								(byte_idx + 1) as u8);
						} else {
							// If this was the last byte in the sequence, then we successfully
							// scanned the key, and we notify the keyboard state. A pause key does
							// not have different press/release scan codes, and instead acts as if
							// the key was immediately released after pressing
							crate::keyboard::key_pressed_event(KeyCode::KeyPause);
							crate::keyboard::key_released_event(KeyCode::KeyPause);
							self.state = PS2KeyboardState::ScanningKey;
						}
					} else {
						// If we did not get the byte that we expected, this is an unknown scan code
						crate::keyboard::key_pressed_event(KeyCode::Unknown);
						self.state = PS2KeyboardState::ScanningKey;
					}
				},
			}
		}
	}
}

/// The current keyboard state. We should only get one keyboard interrupt at a time, so exclusivity
/// is inherent.
static KEYBOARD_DRIVER: ExclusiveCell<PS2KeyboardDriver> = ExclusiveCell::new(PS2KeyboardDriver::new());

/// Handles an interrupt from the PS/2 keyboard (should only be called when an interrupt happens)
pub fn handle_interrupt(keyboard_message: u8) {
	KEYBOARD_DRIVER.acquire().handle_interrupt(keyboard_message);
}

/// Converts a simple 1-byte set 2 scan code to the corresponding key code
fn simple_scancode_to_keycode(scan_code: u8) -> KeyCode {
	match scan_code {
		0x01 => KeyCode::KeyF9,
		0x03 => KeyCode::KeyF5,
		0x04 => KeyCode::KeyF3,
		0x05 => KeyCode::KeyF1,
		0x06 => KeyCode::KeyF2,
		0x07 => KeyCode::KeyF12,
		0x09 => KeyCode::KeyF10,
		0x0A => KeyCode::KeyF8,
		0x0B => KeyCode::KeyF6,
		0x0C => KeyCode::KeyF4,
		0x0D => KeyCode::KeyTab,
		0x0E => KeyCode::KeyBackTick,
		0x11 => KeyCode::KeyLeftAlt,
		0x12 => KeyCode::KeyLeftShift,
		0x14 => KeyCode::KeyLeftControl,
		0x15 => KeyCode::KeyQ,
		0x16 => KeyCode::Key1,
		0x1A => KeyCode::KeyZ,
		0x1B => KeyCode::KeyS,
		0x1C => KeyCode::KeyA,
		0x1D => KeyCode::KeyW,
		0x1E => KeyCode::Key2,
		0x21 => KeyCode::KeyC,
		0x22 => KeyCode::KeyX,
		0x23 => KeyCode::KeyD,
		0x24 => KeyCode::KeyE,
		0x25 => KeyCode::Key4,
		0x26 => KeyCode::Key3,
		0x29 => KeyCode::KeySpace,
		0x2A => KeyCode::KeyV,
		0x2B => KeyCode::KeyF,
		0x2C => KeyCode::KeyT,
		0x2D => KeyCode::KeyR,
		0x2E => KeyCode::Key5,
		0x31 => KeyCode::KeyN,
		0x32 => KeyCode::KeyB,
		0x33 => KeyCode::KeyH,
		0x34 => KeyCode::KeyG,
		0x35 => KeyCode::KeyY,
		0x36 => KeyCode::Key6,
		0x3A => KeyCode::KeyM,
		0x3B => KeyCode::KeyJ,
		0x3C => KeyCode::KeyU,
		0x3D => KeyCode::Key7,
		0x3E => KeyCode::Key8,
		0x41 => KeyCode::KeyComma,
		0x42 => KeyCode::KeyK,
		0x43 => KeyCode::KeyI,
		0x44 => KeyCode::KeyO,
		0x45 => KeyCode::Key0,
		0x46 => KeyCode::Key9,
		0x49 => KeyCode::KeyPeriod,
		0x4A => KeyCode::KeySlash,
		0x4B => KeyCode::KeyL,
		0x4C => KeyCode::KeySemicolon,
		0x4D => KeyCode::KeyP,
		0x4E => KeyCode::KeyMinus,
		0x52 => KeyCode::KeyApostrophe,
		0x54 => KeyCode::KeyLeftSquareBracket,
		0x55 => KeyCode::KeyEquals,
		0x58 => KeyCode::KeyCapsLock,
		0x59 => KeyCode::KeyRightShift,
		0x5A => KeyCode::KeyEnter,
		0x5B => KeyCode::KeyRightSquareBracket,
		0x5D => KeyCode::KeyBackSlash,
		0x61 => KeyCode::KeyExtraBackSlash,
		0x66 => KeyCode::KeyBackspace,
		0x69 => KeyCode::KeyNumpad1,
		0x6B => KeyCode::KeyNumpad4,
		0x6C => KeyCode::KeyNumpad7,
		0x70 => KeyCode::KeyNumpad0,
		0x71 => KeyCode::KeyNumpadPeriod,
		0x72 => KeyCode::KeyNumpad2,
		0x73 => KeyCode::KeyNumpad5,
		0x74 => KeyCode::KeyNumpad6,
		0x75 => KeyCode::KeyNumpad8,
		0x76 => KeyCode::KeyEscape,
		0x77 => KeyCode::KeyNumberLock,
		0x78 => KeyCode::KeyF11,
		0x79 => KeyCode::KeyNumpadPlus,
		0x7A => KeyCode::KeyNumpad3,
		0x7B => KeyCode::KeyNumpadMinus,
		0x7C => KeyCode::KeyNumpadAsterisk,
		0x7D => KeyCode::KeyNumpad9,
		0x7E => KeyCode::KeyScrollLock,
		0x83 => KeyCode::KeyF7,
		_ => KeyCode::Unknown,
	}	
}

/// Converts an extended set 2 scan code to the corresponding key code
fn extended_scancode_to_keycode(scan_code: u8) -> KeyCode {
	match scan_code {
		0x10 => KeyCode::KeyMultimediaSearch,
		0x11 => KeyCode::KeyRightAlt,
		0x14 => KeyCode::KeyRightControl,
		0x15 => KeyCode::KeyMultimediaPreviousTrack,
		0x18 => KeyCode::KeyMultimediaFavorites,
		0x1F => KeyCode::KeyLeftLogo,
		0x20 => KeyCode::KeyMultimediaRefresh,
		0x21 => KeyCode::KeyMultimediaVolumeDown,
		0x23 => KeyCode::KeyMultimediaMute,
		0x27 => KeyCode::KeyRightLogo,
		0x28 => KeyCode::KeyMultimediaWebStop,
		0x2B => KeyCode::KeyMultimediaCalculator,
		0x2F => KeyCode::KeyMenu,
		0x30 => KeyCode::KeyMultimediaWebForward,
		0x32 => KeyCode::KeyMultimediaVolumeUp,
		0x34 => KeyCode::KeyMultimediaPlayPause,
		0x37 => KeyCode::KeyACPIPower,
		0x38 => KeyCode::KeyMultimediaWebBack,
		0x3A => KeyCode::KeyMultimediaWebHome,
		0x3B => KeyCode::KeyMultimediaStop,
		0x3F => KeyCode::KeyACPISleep,
		0x40 => KeyCode::KeyMultimediaMyComputer,
		0x48 => KeyCode::KeyMultimediaEmail,
		0x4A => KeyCode::KeyNumpadSlash,
		0x4D => KeyCode::KeyMultimediaNextTrack,
		0x50 => KeyCode::KeyMultimediaMediaSelect,
		0x5A => KeyCode::KeyNumpadEnter,
		0x5E => KeyCode::KeyACPIWake,
		0x69 => KeyCode::KeyEnd,
		0x6B => KeyCode::KeyLeftArrow,
		0x6C => KeyCode::KeyHome,
		0x70 => KeyCode::KeyInsert,
		0x71 => KeyCode::KeyDelete,
		0x72 => KeyCode::KeyDownArrow,
		0x74 => KeyCode::KeyRightArrow,
		0x75 => KeyCode::KeyUpArrow,
		0x7A => KeyCode::KeyPageDown,
		0x7D => KeyCode::KeyPageUp,
		_ => KeyCode::Unknown,
	}
}