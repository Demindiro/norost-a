#undef __STDC_HOSTED__
// Note: this header is not part of the standard library, but there is
// no other way to iterate directories.
#include <dirent.h>
#include <kernel.h>
#include <stdio.h>
#include <string.h>

#define VERSION_MAJ 0
#define VERSION_MIN 0
#define VERSION_REV 2

#define ARG_SEPARATORS " \t"

static const char *next_cmd(char *line) {
	for (;;) {
		const char *s = strtok(line, ARG_SEPARATORS);
		line = NULL;
		if (s == NULL) {
			return NULL;
		}
		if (s[0] != '\0') {
			return s;
		}
	}
}

static const char *next_arg() {
	for (;;) {
		const char *s = strtok(NULL, ARG_SEPARATORS);
		if (s == NULL) {
			return NULL;
		}
		if (s[0] != '\0') {
			return s;
		}
	}
}

static void echo() {
	const char *arg = next_arg();
	if (arg != NULL) {
		fputs(arg, stdout);
		for (arg = next_arg(); arg != NULL; arg = next_arg()) {
			printf(" %s", arg);
		}
	}
	puts("");
}

static void help() {
	printf(
		"Commands:\n"
		"  echo   [args]\n"
		"  help\n"
		"  list   [path]\n"
		"  read   <path> [offset [len]]\n"
		"  write  <path> [offset [len]]\n"
	);
}

static void list() {
	DIR *dir = opendir(".");
	for (struct dirent *ent = readdir(dir); ent != NULL; ent = readdir(dir)) {
		// d_name is guaranteed to be 0 terminated
		puts(ent->d_name);
	}
}

static void _read() {
	puts("TODO");
}

static void _write() {
	puts("TODO");
}

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

			// Check for special characters such as backspace, newline ... and adjust input accordingly.
			for (char *p = in; *p != '\0'; p++) {
				if (*p == '\n') {
					// Discard the newline & break to begin parsing input
					*p = 0;
					goto parse_input;
				} else if (*p == 8 || *p == 127) { // Backspace or delete
					if (p == in) {
						*p = '\0';
					} else {
						// Remove the backspace and the previous character by shifting the remaining
						// input to the left
						char *w = p - 1;
						char *r = p + 1;
						while (*r != '\0') {
							*w++ = *r++;
						}
						*w = '\0';
						p--;
					}
					p--;
				}
				ptr = p + 1;
			}

			// Clear the input & write out
			printf("\r\33[2K>> %s", in);
		}

	parse_input:
		// Clear the input & write out
		printf("\r\33[2K>> %s\n", in);

#define CMD2(name, fn) else if (strcmp(cmd, #name) == 0) { fn(); }
#define CMD(fn) CMD2(fn, fn)
		const char *cmd = next_cmd(in);
		if (cmd == NULL) {
			// Don't do anything
		}
		CMD(echo)
		CMD(help)
		CMD(list)
		CMD2(read, _read)
		CMD2(write, _write)
		else {
			printf("Unrecognized command '%s'\n", cmd);
		}
#undef CMD
#undef CMD2
	}
}
