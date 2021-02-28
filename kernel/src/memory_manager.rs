//! Responisble for physical and virtual memory management

use core::convert::TryInto;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::alloc::{GlobalAlloc, Layout};

use range_set::{RangeSet, InclusiveRange};
use page_tables::{PAGE_ENTRY_PRESENT, PAGE_ENTRY_WRITE, PageDirectory, PhysAddr, PhysMem, VirtAddr};
use lock_cell::LockCell;
use boot_args::{BootArgs, LAST_PAGE_TABLE_VADDR, KERNEL_ALLOCATIONS_BASE_VADDR};

/// Global to hold the `RangeSet` of available physical memory and the `PageDirectory` which manages
/// page mappings.
/// IMPORTANT: While maskable hardware interrupts are masked while this lock is held, care must be
/// taken to not create dead-locks when using this in non-maskable interrupts like NMIs and
/// exceptions.
pub static PHYS_MEM: LockCell<Option<(PhysicalMemory, PageDirectory)>> = LockCell::new(None);

/// A struct that implements `PhysMem` for use in mappings
pub struct PhysicalMemory{
    /// Actual usable ranges of physical memory
    memory_ranges: RangeSet,

    /// The physical address of the page table of the last page (Used when accessing physical mem)
    last_page_table_paddr: PhysAddr,

    /// The current physical mapping in the last page (That is used to access physical memory)
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

struct FreePagesEntry {
    page_count: usize,
    next: Option<*mut FreePagesEntry>,
}

static NEXT_AVAILABLE_VADDR: AtomicUsize = AtomicUsize::new(KERNEL_ALLOCATIONS_BASE_VADDR as usize);
static FREE_PAGES_LIST: LockCell<Option<*mut FreePagesEntry>> = LockCell::new(None);

/// The global allocator for the bootloader
#[global_allocator]
static GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator;

/// Dummy struct to implement `GlobalAlloc` on
struct GlobalAllocator;

impl GlobalAllocator {
    /// Tries to satisfy the allocation of page-aligned size `aligned_size` using the free list.
    /// Returns `None` if not successful
    fn alloc_from_free_list(&self, aligned_size: usize) -> Option<*mut u8> {
        assert!(aligned_size & 0xFFF == 0);

        let mut start_of_free_list = FREE_PAGES_LIST.lock();
        // Check if there are any entries in the free list
        if let Some(free_list) = *start_of_free_list {
            // Calculate the number of pages we need to fit the allocation
            let num_pages_needed = aligned_size / 4096;

            let mut last_entry: Option<*mut FreePagesEntry> = None;
            let mut entry = free_list;
            loop {
                let free_pages = unsafe { core::ptr::read(entry) };

                // We check if we can fit the allocation in this entry
                if num_pages_needed <= free_pages.page_count {
                    if num_pages_needed < free_pages.page_count {
                        // If the allocation is smaller than the size of entry, we just shorten it
                        let new_page_count = free_pages.page_count - num_pages_needed;
                        unsafe {
                            core::ptr::write(entry, FreePagesEntry {
                                page_count: new_page_count,
                                next: free_pages.next
                            });
                        }

                        // And finally we return a pointer to the end of the updated free area
                        return Some(((entry as usize) + (new_page_count*4096)) as *mut u8);
                    } else {
                        // Else, if the entry is completely used up, we need to update the last
                        // entry's next pointer
                        if let Some(last_entry) = last_entry {
                            unsafe {
                                let mut last = core::ptr::read(last_entry);
                                last.next = free_pages.next;
                                core::ptr::write(last_entry, last);
                            }
                        } else {
                            // If we are using the first entry in the list, we need to update the
                            // start-of-the-list pointer
                            *start_of_free_list = free_pages.next;
                        }

                        return Some(entry as *mut u8);
                    }
                } else if free_pages.next.is_some() {
                    // If we can't, but there are more entries in the list, we advance to the next
                    last_entry = Some(entry);
                    entry = free_pages.next.unwrap();
                } else {
                    // If this is the end of the list, we exit the loop
                    break;
                }
            }
        }

        // If we didn't find any free entry that we can use there is nothing to do
        None
    }

    fn alloc_internal(&self, layout: Layout) -> Option<*mut u8> {
        // The `RangeSet` allocator only supports 32-bit
        let _size: u32 = layout.size().try_into().ok()?;
        let _align: u32 = layout.align().try_into().ok()?;

        // We currently just rely on the fact that we allocate pages which are page-aligned, so any
        // request with alignment larger than 4096 can not actually be fulfilled.
        assert!(layout.align() <= 4096);

        // Round up the size to the next multiple of a page
        let aligned_size = (layout.size().checked_add(4095)?) & !0xFFF;

        // If the free pages list is not empty, we check if we can reuse an existing mapping
        if let Some(allocation) = self.alloc_from_free_list(aligned_size) {
            return Some(allocation);
        }

        // Grab a virtual address for this allocation
        let virt_addr = NEXT_AVAILABLE_VADDR.fetch_add(aligned_size, Ordering::SeqCst);

        // Check we have enough room for the allocation
        if virt_addr.checked_add(aligned_size - 1)? >=
            KERNEL_ALLOCATIONS_BASE_VADDR as usize + 0x200000 {
            // TODO: Move the size to a better place
            return None;
        }

        // Get access to physical memory and the page directory
        let mut pmem = PHYS_MEM.lock();
        let (phys_mem, page_dir) = pmem.as_mut()?;

        // Map the memory for the allocation
        page_dir.map(phys_mem, VirtAddr(virt_addr as u32), aligned_size as u32, true, false)?;

        Some(virt_addr as *mut u8)
    }

    fn dealloc_internal(&self, ptr: *mut u8, layout: Layout) -> Option<()> {
        if layout.size() == 0 {
            panic!("Attempt to dealloc a zero sized allocation");
        }

        // Round up the size to the next multiple of a page (which is the actual allocation size
        // our allocator provides)
        let aligned_size = (layout.size().checked_add(4095)?) & !0xFFF;

        let mut start_of_free_list = FREE_PAGES_LIST.lock();

        let mut new_entry_ptr = ptr as *mut FreePagesEntry;
        let mut new_entry = FreePagesEntry {
            page_count: aligned_size / 4096,
            next: *start_of_free_list
        };

        // If the free list is not empty, we need to check if the freed allocation is adjacent to
        // any of the existing free entries and merge them
        if let Some(free_list) = *start_of_free_list {
            let mut last_entry: Option<*mut FreePagesEntry> = None;
            let mut entry = free_list;
            loop {
                let free_pages = unsafe { core::ptr::read(entry) };

                if ptr as usize + aligned_size == entry as usize {
                    // If the freed allocation ends at the start of this free entry, we remove the
                    // existing entry and update our new one
                    new_entry.page_count += free_pages.page_count;

                    if let Some(last_entry) = last_entry {
                        unsafe {
                            let mut last = core::ptr::read(last_entry);
                            last.next = free_pages.next;
                            core::ptr::write(last_entry, last);
                        }
                    } else {
                        // If this is the free entry, we update the list heads
                        *start_of_free_list = free_pages.next;
                    }
                } else if ptr as usize == entry as usize + (free_pages.page_count * 4096) {
                    // Else, if the freed allocation start at the end of this free entry, we remove
                    // the existing entry and update our new one
                    new_entry.page_count += free_pages.page_count;
                    new_entry_ptr = entry;

                    if let Some(last_entry) = last_entry {
                        unsafe {
                            let mut last = core::ptr::read(last_entry);
                            last.next = free_pages.next;
                            core::ptr::write(last_entry, last);
                        }
                    } else {
                        // If this is the free entry, we update the list heads
                        *start_of_free_list = free_pages.next;
                    }
                }

                // If there is another entry in the list we continue to it, else we finish
                if free_pages.next.is_some() {
                    last_entry = Some(entry);
                    entry = free_pages.next.unwrap();
                } else {
                    break;
                }
            }
        }
        
        // Save the list entry at the start of the allocation
        unsafe {
            core::ptr::write(new_entry_ptr, new_entry);
        }
        
        // Update the head of the free list
        *start_of_free_list = Some(new_entry_ptr);

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
    
    *pmem = Some((phys_mem, page_directory));
}