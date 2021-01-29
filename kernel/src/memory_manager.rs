use core::convert::TryInto;
use core::alloc::{GlobalAlloc, Layout};
use boot_args::BootArgs;
use page_tables::{PhysMem, PhysAddr};
use range_set::{RangeSet, InclusiveRange};
use lock_cell::LockCell;

pub struct PhysicalMemory(RangeSet);

impl PhysMem for PhysicalMemory {
    unsafe fn translate_phys(&mut self, phys_addr: PhysAddr, size: usize) -> Option<*mut u8> {
        // No meaning for a ptr to be valid for 0 bytes
        if size == 0 {
            return None;
        }

        // If we change this to 64-bit later on, this will make sure we are not trying to access
        // 64 bit addresses while in 32-bit mode
        let phys_addr_start: usize = phys_addr.0.try_into().ok()?;
        // Make sure the entire size bytes fit in the address space
        phys_addr_start.checked_add(size - 1)?;

        // We have a flat memory mapping, so a physical address is also the virtual address
        Some(phys_addr_start as *mut u8)
    }

    fn allocate_phys_mem(&mut self, layout: Layout) -> Option<PhysAddr> {
        let addr = self.0.allocate(layout.size().try_into().ok()?, layout.align().try_into().ok()?);

        addr.map(|x| PhysAddr(x))
    }
}

/// Global to hold the `RangeSet` of available physical memory
pub static PHYS_MEM: LockCell<Option<PhysicalMemory>> = LockCell::new(None);

/// The global allocator for the bootloader
#[global_allocator]
static GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator;

/// Dummy struct to implement `GlobalAlloc` on
struct GlobalAllocator;

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		// The `RangeSet` allocator only supports 32-bit
		if layout.size() > core::u32::MAX as usize || layout.align() > core::u32::MAX as usize {
			return core::ptr::null_mut()
		}

		// We can now safely convert
		let size = layout.size() as u32;
		let align = layout.align() as u32;

        // Check that the physical memory manager was initialized
        let mut pmem = PHYS_MEM.lock();
        if pmem.is_none() {
            return core::ptr::null_mut();
        }
        
		// Allocate physical memory from the `RangeSet`
    	if let Some(addr) = pmem.as_mut().unwrap().0.allocate(size, align) {
            addr as *mut u8
		} else {
			core::ptr::null_mut()
		}
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut pmem = PHYS_MEM.lock();

        if layout.size() == 0 {
            panic!("Attempt to dealloc a zero sized allocation");
        }

        // Check the memory manager is initialized
        if let Some(free_mem) = pmem.as_mut() {
            // Insert the range back into the set as free memory
            free_mem.0.insert(InclusiveRange {
                start: ptr as u32,
                end: ptr as u32 + (layout.size() as u32 - 1)
            });
        } else {
            panic!("Attempt to dealloc before memory manager was initialized");
        }
    }
}

pub fn init(boot_args: &BootArgs) {
    // let pmem = PHYS_MEM.lock();
    // *pmem = Some(PhysicalMemory(boot_args.free_memory));
}