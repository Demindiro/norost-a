#include <stddef.h>
#include <string.h>

size_t strlen(const char *s)
{
	const char *e = s;
	while (*e != 0) {
		e++;
	}
	return e - s;
}

#include <kernel.h>
char *strtok(char *str, const char *delim)
{
	static char *prev_str = NULL;

	if (str == NULL) {
		str = prev_str;
	}
	if (str == NULL) {
		return NULL;
	}

	char *p = str;

	for (;;) {
		if (*p == '\0') {
			prev_str = NULL;
			if (p == str) {
				return NULL;
			} else {
				return str;
			}
		}
		for (const char *c = delim; *c != '\0'; c++) {
			if (*p == *c) {
				*p = 0;
				prev_str = p + 1;
				return str;
			}
		}
		p++;
	}
}

int strcmp(const char *a, const char *b) {
	return strncmp(a, b, -1);
}

int strncmp(const char *a, const char *b, size_t n) {
	while (*a++ == *b++) {
		if (*a == 0) {
			return 0;
		}
	}
	unsigned char x = *a;
	unsigned char y = *b;
	return ((int)x) - ((int)y);
}
