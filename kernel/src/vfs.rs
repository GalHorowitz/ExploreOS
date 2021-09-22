use lock_cell::LockCell;

#[derive(Clone, Copy, Debug)]
pub enum FileType {
	File,
	Directory,
}

#[derive(Clone, Copy, Debug)]
pub struct FileDescription {
	pub inode: u32,
	pub offset: u32,
	pub status: u32,
	pub file_type: FileType,
}

pub struct FileDescriptionTable {
	descriptions: [Option<FileDescription>; 256],
}

impl FileDescriptionTable {
	pub fn add_description(&mut self, desc: FileDescription) -> Option<usize> {
		for (i, entry) in self.descriptions.iter_mut().enumerate() {
			if entry.is_none() {
				*entry = Some(desc);
				return Some(i);
			}
		}

		None
	}

	pub fn get_description(&mut self, idx: usize) -> Option<&mut FileDescription> {
		if idx < self.descriptions.len() {
			self.descriptions[idx].as_mut()
		} else {
			None
		}
	}
}

pub static FILE_DESCRIPTIONS: LockCell<FileDescriptionTable> = LockCell::new(FileDescriptionTable {
	descriptions: [None; 256],
});

