//! PS/2 mouse driver

use exclusive_cell::ExclusiveCell;
use super::command_queue::{PS2CommandQueue, PS2Command};

/// Command acknowledged response
const MOUSE_MSG_ACK: u8 = 0xFA;
/// Self-test successful response
const MOUSE_MSG_SELF_TEST_PASSED: u8 = 0xAA;
/// Self-test failed response
const MOUSE_MSG_SELF_TEST_FAILED: u8 = 0xFC;
/// The initial device ID of a PS/2 mouse
const MOUSE_ID_STANDARD: u8 = 0x00;
/// The device ID of a PS/2 mouse which supports a scroll wheel
const MOUSE_ID_INTELLIMOUSE: u8 = 0x03;
/// The device ID of a PS/2 mouse which supports a scroll wheel and 5 buttons
const MOUSE_ID_INTELLIMOUSE_EXPLORER: u8 = 0x04;
/// Command to enable packet streaming of movement/presses
const MOUSE_CMD_ENABLE_STREAMING: u8 = 0xF4;
/// Command to set the mouse sample rate
const MOUSE_CMD_SET_SAMPLE_RATE: u8 = 0xF3;
/// Command to get the mouse device ID
const MOUSE_CMD_GET_MOUSE_ID: u8 = 0xF2;
/// Mouse sample rate
const MOUSE_SAMPLE_RATE: u8 = 10;

/// Mouse driver state-machine states
#[derive(Debug)]
enum PS2MouseState {
	Uninitialized,
	PassedSelfTest,
	TryInitScrollWheel,
	TryInit5Buttons,
	Initialized,
}

/// A PS/2 mouse driver
struct PS2MouseDriver {
	/// The state of the driver
	state: PS2MouseState,
	/// Command queue for command sequences
	command_queue: PS2CommandQueue,
	/// Whether or not the mouse has a scroll wheel
	supports_scroll_wheel: bool,
	/// Whether or not the mouse has two extra side buttons
	supports_5_buttons: bool,
	/// Accumalated packet data
	packet_data: [u8; 4],
	/// Amount of packet bytes accumualted
	packet_sequence: usize,
}

impl PS2MouseDriver {
	/// Construct an uninitialized mouse driver
	const fn new() -> Self {
		PS2MouseDriver {
			state: PS2MouseState::Uninitialized,
			command_queue: PS2CommandQueue::new(true),
			supports_scroll_wheel: false,
			supports_5_buttons: false,
			packet_data: [0; 4],
			packet_sequence: 0,
		}
	}

	/// Handle a mouse IRQ
	pub fn handle_interrupt(&mut self, mouse_message: u8) {
		// We first check if this is a response to a command we queued, and handle the response if
		// it is
		let queue_empty = self.command_queue.handle_message(mouse_message);

		// If there no commands on in the queue then we need to handle the message based on the
		// current state. On the other hand, if the receiving of this message acknowledged the last
		// command in the queue then we just finished a transition from one state to another and
		// need to take the relevant action for the new state.
		if queue_empty {
			match self.state {
				PS2MouseState::Uninitialized => {
					// If the mouse is uninitialized because we sent a `reset` command, it will first send
					// an ACK response. If it is uninitialized because it was just plugged in, it will not
					// send an ACK first, so we just discard an ACK if we see it.
					if mouse_message == MOUSE_MSG_ACK {
						return;
					}

					// We first expect a message with the result of the self-test
					if mouse_message == MOUSE_MSG_SELF_TEST_PASSED {
						self.state = PS2MouseState::PassedSelfTest;
					} else if mouse_message == MOUSE_MSG_SELF_TEST_FAILED {
						panic!("Mouse failed Basic Assurance Test, what should we do here?");
					} else {
						panic!("Unexpected mouse message before initialization");
					}
				},
				PS2MouseState::PassedSelfTest => {
					// We then expect a message with the mouse's device ID
					assert!(mouse_message == MOUSE_ID_STANDARD);

					// We send the "secret knock" to try and enable the scroll wheel
					self.command_queue.queue(PS2Command {
						command: MOUSE_CMD_SET_SAMPLE_RATE, data: Some(200)
					});
					self.command_queue.queue(PS2Command {
						command: MOUSE_CMD_SET_SAMPLE_RATE, data: Some(100)
					});
					self.command_queue.queue(PS2Command {
						command: MOUSE_CMD_SET_SAMPLE_RATE, data: Some(80)
					});
					// We then inquire about the device ID, which should change to reflect the
					// scroll wheel being enabled
					self.command_queue.queue(PS2Command {
						command: MOUSE_CMD_GET_MOUSE_ID, data: None
					});

					self.state = PS2MouseState::TryInitScrollWheel;
				},
				PS2MouseState::TryInitScrollWheel => {
					let device_id = super::controller::receive_data();
					if device_id == MOUSE_ID_STANDARD {
						// If the device ID did not change, the mouse does not have a scroll wheel
						// We set the sample rate and then enable packet streaming
						self.command_queue.queue(PS2Command {
							command: MOUSE_CMD_SET_SAMPLE_RATE, data: Some(MOUSE_SAMPLE_RATE)
						});
						self.command_queue.queue(PS2Command {
							command: MOUSE_CMD_ENABLE_STREAMING, data: None
						});
						self.state = PS2MouseState::Initialized;
					} else if device_id == MOUSE_ID_INTELLIMOUSE {
						// The device ID changed, so scroll wheel is now enabled
						self.supports_scroll_wheel = true;

						// We send the "secret knock" to try and enable the two extra buttons
						self.command_queue.queue(PS2Command {
							command: MOUSE_CMD_SET_SAMPLE_RATE, data: Some(200)
						});
						self.command_queue.queue(PS2Command {
							command: MOUSE_CMD_SET_SAMPLE_RATE, data: Some(200)
						});
						self.command_queue.queue(PS2Command {
							command: MOUSE_CMD_SET_SAMPLE_RATE, data: Some(80)
						});
						// We then inquire about the device ID, which should change to reflect the
						// buttons being enabled
						self.command_queue.queue(PS2Command {
							command: MOUSE_CMD_GET_MOUSE_ID, data: None
						});
	
						self.state = PS2MouseState::TryInit5Buttons;
					} else {
						panic!("Unrecognized mouse device id {:#X}", mouse_message);
					}
				},
				PS2MouseState::TryInit5Buttons => {
					let device_id = super::controller::receive_data();
					if device_id == MOUSE_ID_INTELLIMOUSE_EXPLORER {
						// The device ID changed, so the buttons are now enabled
						self.supports_5_buttons = true;
					} else if device_id != MOUSE_ID_INTELLIMOUSE {
						panic!("Unrecognized mouse device id {:#X}", mouse_message);
					}

					// We set the sample rate and then enable packet streaming
					self.command_queue.queue(PS2Command {
						command: MOUSE_CMD_SET_SAMPLE_RATE, data: Some(MOUSE_SAMPLE_RATE)
					});
					self.command_queue.queue(PS2Command {
						command: MOUSE_CMD_ENABLE_STREAMING, data: None
					});
					self.state = PS2MouseState::Initialized;
				},
				PS2MouseState::Initialized => {
					// We recieve an ack for the initial enable streaming command which we ignore
					if mouse_message == MOUSE_MSG_ACK {
						return;
					}

					// FIXME: It seems that for some reason the mouse packets seem to get out of
					// sync sometimes. We need to figure out why that happens. Currently, we use the
					// fact that the fourth bit in the first packet byte is always zero, to try and
					// re-sync.
					if self.packet_sequence == 0 && (mouse_message & 0x8) == 0 {
						return;
					}

					// Record the byte of the packet
					self.packet_data[self.packet_sequence] = mouse_message;
					self.packet_sequence += 1;

					// If we recieved the entire packet, dispatch it and restart the sequence
					if (!self.supports_scroll_wheel && self.packet_sequence == 3)
						|| self.packet_sequence == 4 {
						self.dispatch_packet();
						self.packet_sequence = 0;
					}
				},
			}
		}
	}

	/// Dispatched a mouse packet based on the packet bytes stored in `self.packet_sequence`
	fn dispatch_packet(&self) {
		// A mouse packet is 3 bytes long, or 4 bytes long if the scroll wheel has been enabled.
		// 1st byte: y_overflow | x_overflow | y_sign | x_sign | 0 | middle | right | left
		// 2nd byte: x delta magnitude
		// 3rd byte: y delta magnitude
		// If only the scroll wheel was enabled, the 4th byte is the z delta
		// If five buttons have been enabled, the 4th byte is: 0 | 0 | fifth | fourth | z delta (4 bits)

		let left_down = self.packet_data[0] & 0x1 != 0;
		let right_down = self.packet_data[0] & 0x2 != 0;
		let middle_down = self.packet_data[0] & 0x4 != 0;
		let x_sign = self.packet_data[0] & 0x10 != 0;
		let y_sign = self.packet_data[0] & 0x20 != 0;
		let x_overflow = self.packet_data[0] & 0x40 != 0;
		let y_overflow = self.packet_data[0] & 0x80 != 0;
		let x_unsigned_delta = self.packet_data[1] as u32;
		let y_unsigned_delta = self.packet_data[2] as u32;

		// Combine the sign, and sign-extend
		let mut x_delta = (x_unsigned_delta | (if x_sign { 0xFFFFFF00 } else { 0x0 })) as i32;
		let mut y_delta = (y_unsigned_delta | (if y_sign { 0xFFFFFF00 } else { 0x0 })) as i32;

		// Discard the x/y movement if the values overflowed (and are thus meaningless)
		if x_overflow || y_overflow {
			x_delta = 0;
			y_delta = 0;
		}

		if self.supports_scroll_wheel {
			let extended_data = self.packet_data[3];
			if self.supports_5_buttons {
				// Sign extend the z delta
				let z_delta = (((extended_data & 0xF) as i32) << 28) >> 28;
				
				let fourth_down = extended_data & 0x10 != 0;
				let fifth_down = extended_data & 0x20 != 0;
				crate::mouse::mouse_event(left_down, right_down, middle_down, fourth_down,
					fifth_down, x_delta, y_delta, z_delta);
			} else {
				let z_delta = extended_data as i8 as i32;
				crate::mouse::mouse_event(left_down, right_down, middle_down, false, false, x_delta,
					y_delta, z_delta);
			}
		} else {
			crate::mouse::mouse_event(left_down, right_down, middle_down, false, false, x_delta,
				y_delta, 0);
		}
	}
}

/// The current mouse state. We should only get one mouse interrupt at a time, so exclusivity
/// is inherent.
static MOUSE_DRIVER: ExclusiveCell<PS2MouseDriver> = ExclusiveCell::new(PS2MouseDriver::new());

/// Handles an interrupt from the PS/2 mouse (should only be called when an interrupt happens)
pub fn handle_interrupt(mouse_message: u8) {
	MOUSE_DRIVER.acquire().handle_interrupt(mouse_message);
}