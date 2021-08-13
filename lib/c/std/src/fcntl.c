#include <errno.h>
#include <fcntl.h>
#include <sys/uio.h>
#include "common.h"
#include "dux.h"
#include "kernel.h"

ssize_t write(int fd, const void *buf, size_t count)
{
	struct iovec iov = {
		// Discarding const is fine as writev doesn't write to buf
		.iov_base = (void *)buf,
		.iov_len = count,
	};
	return writev(fd, &iov, 1);
}

ssize_t read(int fd, void *buf, size_t count)
{
	count = count < universal_buffer_size ? count : universal_buffer_size;

	struct kernel_ipc_packet *cre;
	uint16_t slot = dux_reserve_transmit_entry(&cre);
	if (slot == -1) {
		return EAGAIN;
	}

	cre->address = 0;
	cre->uuid = kernel_uuid(0, 0);

	cre->id = 0;

	cre->name = NULL;
	cre->name_len = 0;

	cre->flags = 0;
	cre->offset = 0;
	cre->data.raw = universal_buffer;
	cre->length = count;

	cre->opcode = KERNEL_IPC_OP_READ;

	kernel_io_wait(-1);

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

int close(int fd)
{
	return 0;
}
