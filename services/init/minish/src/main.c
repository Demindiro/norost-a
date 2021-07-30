#include <stdio.h>

#define VERSION_MAJ 0
#define VERSION_MIN 0
#define VERSION_REV 0

int main() {

	printf("MiniSH %d.%d.%d\n", VERSION_MAJ, VERSION_MIN, VERSION_REV);

	for (;;) {
		printf(">> ");

		char in[1024];

		char *ptr = in;
		if (fgets(ptr, sizeof in, stdin) == NULL) {
			printf("Exiting...\n");
			return 0;
		}

		printf("You typed '%s'\n", in);
	}
}
