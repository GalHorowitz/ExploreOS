use core::convert::TryInto;
use core::alloc::{GlobalAlloc, Layout};

use boot_args::{BootArgs, PAGE_DIRECTORY_VADDR, LAST_PAGE_TABLE_VADDR};
use cpu::halt;
use range_set::{RangeSet, InclusiveRange};
use page_tables::{PageDirectory, PhysMem, PhysAddr, VirtAddr};
use lock_cell::LockCell;

pub struct PhysicalMemory{
    memory_ranges: RangeSet,
    page_directory_paddr: PhysAddr,
    last_page_table_paddr: PhysAddr
}

impl PhysMem for PhysicalMemory {
    unsafe fn translate_phys(&mut self, page_dir: Option<&mut PageDirectory>, phys_addr: PhysAddr,
        size: usize) -> Option<*mut u8> {
        // No meaning for a ptr to be valid for 0 bytes
        if size == 0 {
            return None;
        }

        // If we change this to 64-bit later on, this will make sure we are not trying to access
        // 64 bit addresses while in 32-bit mode
        let phys_addr_start: usize = phys_addr.0.try_into().ok()?;
        // Make sure the entire size bytes fit in the address space
        phys_addr_start.checked_add(size - 1)?;

        // Check if this physical address is inside the page directory
        if phys_addr.0 >= self.page_directory_paddr.0
            && phys_addr.0 <= self.page_directory_paddr.0 + 4095 {
            // The page directory is mapped at `PAGE_DIRECTORY_VADDR`, so the translation of the
            // requested physical address is just at the relevant offset of that virtual address
            let page_offset = phys_addr.0 - self.page_directory_paddr.0;

            // Check that the requested physical window resides entirely inside the page directory
            if page_offset.checked_add(size as u32 - 1)? > 4095 {
                return None;
            }

            return Some((PAGE_DIRECTORY_VADDR + page_offset) as *mut u8);
        }

        // Check if this physical address is inside the page table of the last page
        if phys_addr.0 >= self.last_page_table_paddr.0
            && phys_addr.0 <= self.last_page_table_paddr.0 + 4095 {
            // This page table is mapped at `LAST_PAGE_TABLE_VADDR`, so the translation of the
            // requested physical address is just at the relevant offset of that virtual address
            let page_offset = phys_addr.0 - self.last_page_table_paddr.0;

            // Check that the requested physical window resides entirely inside the page directory
            if page_offset.checked_add(size as u32 - 1)? > 4095 {
                return None;
            }

            return Some((LAST_PAGE_TABLE_VADDR + page_offset) as *mut u8);
        }

        // If the physical address is not on of the permanent mappings, we must make a mapping for
        // it, so we need access to the page directory
        let page_dir = page_dir?;

        // Calculate the address of the page containing the physical address
        let phys_addr_page = phys_addr.0 & !0xFFF;
        // Make sure the requested physical window does not extend beyond this one page. This should
        // not be a problem: the page table functions only ever use this to read and write to page
        // tables which are one page long.
        if phys_addr.0.checked_add(size as u32 - 1)? > (phys_addr_page + 4095) {
            return None;
        }

        // Make the mapping of the last virtual page (0xFFFFF000-0xFFFFFFFF) to the physical page
        let raw_table_entry =
            phys_addr_page | page_tables::PAGE_ENTRY_PRESENT | page_tables::PAGE_ENTRY_WRITE;
        page_dir.map_raw(self, VirtAddr(0xFFFFF000), raw_table_entry, true)?;

        // Calculate the virtual address based on the offset from the start of the page
        let virt_addr = 0xFFFFF000 + (phys_addr.0 - phys_addr_page);
        Some(virt_addr as *mut u8)
    }

    fn allocate_phys_mem(&mut self, layout: Layout) -> Option<PhysAddr> {
        let addr = self.memory_ranges.allocate(layout.size().try_into().ok()?,
            layout.align().try_into().ok()?);

        addr.map(|x| PhysAddr(x))
    }

    fn release_phys_mem(&mut self, phys_addr: PhysAddr, layout: Layout) {
        self.memory_ranges.remove(InclusiveRange {
            start: phys_addr.0,
            end: phys_addr.0 + (layout.size() - 1) as u32
        });
    }
}

/// Global to hold the `RangeSet` of available physical memory
pub static PHYS_MEM: LockCell<Option<PhysicalMemory>> = LockCell::new(None);
/// Global to hold the `PageDirectory` which manages pages
pub static PAGES: LockCell<Option<PageDirectory>> = LockCell::new(None);

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
    	if let Some(addr) = pmem.as_mut().unwrap().memory_ranges.allocate(size, align) {
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
            free_mem.memory_ranges.insert(InclusiveRange {
                start: ptr as u32,
                end: ptr as u32 + (layout.size() as u32 - 1)
            });
        } else {
            panic!("Attempt to dealloc before memory manager was initialized");
        }
    }
}

/// Initializes the memory manager and unmaps the temp identity map
pub fn init(boot_args: &BootArgs) {
    // Grab the physical memory and page locks
    let mut pmem = PHYS_MEM.lock();
    let mut pages = PAGES.lock();

    // Get the CR3 set by the bootloader which is the base address of the page directory
    let cr3 = unsafe { cpu::get_cr3() as u32 };

    // Setup the physical memory based on the boot args
    let mut phys_mem = PhysicalMemory{
        memory_ranges: boot_args.free_memory,
        page_directory_paddr: PhysAddr(cr3 & !0xFFF),
        last_page_table_paddr: boot_args.last_page_table_paddr
    };
    
    // Setup the page directory
    let mut page_directory = unsafe { PageDirectory::from_cr3(cr3) };

    
    // Unmap the temp identity map of the first physical 1MiB
    for paddr in (0..(1024*1024)).step_by(4096) {
        // TODO: Make a dedicated unmapping procedure which releases empty page tables
        unsafe {
            page_directory.map_raw(&mut phys_mem, VirtAddr(paddr), 0, true)
            .expect("Failed to unmap identity map");
        }
    }
    
    *pmem = Some(phys_mem);
    *pages = Some(page_directory);
}