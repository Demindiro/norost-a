#ifndef __LIBC_STRING_H
#define __LIBC_STRING_H

#include <stddef.h>

extern size_t strlen(const char *);

extern void *memcpy(void *, const void *, size_t);

extern void strcat(char *, const char *);

extern char *strchr(const char *, int);

#endif
