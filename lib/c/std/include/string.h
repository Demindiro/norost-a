#ifndef __LIBC_STRING_H
#define __LIBC_STRING_H

#include <stddef.h>

extern size_t strlen(const char *);

extern void *memcpy(void *, const void *, size_t);

extern void strcat(char *, const char *);

extern char *strchr(const char *, int);

extern char *strtok(char *str, const char *delim);

extern int strcmp(const char *a, const char *b);

extern int strncmp(const char *a, const char *b, size_t n);

#endif
