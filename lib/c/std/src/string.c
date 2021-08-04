#include <stddef.h>
#include <string.h>

void *memcpy(void *dest, const void *src, size_t n)
{
	char *d = dest;
	const char *s = src;
	const char *e = s + n;
	while (s != e) {
		*d++ = *s++;
	}
	return dest;
}

void *memmove(void *dest, const void *src, size_t n)
{
	char *d = dest;
	const char *s = src;
	const char *e = s + n;
	// Check the relative position of the pointers to make sure we don't accidently
	// overwrite the data we're reading.
	if (dest < src) {
		while (s != e) {
			*d++ = *s++;
		}
	} else {
		d += n - 1;
		e--, s--;
		while (s != e) {
			*d-- = *e--;
		}
	}
	return dest;
}

void *memset(void *dest, int c, size_t n)
{
	char *d = dest;
	char *e = dest + n;
	while (d != e) {
		*d++ = c;
	}
	return dest;
}

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

int strcmp(const char *a, const char *b)
{
	return strncmp(a, b, -1);
}

int strncmp(const char *a, const char *b, size_t n)
{
	while (n-- > 0 && *a == *b) {
		if (*a == 0) {
			return 0;
		}
		a++, b++;
	}
	unsigned char x = *a;
	unsigned char y = *b;
	return ((int)x) - ((int)y);
}

char *strcpy(char *dest, const char *src)
{
	return strncpy(dest, src, -1);
}

char *strncpy(char *dest, const char *src, size_t n)
{
	char *d = dest;
	while (n-- > 0 && *src != 0) {
		*d++ = *src++;
	}
	return dest;
}

#ifdef __STD_TEST

#include <stdio.h>

void __std_test_memmove(void)
{
	char buf[64] = { "kitty" };

	puts("Original:");
	puts(buf);

	puts("Not shifted:");
	memmove(buf, buf, 10);
	puts(buf);

	puts("Shifted 1 to the left:");
	memmove(buf, buf + 1, 10);
	puts(buf);

	puts("Shifted 2 to the right:");
	memmove(buf + 2, buf, 10);
	puts(buf);
}

void __std_test_memcpy(void)
{
	char buf[64] = { "kitty" };
	char buf2[sizeof(buf)];

	puts("Original:");
	puts(buf);

	puts("Copy:");
	memcpy(buf2, buf, sizeof(buf));
	puts(buf2);
}

#endif
