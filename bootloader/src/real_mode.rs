//! Definitions for the `rust_asm_routines.asm` real-mode interaction

/// x86 register state for invoking interrupts
#[derive(Default)]
#[repr(C)]
pub struct RegisterState {
    pub eax: u32,
    pub ecx: u32,
    pub edx: u32,
    pub ebx: u32,
    pub ebp: u32,
    pub esi: u32,
    pub edi: u32,
    pub eflags: u32,
    pub ds: u16,
    pub es: u16,
    pub fs: u16,
    pub gs: u16,
    pub ss: u16
}

extern {
	/// Invokes the interrupt `interrupt_num` in real mode with the specified context `regs`.
	/// The function overwrites `regs` with the state of the interrupt result.
	/// The eflags and the segment registers passed in are ignored.
    pub fn invoke_realmode_interrupt(interrupt_num: u8, regs: *mut RegisterState);
}