#include "common.h"
#include "dux.h"
#include "fcntl.h"
#include "kernel.h"

#define NULL 0

ssize_t write(int fd, const void *buf, size_t count) {

	const char *in = buf;
	char *out = universal_buffer;

	count = count < universal_buffer_size ? count : universal_buffer_size;
	for (size_t i = 0; i < count; i++) {
		out[i] = in[i];
	}

	struct kernel_client_request_entry *cre = dux_reserve_client_request_entry();
	if (cre == NULL) {
		// FIXME formal error codes.
		return -1;
	}
	cre->priority = 0;
	cre->flags = 0;
	cre->file_handle = fd;
	cre->offset = 0;
	cre->data.page = universal_buffer;
	cre->length = count;
	asm volatile ("fence");
	cre->opcode = IO_WRITE;
	kernel_io_wait(0, 0);

	/*
	struct kernel_client_completion_entry *cce = &completion_queue[completion_index];
	completion_index++;
	completion_index &= request_mask;

	return cce->status;
	*/

	return 0;
}
