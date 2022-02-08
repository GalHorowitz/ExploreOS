extern crate userland; // Required for panic handler

pub use userland::{print, println};

struct LaunchArgs {
	args: &'static [*const u8],
	idx: usize,
}

unsafe fn cstr_to_str(cstr: *const u8) -> &'static str {
	let mut i = 0;
	while *cstr.offset(i) != 0 {
		i += 1;
	}
	
	core::str::from_utf8(core::slice::from_raw_parts(cstr, i as usize)).unwrap()
}

impl Iterator for LaunchArgs {
	type Item = &'static str;

	fn next(&mut self) -> Option<Self::Item> {
		if self.idx >= self.args.len() {
			return None;
		}
		
		let res = unsafe { cstr_to_str(self.args[self.idx]) };
		self.idx += 1;
		Some(res)
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		(self.args.len(), Some(self.args.len()))
	}
}

impl ExactSizeIterator for LaunchArgs {}

#[no_mangle]
#[naked]
pub unsafe extern fn entry() -> ! {
	core::arch::asm!("
		xor ebp, ebp
		call {}
	", sym rust_entry, options(noreturn))
}

unsafe extern "cdecl" fn rust_entry(argc: u32, argv: *const *const u8, envp: *const *const u8) -> ! {
	let args = core::slice::from_raw_parts(argv, argc as usize);

	let mut var_count = 0;
	while !(*envp.offset(var_count)).is_null() {
		var_count += 1;
	}
	let vars = core::slice::from_raw_parts(envp, var_count as usize);

	crate::main(LaunchArgs { args, idx: 0 }, LaunchArgs { args: vars, idx: 0 });
	crate::userland::syscalls::exit(0);
}