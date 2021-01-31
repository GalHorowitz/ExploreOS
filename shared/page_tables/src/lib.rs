//! Provides management of x86 32-bit paging

#![no_std]

use core::alloc::Layout;

pub const PAGE_ENTRY_PRESENT: u32   = 1<<0;
pub const PAGE_ENTRY_WRITE: u32     = 1<<1;
pub const PAGE_ENTRY_USER: u32      = 1<<2;

/// Strongly typed physical address to diffreniate addresses
#[derive(Clone, Copy)]
pub struct PhysAddr(pub u32);

/// Strongly typed virtual address to diffreniate addresses
#[derive(Clone, Copy)]
pub struct VirtAddr(pub u32);

pub trait PhysMem {
    /// If successful, returns a virtual address which maps to the physical address and is valid for
    /// at least `size` bytes.
    /// A translation might require remapping of virtual memory, which only happens if `page_dir`
    /// is not `None`.
    /// The address is only guaranteed to be valid until the next call to `translate_phys`.
    unsafe fn translate_phys(&mut self, page_dir: Option<&mut PageDirectory>, phys_addr: PhysAddr,
        size: usize) -> Option<*mut u8>;

    /// Allocates physical memory with the requested `layout`
    fn allocate_phys_mem(&mut self, layout: Layout) -> Option<PhysAddr>;

    /// Releases physical memory allocated with `allocate_phys_mem`
    fn release_phys_mem(&mut self, phys_addr: PhysAddr, layout: Layout);

    /// Same as `allocate_phys_mem` except the memory is also zeroed. A reference to `page_dir` is
    /// required if the zero-ing of memory would require to map the memory in.
    fn allocate_zeroed_phys_mem(&mut self, page_dir: Option<&mut PageDirectory>, layout: Layout)
        -> Option<PhysAddr> {
        // Allocate the memory
        let phys_addr = self.allocate_phys_mem(layout)?;

        unsafe {
            // Get a virtual address to the allocation
            let virt_addr = self.translate_phys(page_dir, phys_addr, layout.size()).or_else(|| {
                // Translation of the address failed and so we can not zero the memory, but before
                // we exit with failure, we need to release the physical memory we allocated
                self.release_phys_mem(phys_addr, layout);
                
                None
            })?;
            // Zero it out
            core::ptr::write_bytes(virt_addr, 0, layout.size());
        }

        Some(phys_addr)
    }
}

/// A 32-bit x86 page directory
pub struct PageDirectory {
    // The physical address of the page directory, i.e. the address stored in CR3
    directory: PhysAddr
}

impl PageDirectory {
    // Creates a new empty page table
    pub fn new(phys_mem: &mut impl PhysMem) -> Option<Self> {
        // Allocate a page-aligned page directory
        let directory_layout = Layout::from_size_align(4096, 4096).ok()?;
        let directory = phys_mem.allocate_zeroed_phys_mem(None, directory_layout)?;
        Some(PageDirectory { directory })
    }

    /// Creates a page directory from an existing CR3
    pub unsafe fn from_cr3(cr3: u32) -> Self {
        // We mask off the lower 12 bits of cr3 to get the address of the page directory
        PageDirectory { directory: PhysAddr(cr3 & !0xfff) }
    }

    /// Get the physical address of the base page directory
    pub fn get_directory_addr(&self) -> PhysAddr {
        self.directory
    }

    /// Maps at least `size` bytes at virtual address `virt_addr` to physical memory with permissions
    /// `write` and `user`.
    /// In practice, this maps all the pages that containg the `size` bytes.
    /// The bytes are uninitialized.
    pub unsafe fn map(&mut self, phys_mem: &mut impl PhysMem, virt_addr: VirtAddr, size: u32,
        write: bool, user: bool) -> Option<()> {
        self.map_internal(phys_mem, virt_addr, size, write, user, None::<fn(usize) -> u8>)
    }

    /// Maps at least `size` bytes at virtual address `virt_addr` to physical memory with permissions
    /// `write` and `user`.
    /// In practice, this maps all the pages that containg the `size` bytes.
    /// Each byte in the pages containing the requested bytes will be initialized by calling `init`
    /// with its offset.
    pub unsafe fn map_init<F>(&mut self, phys_mem: &mut impl PhysMem, virt_addr: VirtAddr,
        size: u32, write: bool, user: bool, init: F) -> Option<()>
        where F: Fn(usize) -> u8 {
        self.map_internal(phys_mem, virt_addr, size, write, user, Some(init))
    }

    /// Maps at least `size` bytes at virtual address `virt_addr` to physical memory with permissions
    /// `write` and `user`.
    /// In practice, this maps all the pages that containg the `size` bytes.
    /// 
    /// If `init` is not None, Each byte in the pages containing the requested bytes will be
    /// initialized by calling `init` with its offset.
    unsafe fn map_internal<F>(&mut self, phys_mem: &mut impl PhysMem, virt_addr: VirtAddr, size: u32, write: bool,
        user: bool, init: Option<F>) -> Option<()> 
        where F: Fn(usize) -> u8 {
        // Make sure the size is not zero and that the virtual address is page-aligned
        if size == 0 || virt_addr.0 & 0xfff != 0 {
            return None;
        }

        // Calculate the page of the first address and the page of the last address
        let first_addr_page = (virt_addr.0) >> 12;
        let last_addr = (virt_addr.0).checked_add(size - 1)?;
        let last_addr_page = last_addr >> 12;
        
        // Iterate through each page containing the `size` bytes
        for page in first_addr_page..=last_addr_page {
            // Allocate page-aligned pysical memory for the page
            let page_layout = Layout::from_size_align(4096, 4096).ok()?;
            let physical_page = phys_mem.allocate_phys_mem(page_layout)?;

            // Check if we need to initialize
            if let Some(init_bytes) = &init {
                // Calculate the virtul address of the page
                let page_vaddr = page << 12 as usize;
                // Calculate the offset of the page from the original address
                let page_offset: usize = (page_vaddr - virt_addr.0) as usize;

                // Get a pointer to the memory we just allocated for the page
                let page_ptr = phys_mem.translate_phys(Some(self), physical_page, 4096)?;

                for byte_offset in 0..4096 {
                    // For each byte in the page, get its initial value from the closure
                    let init_byte = init_bytes(page_offset + byte_offset);
                    // Write the initial value
                    core::ptr::write(page_ptr.offset(byte_offset as isize), init_byte);
                }
            }
            
            // Build the page table entry
            let mut raw_page_table_entry = physical_page.0 | PAGE_ENTRY_PRESENT;
            if write {
                raw_page_table_entry |= PAGE_ENTRY_WRITE;
            }
            if user {
                raw_page_table_entry |= PAGE_ENTRY_USER;
            }

            // Make the virtual address mapping
            self.map_raw(phys_mem, VirtAddr(page << 12), raw_page_table_entry, false)?;
        }

        Some(())
    }

    /// Set the page table entry for `virt_addr` to be `raw`. If `update` is false, this will not
    /// overwrite an existing mapping.
    /// 
    /// If the page directory entry (and matching page table) doesn't exist it will be created.
    /// The function will return the physical address of the page table, or `None` if the mapping
    /// was not updated for any reason.
    pub unsafe fn map_raw(&mut self, phys_mem: &mut impl PhysMem, virt_addr: VirtAddr, raw: u32,
        update: bool) -> Option<PhysAddr> {
        // Make sure that the requested virtual address is aligned to to a page
        if (virt_addr.0 & 0xfff) != 0 {
            return None;
        }

        // Index of the entry in the page directory
        let directory_index = virt_addr.0 >> 22;
        // Index of the entry in the page table
        let table_index = (virt_addr.0 >> 12) & 0x3FF;

        // Compute the physical address of the PDE
        let directory_entry_paddr = PhysAddr(self.directory.0 + directory_index * 4);
        // Translate it into a virtual address
        let directory_entry_vaddr = phys_mem.translate_phys(Some(self), directory_entry_paddr, 4)?;
        // Get the entry in the directory
        let mut directory_entry = *(directory_entry_vaddr as *const u32);

        // Check if the PDE is not present (i.e. the page table doesn't exist)
        if (directory_entry & PAGE_ENTRY_PRESENT) == 0 {
            // We need to add a new page table, so we allocate an aligned page
            let table_layout = Layout::from_size_align(4096, 4096).ok()?;
            let new_table = phys_mem.allocate_zeroed_phys_mem(Some(self), table_layout)?;

            // Update the PDE
            directory_entry = new_table.0 | PAGE_ENTRY_USER | PAGE_ENTRY_WRITE | PAGE_ENTRY_PRESENT;
            *(directory_entry_vaddr as *mut u32) = directory_entry;
        }

        // Compute the physical address of the PTE
        let table_entry_paddr = PhysAddr((directory_entry & !0xfff) + table_index * 4);
        // Translate it into a virtual address
        let table_entry_vaddr = phys_mem.translate_phys(Some(self), table_entry_paddr, 4)?;
        // Get the entry in the table
        let table_entry = *(table_entry_vaddr as *const u32);

        // Check if the PTE is present (i.e. the page is already mapped)
        if (table_entry & PAGE_ENTRY_PRESENT) != 0 && !update {
            // The page is already mapped, and `update` is false, so there is nothing to do
            return None;
        }

        // Update the table entry
        *(table_entry_vaddr as *mut u32) = raw;

        // The entry already existed, so we need to invalidate any cached translations
        if (table_entry & PAGE_ENTRY_PRESENT) != 0 {
            cpu::invlpg(virt_addr.0 as usize);
        }
        
        Some(PhysAddr(directory_entry & !0xfff))
    }
}