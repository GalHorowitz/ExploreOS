//! 8254 PIT controller

use core::sync::atomic::{AtomicU32, Ordering};

use exclusive_cell::ExclusiveCell;

// Reference: https://www.scs.stanford.edu/10wi-cs140/pintos/specs/8254.pdf

const PIT_CHANNEL_0_DATA_PORT: u16 = 0x40;
const PIT_CONTROL_WORD_REGISTER_PORT: u16 = 0x43;

/// The interrupt frequency we want to achieve (in Hz)
const TARGET_FREQ_HZ: f64 = 100f64;
/// The frequency the PIT's clock runs on
const PIT_FREQ_HZ: f64 = 1_000_000f64 * 105f64 / 88f64;
/// The calculated frequency divisor for the PIT
// TODO: The PIT supports using a divisor of 0 as 2^16, so if we need a small frequency than we need
// to add a case for that
const PIT_FREQ_DIV: u16 = (PIT_FREQ_HZ / TARGET_FREQ_HZ) as u16;
/// The actual interrupt frequency (it is different from the target frequency because we are forced
/// to truncate the divisor to an integer)
const REAL_FREQ_HZ: f64 = PIT_FREQ_HZ / (PIT_FREQ_DIV as f64);

/// Initiailizes the PIT's first counter as a rate generator
pub fn init() {
	unsafe {
		// Initialize counter 0 by writing a setup control-word:
		// 00  - select counter 0
		// 11  - write least signifcant byte first, then most significant byte
		// 010 - mode 2 (rate generator)
		// 0   - 16-bit binary (instead of BCD)
		cpu::out8(PIT_CONTROL_WORD_REGISTER_PORT, 0b0011_0100);

		// Write the least sig and most sig bytes of the freq divisor
		cpu::out8(PIT_CHANNEL_0_DATA_PORT, PIT_FREQ_DIV as u8);
		cpu::out8(PIT_CHANNEL_0_DATA_PORT, (PIT_FREQ_DIV >> 8) as u8);
	}
}

// static mut time: f64 = 0f64; TODO: DEBUG CODE

pub static CURRENT_UNIX_TIME: AtomicU32 = AtomicU32::new(0);
static ONLINE_TIME: ExclusiveCell<f64> = ExclusiveCell::new(0.0);

// Handles an interrupt from the PIT (should only be called when an interrupt happens)
pub unsafe fn handle_interrupt() {
	let mut online_time = ONLINE_TIME.acquire();
	*online_time += 1f64/REAL_FREQ_HZ;

	let boot_time = crate::time::BOOT_UNIX_TIME.load(Ordering::Relaxed);
	CURRENT_UNIX_TIME.store(boot_time + (*online_time as u32), Ordering::Relaxed);
}