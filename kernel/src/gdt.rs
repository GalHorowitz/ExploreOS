//! GDT initialization after transition to virtual memory

use exclusive_cell::ExclusiveCell;

const GDT_ENTRIES: usize = 6;

pub const KERNEL_CS_SELECTOR: u16 = 1*8;
pub const KERNEL_DS_SELECTOR: u16 = 2*8;
pub const USER_CS_SELECTOR: u16 = 3*8;
pub const USER_DS_SELECTOR: u16 = 4*8;

/// Struct to wrap GDT entries to so we can set the alignment to 8 bytes (best performance according
/// to the Intel manual)
#[derive(Clone, Copy)]
#[repr(C, align(8))]
pub struct GDTEntry(u64);

static GDT: ExclusiveCell<[GDTEntry; GDT_ENTRIES]> = ExclusiveCell::new([GDTEntry(0); GDT_ENTRIES]);

/// Sets up a GDT in virtual kernel space and loads the TSS. Should only be called once
pub unsafe fn init() {
    // Even though we already have a GDT which was set up by the bootloader, we now have paging
    // enabled and so must reload with a virtual address

    let mut gdt = GDT.acquire();

    assert!((gdt.as_ptr() as usize) & 7 == 0);

    // The actual documentation for these entries is at `bootloader/src/stage0/gdt.asm`
    // Null entry
    gdt[0] = GDTEntry(0x0000000000000000);
    // Ring 0 Code segment Descriptor
    gdt[1] = GDTEntry::new(0x0, 0xFFFFF, 0b1010, 0b1001, 0b1100);
    // Ring 0 Data segment Descriptor
    gdt[2] = GDTEntry::new(0x0, 0xFFFFF, 0b0010, 0b1001, 0b1100);
    // Ring 3 Code Segment Descriptor
    gdt[3] = GDTEntry::new(0x0, 0xFFFFF, 0b1010, 0b1111, 0b1100);
    // Ring 3 Data Segment Descriptor
    gdt[4] = GDTEntry::new(0x0, 0xFFFFF, 0b0010, 0b1111, 0b1100);
    // TSS Descriptor
    gdt[5] = crate::tss::init();

    // Load the GDT. There is no need to actually reload the segment selectors, as the GDT is the
    // same, but this will be important later on when the GDT is accessed (e.g. in interrupts)
    cpu::load_gdt(gdt.as_ptr() as u32, (GDT_ENTRIES*8 - 1) as u16);

    // Loads the task register with the TSS descriptor we setup
    cpu::load_task_register(5*8);
}


impl GDTEntry {
    /// Constructs the u64 representing a GDT entry based on the given parameters
    /// 
    /// * `base_addr` - the linear address of segment's base
    /// * `limit` - one less than the size of the segment (20 bits)
    /// * `typ` - the 4 bits which select the segment type
    /// * `flags12_15` - the segment flags in bits 12-15 (S, DPL, P)
    /// * `flags20_23` - the segment flags in bits 20-23 (AVL, L, D/B, G)
    pub const fn new(base_addr: u32, limit: u32, typ: u8, flags12_15: u8, flags20_23: u8) -> Self {
        assert!(limit & !0xFFFFF == 0);  // Limit is 20 bits
        assert!(typ & 0xF0 == 0);        // Type field is 4 bits
        assert!(flags12_15 & 0xF0 == 0); // First flags field is 4 bits 
        assert!(flags20_23 & 0xF0 == 0); // Second flags field is 4 bits

        let low_dword = ((base_addr & 0xFFFF) << 16) | (limit & 0xFFFF);
        let high_dword = (base_addr & 0xFF000000) | ((flags20_23 as u32) << 20) | (limit & 0xF0000)
            | ((flags12_15 as u32) << 12) | ((typ as u32) << 8) | ((base_addr >> 16) & 0xFF);

        GDTEntry(((high_dword as u64) << 32) | (low_dword as u64))
    }
}

