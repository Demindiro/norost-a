#ifndef __LIBC_STDIO_H
#define __LIBC_STDIO_H

#include <stddef.h>
#include <stdarg.h>

enum {
	SEEK_SET,
};

typedef void FILE;

#define stdout ((void *)1)
#define stderr ((void *)2)

int puts(const char *str) {
	return 0;
}

int printf(const char *, ...);

int fprintf(FILE *, const char *, ...);

int fflush(FILE *);

void abort(void);

void fseek(FILE *, long, int);

FILE *fopen(const char *, const char *);

void setbuf(FILE *, char *);

void fclose(FILE *);

int fread(void *, size_t, size_t, FILE *);

int fwrite(const void *, size_t, size_t, FILE *);

int sprintf(char *, const char *, ...);

int vfprintf(FILE *, const char *, va_list);

size_t ftell(FILE *);

#endif
