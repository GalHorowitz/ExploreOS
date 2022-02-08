#![no_std]
#![no_main]

extern crate userland; // Required for panic handler

pub use userland::{print, println};

// TODO: Allocate a buffer when we have allocations
pub fn get_line(buffer: &mut [u8]) -> usize {
	let mut i = 0;
	while i < buffer.len() {
		assert!(userland::syscalls::read(userland::STDIN_FD, &mut buffer[i..i+1]).unwrap() == 1);

		if buffer[i] == 8 { // FIXME: Backspace HACK
			if i > 0 {
				assert!(userland::syscalls::write(userland::STDOUT_FD, &buffer[i..i+1]).unwrap() == 1);
				i -= 1;
			}
			continue;
		}

		assert!(userland::syscalls::write(userland::STDOUT_FD, &buffer[i..i+1]).unwrap() == 1);

		if buffer[i] == b'\n' {
			break;
		}

		i += 1;
	}

	i
}

fn handle_cd(cmd: &str) {
	if cmd.len() == 2 {
		// TODO: cd to home
	} else if let Err(err) = userland::syscalls::change_cwd(&cmd[3..]) {
			println!("ERROR: Failed to change directory: {:?}", err);
	}
}

#[no_mangle]
pub extern fn entry() -> ! {
	println!("Temp Shell (TM)");

	let mut cwd_buffer = [0u8; 256];
	let mut cmd_buffer = [0u8; 256];
	loop {
		let cwd_length = userland::syscalls::get_cwd(&mut cwd_buffer).unwrap();
		let cwd = &cwd_buffer[..cwd_length];
		print!("{}$ ", core::str::from_utf8(cwd).unwrap());

		let cmd_length = get_line(&mut cmd_buffer);
		let cmd = core::str::from_utf8(&cmd_buffer[..cmd_length]).unwrap();

		if cmd == "cd" || cmd.starts_with("cd ") {
			handle_cd(cmd);
		} else {
			println!("Running command `{}`...", cmd);
			let program_name = match cmd.split_once(' ') {
				Some((name, _)) => name,
				None => cmd,
			};

			let child_pid = match userland::syscalls::fork() {
				Ok(pid) => pid,
				Err(err) => {
					println!("ERROR: Failed to fork: {:?}", err);
					continue;
				},
			};

			if child_pid == 0 {
				match userland::syscalls::execve(program_name, cmd.split(' ').filter(|x| !x.is_empty()), core::iter::empty()) {
					Err(err) => panic!("ERROR: Failed to execve: {:?}", err),
					Ok(()) => unreachable!(),
				};
			} else {
				userland::syscalls::wait_pid(child_pid, 0).unwrap();
			}
		}
	}
}