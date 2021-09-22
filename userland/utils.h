#pragma once

#include "syscall.h"

int strlen(char* str) {
	int len = 0;
	while(str[len]) {
		len++;
	}
	return len;
}

int strcmp(char* a, char* b) {
	int i = 0;
	while(a[i] && b[i]) {
		if(a[i] != b[i])
			break;
		i++;
	}

	return a[i] - b[i];
}

int strncmp(char* a, char* b, unsigned int n) {
	int i = 0;
	while(a[i] && b[i]) {
		if(a[i] != b[i])
			break;
		i++;
		if(i == n) {
			return 0;
		}
	}

	return a[i] - b[i];
}

char* strstr(char* s1, char* s2) {
	int i = 0;
	while(s1[i]) {
		int j = 0;
		while(s2[j]) {
			if(s1[i+j] != s2[j])
				break;
			j++;
		}

		if(!s2[j]) {
			return s1+i;
		}
		i++;
	}

	return NULL;
}

void get_line(char* out_buf, int len) {
	for(int i = 0; i < len - 1; i++){
		read(0, &out_buf[i], 1);
		write(1, &out_buf[i], 1);
		if(out_buf[i] == 8) { // FIXME: backspace hack
			if(i == 0) {
				i = -1;
			} else {
				i -= 2;
			}
			continue;
		}

		if(out_buf[i] == '\n') {
			out_buf[i] = 0;
			return;
		}
	}
	out_buf[len - 1] = 0;
}

void put_char(char ch) {
	write(1, &ch, 1);
}

void print(char* str) {
	write(1, str, strlen(str));
}

void print_num(int x) {
	if(x == 0) {
		put_char('0');
		return;
	}

	if(x < 0) {
		put_char('-');
		x *= -1;
		// FIXME: This doesn't handle MIN_INTEGER
	}

	char digits[10];
	int idx = 0;
	while(x != 0) {
		digits[idx++] = (x%10) + '0';
		x /= 10;
	}
	for(idx--; idx >= 0; idx--) {
		put_char(digits[idx]);
	}
}

void println(char* str){
	print(str);
	put_char('\n');
}

#define S_MODE_TEST(m, v) ((((m)>>12)&0b1111) == (v))
#define S_ISBLK(m) S_MODE_TEST(m, 0x6)
#define S_ISCHR(m) S_MODE_TEST(m, 0x2)
#define S_ISDIR(m) S_MODE_TEST(m, 0x4)
#define S_ISFIFO(m) S_MODE_TEST(m, 0x1)
#define S_ISREG(m) S_MODE_TEST(m, 0x8)
#define S_ISLNK(m) S_MODE_TEST(m, 0xA)
#define S_ISSOCK(m) S_MODE_TEST(m, 0xC)

typedef unsigned int DIR;
static DIR dir;
static int dir_is_open = 0;

DIR* opendir(char* path) {
	if(dir_is_open) {
		return NULL;
	}

	struct stat file_stat;
	if(stat(path, &file_stat) != 0) {
		return NULL;
	}

	if(!S_ISDIR(file_stat.st_mode)) {
		return NULL;
	}

	int fd = open(path, O_RDONLY);
	if(fd >= 0){
		dir = fd;
	} else {
		return NULL;
	}
	dir_is_open = 1;

	return &dir;
}

int closedir(DIR* dir_stream) {
	if(!dir_is_open || dir_stream != &dir) {
		return -1;
	}

	dir_is_open = 0;
	return close(dir);
}

struct dirent* readdir(DIR* dir_stream) {
	if(!dir_is_open || dir_stream != &dir) {
		return NULL;
	}

	static struct dirent entry;
	int num_read = read(dir, &entry, sizeof(entry));
	if (num_read != sizeof(entry)) {
		return NULL;
	}
	return &entry;
}