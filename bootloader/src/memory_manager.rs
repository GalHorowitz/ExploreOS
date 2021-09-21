//! Responisble for physical memory management in the bootloader

use crate::real_mode::{invoke_realmode_interrupt, RegisterState};

use core::convert::TryInto;
use range_set::{RangeSet, InclusiveRange};
use core::alloc::{GlobalAlloc, Layout};
use lock_cell::LockCell;
use page_tables::{PhysAddr, PhysMem};

pub struct PhysicalMemory(pub RangeSet);

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

        addr.map(PhysAddr)
    }

    fn release_phys_mem(&mut self, phys_addr: PhysAddr, size: usize) {
        if size == 0 {
            return;
        }

        self.0.insert(InclusiveRange {
            start: phys_addr.0,
            end: phys_addr.0.saturating_add((size - 1) as u32)
        });
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

/// A range descriptor which is returned from a BIOS E820 call
#[derive(Default)]
#[repr(C)]
struct E820RangeDescriptor {
	base_addr: u64,
	length: u64,
	mem_type: u32
}

/// Initialize the physical memory manager. Builds a memory map of available and reserved memory.
/// 
/// We get the memory map from the E820 BIOS call and store in a `RangeSet`. We also mark out own
/// bootloader code and stack, and some BIOS structures as reserved memory.
pub fn init(bootloader_size: u32) {
    let mut pmem = PHYS_MEM.lock();
    assert!(pmem.is_none(), "The memory manager should only be initialized once");

	// This the map of available memory we are building
    let mut available_memory = RangeSet::new();
    // This is the set of reserved ranges. We save the reserved ranges in a `RangeSet` instead of
    // just removing the reserved ranges from the `memory_map` because a reserved range which
    // overlaps a "free" range might appear before the free range in the E820 list.
    let mut reserved_ranges = RangeSet::new();

    // An opaque value used by the BIOS to report the next entry every time we call it. The initial
    // value is zero
    let mut continuation_value = 0;
    let mut result_descriptor = E820RangeDescriptor::default();
    let mut register_context = RegisterState::default();
    loop {
        // Set the parameters for the E820 call
        register_context.eax = 0xE820;
        register_context.ebx = continuation_value;
        register_context.ecx = 20;
        register_context.edi = &mut result_descriptor as *mut E820RangeDescriptor as u32;
        register_context.edx = u32::from_be_bytes(*b"SMAP");
        unsafe { invoke_realmode_interrupt(0x15, &mut register_context); }

        // Assert we recieved the correct signature and descriptor size
        assert_eq!(register_context.eax, u32::from_be_bytes(*b"SMAP"));
        assert_eq!(register_context.ecx, 20);
        
        // Save the continuation value for the next E820 call
        continuation_value = register_context.ebx;
        
        // We can only use ranges which start inside the 32-bit address limit
        if result_descriptor.base_addr <= core::u32::MAX as u64 {
            // If the range extends beyond the address limit, we trim it
            let range_end = core::cmp::min(result_descriptor.base_addr + result_descriptor.length - 1,
                core::u32::MAX as u64) as u32;

            let new_range = InclusiveRange {
                start: result_descriptor.base_addr as u32,
                end: range_end
            };
            // A memory type of 1 is memory that we are free to use. Any other type is reserved
            if result_descriptor.mem_type == 1 {
                available_memory.insert(new_range);
            } else {
                reserved_ranges.insert(new_range);
            }
        }

        // If CF is set or the continuation is zero, this is the last range
        if register_context.eflags&0x1 == 1 || register_context.ebx == 0 {
            break;
        }
    }
    // Subtract all reserved ranges from the memory map, this is to guard against some BIOSes which
    // have overlapping free and reserved ranges
    available_memory.subtract(&reserved_ranges);

    // Mark the IVT and the BDA as reserved
    available_memory.remove(InclusiveRange {
        start: 0x0,
        end: 0x4FF
    });

    // Mark the stack as reserved (0x1500 bytes)
    available_memory.remove(InclusiveRange {
        start: 0x6700,
        end: 0x7c00 - 1
    });

    // Mark the bootloader code and data as reserved
    available_memory.remove(InclusiveRange {
        start: 0x7c00,
        end: 0x7c00 + (bootloader_size - 1)
    });

    serial::println!("Initial memory map: ");
    serial::println!("{:#x?}", available_memory.ranges());

    // Store the initialized physical memory RangeSet
    *pmem = Some(PhysicalMemory(available_memory));
}