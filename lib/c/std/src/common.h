#ifndef __POSIX_COMMON_H
#define __POSIX_COMMON_H

#include "stddef.h"

extern void *universal_buffer;
extern size_t universal_buffer_size;

void __posix_init(void);

#endif
