#![no_std]
#![no_main]
#![feature(asm_sym, naked_functions)]
include!("../../prelude.rs");

use userland::syscalls::{exit, stat, open, read, close};

fn main(mut args: LaunchArgs, _envp: LaunchArgs) {
	if args.len() != 2 {
		println!("Unknown/missing arguments. See `cat --help`");
		exit(1);
	}

	let path = args.nth(1).unwrap();
	if path == "--help" {
		println!("Usage: cat [file]");
		exit(1);
	}

	let file_stat = stat(path).expect("cat: Failed to stat file");

	if !file_stat.is_regular_file() {
		panic!("cat: Path is not a file");
	}

	let fd = open(path, 1).expect("cat: Failed to open file"); // TODO: O_RDONLY constant somewhere

	let mut buffer = [0u8; 256];
	loop {
		let num_bytes = read(fd, &mut buffer).expect("cat: Failed to read file");
		if num_bytes == 0 {
			break;
		}
		
		// FIXME: This is obviously incorrect as the file data may be invalid utf8
		let data = core::str::from_utf8(&buffer[..num_bytes as usize]).unwrap();
		print!("{}", data);
	}

	let _ = close(fd);
}