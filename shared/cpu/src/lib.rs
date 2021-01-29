//! x86-specific routines

#![no_std]
#![feature(asm)]

/// Reads a byte from the specified IO port `addr`
#[inline]
pub unsafe fn in8(addr: u16) -> u8 {
    let result: u8;
    asm!("in al, dx", out("al") result, in("dx") addr, options(nomem, preserves_flags, nostack));
    return result;
}

/// Reads a word from the specified IO port `addr`
#[inline]
pub unsafe fn in16(addr: u16) -> u16 {
    let result: u16;
    asm!("in ax, dx", out("ax") result, in("dx") addr, options(nomem, preserves_flags, nostack));
    return result;
}

/// Writes `data` to the specified IO port `addr`
#[inline]
pub unsafe fn out8(addr: u16, data: u8) {
    asm!("out dx, al", in("al") data, in("dx") addr, options(nomem, preserves_flags, nostack));
}

/// Writes `data` to the specified IO port `addr`
#[inline]
pub unsafe fn out16(addr: u16, data: u16) {
    asm!("out dx, ax", in("ax") data, in("dx") addr, options(nomem, preserves_flags, nostack));
}

/// Invalidates TLB entries for the page of the address `addr`
#[inline]
pub unsafe fn invlpg(addr: usize) {
    asm!("invlpg [{}]", in(reg) addr, options(preserves_flags, nostack));
}

/// Gets the value held in CR3
#[inline]
pub unsafe fn get_cr3() -> usize {
    let cr3: usize;
    asm!("mov {}, cr3", out(reg) cr3, options(nomem, preserves_flags, nostack));
    cr3
}

/// Loads the IDTR with the table descriptor at `table_descriptor_addr`
#[inline]
pub unsafe fn load_idt(table_descriptor_addr: usize) {
    asm!("lidt [{}]", in(reg) table_descriptor_addr, options(nomem, preserves_flags, nostack));
}

/// Disables interrupts and halts the cpu
#[inline]
pub fn halt() -> ! {
    loop {
        unsafe {
            asm!("
                cli
                hlt
            ", options(nomem, nostack));
        }
    }
}