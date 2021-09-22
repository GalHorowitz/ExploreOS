//! x86-specific routines

#![no_std]
#![feature(asm)]

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PushADRegisterState {
    pub edi: u32,
    pub esi: u32,
    pub ebp: u32,
    pub esp: u32,
    pub ebx: u32,
    pub edx: u32,
    pub ecx: u32,
    pub eax: u32,
}

/// Reads a byte from the specified IO port `addr`
///
/// ### Safety
/// If the CPL is greater than the IOPL this will cause a GPF
#[inline]
pub unsafe fn in8(addr: u16) -> u8 {
    let result: u8;
    asm!("in al, dx", out("al") result, in("dx") addr, options(nomem, preserves_flags, nostack));
    result
}

/// Reads a word from the specified IO port `addr`
///
/// ### Safety
/// If the CPL is greater than the IOPL this will cause a GPF
#[inline]
pub unsafe fn in16(addr: u16) -> u16 {
    let result: u16;
    asm!("in ax, dx", out("ax") result, in("dx") addr, options(nomem, preserves_flags, nostack));
    result
}

/// Writes `data` to the specified IO port `addr`
///
/// ### Safety
/// If the CPL is greater than the IOPL this will cause a GPF
#[inline]
pub unsafe fn out8(addr: u16, data: u8) {
    asm!("out dx, al", in("al") data, in("dx") addr, options(nomem, preserves_flags, nostack));
}

/// Writes `data` to the specified IO port `addr`
///
/// ### Safety
/// If the CPL is greater than the IOPL this will cause a GPF
#[inline]
pub unsafe fn out16(addr: u16, data: u16) {
    asm!("out dx, ax", in("ax") data, in("dx") addr, options(nomem, preserves_flags, nostack));
}

/// Invalidates TLB entries for the page of the address `addr`
///
/// ### Safety
/// If the CPL is not zero this will cause a GPF
#[inline]
pub unsafe fn invlpg(addr: usize) {
    asm!("invlpg [{}]", in(reg) addr, options(preserves_flags, nostack));
}

/// Gets the value held in CR3
///
/// ### Safety
/// If the CPL is not zero this will cause a GPF
#[inline]
pub unsafe fn get_cr3() -> usize {
    let cr3: usize;
    asm!("mov {}, cr3", out(reg) cr3, options(nomem, preserves_flags, nostack));
    cr3
}

/// Gets the value held in CR2
///
/// ### Safety
/// If the CPL is not zero this will cause a GPF
#[inline]
pub unsafe fn get_cr2() -> usize {
    let cr2: usize;
    asm!("mov {}, cr2", out(reg) cr2, options(nomem, preserves_flags, nostack));
    cr2
}

/// Gets the value of the EFLAGS register
#[inline]
pub fn get_eflags() -> u32 {
    let eflags: u32;
    unsafe {
        asm!("
            pushfd
            pop {:e}
        ", out(reg) eflags, options(nomem, preserves_flags));
    }
    eflags
}

/// Reads the timestamp counter (with an LFENCE on either side to keep instructions from reordering)
#[inline]
pub fn serializing_rdtsc() -> u64 {
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

/// Halts the cpu in a loop while servicing interrupts
///
/// ### Safety
/// If the CPL is not zero this will cause a GPF
#[inline]
pub unsafe fn halt_and_service_interrupts() -> ! {
    loop {
        asm!("hlt", options(nomem, nostack));
    }
}

/// Disables interrupts and halts the cpu
///
/// ### Safety
/// If the CPL is not zero this will cause a GPF
#[inline]
pub unsafe fn halt() -> ! {
    loop {
        asm!("
            cli
            hlt
        ", options(nomem, nostack));
    }
}

/// Loads the IDTR with the table at `base` with whose last byte is at `base+limit`
///
/// ### Safety
/// If the CPL is not zero this will cause a GPF
#[inline]
pub unsafe fn load_idt(base: u32, limit: u16) {
    // LIDT expects a 6-byte memory location [limit:base] so we just push it on the stack
    asm!("
        push {0:e}
        push {1:x}
        lidt [esp]
        pop {1:x}
        pop {0:e}
    ", in(reg) base, in(reg) limit, options(nomem, preserves_flags));
}

/// Loads the GDTR with the table at `base` with whose last byte is at `base+limit`
///
/// ### Safety
/// If the CPL is not zero this will cause a GPF. The new GDT must contain code and data segments
/// for this and the calling function or else execution cannot continue
#[inline]
pub unsafe fn load_gdt(base: u32, limit: u16) {
    // LGDT expects a 6-byte memory location [limit:base] so we just push it on the stack
    asm!("
        push {0:e}
        push {1:x}
        lgdt [esp]
        pop {1:x}
        pop {0:e}
    ", in(reg) base, in(reg) limit, options(nomem, preserves_flags));
}

/// Loads the Task Register with the GDT segment `segment_selector`
///
/// ### Safety
/// If the CPL is not zero this will cause a GPF. The segment selector must point to an already
/// loaded TSS descriptor in the GDT
#[inline]
pub unsafe fn load_task_register(segment_selector: u16) {
    asm!("ltr {:x}", in(reg) segment_selector, options(nomem, preserves_flags, nostack));
}

/// Clears the interrupt flag (IF) in the EFLAGS register, this causes the processor to ignore
/// maskable hardware interrupts
///
/// ### Safety
/// If the CPL is greater than the IOPL this will cause a GPF
#[inline]
pub unsafe fn cli() {
    asm!("cli", options(nomem, nostack));
}

/// Sets the interrupt flag (IF) in the EFLAGS register, this allows the processor to respond to
/// maskable hardware interrupts
///
/// ### Safety
/// If the CPL is greater than the IOPL this will cause a GPF. An IDT must already be loaded
#[inline]
pub unsafe fn sti() {
    asm!("sti", options(nomem, nostack));
}

/// Gets the interrupt flag (IF) from the EFLAGS register
#[inline]
pub fn get_if() -> bool {
    (get_eflags() & (1 << 9)) != 0
}

/// Performs a long jump to `cs_selector:eip`. The eflags register will be set to `eflgas` and the
/// stack will atomically change to `ds_selector:esp`. The data segment selector will also be used
/// to set all other data segment selectors.
///
/// IMPORTANT: The stack-switching is done using an `iret`, which only happens on a CPL change, so
/// this should only be used in such a context
///
/// ### Safety
/// The stack-switching is done using an `iret`, which only happens on a CPL change, so
/// this should only be used in such a context. For further constraints, see the intel manual
/// `iretd`
#[inline]
pub unsafe fn jump_to_ring3(eip: u32, cs_selector: u16, eflags: u32, ds_selector: u16,
    regs: &PushADRegisterState, cr3: u32) -> ! {
    let cs_selector = cs_selector as u32;
    let ds_selector = ds_selector as u32;

    asm!("
            cli         // Disable interrupts during segment selector switching, will be re-enabled
            mov ds, {0:x} // by the EFLAGS swap in `iretd`
            mov es, {0:x}
            mov fs, {0:x}
            mov gs, {0:x}

            // Setup fake interrupt stack frame
            push {0:e}      // SS selector (high 16-bits discarded)
            push [eax + 12] // ESP
            push {1:e}      // EFLAGS
            push {2:e}      // CS selector (high 16-bits discarded)
            push {3:e}      // EIP

            // Setup page directory
            mov cr3, {4:e}

            // Restore registers
            mov edi, [eax]
            mov esi, [eax + 4]
            mov ebp, [eax + 8]
            mov ebx, [eax + 16]
            mov edx, [eax + 20]
            mov ecx, [eax + 24]
            mov eax, [eax + 28]
            iretd
        ",
        in(reg) ds_selector, in(reg) eflags, in(reg) cs_selector, in(reg) eip, in(reg) cr3,
        in("eax") regs, options(noreturn)
    );
}

#[inline]
pub unsafe fn ring0_context_switch(eip: u32, eflags: u32, regs: &PushADRegisterState, cr3: u32) -> ! {
    asm!("
            // Restore eflags
            push {0:e}
            popfd

            // Restore esp first, so we can push the fake return address
            mov esp, [eax + 12]

            // Switch page directory (This is a ring 0 to ring 0 switch, so this kernel code is
            // mapped-in in both page directories)
            mov cr3, {2:e}

            // Setup fake return address (we must do this now because all registers are overwritten)
            push {1:e}

            // Restore registers
            mov edi, [eax]
            mov esi, [eax + 4]
            mov ebp, [eax + 8]
            mov ebx, [eax + 16]
            mov edx, [eax + 20]
            mov ecx, [eax + 24]
            mov eax, [eax + 28]

            // Make the jump
            ret
        ",
        in(reg) eflags, in(reg) eip, in(reg) cr3, in("eax") regs, options(noreturn)
    );
}