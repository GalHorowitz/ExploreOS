#![no_std]
#![no_main]
#![feature(asm_sym, naked_functions)]
include!("../../prelude.rs");

use core::mem::MaybeUninit;

use syscall_interface::SyscallDirectoryEntry;
use userland::syscalls::{exit, stat, open, read, close};

fn main(mut args: LaunchArgs, _envp: LaunchArgs) {
	let dir_path = if args.len() == 2 {
		let arg = args.nth(1).unwrap();
		if arg == "--help" {
			println!("Usage: ls [directory]");
			exit(1);
		}

		arg
	} else if args.len() == 1 {
		"." // List local directory if no arguments provided
	} else {
		println!("Unknown arguments. See `ls --help`");
		exit(1);
	};

	let dir_stat = stat(dir_path).expect("ls: Failed to stat directory");

	if !dir_stat.is_dir() {
		panic!("ls: File is not a directory");
	}

	let fd = open(dir_path, 1).expect("ls: Failed to open directory"); // TODO: O_RDONLY constant somewhere

	loop {
		// TODO: Wrap this up in a function for safe usage
		let mut dir_entry: MaybeUninit<SyscallDirectoryEntry> = MaybeUninit::uninit();
		let dir_entry_slice = unsafe {
			core::slice::from_raw_parts_mut(
				dir_entry.as_mut_ptr() as *mut u8,
				core::mem::size_of::<SyscallDirectoryEntry>()
			)
		};

		let num_bytes = read(fd, dir_entry_slice).expect("ls: Failed to read directory entry");
		if num_bytes as usize != core::mem::size_of::<SyscallDirectoryEntry>() {
			break;
		}
		let dir_entry = unsafe { dir_entry.assume_init_ref() };

		println!("{}", dir_entry.get_name());
	}

	let _ = close(fd);
}