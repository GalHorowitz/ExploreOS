//! Responisble for physical and virtual memory management

use core::convert::TryInto;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::alloc::{GlobalAlloc, Layout};

use range_set::{RangeSet, InclusiveRange};
use page_tables::{PAGE_ENTRY_PRESENT, PAGE_ENTRY_WRITE, PageDirectory, PhysAddr, PhysMem, VirtAddr};
use lock_cell::LockCell;
use boot_args::{BootArgs, LAST_PAGE_TABLE_VADDR, KERNEL_ALLOCATIONS_BASE_VADDR};

/// Global to hold the `RangeSet` of available physical memory.
/// IMPORTANT: While maskable hardware interrupts are masked while this lock is held, care must be
/// taken to not create dead-locks when using this in non-maskable interrupts like NMIs and
/// exceptions.
pub static PHYS_MEM: LockCell<Option<PhysicalMemory>> = LockCell::new(None, true);
/// Global to hold the `PageDirectory` which manages pages.
/// IMPORTANT: While maskable hardware interrupts are masked while this lock is held, care must be
/// taken to not create dead-locks when using this in non-maskable interrupts like NMIs and
/// exceptions.
pub static PAGES: LockCell<Option<PageDirectory>> = LockCell::new(None, true);
// FIXME: When accessing pages we almost always need access to physical memory too, and currently
// I make sure to always grab the lock on physical memory first, but this is error-prone and if one
// thread grabs the PAGES lock first and another grabs the PHYS_MEM lock first we could dead-lock

static NEXT_AVAILABLE_VADDR: AtomicUsize = AtomicUsize::new(KERNEL_ALLOCATIONS_BASE_VADDR as usize);

pub struct PhysicalMemory{
    memory_ranges: RangeSet,
    last_page_table_paddr: PhysAddr,
    current_phys_mapping: Option<PhysAddr>
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

        // Small optimization: If we ever need to acces the physical page of the last page table, we
        // can use the permanent mapping we make anyway.
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

        // Calculate the address of the page containing the physical address
        let phys_addr_page = phys_addr.0 & !0xFFF;

        if self.current_phys_mapping.is_none()
            || self.current_phys_mapping.unwrap().0 != phys_addr_page {
            // If the physical address is not already mapped in, we must make a mapping for it, so we
            // need access to the page directory struct
            let page_dir = page_dir?;

            // Make sure the requested physical window does not extend beyond this one page. This should
            // not be a problem: the page table functions only ever use this to read and write to page
            // tables which are one page long.
            if phys_addr.0.checked_add(size as u32 - 1)? > (phys_addr_page + 4095) {
                return None;
            }

            // Make the mapping of the last virtual page (0xFFFFF000-0xFFFFFFFF) to the physical page.
            // It is critical we use the `map_raw_directly` method, which uses the virtual address we
            // provide to it, instead of asking for a virtual address from this function, which would
            // cause an inifnite loop
            let raw_pte = PAGE_ENTRY_PRESENT | PAGE_ENTRY_WRITE | phys_addr_page;
            page_dir.map_raw_directly(VirtAddr(0xFFFFF000), raw_pte, true,
                VirtAddr(LAST_PAGE_TABLE_VADDR));   
            self.current_phys_mapping = Some(PhysAddr(phys_addr_page));
        }
        

        // Calculate the virtual address based on the offset from the start of the page
        let virt_addr = 0xFFFFF000 + (phys_addr.0 - phys_addr_page);
        Some(virt_addr as *mut u8)
    }

    fn allocate_phys_mem(&mut self, layout: Layout) -> Option<PhysAddr> {
        let addr = self.memory_ranges.allocate(layout.size().try_into().ok()?,
            layout.align().try_into().ok()?);
        
        addr.map(|x| PhysAddr(x))
    }

    fn release_phys_mem(&mut self, phys_addr: PhysAddr, size: usize) {
        if size == 0 {
            return;
        }

        self.memory_ranges.insert(InclusiveRange {
            start: phys_addr.0,
            end: phys_addr.0.saturating_add((size - 1) as u32)
        });
    }
}

/// The global allocator for the bootloader
#[global_allocator]
static GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator;

/// Dummy struct to implement `GlobalAlloc` on
struct GlobalAllocator;

impl GlobalAllocator {
    unsafe fn alloc_internal(&self, layout: Layout) -> Option<*mut u8> {
        // The `RangeSet` allocator only supports 32-bit
        let size: u32 = layout.size().try_into().ok()?;
        let _align: u32 = layout.align().try_into().ok()?;

        // We currently just rely on the fact that we allocate pages which are page-aligned, so any
        // request with alignment larger than 4096 can not actually be fulfilled.
        assert!(layout.align() <= 4096);

        // Round up the size to the next multiple of a page
        let aligned_size = (size.checked_add(4095)?) & !0xFFF;
        // Grab a virtual address for this allocation
        let virt_addr = NEXT_AVAILABLE_VADDR.fetch_add(aligned_size as usize, Ordering::SeqCst);

        // Get access to physical memory
        let mut pmem = PHYS_MEM.lock();
        let phys_mem = pmem.as_mut()?;

        // Get access to the page directory
        let mut pages = PAGES.lock();
        let page_dir = pages.as_mut()?;

        // Map the memory for the allocation
        page_dir.map(phys_mem, VirtAddr(virt_addr as u32), aligned_size as u32, true, false)?;

        Some(virt_addr as *mut u8)
    }

    fn dealloc_internal(&self, ptr: *mut u8, layout: Layout) -> Option<()> {
        if layout.size() == 0 {
            panic!("Attempt to dealloc a zero sized allocation");
        }

        // Get access to physical memory
        let mut pmem = PHYS_MEM.lock();
        let phys_mem = pmem.as_mut()?;

        // Get access to the page directory
        let mut pages = PAGES.lock();
        let page_dir = pages.as_mut()?;

        // Calculate the first and last virtual address so we can iterate over the pages
        let start_vaddr = ptr as u32;
        let last_vaddr = (ptr as u32).checked_add(layout.size() as u32 - 1)?;

        // Our allocator always gives page aligned virtual addressed, so we can rely on this
        // assumption, but we assert for future-proofing
        assert!(start_vaddr & 0xFFF == 0);

        for vaddr in (start_vaddr..last_vaddr).step_by(4096) {
            // Go through each page in the allocation and unmap it (while freeing the backing
            // physical page)
            page_dir.unmap(phys_mem, VirtAddr(vaddr), true)?;
        }

        Some(())
    }
}

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_internal(layout).unwrap_or(core::ptr::null_mut())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        assert!(self.dealloc_internal(ptr, layout).is_some());
    }
}

/// Initializes the memory manager and unmaps the temp identity map
pub fn init(boot_args: &BootArgs) {
    // Grab the physical memory and page locks
    let mut pmem = PHYS_MEM.lock();
    let mut pages = PAGES.lock();

    // Get the CR3 set by the bootloader which is the base address of the page directory
    let cr3 = cpu::get_cr3() as u32;

    // Setup the physical memory based on the boot args
    let mut phys_mem = PhysicalMemory{
        memory_ranges: boot_args.free_memory,
        last_page_table_paddr: boot_args.last_page_table_paddr,
        current_phys_mapping: None
    };
    
    // Setup the page directory
    let mut page_directory = unsafe { PageDirectory::from_cr3(cr3) };

    
    // Unmap the temp identity map of the first physical 1MiB
    for paddr in (0..(1024*1024)).step_by(4096) {
        page_directory.unmap(&mut phys_mem, VirtAddr(paddr), false);
    }
    
    *pmem = Some(phys_mem);
    *pages = Some(page_directory);
}