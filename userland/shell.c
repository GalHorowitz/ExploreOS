#include "start.h"
#include "syscall.h"
#include "utils.h"

void handle_cd(char* line, int length) {
	if(length == 2) {
		// TODO: cd to home
	} else {
		if(chdir(line+3) < 0) {
			println("Failed to change directory");
		}
	}
}

int main() {
	println("Temp Shell (TM)");

	char cmd_buffer[100];
	char cwd_buffer[256];

	while(1) {
		// TODO: Calling getcwd each time is dumb, we should just maintain this locally
		int res = getcwd(cwd_buffer, sizeof(cwd_buffer));
		if(res > 0) {
			print(cwd_buffer);
		} else {
			print_num(res);
		}
		print("$ ");
		int line_length = get_line(cmd_buffer, sizeof(cmd_buffer));

		if(strcmp(cmd_buffer, "cd") == 0 || strncmp(cmd_buffer, "cd ", 3) == 0) {
			handle_cd(cmd_buffer, line_length);
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