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
pub fn get_cr3() -> usize {
    let cr3: usize;
    unsafe {
        asm!("mov {}, cr3", out(reg) cr3, options(nomem, preserves_flags, nostack));
    }
    cr3
}

/// Gets the value held in CR2
#[inline]
pub fn get_cr2() -> usize {
    let cr2: usize;
    unsafe {
        asm!("mov {}, cr2", out(reg) cr2, options(nomem, preserves_flags, nostack));
    }
    cr2
}

/// Reads the timestamp counter (with an LFENCE on either side to keep instructions from reordering)
#[inline]
pub fn rdtsc() -> u64 {
    let result_high: u32;
    let result_low: u32;
    unsafe {
        asm!("
            lfence
            rdtsc
            lfence
        ", out("edx") result_high, out("eax") result_low);
    }
    ((result_high as u64) << 32) | (result_low as u64)
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

/// Loads the IDTR with the table at `base` with whose last byte is at `base+limit`
#[inline]
pub unsafe fn load_idt(base: u32, limit: u16) {
    // LIDT expects a 6-byte memory location [limit:base] so we just push it on the stack
    asm!("
        push ebx
        push ax
        lidt [esp]
        pop ax
        pop ebx
    ", in("ebx") base, in("ax") limit, options(nomem, preserves_flags));
}

/// Loads the GDTR with the table at `base` with whose last byte is at `base+limit`
#[inline]
pub unsafe fn load_gdt(base: u32, limit: u16) {
    // LGDT expects a 6-byte memory location [limit:base] so we just push it on the stack
    asm!("
        push ebx
        push ax
        lgdt [esp]
        pop ax
        pop ebx
    ", in("ebx") base, in("ax") limit, options(nomem, preserves_flags));
}

/// Sets the interrupt flag (IF) in the EFLAGS register, this allows the processor to respond to
/// maskable hardware interrupts
#[inline]
pub unsafe fn sti() {
    asm!("sti", options(nomem, nostack));
}