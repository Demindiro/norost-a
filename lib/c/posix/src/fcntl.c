#include "common.h"
#include "dux.h"
#include "fcntl.h"
#include "kernel.h"
#include "errno.h"

ssize_t write(int fd, const void *buf, size_t count)
{

	const char *in = buf;
	char *out = universal_buffer;

	count = count < universal_buffer_size ? count : universal_buffer_size;
	for (size_t i = 0; i < count; i++) {
		out[i] = in[i];
	}

	struct kernel_client_request_entry *cre =
	    dux_reserve_client_request_entry();
	if (cre == NULL) {
		return EAGAIN;
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

ssize_t read(int fd, void *buf, size_t count)
{
	count = count < universal_buffer_size ? count : universal_buffer_size;

	struct kernel_client_request_entry *cre =
	    dux_reserve_client_request_entry();
	if (cre == NULL) {
		return EAGAIN;
	}

	cre->priority = 0;
	cre->flags = 0;
	cre->file_handle = fd;
	cre->offset = 0;
	cre->data.page = universal_buffer;
	cre->length = count;
	asm volatile ("fence");
	cre->opcode = IO_READ;
	kernel_io_wait(0, 0);

	/*
	struct kernel_client_completion_entry *cce =
	    &completion_queue[completion_index];

	const char *in = cce->data.page;
	char *out = buf;
	// This is necessary since the server may be bugged and have written more data than requested.
	count = cce->length < count ? cce->length : count;
	for (size_t i = 0; i < count; i++) {
		out[i] = in[i];
	}

	return count;
	*/

	return 0;
}

int close(int fd) {
	return 0;
}
