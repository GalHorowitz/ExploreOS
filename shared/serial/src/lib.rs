//! Basic UART serial driver

#![no_std]

use lock_cell::LockCell;

/// Global state for serial ports.
/// IMPORTANT: While maskable hardware interrupts are masked while this lock is held, care must be
/// taken to not use the lock in non-maskable interrupts like NMIs and exceptions.
pub static SERIAL: LockCell<Option<SerialPort>> = LockCell::new(None);

/// A collection of 4 serial ports. These are the 4 serial ports identified by the BIOS, i.e. these
/// are COM1-COM4 in the BDA.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct SerialPort {
    ports: [Option<u16>; 4]
}

/// Initializes all available serial ports with 115200 baud, 8n1.
pub fn init() {
    let mut serial = SERIAL.lock();
    assert!(serial.is_none());
    *serial = unsafe { Some(SerialPort::new()) };
}

pub fn init_with_ports(serial_port: SerialPort) {
    let mut serial = SERIAL.lock();
    assert!(serial.is_none());
    *serial = Some(serial_port);
}

impl SerialPort {
    /// Initializes all available serial ports with 115200 baud, 8n1.
    /// This function is unsafe because it relies on two unverified assumptions: that this function
    /// is only called once, and that address 0x400 is identity mapped such that the BDA can be
    /// accessed
    unsafe fn new() -> Self {
        // Initially mark all ports as not present
        let mut ports = [None; 4];

        for com_id in 0..4 {
            // Get the COM IO port address from the BIOS data area (BDA)
            let com_port = *(0x400 as *const u16).offset(com_id);
    
            // If the COM port is not present (zero), check the next one
            if com_port == 0 {
                continue;
            }
    
            // Serial port initialization sequence

            // Disable all serial interrupts
            cpu::out8(com_port + 1, 0x0);

            // Enable DLAB to set baud rate divisor
            cpu::out8(com_port + 3, 0x80);

            // We set the baud rate divisor to 1, i.e. the baud rate is 115200
            // Set baud rate divisor low byte
            cpu::out8(com_port, 0x1);
            // Set baud rate divisor high byte
            cpu::out8(com_port + 1, 0x0);

            // Disable DLAB, set mode to 8 bits, no parity, 1 stop bit
            cpu::out8(com_port + 3, 0x3);

            // Disable FIFO
            cpu::out8(com_port + 2, 0x0);

            // Set DTR/RTS (Data Terminal Ready, Request to Send)
            cpu::out8(com_port + 4, 0x3);
    
            // Store the port address
            ports[com_id as usize] = Some(com_port);
        }

        SerialPort { ports }
    }

    /// Writes `message` to all present serial ports. This function blocks until it can write all bytes.
    pub fn write(&mut self, message: &str) {
        // Broadcast each byte to all present ports
        for byte in message.bytes() {
            for port in 0..self.ports.len() {
                // For every port, check if it is present
                if let Some(com_port) = self.ports[port] {
                    // If the port is present, write the byte to the port
                    unsafe { self.write_byte(com_port, byte); }
                }
            }
        }
    }

    /// Writes `byte` to `com_port`. This is only used internally and assumes the serial lock is held.
    unsafe fn write_byte(&mut self, com_port: u16, byte: u8) {
        // Altough the mutability of the self reference is not required, this ensures that the ports
        // aren't written to from multiple threads at once

        // Some serial consoles expect a CRLF to move to the start of the next line, so if we encounter
        // a LF we can just prepend a CR.
        if byte == b'\n' {
            self.write_byte(com_port, b'\r');
        }
    
        // Wait until we can transmit
        while cpu::in8(com_port + 5) & 0x20 == 0 {
            core::hint::spin_loop();
        }
        // Write the character to the serial port
        cpu::out8(com_port, byte);
    }

}

/// Dummy struct to implement `core::fmt::Write` on
pub struct SerialWriter;

impl core::fmt::Write for SerialWriter {
    fn write_str(&mut self, msg: &str) -> core::fmt::Result {
		// Grab serial lock
		let mut serial = SERIAL.lock();
		if serial.is_some() {
			// If serial is initialized, write the message
			serial.as_mut().unwrap().write(msg);
		}
		
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            let _ = write!($crate::SerialWriter, $($arg)*);
        }
    };
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            let _ = writeln!($crate::SerialWriter, $($arg)*);
        }
    };
}