__attribute__((naked)) void _start() {
	asm volatile (
		"xor ebp, ebp\n"
		"call main\n"
		"push eax\n"
		"call _exit\n"
	);
}

__attribute__((noreturn)) void _exit(int status) {
	asm volatile (
		"mov eax, 6\n"
		"mov ebx, %0\n"
		"int 0x67\n"
		:: "g" (status)
	);
	__builtin_unreachable();
}