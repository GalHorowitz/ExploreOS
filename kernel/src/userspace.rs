use core::marker::PhantomData;

pub struct UserVaddr<T>(u32, PhantomData<T>);

#[repr(transparent)]
pub struct UserCStr(u32);

impl<T> UserVaddr<T> {
    pub const fn new(vaddr: u32) -> Self {
		Self(vaddr, PhantomData)
	}

	pub fn as_ref(&self) -> Option<&T> {
		if is_valid_for_reading(self.0 as usize, core::mem::size_of::<T>()) {
			unsafe { Some(&*(self.0 as *const T)) }
		} else {
			None
		}
	}

	pub fn as_ref_mut(&self) -> Option<&mut T> {
		if is_valid_for_writing(self.0 as usize, core::mem::size_of::<T>()) {
			unsafe { Some(&mut *(self.0 as *mut T)) }
		} else {
			None
		}
	}

	pub fn as_slice(&self, count: usize) -> Option<&[T]> {
		if is_valid_for_reading(self.0 as usize, core::mem::size_of::<T>().checked_mul(count)?) {
			unsafe { Some(core::slice::from_raw_parts(self.0 as *const T, count as usize)) }
		} else {
			None
		}
	}

	pub fn as_slice_mut(&self, count: usize) -> Option<&mut [T]> {
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

impl UserVaddr<UserCStr> {
	pub fn as_null_terminated_slice(&self) -> Option<&[UserCStr]> {
		let mut size = 0;
		const PTR_SIZE: usize = core::mem::size_of::<UserCStr>();
		loop {
			let element_vaddr = self.0 as usize + size * PTR_SIZE;
			if is_valid_for_reading(element_vaddr, PTR_SIZE) {
				let element = unsafe { &*(element_vaddr as *const UserCStr) };
				
				if element.is_null() {
					break;
				}

				let _ = element.as_str()?;
			} else {
				return None;
			}

			size += 1;
		}

		self.as_slice(size)
	}
}

impl UserCStr {
	pub const fn new(vaddr: u32) -> Self {
		Self(vaddr)
	}

	pub fn as_str(&self) -> Option<&str> {
		let mut length: usize = 0;
		loop {
			if !is_valid_for_reading(length.checked_add(self.0 as usize)?, 1) {
				return None;
			}

			let byte = unsafe { *((self.0 as usize + length) as *const u8) };
			if byte == 0 {
				break;
			} else {
				length = length.checked_add(1)?;
			}
		}
		
		let slice = unsafe {
			core::slice::from_raw_parts(self.0 as *const u8, length as usize)
		};
		core::str::from_utf8(slice).ok()
	}

	pub fn is_null(&self) -> bool {
		self.0 == 0
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