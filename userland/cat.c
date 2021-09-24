#include "start.h"
#include "syscall.h"
#include "utils.h"

void print_help_message(char* prog) {
	print("Usage: ");
	print(prog);
	println(" [path_to_file]");
}

int main(int argc, char** argv) {
	if(argc == 0) {
		print_help_message("cat");
		return 1;
	} else if(argc != 2 || strcmp(argv[1], "--help") == 0) {
		print_help_message(argv[0]);
		return 1;
	}

	struct stat file_stat;
	if(stat(argv[1], &file_stat) != 0) {
		println("Failed to open file");
		return 2;
	}

	if(S_ISDIR(file_stat.st_mode)) {
		println("Path is a directory");
		return 3;
	}

	int fd = open(argv[1], O_RDONLY);
	if(fd < 0) {
		println("Failed to open file");
		return 3;
	}

	char buffer[256];
	while(1) {
		int num_read = read(fd, buffer, sizeof(buffer));
		if(num_read < 0) {
			println("Failed to read file");
			return 4;
		} else if (num_read == 0) {
			break;
		}
		write(1, buffer, num_read);
	}

	close(fd);

	return 0;
}