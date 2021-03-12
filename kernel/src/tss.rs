//! TSS initialization and control for user-land interrupts

use exclusive_cell::ExclusiveCell;
use crate::gdt::GDTEntry;

/// Global that holds the single TSS we use for all tasks. Should only ever be accessed by the
/// scheduler which is not multi-threaded.
static TSS: ExclusiveCell<TaskStateSegment> = ExclusiveCell::new(TaskStateSegment::empty());

/// Represents an x86 32-bit TSS
#[repr(C)]
struct TaskStateSegment {
    prev_task_link: u16,
    reserved0: u16,
    esp0: u32,
    ss0: u16,
    reserved1: u16,
    esp1: u32,
    ss1: u16,
    reserved2: u16,
    esp2: u32,
    ss2: u16,
    reserved4: u16,
    cr3: u32,
    eip: u32,
    eflags: u32,
    eax: u32,
    ecx: u32,
    edx: u32,
    ebx: u32,
    esp: u32,
    ebp: u32,
    esi: u32,
    edi: u32,
    es: u16,
    reserved5: u16,
    cs: u16,
    reserved6: u16,
    ss: u16,
    reserved7: u16,
    ds: u16,
    reserved8: u16,
    fs: u16,
    reserved9: u16,
    gs: u16,
    reserved10: u16,
    ldt_seg_selector: u16,
    reserved11: u16,
    debug_trap_flag: u16,
    io_map_base_addr: u16,
    ssp: u32,
}

impl TaskStateSegment {
	/// Constructs an empty (zero-ed out) TSS
    const fn empty() -> Self {
        Self {
            prev_task_link: 0,
            reserved0: 0,
            esp0: 0,
            ss0: 0,
            reserved1: 0,
            esp1: 0,
            ss1: 0,
            reserved2: 0,
            esp2: 0,
            ss2: 0,
            reserved4: 0,
            cr3: 0,
            eip: 0,
            eflags: 0,
            eax: 0,
            ecx: 0,
            edx: 0,
            ebx: 0,
            esp: 0,
            ebp: 0,
            esi: 0,
            edi: 0,
            es: 0,
            reserved5: 0,
            cs: 0,
            reserved6: 0,
            ss: 0,
            reserved7: 0,
            ds: 0,
            reserved8: 0,
            fs: 0,
            reserved9: 0,
            gs: 0,
            reserved10: 0,
            ldt_seg_selector: 0,
            reserved11: 0,
            debug_trap_flag: 0,
            io_map_base_addr: 0,
            ssp: 0,
        }
    }
}

/// Initializes the TSS and returns a GDT entry that refrences the TSS. Should only be called once
pub unsafe fn init() -> GDTEntry {
	// The TSS is only used for stack-switching during interrupt handling while in ring 3, so we
	// only care about the ss0 and esp0 fields (ss:esp for ring 0).
	let mut tss = TSS.acquire();
    tss.ss0 = crate::gdt::KERNEL_DS_SELECTOR;
	// We initially set esp0 to an invalid value so that we hopefully fault if for some reason the
	// stack pointer was not set before jumping to user land
    tss.esp0 = 0xdeadbeef;

	// Calculating the GDT entry's limit field. The limit is (size_of - 1)
	let tss_limit = core::mem::size_of::<crate::tss::TaskStateSegment>() as u32 - 1;
    GDTEntry::new(&*tss as *const _ as u32, tss_limit, 0b1001, 0b1000, 0b0000)
}

/// Sets the esp that will be used for the kernel when handling interrupts while in ring 3
pub fn set_kernel_esp(esp: u32) {
    TSS.acquire().esp0 = esp;
}
