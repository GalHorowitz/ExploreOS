//! Basic graphics frame buffer management

// FIXME: NOT THREAD SAFE

use core::mem::size_of;

use exclusive_cell::ExclusiveCell;
use page_tables::{VirtAddr, PhysAddr};

const SCREEN_BUFFER_VADDR: u32 = 0xCB800000;

pub struct FrameBuffer {
	pub width: usize,
	pub height: usize
}

impl FrameBuffer {
	/// Returns a slice of the screen buffer
	pub fn get_buffer(&mut self) -> &mut [u32] {
		unsafe {
			core::slice::from_raw_parts_mut(SCREEN_BUFFER_VADDR as *mut u32, self.width*self.height)
		}
	}

	pub const fn get_size(&self) -> usize {
		return self.width * self.height * size_of::<u32>();
	}
}

pub static FRAME_BUFFER: ExclusiveCell<Option<FrameBuffer>> = ExclusiveCell::new(None);

/// Initializes the screen
pub fn init(screen_buffer_paddr: PhysAddr, screen_width: u16, screen_height: u16) {
    {
		// Get access to physical memory and the page directory
		let mut pmem = crate::memory_manager::PHYS_MEM.lock();
		let (phys_mem, page_dir) = pmem.as_mut().unwrap();

		// Map the screen buffer so we can write to it
		let buffer_size = (screen_width as usize) * (screen_height as usize) * size_of::<u32>();
		let buffer_page_count = buffer_size.div_ceil(4096) as u32;
		for i in 0..buffer_page_count {
			page_dir.map_to_phys_page(phys_mem, VirtAddr(SCREEN_BUFFER_VADDR + 4096*i),
				PhysAddr(screen_buffer_paddr.0 + 4096*i), true, false, true, false)
				.expect("Failed to map screen buffer");
		}
	}

	{
		let mut fb = FRAME_BUFFER.acquire();
		assert!(fb.is_none());
		*fb = Some(FrameBuffer { width: screen_width as usize, height: screen_height as usize });
	}
}