//! Defines shares structs and constants for the kernel-userspace interface

#![no_std]

use core::marker::PhantomData;

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

    Count, // This must be kept last
}

#[derive(Debug)]
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

    UnknownSyscallError, // This must be kept last, because `from_i32` uses it to determine if the
                         // error number is recognized
}

impl SyscallError {
	pub const fn to_i32(self) -> i32 {
		-(self as i32)
	}

    pub const fn from_i32(val: i32) -> Result<u32, SyscallError> {
        if val >= 0 {
            Ok(val as u32)
        } else {
            if val <= SyscallError::UnknownSyscallError.to_i32() {
                Err(SyscallError::UnknownSyscallError)
            } else {
                unsafe { Err(core::mem::transmute((-val) as u16)) }
            }
        }
    }
}

impl Syscall {
	pub const fn from_u32(val: u32) -> Option<Self> {
		if val < Syscall::Count as u32 {
			unsafe { core::mem::transmute(val) }
		} else {
			None
		}
	}
}

#[repr(C)]
pub struct SyscallArray<'a, T> {
	pub ptr: u32,
	pub length: u32,
	_phantom: PhantomData<&'a T>,
}
pub type SyscallString<'a> = SyscallArray<'a, u8>;

impl<'a, T> SyscallArray<'a, T> {
	pub fn new(arr: &'a [T]) -> Self {
		Self {
			ptr: arr.as_ptr() as u32,
			length: arr.len() as u32,
			_phantom: PhantomData::<&'a T>,
		}
	}
}

#[derive(Default)]
#[repr(C)]
pub struct SyscallFileStat {
	pub inode: u32,
	pub containing_device_id: u16,
	pub mode_and_type: u16,
	pub num_hard_links: u16,
	pub owner_user_id: u16,
	pub owner_group_id: u16,
	pub total_size: u32,
	pub last_access_time: u32,
	pub last_modification_time: u32,
	pub last_status_change_time: u32,
}

impl SyscallFileStat {
	pub fn is_fifo(&self) -> bool {
		self.mode_test(0x1)
	}

	pub fn is_char_device(&self) -> bool {
		self.mode_test(0x2)
	}

	pub fn is_dir(&self) -> bool {
		self.mode_test(0x4)
	}

	pub fn is_block_device(&self) -> bool {
		self.mode_test(0x6)
	}

	pub fn is_regular_file(&self) -> bool {
		self.mode_test(0x8)
	}

	pub fn is_symbolic_link(&self) -> bool {
		self.mode_test(0xA)
	}

	pub fn is_socket(&self) -> bool {
		self.mode_test(0xC)
	}

	fn mode_test(&self, test: u16) -> bool {
		(self.mode_and_type >> 12)&0b1111 == test
	}
}

#[repr(C)]
pub struct SyscallDirectoryEntry {
	pub inode: u32,
	pub entry_type: u8,
	pub name_length: u8,
	pub name: [u8; 256],
}

impl SyscallDirectoryEntry {
	pub fn get_name(&self) -> &str {
		core::str::from_utf8(&self.name[..self.name_length as usize]).unwrap()
	}
}