use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;
use elf_parser::ElfParser;
use ext2_parser::{DirEntryType, IterationDecision};
use page_tables::VirtAddr;
use crate::ext2;
use crate::keyboard::{KEYBOARD_EVENTS_QUEUE, KeyEventType};
use crate::process::{Process, SCHEDULER_STATE};
use crate::vfs::{FILE_DESCRIPTIONS, FileDescription, FileType};
use crate::userspace::{UserCStr, UserVaddr};

#[allow(dead_code)]
#[derive(Debug)]
#[repr(u32)]
pub enum Syscall {
    Read = 0,
    Write,
	Open,
	Close,
	Execve,
	Fork,
	Exit,
	WaitPID,
	Stat,
	GetCWD,
	ChangeCWD,
    Count,
}

#[derive(Clone, Copy)]
#[repr(u16)]
pub enum SyscallError {
	UnknownSyscall = 1,
	InvalidFileDescriptor,
	OpenFileLimitReached,
	InvalidAddress,
	InvalidPath,
	PathIsDirectory,
	PathIsNotDirectory,
	BufferTooSmall,
	InvalidElfFile,
}

impl SyscallError {
	pub fn to_i32(self) -> i32 {
		-(self as i32)
	}
}

macro_rules! unwrap_or_return {
	( $x:expr, $err:expr ) => {
		if $x.is_none() {
			return ($err).to_i32();
		} else {
			$x.unwrap()
		}
	};
}

impl Syscall {
	pub fn from_u32(val: u32) -> Option<Self> {
		if val < Syscall::Count as u32 {
			unsafe { core::mem::transmute(val) }
		} else {
			None
		}
	}

	pub fn handle(&self, arg0: u32, arg1: u32, arg2: u32) -> i32 {
		match self {
			Syscall::Read => syscall_read(arg0, UserVaddr::new(arg1), arg2),
			Syscall::Write => syscall_write(arg0, UserVaddr::new(arg1), arg2),
			Syscall::Open => syscall_open(UserCStr::new(arg0), arg1),
			Syscall::Close => syscall_close(arg0),
			Syscall::Execve => syscall_execve(UserCStr::new(arg0), UserVaddr::new(arg1), UserVaddr::new(arg2)),
			Syscall::Fork => syscall_fork(),
			Syscall::Exit => syscall_exit(arg0),
			Syscall::WaitPID => syscall_waitpid(arg0, UserVaddr::new(arg1), arg2),
			Syscall::Stat => syscall_stat(UserCStr::new(arg0), UserVaddr::new(arg1)),
			Syscall::GetCWD => syscall_getcwd(UserVaddr::new(arg0), arg1),
			Syscall::ChangeCWD => syscall_changecwd(UserCStr::new(arg0)),
			Syscall::Count => SyscallError::UnknownSyscall.to_i32(),
		}
	}
}


#[repr(C)]
struct SyscallDirectoryEntry {
	inode: u32,
	entry_type: u8,
	name_length: u8,
	name: [u8; 256],
}

fn syscall_read(fd: u32, buf: UserVaddr<u8>, num_bytes: u32) -> i32 {
	let num_bytes = if num_bytes > i32::MAX as u32 {
		i32::MAX as u32
	} else {
		num_bytes
	};

	let buf = buf.as_slice_mut(num_bytes as usize).unwrap();
	if fd == 0 {
		for byte in buf.iter_mut().take(num_bytes as usize) {
			'try_get_ascii: loop {
				let event = KEYBOARD_EVENTS_QUEUE.consume_blocking();
				if event.event_type == KeyEventType::KeyDown {
					if let Some(ascii) = event.as_ascii() {
						*byte = ascii;
						break 'try_get_ascii;
					}
				}
			}
		}

		num_bytes as i32
	} else {
		let mut proc_state = SCHEDULER_STATE.lock();
		let descriptor = unwrap_or_return!(
			proc_state.get_current_process().get_file_descriptor(fd as usize),
			SyscallError::InvalidFileDescriptor
		);
		let mut file_descriptions = FILE_DESCRIPTIONS.lock();
		let description = file_descriptions.get_description(descriptor).unwrap();

		let ext2_parser = ext2::EXT2_PARSER.lock();
		let ext2_parser = ext2_parser.as_ref().unwrap();

		match description.file_type {
			FileType::File => {
				let num_read = ext2_parser.get_contents_with_offset(description.inode, buf, description.offset as usize);
				description.offset += num_read as u32;
				num_read as i32
			},
			FileType::Directory => {
				let entry = ext2_parser.get_next_directory_entry(description.inode, description.offset);
				if entry.is_none() {
					// No more entries
					return 0;
				}

				if (num_bytes as usize) < core::mem::size_of::<SyscallDirectoryEntry>() {
					return SyscallError::BufferTooSmall.to_i32();
				}

				let (next_opaque_offset, entry_inode, entry_name, entry_type) = entry.unwrap();
				description.offset = next_opaque_offset;

				let name_len = entry_name.as_bytes().len();
				assert!(name_len < u8::MAX as usize);
				let mut syscall_struct = SyscallDirectoryEntry {
					inode: entry_inode,
					entry_type: entry_type as u8,
					name_length: name_len as u8,
					name: [0u8; 256]
				};
				syscall_struct.name[..name_len].copy_from_slice(entry_name.as_bytes());

				buf[..core::mem::size_of::<SyscallDirectoryEntry>()].copy_from_slice(unsafe {
					core::slice::from_raw_parts(
						&syscall_struct as *const SyscallDirectoryEntry as *const u8,
						core::mem::size_of::<SyscallDirectoryEntry>()
					)
				});

				assert!(core::mem::size_of::<SyscallDirectoryEntry>() < i32::MAX as usize);
				core::mem::size_of::<SyscallDirectoryEntry>() as i32
			},
		}
	}
}

fn syscall_write(fd: u32, buf: UserVaddr<u8>, num_bytes: u32) -> i32 {
	let num_bytes = if num_bytes > i32::MAX as u32 {
		i32::MAX as u32
	} else {
		num_bytes
	};

	let buf = buf.as_slice(num_bytes as usize).unwrap();
	if fd == 1 {
		let buf_str = core::str::from_utf8(buf).unwrap();
		crate::screen::print(buf_str);
	}

	num_bytes as i32
}

fn syscall_open(path: UserCStr, flags: u32) -> i32 {
	let path = unwrap_or_return!(path.as_str(), SyscallError::InvalidAddress);

	let mut sched_state = SCHEDULER_STATE.lock();
	let cur_proc = sched_state.get_current_process();

	let (inode, entry_type) = unwrap_or_return!(
		ext2::EXT2_PARSER.lock().as_ref().unwrap().resolve_path_to_inode(path, cur_proc.cwd_inode),
		SyscallError::InvalidPath
	);

	let desc_idx = unwrap_or_return!(FILE_DESCRIPTIONS.lock().add_description(FileDescription {
		inode,
		offset: 0,
		status: flags,
		file_type: match entry_type {
			ext2_parser::DirEntryType::Directory => FileType::Directory,
			_ => FileType::File,
		},
	}), SyscallError::OpenFileLimitReached);

	let fd = unwrap_or_return!(
		cur_proc.alloc_file_descriptor(desc_idx),
		SyscallError::OpenFileLimitReached
	);

	fd as i32
}

fn syscall_close(fd: u32) -> i32 {
	if SCHEDULER_STATE.lock().get_current_process().close_file_descriptor(fd as usize) {
		0
	} else {
		SyscallError::InvalidFileDescriptor.to_i32()
	}
}

fn syscall_execve(path: UserCStr, argv: UserVaddr<UserCStr>, envp: UserVaddr<UserCStr>) -> i32 {
	let path = unwrap_or_return!(path.as_str(), SyscallError::InvalidAddress);

	let resolved_argv: Vec<String> = unwrap_or_return!(
		argv.as_null_terminated_slice(),
		SyscallError::InvalidAddress
	).iter().map(|cstr| cstr.as_str().unwrap().to_owned()).collect();

	let resolved_envp: Vec<String> = unwrap_or_return!(
		envp.as_null_terminated_slice(),
		SyscallError::InvalidAddress
	).iter().map(|cstr| cstr.as_str().unwrap().to_owned()).collect();

	let mut sched_state = SCHEDULER_STATE.lock();
	let cur_proc = sched_state.get_current_process();

	let user_program = {
		let ext2_parser = ext2::EXT2_PARSER.lock();
		let ext2_parser = ext2_parser.as_ref().unwrap();
		let (inode, entry_type) = unwrap_or_return!(
			ext2_parser.resolve_path_to_inode(path, cur_proc.cwd_inode),
			SyscallError::InvalidPath
		);
		if entry_type != DirEntryType::RegularFile {
			return SyscallError::PathIsDirectory.to_i32();
		}

		let user_program_metadata = ext2_parser.get_inode(inode);
		let user_program_size = user_program_metadata.size_low as usize;
		let mut user_program = crate::vec![0u8; user_program_size];
		assert!(ext2_parser.get_contents(inode, &mut user_program) == user_program_size);
		user_program
	};

	let elf_parser = unwrap_or_return!(ElfParser::parse(&user_program), SyscallError::InvalidElfFile);
	sched_state.get_current_process().replace_with_elf(elf_parser, &resolved_argv, &resolved_envp);

	drop(sched_state);
	crate::process::switch_to_current_process();
}

fn syscall_fork() -> i32 {
	let mut sched_state = SCHEDULER_STATE.lock();

	const KERNEL_INTR_STACK_VADDR: VirtAddr = VirtAddr(0xFFFF9000); // FIXME: temp
	let child = Process::new_from_fork(KERNEL_INTR_STACK_VADDR, sched_state.get_current_process());

	sched_state.processes[1] = Some(child); // FIXME: temp

	1
}

fn syscall_exit(exit_code: u32) -> i32 {
	{
		let mut sched_state = SCHEDULER_STATE.lock();
		assert!(sched_state.current_process == 1);
		sched_state.processes[1].as_mut().unwrap().exit((exit_code & 0xFF) as u8);
		sched_state.current_process = 0;
	}

	crate::process::switch_to_current_process();
}

fn syscall_waitpid(pid: u32, wstatus: UserVaddr<u32>, options: u32) -> i32 {
	if !wstatus.is_null() {
		todo!("wstatus@waitpid");
	}

	if options != 0 {
		todo!("options@waitpid");
	}

	// FIXME: Check if the target process exited
	crate::process::yield_execution();

	assert!(pid < (i32::MAX as u32));
	pid as i32
}

#[repr(C)]
struct FileStat {
	inode: u32,
	containing_device_id: u16,
	mode_and_type: u16,
	num_hard_links: u16,
	owner_user_id: u16,
	owner_group_id: u16,
	total_size: u32,
	last_access_time: u32,
	last_modification_time: u32,
	last_status_change_time: u32,
}
fn syscall_stat(path: UserCStr, stat_buf: UserVaddr<FileStat>) -> i32 {
	let path = unwrap_or_return!(path.as_str(), SyscallError::InvalidAddress);
	let stat_buf = unwrap_or_return!(stat_buf.as_ref_mut(), SyscallError::InvalidAddress);

	let mut sched_state = SCHEDULER_STATE.lock();
	let cur_proc = sched_state.get_current_process();

	let ext2_parser = ext2::EXT2_PARSER.lock();
	let ext2_parser = ext2_parser.as_ref().unwrap();

	let (inode, _) = unwrap_or_return!(
		ext2_parser.resolve_path_to_inode(path, cur_proc.cwd_inode),
		SyscallError::InvalidPath
	);

	let inode_metadata = ext2_parser.get_inode(inode);

	let stat_result = FileStat {
		containing_device_id: 0,
		inode,
		mode_and_type: inode_metadata.type_and_perms,
		num_hard_links: inode_metadata.hard_link_count,
		owner_user_id: inode_metadata.user_id,
		owner_group_id: inode_metadata.group_id,
		total_size: inode_metadata.size_low, // FIXME: 64-bit size
		last_access_time: inode_metadata.last_access_time,
		last_modification_time: inode_metadata.last_modification_time,
		last_status_change_time: 0, // TODO:
	};

	*stat_buf = stat_result;

	0
}

fn syscall_getcwd(buf: UserVaddr<u8>, size: u32) -> i32 {
	let size = size.min(i32::MAX as u32) as usize;
	let buf = unwrap_or_return!(buf.as_slice_mut(size), SyscallError::InvalidAddress);

	let mut sched_state = SCHEDULER_STATE.lock();
	let cur_proc = sched_state.get_current_process();

	if cur_proc.cwd_inode == ext2_parser::ROOT_INODE {
		if size < 2 {
			return SyscallError::BufferTooSmall.to_i32();
		} else {
			buf[0] = b'/';
			buf[1] = 0;
			return 1;
		}
	}

	let ext2_parser = ext2::EXT2_PARSER.lock();
	let ext2_parser = ext2_parser.as_ref().unwrap();

	let mut inode_walk = [0u32; 128];
	let mut walk_index = 0;

	inode_walk[0] = cur_proc.cwd_inode;
	while inode_walk[walk_index] != ext2_parser::ROOT_INODE {
		assert!(walk_index + 1 < inode_walk.len());

		ext2_parser.for_each_directory_entry(inode_walk[walk_index],
			|entry_inode, entry_name, _| {
				if entry_name == ".." {
					inode_walk[walk_index + 1] = entry_inode;
					IterationDecision::Break
				} else {
					IterationDecision::Continue
				}
			}
		);

		walk_index += 1;
	}

	// TODO: Calling for_each_directory_entry twice is bad, optimize this

	let mut write_index = 0;
	let mut success = true;
	for i in (1..=walk_index).rev() {
		ext2_parser.for_each_directory_entry(inode_walk[i],
			|entry_inode, entry_name, _| {
				if entry_inode == inode_walk[i-1] {
					if write_index + entry_name.len() + 2 > size {
						success = false;
						return IterationDecision::Break;
					}
					
					buf[write_index] = b'/';
					write_index += 1;
					buf[write_index..write_index + entry_name.len()].copy_from_slice(entry_name.as_bytes());
					write_index += entry_name.len();

					IterationDecision::Break
				} else {
					IterationDecision::Continue
				}
			}
		);

		if !success {
			return SyscallError::BufferTooSmall.to_i32();
		}
	}

	buf[write_index] = 0;

	write_index as i32
}

fn syscall_changecwd(path: UserCStr) -> i32 {
	let path = unwrap_or_return!(path.as_str(), SyscallError::InvalidAddress);
	
	let mut sched_state = SCHEDULER_STATE.lock();
	let cur_proc = sched_state.get_current_process();

	let (inode, entry_type) = unwrap_or_return!(
		ext2::EXT2_PARSER.lock().as_ref().unwrap().resolve_path_to_inode(path, cur_proc.cwd_inode),
		SyscallError::InvalidPath
	);

	if entry_type != DirEntryType::Directory {
		return SyscallError::PathIsNotDirectory.to_i32();
	}

	cur_proc.cwd_inode = inode;

	0
}