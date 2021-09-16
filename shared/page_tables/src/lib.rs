//! Provides management of x86 32-bit paging

#![no_std]

use core::alloc::Layout;

pub const PAGE_ENTRY_PRESENT: u32   = 1<<0;
pub const PAGE_ENTRY_WRITE: u32     = 1<<1;
pub const PAGE_ENTRY_USER: u32      = 1<<2;
pub const PAGE_ENTRY_PWT: u32       = 1<<3; // Page-level write-through
pub const PAGE_ENTRY_PCD: u32       = 1<<4; // Page-level cache disable

/// Strongly typed physical address to diffreniate addresses
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysAddr(pub u32);

/// Strongly typed virtual address to diffreniate addresses
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtAddr(pub u32);

pub trait PhysMem {
    /// If successful, returns a virtual address which maps to the physical address and is valid for
    /// at least `size` bytes.
    /// A translation might require remapping of virtual memory, which only happens if `page_dir`
    /// is not `None`.
    ///
    /// ### Safety
    /// The address is only guaranteed to be valid until the next call to `translate_phys`.
    unsafe fn translate_phys(&mut self, page_dir: Option<&mut PageDirectory>, phys_addr: PhysAddr,
        size: usize) -> Option<*mut u8>; // TODO: May need to think about this, if the address is
    // only valid until the next call to `translate_phys`, this is not thread-safe. Do we need to
    // worry about this, or are page mappings only ever made in a single context at a time?

    /// Allocates physical memory with the requested `layout`
    fn allocate_phys_mem(&mut self, layout: Layout) -> Option<PhysAddr>;

    /// Releases physical memory allocated with `allocate_phys_mem`
    fn release_phys_mem(&mut self, phys_addr: PhysAddr, size: usize);

    /// Same as `allocate_phys_mem` except the memory is also zeroed. A reference to `page_dir` is
    /// required if the zero-ing of memory would require to map the memory in.
    /// Calls `translate_phys`, so past translations are invalidated.
    fn allocate_zeroed_phys_mem(&mut self, page_dir: Option<&mut PageDirectory>, layout: Layout)
        -> Option<PhysAddr> {
        // Allocate the memory
        let phys_addr = self.allocate_phys_mem(layout)?;

        unsafe {
            // Get a virtual address to the allocation
            let virt_addr = self.translate_phys(page_dir, phys_addr, layout.size()).or_else(|| {
                // Translation of the address failed and so we can not zero the memory, but before
                // we exit with failure, we need to release the physical memory we allocated
                self.release_phys_mem(phys_addr, layout.size());
                
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
        let directory_layout = Layout::from_size_align(4096, 4096).unwrap();
        let directory = phys_mem.allocate_zeroed_phys_mem(None, directory_layout)?;
        Some(PageDirectory { directory })
    }

    /// Creates a page directory from an existing CR3
    ///
    /// ### Safety
    /// The CR3 value must be a valid page-directory physical address
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
    #[must_use]
    pub fn map(&mut self, phys_mem: &mut impl PhysMem, virt_addr: VirtAddr, size: u32,
        write: bool, user: bool) -> Option<()> {
        self.map_internal(phys_mem, virt_addr, size, write, user, None::<fn(usize) -> u8>)
    }

    /// Maps at least `size` bytes at virtual address `virt_addr` to physical memory with permissions
    /// `write` and `user`.
    /// In practice, this maps all the pages that containg the `size` bytes.
    /// Each byte in the pages containing the requested bytes will be initialized by calling `init`
    /// with its offset.
    #[must_use]
    pub fn map_init<F>(&mut self, phys_mem: &mut impl PhysMem, virt_addr: VirtAddr,
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
    #[must_use]
    fn map_internal<F>(&mut self, phys_mem: &mut impl PhysMem, virt_addr: VirtAddr, size: u32,
        write: bool, user: bool, init: Option<F>) -> Option<()> 
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
            let page_layout = Layout::from_size_align(4096, 4096).unwrap();
            let physical_page = phys_mem.allocate_phys_mem(page_layout)?;

            // Check if we need to initialize
            if let Some(init_bytes) = &init {
                // Calculate the virtul address of the page
                let page_vaddr = page << 12;
                // Calculate the offset of the page from the original address
                let page_offset: usize = (page_vaddr - virt_addr.0) as usize;

                // Get a pointer to the memory we just allocated for the page
                let page_slice = unsafe { 
                    let page_ptr = phys_mem.translate_phys(Some(self), physical_page, 4096)?;
                    core::slice::from_raw_parts_mut(page_ptr, 4096)
                };

                for (byte_offset, byte) in page_slice.iter_mut().enumerate() {
                    // For each byte in the page, get its initial value from the closure
                    *byte = init_bytes(page_offset + byte_offset);
                }
            }

            // Make the virtual address mapping
            let page_virt_addr = VirtAddr(page << 12);
            self.map_to_phys_page(phys_mem, page_virt_addr, physical_page, write, user, false,
                true)?;
        }

        Some(())
    }

    /// Maps the virtual page at `virt_addr` to the physical page at `phys_addr` with the specified
    /// permissions `write` and `user`. If `update` is false, this will not overwrite an existing
    /// mapping. If `cacheable` is false, the mapping will be marked as 'Strong Uncacheable (UC)'.
    #[must_use]
    pub fn map_to_phys_page(&mut self, phys_mem: &mut impl PhysMem, virt_addr: VirtAddr,
        phys_addr: PhysAddr, write: bool, user: bool, update: bool, cacheable: bool) -> Option<()> {
        // Make sure that the requested virtual address is aligned to a page
        if (virt_addr.0 & 0xfff) != 0 {
            return None;
        }

        // Make sure that the requested physical address is aligned to a page
        if (phys_addr.0 & 0xfff) != 0 {
            return None;
        }

        // Build the page table entry
        let mut raw_page_table_entry = phys_addr.0 | PAGE_ENTRY_PRESENT;
        if write {
            raw_page_table_entry |= PAGE_ENTRY_WRITE;
        }
        if user {
            raw_page_table_entry |= PAGE_ENTRY_USER;
        }
        if !cacheable {
            // TODO: This is an extremely simplistic implementation which toggles between the
            // default state and UC, which I added when working on the local APIC. I should read the
            // relevant chapters in the manual (for future reference, 4.9, 4.10, 11) and update this
            // for increased performance where I can enable some caching (i.e. screen buffers)
            raw_page_table_entry |= PAGE_ENTRY_PWT | PAGE_ENTRY_PCD;
        }

        // Make the virtual address mapping
        unsafe {
            self.map_raw(phys_mem, virt_addr, raw_page_table_entry, update, true)?;
        }

        Some(())
    }

    /// Set the page table entry for `virt_addr` to be `raw`. If `update` is false, this will not
    /// overwrite an existing mapping. If `create` is false, a page table won't be created if it
    /// doesn't exist (and the mapping will not occur).
    ///
    /// ### Safety
    /// `raw` must be a valid page table entry
    #[must_use]
    pub unsafe fn map_raw(&mut self, phys_mem: &mut impl PhysMem, virt_addr: VirtAddr, raw: u32,
        update: bool, create: bool) -> Option<()> {
        // Make sure that the requested virtual address is aligned to a page
        if (virt_addr.0 & 0xfff) != 0 {
            return None;
        }

        // Index of the entry in the page directory
        let directory_index = virt_addr.0 >> 22;
        // Index of the entry in the page table
        let table_index = (virt_addr.0 >> 12) & 0x3FF;

        // Compute the physical address of the PDE
        let directory_entry_paddr = PhysAddr(self.directory.0 + directory_index * 4);
        // Get the entry in the directory by translating the physical address to a virtual one
        let mut directory_entry = 
            *(phys_mem.translate_phys(Some(self), directory_entry_paddr, 4)? as *const u32);

        // Check if the PDE is not present (i.e. the page table doesn't exist)
        if (directory_entry & PAGE_ENTRY_PRESENT) == 0 {
            if !create  {
                // If the page table doesn't exist and we were asked not to create one there is
                // nothing to do
                return None;
            }

            // We need to add a new page table, so we allocate an aligned page
            let table_layout = Layout::from_size_align(4096, 4096).unwrap();
            let new_table = phys_mem.allocate_zeroed_phys_mem(Some(self), table_layout)?;

            // Update the PDE
            directory_entry = new_table.0 | PAGE_ENTRY_USER | PAGE_ENTRY_WRITE | PAGE_ENTRY_PRESENT;
            let entry_vaddr = phys_mem.translate_phys(Some(self), directory_entry_paddr, 4)?;
            *(entry_vaddr as *mut u32) = directory_entry;
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
        
        Some(())
    }

    /// Set the page table entry for `virt_addr` to be `raw`. If `update` is false, this will not
    /// overwrite an existing mapping. The page table of the specified page must be mapped at the
    /// virtual address `page_table_vaddr`.
    /// 
    /// The function will return `None` if the mapping was not updated for any reason.
    ///
    /// ### Safety
    /// `raw` must be a valid page table entry
    #[must_use]
    pub unsafe fn map_raw_directly(&mut self, virt_addr: VirtAddr, raw: u32, update: bool,
        page_table_vaddr: VirtAddr) -> Option<()> {
        // Make sure that the requested virtual address is aligned to a page
        if (virt_addr.0 & 0xfff) != 0 {
            return None;
        }

        // Index of the entry in the page table
        let table_index = (virt_addr.0 >> 12) & 0x3FF;

        // Compute the virtual address of the PTE
        let table_entry_vaddr = page_table_vaddr.0.checked_add(table_index * 4)?;
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

        Some(())
    }

    /// Returns the physical address of the page table which is responisble for the page at `virt_addr`.
    /// If the page table does not exist it will be created (and the relevant PDE will be updated)
    pub fn get_page_table(&mut self, phys_mem: &mut impl PhysMem, virt_addr: VirtAddr)
        -> Option<PhysAddr> {
        // Make sure that the requested virtual address is aligned to a page
        if (virt_addr.0 & 0xfff) != 0 {
            return None;
        }

        // Index of the entry in the page directory
        let directory_index = virt_addr.0 >> 22;

        // Compute the physical address of the PDE
        let directory_entry_paddr = PhysAddr(self.directory.0 + directory_index * 4);
        // Get the entry in the directory by translating the physical address to a virtual one
        let directory_entry = unsafe {
            *(phys_mem.translate_phys(Some(self), directory_entry_paddr, 4)? as *const u32)
        };

        // Check if the PDE is not present (i.e. the page table doesn't exist)
        if (directory_entry & PAGE_ENTRY_PRESENT) == 0 {

            // We need to add a new page table, so we allocate an aligned page
            let table_layout = Layout::from_size_align(4096, 4096).unwrap();
            let new_table = phys_mem.allocate_zeroed_phys_mem(Some(self), table_layout)?;

            // Update the PDE
            let directory_entry =
                new_table.0 | PAGE_ENTRY_USER | PAGE_ENTRY_WRITE | PAGE_ENTRY_PRESENT;
            unsafe {
                let entry_vaddr = phys_mem.translate_phys(Some(self), directory_entry_paddr, 4)?;
                *(entry_vaddr as *mut u32) = directory_entry;
            }

            Some(new_table)
        } else {
            // If the page table already exists, we just grab its physical address by masking off
            // the PDE flags
            Some(PhysAddr(directory_entry & !0xfff))
        }
    }

    /// Unmaps the page at `virt_addr`. If the page table containing this page becomes empty as a
    /// result of this, it will be freed. If `free_page` is true, the physical page will also be
    /// freed.
    #[must_use]
    pub fn unmap(&mut self, phys_mem: &mut impl PhysMem, virt_addr: VirtAddr, free_page: bool)
        -> Option<()> {
        // Make sure that the requested virtual address is aligned to a page
        if (virt_addr.0 & 0xfff) != 0 {
            return None;
        }

        // Check if we need to free the physical page before unmapping it
        if free_page {
            // Get the physical address of the page
            let page_paddr = self.translate_virt(phys_mem, virt_addr)?;
            // Release the page
            phys_mem.release_phys_mem(page_paddr, 4096);
        }
        
        unsafe {
            // Set the page entry as not present
            self.map_raw(phys_mem, virt_addr, 0, true, false)?;
        }

        // Index of the entry in the page directory
        let directory_index = virt_addr.0 >> 22;
        
        // Compute the physical address of the PDE
        let directory_entry_paddr = PhysAddr(self.directory.0 + directory_index * 4);
        // Get the entry in the directory
        let directory_entry = unsafe { 
            // Translate the physical address into a virtual address
            let directory_entry_vaddr = 
                phys_mem.translate_phys(Some(self), directory_entry_paddr, 4)?;

            *(directory_entry_vaddr as *const u32)
        };

        // Calculate the physical address of the relevant page table
        let table_paddr = directory_entry & !0xfff;

        for table_index in 0..1024 {
            // Compute the physical address of the PTE
            let table_entry_paddr = PhysAddr(table_paddr + table_index * 4);
            // Translate it into a virtual address
            let table_entry_vaddr = unsafe {
                phys_mem.translate_phys(Some(self), table_entry_paddr, 4)?
            };
            // Get the entry in the table
            let table_entry = unsafe { *(table_entry_vaddr as *const u32) };

            if (table_entry & PAGE_ENTRY_PRESENT) != 0 {
                // The PTE is present, i.e. the page is mapped so this page table is not empty
                // and there is nothing left to do
                return Some(());
            }
        }

        // If we exited the loop every page table entry is not present, so this table can be freed
        phys_mem.release_phys_mem(PhysAddr(table_paddr), 4096);
        
        // We also need to mark the PDE as not present
        unsafe {
            // Translate the physical address into a virtual address
            let directory_entry_vaddr = 
                phys_mem.translate_phys(Some(self), directory_entry_paddr, 4)?;

            *(directory_entry_vaddr as *mut u32) = 0;
        }

        Some(())
    }

    /// Translates the virtual address `virt_addr` into the corresponding physical address based on
    /// the page tables.
    pub fn translate_virt(&mut self, phys_mem: &mut impl PhysMem, virt_addr: VirtAddr)
        -> Option<PhysAddr> {
        // Index of the entry in the page directory
        let directory_index = virt_addr.0 >> 22;
        // Index of the entry in the page table
        let table_index = (virt_addr.0 >> 12) & 0x3FF;

        // Compute the physical address of the PDE
        let directory_entry_paddr = PhysAddr(self.directory.0 + directory_index * 4);
        // Get the entry in the directory
        let directory_entry = unsafe {
            // Translate the physical address into a virtual address
            let directory_entry_vaddr =
                phys_mem.translate_phys(Some(self), directory_entry_paddr, 4)?;
            *(directory_entry_vaddr as *const u32)
        };

        // Check if the PDE is not present (i.e. the page table doesn't exist)
        if (directory_entry & PAGE_ENTRY_PRESENT) == 0 {
            return None;
        }

        // Compute the physical address of the PTE
        let table_entry_paddr = PhysAddr((directory_entry & !0xfff) + table_index * 4);
        // Get the entry in the table
        let table_entry = unsafe {
            // Translate the physical address into a virtual address
            let table_entry_vaddr = phys_mem.translate_phys(Some(self), table_entry_paddr, 4)?;
            *(table_entry_vaddr as *const u32)
        };

        // Check if the PTE is present (i.e. the page is already mapped)
        if (table_entry & PAGE_ENTRY_PRESENT) != 0 {
            // Calculate the physical address by adding the page address from the PTE and the page
            // offset from the virtual address
            let paddr = (table_entry & !0xFFF) + (virt_addr.0 & 0xFFF);
            Some(PhysAddr(paddr))
        } else {
            None
        }
    }
}