#include "../common.h"
#include <errno.h>
#include <dux.h>
#include <kernel.h>
#include <sys/uio.h>

// FIXME this is temporary as we currently rely on GCC's stddef, which doesn't have ssize_t
typedef signed long ssize_t;

ssize_t writev(int fd, const struct iovec *iov, int iov_count)
{
	char *out = universal_buffer;
	size_t total_written = 0;
	int iov_offset = 0;
	size_t iov_base_offset = 0;

	// Loop until all data has been written out
	while (1) {

		// Get a request entry
		struct kernel_ipc_packet *cre =
		    dux_reserve_transmit_entry();
		if (cre == NULL) {
			// If we didn't write any data yet, tell the caller to try again
			// Otherwise, return the amount of data written
			if (total_written == 0) {
				return EAGAIN;
			} else {
				return total_written;
			}
		}
		// Copy data until the buffer is full or no data is left
		size_t copied = 0;
		{
			for (; iov_offset < iov_count; iov_offset++) {
				const char *in = iov[iov_offset].iov_base;
				for (;
				     iov_base_offset < iov[iov_offset].iov_len;
				     iov_base_offset++) {
					if (copied < universal_buffer_size) {
						out[copied++] =
						    in[iov_base_offset];
					} else {
						goto buffer_full;
					}
				}
				iov_base_offset = 0;
			}
 buffer_full:		;
		}

		// If we didn't write any data, return
		if (copied == 0) {
			return total_written;
		}

		total_written += copied;

		// Fill out the request entry
		cre->address = 0;
		cre->uuid = kernel_uuid(0, 0);

		cre->id = 0;

		cre->flags = 0;
		cre->offset = total_written;
		cre->data.raw = universal_buffer;
		cre->length = copied;

		asm volatile ("fence");

		cre->opcode = KERNEL_IPC_OP_WRITE;

		// Flush the queue
		kernel_io_wait(0, 0);
		kernel_io_wait(0, 0); // FIXME

		// TODO check if the request was processed successfully
		/*
		   struct kernel_client_completion_entry *cce = &completion_queue[completion_index];
		   completion_index++;
		   completion_index &= request_mask;

		   return cce->status;
		 */
	}
}
