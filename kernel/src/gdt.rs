use alloc::alloc::{alloc, Layout};

/// Sets up a GDT in virtual kernel space
pub fn init() {
    // Even though we already have a GDT which was set up by the bootloader, we now have paging
    // enabled and so must reload with a virtual address

    /// A null entry, a code segment entry, and a data segment entry
    const GDT_ENTRIES: usize = 3;

    // Allocate the table which according to the intel manual should be 8-byte aligned for best
    // performance
	let gdt = unsafe {
        let gdt_ptr = alloc(Layout::from_size_align(GDT_ENTRIES * 8, 8).unwrap());
        core::slice::from_raw_parts_mut(gdt_ptr as *mut u64, GDT_ENTRIES)
    };

    // The actual documentation of these entries is at `bootloader/src/stage0/gdt.asm`
    // Null entry
    gdt[0] = 0x0000000000000000;
    // Code segment
    gdt[1] = 0x00cf9a000000ffff;
    // Data segment
    gdt[2] = 0x00cf92000000ffff;

    // Load the GDT. There is no need to actually reload the segment selectors, as the GDT is the
    // same, but this will be important later on when the GDT is accessed (e.g. in interrupts)
    unsafe {
        cpu::load_gdt(gdt.as_ptr() as u32, (GDT_ENTRIES*8 - 1) as u16);
    }
}