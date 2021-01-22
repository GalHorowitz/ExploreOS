//! Basic UART serial driver

#![no_std]

use lock_cell::LockCell;
use core::sync::atomic::spin_loop_hint;

/// A collection of 4 serial ports. These are the 4 serial ports identified by the BIOS, i.e. these
/// are COM1-COM4 in the BDA.
struct SerialPort {
    ports: [Option<u16>; 4]
}

// Global state for serial ports.
// FIXME: This uses a spin-lock which doesn't disable interrupts. If we want to use this in
// interrupts we must switch to different lock or we could dead-lock.
static SERIAL: LockCell<SerialPort> = LockCell::new(SerialPort {
    ports: [None; 4]
});

/// Initializes all available serial port with 115200 baud, 8n1.
pub fn init() {
    let mut serial = SERIAL.lock();

    // Initially mark all ports as not present
    serial.ports = [None; 4];

    for com_id in 0..4 {
        // Get the COM IO port address from the BIOS data area (BDA)
        let com_port = unsafe {
            *(0x400 as *const u16).offset(com_id)
        };

        // If the COM port is not present (zero), check the next one
        if com_port == 0 {
            continue;
        }

        // Serial port initialization sequence
        unsafe {
            // Disable all interrupts
            cpu::out8(com_port + 1, 0x0);

            // Enable DLAB to set baud rate divisor
            cpu::out8(com_port + 3, 0x80);

            // We set the baud rate divisor to 1, i.e. the baud rate is 115200
            // Set baud rate divisor low byte
            cpu::out8(com_port + 0, 0x1);
            // Set baud rate divisor high byte
            cpu::out8(com_port + 1, 0x0);

            // Disable DLAB, set mode to 8 bits, no parity, 1 stop bit
            cpu::out8(com_port + 3, 0x3);

            // Disable FIFO
            cpu::out8(com_port + 2, 0x0);

            // Set DTR/RTS (Data Terminal Ready, Request to Send)
            cpu::out8(com_port + 4, 0x3);
        }

        // Store the port address
        serial.ports[com_id as usize] = Some(com_port);
    }
}

/// Writes `message` to all present serial ports. This function blocks until it can write all bytes.
pub fn write(message: &str) {
    let serial = SERIAL.lock();

    for byte in message.bytes() {
        // Check for every port if it is present
        for port in &serial.ports {
            if let Some(com_port) = *port {
                unsafe { write_byte(com_port, byte); }
            }
        }
    }
}

/// Writes `byte` to `com_port`. This is only used internally and assumes the serial lock is held.
unsafe fn write_byte(com_port: u16, byte: u8) {
    // Some serial consoles expect a CRLF to move to the start of the next line, so if we encounter
    // a LF we can just prepend a CR.
    if byte == b'\n' {
        write_byte(com_port, b'\r');
    }

    // Wait until we can transmit
    while cpu::in8(com_port + 5) & 0x20 == 0 {
        spin_loop_hint();
    }
    // Write the character to the serial port
    cpu::out8(com_port, byte);
}

/// Dummy struct to implement `core::fmt::Write` on
pub struct SerialWriter;

impl core::fmt::Write for SerialWriter {
    fn write_str(&mut self, msg: &str) -> core::fmt::Result {
        write(msg);
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