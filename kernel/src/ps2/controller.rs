//! PS/2 controller

use core::hint::spin_loop;

/// Data I/O port of the PS/2 controller
const PS2_CTRL_DATA_PORT: u16 = 0x60;
/// I/O port for reading the PS/2 controller status register
const PS2_CTRL_READ_STATUS_PORT: u16 = 0x64;
/// I/O port for writing PS/2 controller commands
const PS2_CTRL_WRITE_CMD_PORT: u16 = 0x64;

/// PS/2 controller status mask for the output buffer full bit
const PS2_CTRL_STATUS_OUTPUT_FULL_MASK: u8 = 0x1;
/// PS/2 controller status mask for the input buffer full bit
const PS2_CTRL_STATUS_INPUT_FULL_MASK: u8 = 0x2;

/// PS/2 controller config mask for the first port interrupt enable bit
const PS2_CTRL_CONFIG_FIRST_INTERRUPT_ENABLE_MASK: u8 = 1 << 0;
/// PS/2 controller config mask for the second port interrupt enable bit
const PS2_CTRL_CONFIG_SECOND_INTERRUPT_ENABLE_MASK: u8 = 1 << 1;
/// PS/2 controller config mask for the second port clock disable bit
const PS2_CTRL_CONFIG_SECOND_PORT_CLOCK_DISABLE_MASK: u8 = 1 << 5;
/// PS/2 controller config mask for the first port translation enable bit
const PS2_CTRL_CONFIG_FIRST_PORT_TRANSLATE_MASK: u8 = 1 << 6;

/// The value returned by the PS/2 controller when the self-test passes
const PS2_CTRL_SELF_TEST_PASSED: u8 = 0x55;
/// The value returned by the PS/2 controller when a port's interface test passes
const PS2_CTRL_PORT_TEST_PASSED: u8 = 0x0;

/// The universal reset command that all PS/2 devices support
const PS2_DEVICE_RESET_CMD: u8 = 0xFF;

/// Timeout for receiving and sending PS/2 controller data
const PS2_TIMEOUT: usize = 0x30000000;

/// Possible commands for the PS/2 controller
#[repr(u8)]
enum PS2Command {
	EnableFirstPort = 0xAE,
	EnableSecondPort = 0xA8,
	DisableFirstPort = 0xAD,
	DisableSecondPort = 0xA7,
	ReadConfigByte = 0x20,
	WriteConfigByte = 0x60,
	SelfTest = 0xAA,
	FirstPortTest = 0xAB,
	SecondPortTest = 0xA9,
	WriteToSecondPort = 0xD4,
}

/// Initializes the PS/2 controller and tries to enable and reset both PS/2 ports
pub fn init() {
	// NOTE: OSDEV wiki says that USB controller init must happen before PS/2 init (and USB legacy
	// support should be disabled) or else it will interfere with PS/2 init
	// TODO: Determine if a 8042-compatible PS/2 controller exists using ACPI

	let mut first_port_avail = true;
	let mut second_port_avail = false;

	// PS/2 init sequence
	unsafe {

		// Initially disable both ports (if a second port does not exist disabling it is a NOP)
		send_command(PS2Command::DisableFirstPort);
		send_command(PS2Command::DisableSecondPort);

		// Flush the output buffer
		cpu::in8(PS2_CTRL_DATA_PORT);

		// Run controller self-test
		if send_command_with_response(PS2Command::SelfTest) != PS2_CTRL_SELF_TEST_PASSED {
			panic!("PS/2 Controller self-test failed!");
		}

		// Configure the controller for the init sequence: no interrupts for either port
		let mut ctrl_config_byte = send_command_with_response(PS2Command::ReadConfigByte);
		ctrl_config_byte &= !PS2_CTRL_CONFIG_FIRST_INTERRUPT_ENABLE_MASK;
		ctrl_config_byte &= !PS2_CTRL_CONFIG_SECOND_INTERRUPT_ENABLE_MASK;
		send_command_with_arg(PS2Command::WriteConfigByte, ctrl_config_byte);

		// To check if the controller has/supports a second port, we check the config bit which
		// is cleared/set by the enable/disable commands. Because the value of the bit is
		// unspecified on a controller which does not support a second port, we first check if the
		// bit is cleared while the second port is supposed to be disabled, and then check enable
		// the second port and check if the bit is set.
		if (ctrl_config_byte & PS2_CTRL_CONFIG_SECOND_PORT_CLOCK_DISABLE_MASK) == 0 {
			second_port_avail = false;
		} else {
			send_command(PS2Command::EnableSecondPort);
			let new_config_byte = send_command_with_response(PS2Command::ReadConfigByte);
			if (new_config_byte & PS2_CTRL_CONFIG_SECOND_PORT_CLOCK_DISABLE_MASK) != 0 {
				second_port_avail = false;
			}
			send_command(PS2Command::DisableSecondPort);
		}

		// Run interface test for the first port
		let first_port_test_result = send_command_with_response(PS2Command::FirstPortTest);
		if first_port_test_result != PS2_CTRL_PORT_TEST_PASSED {
			serial::println!("ERROR: PS/2 first port test failed with error code {:#x}!",
				first_port_test_result);
			first_port_avail = false;
		}

		// Run interface test for the second port
		if second_port_avail {
			let second_port_test_result = send_command_with_response(PS2Command::SecondPortTest);
			if second_port_test_result != PS2_CTRL_PORT_TEST_PASSED {
				serial::println!("ERROR: PS/2 second port test failed with error code {:#x}!",
					second_port_test_result);
				second_port_avail = false;
			}
		}

		// Configure devices (enabling interrupts and disabling legacy translation)
		let mut ctrl_config_byte = send_command_with_response(PS2Command::ReadConfigByte);
		if first_port_avail {
			ctrl_config_byte |= PS2_CTRL_CONFIG_FIRST_INTERRUPT_ENABLE_MASK;
			ctrl_config_byte &= !PS2_CTRL_CONFIG_FIRST_PORT_TRANSLATE_MASK;
		}
		if second_port_avail {
			ctrl_config_byte |= PS2_CTRL_CONFIG_SECOND_INTERRUPT_ENABLE_MASK;
		}
		send_command_with_arg(PS2Command::WriteConfigByte, ctrl_config_byte);
		
		// Enable and reset devices
		if first_port_avail {
			send_command(PS2Command::EnableFirstPort);
			
			send_data(PS2_DEVICE_RESET_CMD);
		}
		if second_port_avail {
			send_command(PS2Command::EnableSecondPort);

			send_data_to_second_port(PS2_DEVICE_RESET_CMD);
		}
	}

	serial::println!("Enabled PS/2 Controller [{}, {}]", first_port_avail, second_port_avail);
}

/// Sends a command the PS/2 controller
fn send_command(command: PS2Command)  {
	unsafe {
		cpu::out8(PS2_CTRL_WRITE_CMD_PORT, command as u8);
	}
}

/// Sends a command that takes an extra argument byte to the PS/2 controller
fn send_command_with_arg(command: PS2Command, arg: u8) {
	send_command(command);
	send_data(arg);
}

/// Sends a command to the PS/2 controller and waits for a response
fn send_command_with_response(command: PS2Command) -> u8 {
	send_command(command);
	receive_data()
}

/// Waits for and returns the value in the PS/2 controller's output buffer. Panics on timeout
pub fn receive_data() -> u8 {
	recieve_data_with_timeout().expect("Timeout in `receive_data()` of PS/2 controller")
}

/// Waits for and returns the value in the PS/2 controller's output buffer. Returns `None` on timeout
pub fn recieve_data_with_timeout() -> Option<u8> {
	let mut timeout = PS2_TIMEOUT;
	while (get_status_register() & PS2_CTRL_STATUS_OUTPUT_FULL_MASK) == 0 && timeout > 0 {
		timeout -= 1;
		spin_loop();
	}

	if timeout == 0 {
		return None;
	}

	unsafe {
		Some(cpu::in8(PS2_CTRL_DATA_PORT))
	}
}

/// Waits for and sends a value to the PS/2 controller's input buffer. Unless the PS/2 controller
/// expects an argument for a command, this is sent to the device connected to the first port
pub fn send_data(byte: u8) {
	let mut timeout = PS2_TIMEOUT;
	while (get_status_register() & PS2_CTRL_STATUS_INPUT_FULL_MASK != 0) && timeout > 0 {
		timeout -= 1;
		spin_loop();
	}

	if timeout == 0 {
		panic!("Timeout in `send_data({:#x})` of PS/2 controller", byte);
	}

	unsafe {
		cpu::out8(PS2_CTRL_DATA_PORT, byte);
	}
}

/// Waits for and sends a value to the device connected to the second port
pub fn send_data_to_second_port(byte: u8) {
	send_command_with_arg(PS2Command::WriteToSecondPort, byte);
}

/// Reads the PS/2 controller's status register
fn get_status_register() -> u8 {
	unsafe {
		cpu::in8(PS2_CTRL_READ_STATUS_PORT)
	}
}