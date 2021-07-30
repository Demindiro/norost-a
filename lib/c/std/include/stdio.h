#ifndef __LIBC_STDIO_H
#define __LIBC_STDIO_H

#include <stddef.h>
#include <stdarg.h>
#include <sys/types.h>

enum {
	SEEK_SET,
};

typedef struct __FILE {
	pid_t _address;
	int _fd;
} FILE;

extern FILE *stdin;
extern FILE *stdout;
extern FILE *stderr;

int fputc(int c, FILE * stream);

int fputs(const char *s, FILE * stream);

#define putc fputc

int putchar(int c);

int puts(const char *s);

int fgetc(FILE * stream);

char *fgets(char *s, int size, FILE * stream);

int getc(FILE * stream);

int getchar(void);

int ungetc(int c, FILE * stream);

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
