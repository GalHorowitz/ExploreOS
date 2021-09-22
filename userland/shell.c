#include "start.h"
#include "syscall.h"
#include "utils.h"

int main() {
	println("Temp Shell (TM)");

	char cmd_buffer[100];

	while(1) {
		print("$ ");
		get_line(cmd_buffer, sizeof(cmd_buffer));

		if(strcmp(cmd_buffer, "ls") == 0) {
			DIR* d = opendir("/");
			struct dirent *dir;
			while(dir = readdir(d)) {
				println(dir->d_name);
			}
			closedir(d);
		} else if(strncmp(cmd_buffer, "cat ", 4) == 0) {
			int fd = open(cmd_buffer+4, O_RDONLY);
			if(fd < 0) {
				print("Failed to open file `");
				print(cmd_buffer+4);
				println("`");
				continue;
			}

			char contents_buffer[64];
			while(1) {
				int res = read(fd, contents_buffer, sizeof(contents_buffer) - 1);
				if(res <= 0)
					break;
				
				contents_buffer[res] = 0;
				print(contents_buffer);

				if(res < sizeof(contents_buffer) - 1)
					break;
			}
			close(fd);
		} else {
			print("Running program `");
			print(cmd_buffer);
			println("`...");
					
			char* argv[10] = {}; // Arbitrary maximum?
			char* cur_start = cmd_buffer;
			for(int i = 0; i < 9; i++) {
				argv[i] = cur_start;

				char* arg_end = strstr(cur_start, " ");
				if(arg_end == NULL) {
					break;
				}
				*arg_end = 0;
				cur_start = arg_end+1;
			}

			char* envp[1] = {NULL};

			int child_pid = fork();
			if(child_pid < 0){
				println("ERROR: Failed to fork");
				continue;
			} else if(child_pid == 0) {
				execve(argv[0], argv, envp);
				println("ERROR: Failed to execve...");
				exit(1);
			} else {
				waitpid(child_pid, NULL, 0);
			}
		}
	}

	return 0;
}