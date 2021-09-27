use core::sync::atomic::{AtomicU32, Ordering};

const CMOS_ADDRESS_PORT: u16 = 0x70;
const CMOS_DATA_PORT: u16 = 0x71;
const CMOS_RTC_SECONDS_REGISTER:  u8 = 0x0;
const CMOS_RTC_MINUTES_REGISTER:  u8 = 0x2;
const CMOS_RTC_HOURS_REGISTER:    u8 = 0x4;
const CMOS_RTC_DAY_REGISTER:      u8 = 0x7;
const CMOS_RTC_MONTH_REGISTER:    u8 = 0x8;
const CMOS_RTC_YEAR_REGISTER:     u8 = 0x9;
const CMOS_RTC_STATUS_A_REGISTER: u8 = 0xA;
const CMOS_RTC_STATUS_B_REGISTER: u8 = 0xB;

/// The unix timestamp on system boot
pub static BOOT_UNIX_TIME: AtomicU32 = AtomicU32::new(0);

/// Initializes the `BOOT_UNIX_TIME` global using the RTC on the CMOS
pub fn init() {
	// TODO: Get century from ACPI century register

	// Reading from the RTC while it is updating the values of the registers can lead to incorrect
	// values, so we don't want to read while the update flag is set in the status a register. It is
	// not enough to check the value once, because we might be checking a moment before an update
	// starts, so in theory we could wait until a updating->not updating edge, but because the RTC
	// updates every second, we might have to wait a whole second. Instead, we wait until the RTC
	// is not updating, and read the time twice, and retry if the times do not match
	loop {
		// Wait until the RTC is not upating
		while (read_cmos_reg(CMOS_RTC_STATUS_A_REGISTER) & (1 << 7)) != 0 {
			core::hint::spin_loop();
		}

		let current_time = read_current_time();

		// Make sure an update did not start
		if (read_cmos_reg(CMOS_RTC_STATUS_A_REGISTER) & (1 << 7)) != 0 {
			continue;
		}

		let current_time_alt = read_current_time();

		// Compare the two times we read
		if current_time == current_time_alt {
			// We got a consistent time, so we store it and finish
			BOOT_UNIX_TIME.store(current_time, Ordering::Relaxed);
			break;
		}
	}
}

/// Reads the current unix timestamp from the RTC
fn read_current_time() -> u32 {
	// The flags in status define the format of the values the RTC provides
	let status_b = read_cmos_reg(CMOS_RTC_STATUS_B_REGISTER);
	let hour_format_24 = (status_b & (1 << 1)) != 0;
	let binary_mode = (status_b & (1 << 2)) != 0;

	let seconds = read_cmos_reg(CMOS_RTC_SECONDS_REGISTER);
	let minutes = read_cmos_reg(CMOS_RTC_MINUTES_REGISTER);
	let hours = read_cmos_reg(CMOS_RTC_HOURS_REGISTER);
	let day = read_cmos_reg(CMOS_RTC_DAY_REGISTER);
	let month = read_cmos_reg(CMOS_RTC_MONTH_REGISTER);
	let year = read_cmos_reg(CMOS_RTC_YEAR_REGISTER);

	// If the time is in 12-hour format, the MSB contains the AM/PM flag, if the format is 24-hour,
	// the MSB is just 0
	let currently_pm = (hours & (1 << 7)) != 0;
	let hours = hours & (!(1 << 7));

	// If the binary flag is not set, the values are stored in binary coded decimal, so we need to
	// deocde them
	let (seconds, minutes, hours, day, month, year) = if binary_mode {
		(seconds, minutes, hours, day, month, year)
	} else {
		(
			decode_bcd(seconds),
			decode_bcd(minutes),
			decode_bcd(hours),
			decode_bcd(day),
			decode_bcd(month),
			decode_bcd(year)
		)
	};

	// If the hour format is not 24-hours, we use the AM/PM flag to convert
	let hours = if hour_format_24 {
		hours
	} else if currently_pm {
		(hours % 12) + 12
	} else {
		hours % 12
	};

	// We assume we are in the 2000s
	let (seconds, minutes, hours, day, month, year) = (
		seconds as u32,
		minutes as u32,
		hours as u32,
		day as u32,
		month as u32,
		year as u32 + 2000
	);

	seconds + 60 * (minutes + 60 * (hours + 24 * get_num_days_since_epoch(day, month, year)))
}

/// Returns the number of days between the unix epoch (1 January 1970) and the given date, where
/// `day` is 1-31 and `month` is 1-12
fn get_num_days_since_epoch(day: u32, month: u32, year: u32) -> u32 {
	let mut num_days = day - 1;

	// Add up days in past years since 1970, adding extra days for leap years
	for passed_year in 1970..year {
		if is_leap_year(passed_year) {
			num_days += 366;
		} else {
			num_days += 365;
		}
	}

	// This array holds the amount of days to add to account for the months that already passed
	const MONTH_ACC_DAY_COUNT: [u32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
	num_days += MONTH_ACC_DAY_COUNT[(month - 1) as usize];

	// If the current year is a leap day, february contains an extra day, so if we passed it, we
	// need to add that day
	if month >= 3 && is_leap_year(year) {
		num_days += 1;
	}

	num_days
}

/// Returns whether `year` is a leap year
fn is_leap_year(year: u32) -> bool {
	if year % 400 == 0 {
		return true;
	}

	if year % 100 == 0 {
		return false;
	}

	year % 4 == 0
}

/// Decodes a binary coded decimal
fn decode_bcd(val: u8) -> u8 {
	let low_nibble = val & 0xF;
	let high_nibble = val >> 4;
	low_nibble + (10 * high_nibble)
}

/// Reads the value of CMOS register `reg`
fn read_cmos_reg(reg: u8) -> u8 {
	unsafe {
		// Accessing a CMOS register is done by writing the register address into the address port,
		// and then reading the value from the data port. The OSDev wiki advises to have a small
		// delay between the operations
		cpu::out8(CMOS_ADDRESS_PORT, reg);
		cpu::busy_loop(0x6c600);
		cpu::in8(CMOS_DATA_PORT)
	}
}