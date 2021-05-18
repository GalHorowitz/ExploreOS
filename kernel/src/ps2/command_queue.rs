//! PS/2 command queues

const PS2_MSG_ACK: u8 = 0xFA;
const PS2_MSG_RESEND: u8 = 0xFE;
/// Maximum amount of command retries when receiving a RESEND response
const MAX_COMMAND_RETRIES: usize = 3;

#[derive(Clone, Copy, Debug)]
pub struct PS2Command {
	pub command: u8,
	pub data: Option<u8>
}

/// A struct that handles a queue of commands to send to a PS/2 device
pub struct PS2CommandQueue {
	/// Command queue for sending and resending commands as needed
	queue: [PS2Command; 5],
	/// Number of commands in the queue
	queue_length: usize,
	/// Number of retries of the current queued command
	command_retries: usize,
	/// Whether or not we are waiting for an ACK of the command's data byte
	waiting_for_data_ack: bool,
	/// Whether the command should be send to the second port of the first port
	second_port: bool,
}

impl PS2CommandQueue {
	pub const fn new(second_port: bool) -> Self {
		PS2CommandQueue {
			queue: [PS2Command {command: 0, data: None }; 5],
			queue_length: 0,
			command_retries: 0,
			waiting_for_data_ack: false,
			second_port
		}
	}

	/// Queues the specified command and dispatches it immediately if it is the first in the queue
	pub fn queue(&mut self, command: impl Into<PS2Command>) {
		let command: PS2Command = command.into();

		// Assert we have enough space left in the queue
		assert!(self.queue_length < self.queue.len());

		// Append the command to the end of the queue and update the queue length
		self.queue[self.queue_length] = command;
		self.queue_length += 1;

		// If this is the first command in the queue we can dispatch it immediately
		if self.queue_length == 1 {
			self.send_command_to_device(command);
		}
	}

	/// Uses the provided keyboard message to update the command queue. Returns true if the queue is
	/// empty after the message is handled
	pub fn handle_message(&mut self, message: u8) -> bool {
		// If no commands are queued this is not a response to a queued command
		if self.queue_length == 0 {
			return true;
		}

		if message == PS2_MSG_RESEND {
			// If this is a RESEND message, we retry the first command in the queue a few times
			if self.command_retries < MAX_COMMAND_RETRIES {
				self.command_retries += 1;
				self.send_command_to_device(self.queue[0]);
			} else {
				panic!("[PS2CommandQueue]: Failed to send command {:?} (Too many retries)",	self.queue[0]);
			}
		} else if message == PS2_MSG_ACK {
			// If this is an acknowledge message, we first check if the command is also expect an
			// ACK for the its data byte, in which case we need to discard the first ACK
			if self.waiting_for_data_ack {
				self.waiting_for_data_ack = false;
				return false;
			}

			// We reset the retry counter for the next command
			self.command_retries = 0;

			// We pop the first element in the queue by shifting all elements back one place
			for i in 1..self.queue_length {
				self.queue[i-1] = self.queue[i];
			}

			// We decrement the queue length
			self.queue_length -= 1;

			// If the queue is not empty, we dispatch the next command
			if self.queue_length > 0 {
				self.send_command_to_device(self.queue[0]);
			}
		} else {
			// If the queue is not empty, but the message we received is not an ACK or a RESEND, the
			// command has a response byte which is discarded. This shouldn't happen(?)
			panic!("[PS2CommandQueue] Discarded command result {:#X}", message);
		}

		self.queue_length == 0
	}
	
	/// Sends the specified command to the keyboard
	fn send_command_to_device(&mut self, command: PS2Command) {
		// We first send the command byte
		if self.second_port {
			super::controller::send_data_to_second_port(command.command);
		}else{
			super::controller::send_data(command.command);
		}

		if let Some(data_byte) = command.data {
			// If the command also has a data byte, we send it as well and remember we need to
			// ignore the first ACK because the keyboard will also ACK the data byte
			if self.second_port {
				super::controller::send_data_to_second_port(data_byte);
			}else{
				super::controller::send_data(data_byte);
			}
			self.waiting_for_data_ack = true;
		}
	}
}