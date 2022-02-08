use core::{arch::asm, mem::MaybeUninit};
use syscall_interface::{Syscall, SyscallError, SyscallString, SyscallFileStat, SyscallArray};

type SyscallResult<T> = Result<T, SyscallError>;

fn syscall0(syscall: Syscall) -> SyscallResult<u32> {
	let return_value: i32;
	unsafe {
		asm!("int 0x67",
			in("eax") syscall as u32,
			lateout("eax") return_value, options(preserves_flags, nostack)
		);
	}

	SyscallError::from_i32(return_value)
}

fn syscall1(syscall: Syscall, arg1: u32) -> SyscallResult<u32> {
	let return_value: i32;
	unsafe {
		asm!("int 0x67",
			in("eax") syscall as u32, in("ebx") arg1,
			lateout("eax") return_value, options(preserves_flags, nostack)
		);
	}

	SyscallError::from_i32(return_value)
}

fn syscall2(syscall: Syscall, arg1: u32, arg2: u32) -> SyscallResult<u32> {
	let return_value: i32;
	unsafe {
		asm!("int 0x67",
			in("eax") syscall as u32, in("ebx") arg1, in("ecx") arg2,
			lateout("eax") return_value, options(preserves_flags, nostack)
		);
	}

	SyscallError::from_i32(return_value)
}

fn syscall3(syscall: Syscall, arg1: u32, arg2: u32, arg3: u32) -> SyscallResult<u32> {
	let return_value: i32;
	unsafe {
		asm!("int 0x67",
			in("eax") syscall as u32, in("ebx") arg1, in("ecx") arg2, in("edx") arg3,
			lateout("eax") return_value, options(preserves_flags, nostack)
		);
	}

	SyscallError::from_i32(return_value)
}

pub fn read(fd: u32, buf: &mut [u8]) -> SyscallResult<u32> {
	syscall3(Syscall::Read, fd, buf.as_mut_ptr() as u32, buf.len() as u32)
}

pub fn write(fd: u32, buf: &[u8]) -> SyscallResult<u32> {
	syscall3(Syscall::Write, fd, buf.as_ptr() as u32, buf.len() as u32)
}

// FIXME: Flags type safety
pub fn open(path: &str, flags: u32) -> SyscallResult<u32> {
	assert!(path.is_ascii());
	let path_arg = SyscallString::new(path.as_bytes());

	syscall2(Syscall::Open, &path_arg as *const SyscallString as u32, flags)
}

pub fn close(fd: u32) -> SyscallResult<()> {
	syscall1(Syscall::Close, fd)?;
	Ok(())
}

pub fn execve<'a, 'b, U, V>(path: &str, argv: U, envp: V) -> SyscallResult<()>
where 
	U: IntoIterator::<Item = &'a str>,
	V: IntoIterator::<Item = &'b str>,
{
	assert!(path.is_ascii());
	let path_arg = SyscallString::new(path.as_bytes());

	// FIXME: HACK because we don't have alloc yet
	let mut argv_arg: [MaybeUninit<SyscallString>; 10] = MaybeUninit::uninit_array();
	let mut envp_arg: [MaybeUninit<SyscallString>; 10] = MaybeUninit::uninit_array();

	let mut argv_len = 0;
	for (i, s) in argv.into_iter().enumerate() {
		argv_arg[i].write(SyscallString::new(s.as_bytes()));
		argv_len += 1;
	}

	let mut envp_len = 0;
	for (i, s) in envp.into_iter().enumerate() {
		envp_arg[i].write(SyscallString::new(s.as_bytes()));
		envp_len += 1;
	}

	let argv_arg = SyscallArray::new(unsafe {
		MaybeUninit::slice_assume_init_ref(&argv_arg[..argv_len])
	});
	let envp_arg = SyscallArray::new(unsafe {
		MaybeUninit::slice_assume_init_ref(&envp_arg[..envp_len])
	});

	syscall3(Syscall::Execve, &path_arg as *const SyscallString as u32,
		&argv_arg as *const SyscallArray<_> as u32, &envp_arg as *const SyscallArray<_> as u32)?;

	Ok(())
}

pub fn fork() -> SyscallResult<u32> {
	syscall0(Syscall::Fork)
}

pub fn exit(exit_code: u32) -> ! {
	panic!("Exit syscall returned with {:?}", syscall1(Syscall::Exit, exit_code));
}

pub struct WaitPIDResult {
	pub child_pid: u32,
	pub wstatus: u32,
}
pub fn wait_pid(pid: u32, options: u32) -> SyscallResult<WaitPIDResult> {
	let mut wstatus = 0u32;
	syscall3(Syscall::WaitPID, pid, &mut wstatus as *mut u32 as u32, options)
		.map(|child_pid| WaitPIDResult {child_pid, wstatus})
}

pub fn stat(path: &str) -> SyscallResult<SyscallFileStat> {
	assert!(path.is_ascii());
	let path_arg = SyscallString::new(path.as_bytes());

	let mut file_stat = SyscallFileStat::default();
	syscall2(Syscall::Stat, &path_arg as *const SyscallString as u32,
		&mut file_stat as *const SyscallFileStat as u32)?;

	Ok(file_stat)
}

pub fn get_cwd(buffer: &mut [u8]) -> SyscallResult<usize> {
	syscall2(Syscall::GetCWD, buffer.as_mut_ptr() as u32, buffer.len() as u32).map(|x| x as usize)
}

pub fn change_cwd(path: &str) -> SyscallResult<()> {
	assert!(path.is_ascii());
	let path_arg = SyscallString::new(path.as_bytes());

	syscall1(Syscall::ChangeCWD, &path_arg as *const SyscallString as u32)?;
	Ok(())
}