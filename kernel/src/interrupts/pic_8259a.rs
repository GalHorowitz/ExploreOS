//! 8259A PIC controller

// Reference: https://pdos.csail.mit.edu/6.828/2014/readings/hardware/8259A.pdf

/// The I/O port of the master PIC command register
const MASTER_PIC_CMD_PORT: u16 = 0x20;
/// The I/O port of the master PIC data register
const MASTER_PIC_DATA_PORT: u16 = 0x21;
/// The I/O port of the slave PIC command register
const SLAVE_PIC_CMD_PORT: u16 = 0xA0;
/// The I/O port of the slave PIC data register
const SLAVE_PIC_DATA_PORT: u16 = 0xA1;

/// The PIC initialization command
const PIC_INIT_CMD: u8 = 0x10;
/// The PIC init command flag that signals that we intend to send an ICW4 configuration byte
const PIC_ICW4_NEEDED: u8 = 0x1;
/// The PIC init command flag that signals that the PIC should work in 8086 microprocessor mode
const PIC_8086_MPM: u8 = 0x1;
/// The master IRQ line the slave INTR line is connected to
const PIC_SLAVE_IRQ: u8 = 0x2;
/// The PIC IRQ-specific end of interrupt command
const PIC_SPECIFIC_EOI_CMD: u8 = 3 << 5;
/// The PIC register read command
const PIC_READ_REGISTER_CMD: u8 = 3 << 3;
/// The PIC read command flag that signals we want to read the IS register
const PIC_READ_IS_REGISTER: u8 = 3;

/// The offset added to the IRQ to get the corresponding interrupt vector
pub const PIC_IRQ_OFFSET: u8 = 0x20;

/// Initializes the 2 cascading 8259A intel PICs
pub fn init() {
	unsafe {
		// Send ICW1 to begin init process for the master PIC. We also set the IC4 bit to signify we
		// want to send an ICW4 configuration byte
		cpu::out8(MASTER_PIC_CMD_PORT, PIC_INIT_CMD | PIC_ICW4_NEEDED);
		// Send ICW2 which is the interrupt offset of the master PIC
		cpu::out8(MASTER_PIC_DATA_PORT, PIC_IRQ_OFFSET);
		// Send ICW3 which tells the master which IRQ the slave is wired to
		cpu::out8(MASTER_PIC_DATA_PORT, 1 << PIC_SLAVE_IRQ);
		// Send ICW4 which configures the 8259A for 8086 microprocessor mode
		cpu::out8(MASTER_PIC_DATA_PORT, PIC_8086_MPM);

		// Send ICW1 to begin init process for the slave PIC. We also set the IC4 bit to signify we
		// want to send an ICW4 configuration byte
		cpu::out8(SLAVE_PIC_CMD_PORT, PIC_INIT_CMD | PIC_ICW4_NEEDED);
		// Send ICW2 which is the interrupt offset of the slave PIC
		cpu::out8(SLAVE_PIC_DATA_PORT, PIC_IRQ_OFFSET + 8);
		// Send ICW3 which tells the slave to which master IRQ it is wired to
		cpu::out8(SLAVE_PIC_DATA_PORT, PIC_SLAVE_IRQ);
		// Send ICW4 which configures the 8259A for 8086 microprocessor mode
		cpu::out8(SLAVE_PIC_DATA_PORT, PIC_8086_MPM);
	}
}

/// Returns the 16-bit interrupt mask. If a bit is set the corresponding IRQ is inhibited
pub fn get_interrupt_mask() -> u16 {
	unsafe {
		// Reading from the data port returns the interrupt mask. The slave handles IRQs 8-15 so we
		// shift it 8 bits to the left before combining
		((cpu::in8(SLAVE_PIC_DATA_PORT) as u16) << 8) | (cpu::in8(MASTER_PIC_DATA_PORT) as u16)
	}
}

/// Sets the 16-bit interrupt mask. If a bit is set the corresponding IRQ is inhibited
pub fn set_interrupt_mask(mask: u16) {
	// The slave handles IRQs 8-15 so so the most significant byte is the slave mask
	let master_mask = (mask & 0xFF) as u8;
	let slave_mask = (mask >> 8) as u8;
	
	// Write the masks out to the PICs
	unsafe {
		cpu::out8(MASTER_PIC_DATA_PORT, master_mask);
		cpu::out8(SLAVE_PIC_DATA_PORT, slave_mask);
	}
}

/// Sends an End Of Interrupt message to the relevant PICs for the specified `irq`
pub fn send_eoi(irq: u8) {
	// Check if this is a slave IRQ or a master one
	if irq >= 8 {
		// This is a slave IRQ, so we need to both end the slave interrupt at the master
		// and also end the interrupt at the slave
		unsafe {
			cpu::out8(MASTER_PIC_CMD_PORT, PIC_SPECIFIC_EOI_CMD | PIC_SLAVE_IRQ);
			cpu::out8(SLAVE_PIC_CMD_PORT, PIC_SPECIFIC_EOI_CMD | (irq - 8));
		}
	} else {
		// This is a master IRQ, so we just need to notify the master of an EOI
		unsafe {
			cpu::out8(MASTER_PIC_CMD_PORT, PIC_SPECIFIC_EOI_CMD | irq);
		}
	}
}

/// Returns the combined 16-bit Interrupt Service Register of the PICs. If a bit is set the
/// corresponding IRQ is currently being serviced
fn read_isr() -> u16 {
	unsafe {
		// Tell the master PIC to output the ISR on the next read
		cpu::out8(MASTER_PIC_CMD_PORT, PIC_READ_REGISTER_CMD | PIC_READ_IS_REGISTER);
		// Tell the slave PIC to output the ISR on the next read
		cpu::out8(SLAVE_PIC_CMD_PORT, PIC_READ_REGISTER_CMD | PIC_READ_IS_REGISTER);
		// Combined the two ISR (The slave handles IRQs 8-15)
		((cpu::in8(SLAVE_PIC_CMD_PORT) as u16) << 8) | (cpu::in8(MASTER_PIC_CMD_PORT) as u16)
	}
}

/// Returns true if this a spurious IRQ which should be discarded. Sends the neccesary commands if
// the IRQ is spurious
pub fn handle_spurious_irq(irq: u8) -> bool {
	// Only IRQ 7 and 15 can be spurious IRQs
	if irq != 7 && irq != 15 {
		return false;
	}

	// This is a spurious IRQ if the relevant PIC does not report it as being serviced
	let spurious = (read_isr() & (1 << irq)) == 0;

	if irq >= 8 && spurious {
		// If this is a slave IRQ, and the IRQ is spurious, even though an EOI should not be sent
		// to the slave, the master still sees this as a normal IRQ, so we need to acknowledge it
		send_eoi(PIC_SLAVE_IRQ);
	}

	// If this is a master IRQ, or the IRQ is not spurious, we just need to return if the IRQ is
	// spurious, no special handling required
	spurious
}