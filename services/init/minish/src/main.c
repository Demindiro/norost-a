#include <stdio.h>
#include <string.h>

#define VERSION_MAJ 0
#define VERSION_MIN 0
#define VERSION_REV 1

int main() {

	printf("MiniSH %d.%d.%d\n", VERSION_MAJ, VERSION_MIN, VERSION_REV);

	for (;;) {
		printf(">> ");

		char in[1024];

		// Read input
		char *ptr = in;
		char *end = in + sizeof(in);
		for (;;) {
			// Get input
			// TODO handle the case where end == ptr
			if (fgets(ptr, end - ptr, stdin) == NULL) {
				// stdin has been closed, which means we should exit.
				return 0;
			}

			// Echo the input
			fputs(ptr, stdout);

			// Check for special characters such as backspace, newline ... and adjust input accordingly.
			for (char *p = in; *p != '\0'; p++) {
				if (*p == '\n') {
					// Discard the newline & break to begin parsing input
					*p = 0;
					goto parse_input;
				}
				if (*p == 8) { // Backspace
					// Remove the backspace and the previous character by shifting the remaining
					// input to the left
					char *w = in < p - 1 ? in : p - 1;
					char *r = p + 1;
					while (*r != '\0') {
						*w++ = *r++;
					}
				}
			}
		}

	parse_input:
		const char *cmd = strtok(in, " ");
		if (cmd == NULL) {
			// Don't do anything
			puts("");
		} else if (strcmp(cmd, "echo") == 0) {
			const char *arg = strtok(NULL, " ");
			if (arg != NULL) {
				fputs(arg, stdout);
				for (arg = strtok(NULL, " "); arg != NULL; arg = strtok(NULL, " ")) {
					printf(" %s", arg);
				}
			}
			puts("");
		} else {
			printf("Unrecognized command '%s'\n", cmd);
		}
	}
}
