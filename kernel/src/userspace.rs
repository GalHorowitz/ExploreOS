use core::marker::PhantomData;

use alloc::{string::String, vec::Vec, borrow::ToOwned};
use syscall_interface::{SyscallString, SyscallArray};

pub struct UserVaddr<'a, T>(u32, PhantomData<&'a T>);

impl<'a, T> UserVaddr<'a, T> {
    pub const fn new(vaddr: &'a u32) -> Self {
		Self(*vaddr, PhantomData::<&'a T>)
	}

	pub fn as_ref(&self) -> Option<&'a T> {
		if is_valid_for_reading(self.0 as usize, core::mem::size_of::<T>()) {
			unsafe { Some(&*(self.0 as *const T)) }
		} else {
			None
		}
	}

	pub fn as_ref_mut(&self) -> Option<&'a mut T> {
		if is_valid_for_writing(self.0 as usize, core::mem::size_of::<T>()) {
			unsafe { Some(&mut *(self.0 as *mut T)) }
		} else {
			None
		}
	}

	pub fn as_slice(&self, count: usize) -> Option<&'a [T]> {
		if is_valid_for_reading(self.0 as usize, core::mem::size_of::<T>().checked_mul(count)?) {
			unsafe { Some(core::slice::from_raw_parts(self.0 as *const T, count as usize)) }
		} else {
			None
		}
	}

	pub fn as_slice_mut(&self, count: usize) -> Option<&'a mut [T]> {
		if is_valid_for_writing(self.0 as usize, core::mem::size_of::<T>().checked_mul(count)?) {
			unsafe { Some(core::slice::from_raw_parts_mut(self.0 as *mut T, count as usize)) }
		} else {
			None
		}
	}

	pub fn is_null(&self) -> bool {
		self.0 == 0
	}
}

impl<'a> UserVaddr<'a, SyscallString<'a>> {
	pub fn as_str(&'a self) -> Option<&'a str> {
		let syscall_str = self.as_ref()?;
		let buf: &'a [u8] = UserVaddr::new(&syscall_str.ptr).as_slice(syscall_str.length as usize)?;
		core::str::from_utf8(buf).ok()
	}
}

impl<'a> UserVaddr<'a, SyscallArray<'a, SyscallString<'a>>> {
	pub fn as_string_vec(&self) -> Option<Vec<String>> {
		let syscall_arr = self.as_ref()?;
		let str_arr: &[SyscallString] = UserVaddr::new(&syscall_arr.ptr).as_slice(syscall_arr.length as usize)?;
		let mut vec = Vec::with_capacity(str_arr.len());
		for (i, s) in str_arr.iter().enumerate() {
			let buf: &[u8] = UserVaddr::new(&s.ptr).as_slice(s.length as usize)?;
			vec.push(core::str::from_utf8(buf).ok()?.to_owned());
		}

		Some(vec)
	}
}

fn is_valid_for_reading(_vaddr: usize, _num_bytes: usize) -> bool {
	//FIXME: Validity Checks
	true
}

fn is_valid_for_writing(_vaddr: usize, _num_bytes: usize) -> bool {
	//FIXME: Validity Checks
	true
}