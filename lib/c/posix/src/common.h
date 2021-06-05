#ifndef __POSIX_COMMON_H
#define __POSIX_COMMON_H

#include "stdint.h"

extern struct kernel_client_request_entry *request_queue;
extern size_t request_mask;
extern size_t request_index;

extern struct kernel_client_completion_entry *completion_queue;
extern size_t completion_mask;
extern size_t completion_index;

extern void *universal_buffer;
extern size_t universal_buffer_size;

void __posix_init(void);

#endif
