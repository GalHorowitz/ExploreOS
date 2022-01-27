use core::arch::asm;

use alloc::{string::String, vec};
use elf_parser::ElfParser;
use lock_cell::LockCell;
use page_tables::{PageDirectory, VirtAddr, PhysMem};
use cpu::PushADRegisterState;
use crate::{gdt, memory_manager::{self, PhysicalMemory}, tss};


const KERNEL_INTR_STACK_SIZE: u32 = 0x1000;
const USER_STACK_VADDR: VirtAddr = VirtAddr(0x0FFF_F000);
const USER_STACK_SIZE: u32 = 0x1000;
const USER_DEFAULT_EFLAGS: u32 = 0b0000_0000_0000_0000_0000_0010_0000_0010;

pub struct Process {
	page_directory: PageDirectory,
	virtual_memory_ranges: [Option<(VirtAddr, u32, bool, bool)>; 16], // First page vaddr, num pages, write, exec
	kernel_intr_stack: VirtAddr,

	file_descriptors: [Option<usize>; 16],
	pub cwd_inode: u32,

	registers: PushADRegisterState,
	eip: u32,
	eflags: u32,
	in_kernel: bool,

	exit_code: Option<u8>,
}

impl Process {
	pub fn new(kernel_intr_stack: VirtAddr) -> Self {
		let mut pd_buffer = box[0u8; 1024];

		let mut pmem = memory_manager::PHYS_MEM.lock();
		let (phys_mem, cur_page_dir) = pmem.as_mut().unwrap();

		let old_cr3 = cur_page_dir.get_directory_addr();
		let cur_pd = unsafe {
			core::slice::from_raw_parts(phys_mem.translate_phys(old_cr3, 4096).unwrap(), 4096)
		};
		pd_buffer.copy_from_slice(&cur_pd[3072..]);
		
		let mut proc_page_dir = PageDirectory::new(phys_mem).unwrap();
		let new_cr3 = proc_page_dir.get_directory_addr();
		let new_pd = unsafe {
			core::slice::from_raw_parts_mut(phys_mem.translate_phys(new_cr3, 4096).unwrap(), 4096)
		};
		(&mut new_pd[3072..]).copy_from_slice(&pd_buffer[..]);
			
		// FIXME: Temp hack because we dont free the kernel stack yet
		let _ = proc_page_dir.unmap(phys_mem, kernel_intr_stack, true);
		// TODO: How does this get updates in other processes' page directories?
		proc_page_dir.map(phys_mem, kernel_intr_stack, KERNEL_INTR_STACK_SIZE, true, false).unwrap();

		Self {
			page_directory: proc_page_dir,
			virtual_memory_ranges: [None; 16],
			kernel_intr_stack,
			file_descriptors: [None; 16],
			cwd_inode: ext2_parser::ROOT_INODE,
			registers: PushADRegisterState::default(),
			eip: 0,
			eflags: USER_DEFAULT_EFLAGS,
			in_kernel: false,
			exit_code: None,
		}
	}

	fn init_elf(&mut self, elf: ElfParser, phys_mem: &mut PhysicalMemory) {
		let (stack_first_page_vaddr, stack_num_pages) = self.page_directory.map(phys_mem,
			USER_STACK_VADDR, USER_STACK_SIZE, true, true).unwrap();
		self.virtual_memory_ranges[0] = Some((stack_first_page_vaddr, stack_num_pages, true, false));
		self.registers.esp = USER_STACK_VADDR.0 + USER_STACK_SIZE;

		let mut virt_mem_range_idx = 1;
		elf.for_segment(|seg_vaddr, seg_size, init_bytes, _read, write, exec| {
			let (first_page_vaddr, num_pages) = self.page_directory.map_init(
				phys_mem,
				VirtAddr(seg_vaddr as u32),
				seg_size as u32,
				write,
				true,
				|off| { 
					if off < init_bytes.len() {
						init_bytes[off]
					} else {
						0
					}
				}
			)?;

			assert!(virt_mem_range_idx < self.virtual_memory_ranges.len());
			self.virtual_memory_ranges[virt_mem_range_idx] = Some((first_page_vaddr, num_pages, write, exec));
			virt_mem_range_idx += 1;
			Some(())
		}).unwrap();

		self.eip = elf.entry_point as u32;
	}

	pub fn new_from_elf(kernel_intr_stack: VirtAddr, elf: ElfParser) -> Self {
		let mut proc = Self::new(kernel_intr_stack);

		let mut pmem = memory_manager::PHYS_MEM.lock();
		let (phys_mem, _) = pmem.as_mut().unwrap();
		proc.init_elf(elf, phys_mem);
		proc
	}

	pub fn new_from_fork(kernel_intr_stack: VirtAddr, parent: &Process) -> Self {
		let mut proc = Self::new(kernel_intr_stack);

		proc.file_descriptors = parent.file_descriptors;
		proc.cwd_inode = parent.cwd_inode;
		proc.registers = parent.registers;
		proc.registers.eax = 0; // The fork-syscall return value is 0 for the child
		proc.eip = parent.eip;
		proc.eflags = parent.eflags;

		// TODO: Copy on write
		proc.virtual_memory_ranges = parent.virtual_memory_ranges;
		
		let mut temp_buf = box[0u8; 4096];

		let mut pmem = memory_manager::PHYS_MEM.lock();
		let (phys_mem, _) = pmem.as_mut().unwrap();

		for mem_range in parent.virtual_memory_ranges {
			if let Some((first_page_vaddr, num_pages, write, _exec)) = mem_range {
				for page in 0..num_pages {
					let page_vaddr = first_page_vaddr.0 + page*4096;
					let page_slice = unsafe {
						core::slice::from_raw_parts(page_vaddr as *const u8, 4096)
					};
					temp_buf.copy_from_slice(page_slice);
					proc.page_directory.map_init(phys_mem, VirtAddr(page_vaddr), 4096, write, true,
						|offset| temp_buf[offset]).unwrap(); // TODO: This is slow, use memcpy
				}
			}
		}

		proc
	}

	fn unmap_user_virtual_memory(&mut self, phys_mem: &mut PhysicalMemory) {
		for mem_range in self.virtual_memory_ranges.iter_mut() {
			if mem_range.is_some() {
				let (first_page_vaddr, num_pages, _, _) = mem_range.unwrap();
				for page in 0..num_pages {
					let page_vaddr = first_page_vaddr.0 + page*4096;
					self.page_directory.unmap(phys_mem, VirtAddr(page_vaddr), true).unwrap();
				}
				*mem_range = None;
			}
		}
	}

	pub fn replace_with_elf(&mut self, elf: ElfParser, argv: &[String], envp: &[String]) {
		let mut envp_ptrs = vec![0u32; envp.len() + 1];
		let mut argv_ptrs = vec![0u32; argv.len() + 1];

		let mut pmem = memory_manager::PHYS_MEM.lock();
		let (phys_mem, _) = pmem.as_mut().unwrap();

		self.unmap_user_virtual_memory(phys_mem);

		self.init_elf(elf, phys_mem);

		let stack_paddr = self.page_directory.translate_virt(phys_mem, USER_STACK_VADDR).unwrap();
		let stack_page = unsafe { 
			core::slice::from_raw_parts_mut(phys_mem.translate_phys(stack_paddr, 4096).unwrap(), 4096)
		};

		let mut stack_off = 0;
		macro_rules! push_on_stack {
			( $x:expr ) => {
				let start_off = 4096 - stack_off - $x.len();
				let end_off = 4096 - stack_off;
				stack_page[start_off..end_off].copy_from_slice($x);
				stack_off += $x.len();
			}
		}

		for (idx, env) in envp.iter().rev().enumerate() {
			push_on_stack!(&[0u8]);
			push_on_stack!(env.as_bytes());
			envp_ptrs[idx + 1] = USER_STACK_VADDR.0 + 4096 - stack_off as u32;
		}

		for (idx, arg) in argv.iter().rev().enumerate() {
			push_on_stack!(&[0u8]);
			push_on_stack!(arg.as_bytes());
			argv_ptrs[idx + 1] = USER_STACK_VADDR.0 + 4096 - stack_off as u32;
		}
		
		for env_ptr in envp_ptrs {
			push_on_stack!(&u32::to_le_bytes(env_ptr));
		}
		let envp_ptr = USER_STACK_VADDR.0 + 4096 - stack_off as u32;

		for arg_ptr in argv_ptrs {
			push_on_stack!(&u32::to_le_bytes(arg_ptr));
		}
		let argv_ptr = USER_STACK_VADDR.0 + 4096 - stack_off as u32;

		push_on_stack!(&u32::to_le_bytes(envp_ptr));
		push_on_stack!(&u32::to_le_bytes(argv_ptr));
		push_on_stack!(&u32::to_le_bytes(argv.len() as u32));
		assert!(stack_off < 4096);

		self.registers.esp -= stack_off as u32;
	}

	pub fn alloc_file_descriptor(&mut self, desc: usize) -> Option<usize> {
		// FIXME: REMOVE DEBUG SKIP 3
		for (i, descriptor) in self.file_descriptors.iter_mut().enumerate().skip(3) {
			if descriptor.is_none() {
				*descriptor = Some(desc);
				return Some(i);
			}
		}
		
		None
	}

	pub fn close_file_descriptor(&mut self, fd: usize) -> bool {
		if fd >= self.file_descriptors.len() {
			return false;
		}

		if self.file_descriptors[fd].is_none() {
			return false;
		}

		// FIXME: Check if we need to close the corresponding description

		self.file_descriptors[fd] = None;
		true
	}

	pub fn get_file_descriptor(&mut self, fd: usize) -> Option<usize> {
		if fd >= self.file_descriptors.len() {
			return None;
		}

		self.file_descriptors[fd]
	}

	pub fn is_zombie(&self) -> bool {
		self.exit_code.is_some()
	}

	pub fn exit(&mut self, exit_code: u8) {
		assert!(!self.is_zombie());

		self.exit_code = Some(exit_code);

		for fd in 0..self.file_descriptors.len() {
			self.close_file_descriptor(fd);
		}
		
		let mut pmem = memory_manager::PHYS_MEM.lock();
		let (phys_mem, _) = pmem.as_mut().unwrap();

		self.unmap_user_virtual_memory(phys_mem);
		
		// FIXME: We can't do this here, because we are currently using this stack! 
		// self.page_directory.unmap(phys_mem, self.kernel_intr_stack, true).unwrap();
	}
}
pub struct SchedulerState {
	pub processes: [Option<Process>; 16],
	pub current_process: usize,
}

impl SchedulerState {
	pub fn get_current_process(&mut self) -> &mut Process {
		self.processes[self.current_process].as_mut().unwrap()
	}
}

const INIT: Option<Process> = None; // There must be a better way...
pub static SCHEDULER_STATE: LockCell<SchedulerState> = LockCell::new(SchedulerState {
	processes: [INIT; 16],
	current_process: 0,
});

pub fn yield_execution() {
	let mut saved_registers = cpu::PushADRegisterState::default();
	let saved_eflags: u32;
	let return_eip: u32;
	let first_exec: u32; // Bools cannot be used as inputs/outputs of asm blocks
	unsafe {
		asm!("
				// Set the _saved_ eax to zero, so we can diffrentiate if we arrived here because this
				// function was called or because we were scheduled again
				mov eax, 0
				
				// Save off eflags
				pushfd
				pop {3:e}

				// Save off registers
				pushad
				pop dword ptr [{0:e}]
				pop dword ptr [{0:e} + 4]
				pop dword ptr [{0:e} + 8]
				pop dword ptr [{0:e} + 12]
				pop dword ptr [{0:e} + 16]
				pop dword ptr [{0:e} + 20]
				pop dword ptr [{0:e} + 24]
				pop dword ptr [{0:e} + 28]

				// Set eax to 1, i.e. we got here because of a call
				mov eax, 1

				// Save eip (the current address) by calling and getting the return address from the
				// stack
				call 1f
				1:
				pop {1:e}
				inc {1:e} // We want to return after the pop or else the stack will be corrupted.
				          // A pop instruction with a 32-bit register takes up 1 byte
				
				// eax holds the value which determines how we got here
				mov {2:e}, eax
				
			",
			in(reg) &mut saved_registers, out(reg) return_eip, out(reg) first_exec,
			out(reg) saved_eflags, out("eax") _
		);
	}

	// If this is the first exec of the asm block we want to yield execution, else we already
	// yielded and were re-scheduled, so we want to just return
	if first_exec != 0 {
		let mut sched_state = SCHEDULER_STATE.lock();
		let cur_proc = sched_state.get_current_process();
		cur_proc.registers = saved_registers;
		cur_proc.eip = return_eip;
		cur_proc.eflags = saved_eflags;
		cur_proc.in_kernel = true;

		// FIXME: temp
		sched_state.current_process = 1;
		drop(sched_state);
		switch_to_current_process();
	} else {
		let mut sched_state = SCHEDULER_STATE.lock();
		let cur_proc = sched_state.get_current_process();
		cur_proc.in_kernel = false;
	}
}

pub fn switch_to_current_process() -> ! {
	let mut proc_state = SCHEDULER_STATE.lock();
	let cur_proc = proc_state.get_current_process();

	tss::set_kernel_esp(cur_proc.kernel_intr_stack.0 + KERNEL_INTR_STACK_SIZE);
	let eip = cur_proc.eip;
	let eflags = cur_proc.eflags;
	let registers = cur_proc.registers;
	let cr3 = cur_proc.page_directory.get_directory_addr().0;
	let in_kernel = cur_proc.in_kernel;
	drop(proc_state);
	// FIXME: This is not correct, we have a race condition here

	if in_kernel {
		unsafe {
			cpu::ring0_context_switch(eip, eflags, &registers, cr3);
		}
	} else {
		unsafe {
			cpu::jump_to_ring3(eip, gdt::USER_CS_SELECTOR | 3, eflags, gdt::USER_DS_SELECTOR | 3,
				&registers, cr3);
		}
	}
}

pub fn set_current_register_state(eip: u32, eflags: u32, register_state: PushADRegisterState) {
	let mut sched_state = SCHEDULER_STATE.lock();
	let cur_proc = sched_state.get_current_process();
	cur_proc.eip = eip;
	cur_proc.eflags = eflags;
	cur_proc.registers = register_state;
}