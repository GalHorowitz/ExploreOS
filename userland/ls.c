#include "start.h"
#include "syscall.h"
#include "utils.h"

int main(int argc, char** argv) {
	char* dir_path = "/";
	if(argc == 2) {
		if(strcmp(argv[1], "--help") == 0) {
			print("Usage: ");
			print(argv[0]);
			println(" [path_to_directory]");
			return 2;
		}

		dir_path = argv[1];
	}
	DIR* d = opendir(dir_path);
	if(d == NULL) {
		println("Failed to open directory");
		return 1;
	}

	struct dirent *dir;
	while(dir = readdir(d)) {
		println(dir->d_name);
	}

	closedir(d);
	return 0;
}