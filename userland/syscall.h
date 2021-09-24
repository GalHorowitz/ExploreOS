#pragma once

#define NULL (void*)0

#define SYSCALL_READ 0
#define SYSCALL_WRITE 1
#define SYSCALL_OPEN 2
#define SYSCALL_CLOSE 3
#define SYSCALL_EXECVE 4
#define SYSCALL_FORK 5
#define SYSCALL_EXIT 6
#define SYSCALL_WAITPID 7
#define SYSCALL_STAT 8
#define SYSCALL_GETCWD 9
#define SYSCALL_CHDIR 10

int syscall(int syscall_num, void* arg1, void* arg2, void* arg3) {
	int return_val;
	asm volatile (
		"mov eax, %1\n"
		"mov ebx, %2\n"
		"mov ecx, %3\n"
		"mov edx, %4\n"
		"int 0x67\n"
		"mov %0, eax\n"
		: "=g" (return_val)
		: "g" (syscall_num), "g" (arg1), "g" (arg2), "g" (arg3)
		: "eax", "ebx", "ecx", "edx", "memory"
	);

	return return_val;
}

typedef unsigned int ino_t;
struct dirent {
	ino_t d_ino;
	unsigned char d_entry_type;
	unsigned char d_name_length;
	char d_name[256];
};
int read(int fd, void* buf, int num_bytes) {
	return syscall(SYSCALL_READ, (void*) fd, buf, (void*) num_bytes);
}

int write(int fd, void* buf, int num_bytes) {
	return syscall(SYSCALL_WRITE, (void*) fd, buf, (void*) num_bytes);
}

#define O_RDONLY 1
#define O_WRONLY 2
#define O_RDWR O_RDONLY|O_WRONLY
int open(char* path, int flags) {
	return syscall(SYSCALL_OPEN, path, (void*) flags, 0);
}

int close(int fd) {
	return syscall(SYSCALL_CLOSE, (void*) fd, 0, 0);
}

int execve(char* path, char** argv, char** envp) {
	return syscall(SYSCALL_EXECVE, path, argv, envp);
}

int fork() {
	return syscall(SYSCALL_FORK, 0, 0, 0);
}

__attribute__((noreturn)) void exit(int status) {
	syscall(SYSCALL_EXIT, (void*) status, 0, 0);
	__builtin_unreachable();
}

int waitpid(int pid, int* wstatus, int options) {
	return syscall(SYSCALL_WAITPID, (void*) pid, wstatus, (void*) options);
}

typedef unsigned short dev_t;
typedef unsigned short mode_t;
typedef unsigned short nlink_t;
typedef unsigned short uid_t;
typedef unsigned short gid_t;
typedef unsigned int off_t;
typedef unsigned int time_t;
struct stat {
	ino_t st_ino;
	dev_t st_dev;
	mode_t st_mode;
	nlink_t st_nlink;
	uid_t st_uid;
	gid_t st_gid;
	off_t st_size;
	time_t st_atime;
	time_t st_mtime;
	time_t st_ctime;
};
int stat(char* path, struct stat* statbuf) {
	return syscall(SYSCALL_STAT, path, statbuf, 0);
}

int getcwd(char* buf, int size) {
	return syscall(SYSCALL_GETCWD, buf, (void*) size, 0);
}

int chdir(char* path) {
	return syscall(SYSCALL_CHDIR, path, 0, 0);
}