#ifndef __LIBC_STDIO_H
#define __LIBC_STDIO_H

#include <stddef.h>
#include <stdarg.h>
#include <sys/types.h>
#include <kernel.h>

enum {
	SEEK_SET,
};

typedef struct {
	kernel_uuid_t _uuid;
	uint64_t _position;
	pid_t _address;
	const char *_path;
} FILE;

extern FILE *__files_list;
extern size_t __files_count;

#define stdin  (&__files_list[0])
#define stdout (&__files_list[1])
#define stderr (&__files_list[2])

int fileno(FILE * stream);

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

int fclose(FILE *);

size_t fread(void *, size_t, size_t, FILE *);

size_t fwrite(const void *, size_t, size_t, FILE *);

int sprintf(char *, const char *, ...);

int vfprintf(FILE *, const char *, va_list);

size_t ftell(FILE *);

#endif
