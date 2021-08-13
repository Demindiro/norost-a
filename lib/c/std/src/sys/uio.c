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
	for (;;) {

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

		// Get a request entry
		struct kernel_ipc_packet *cre;
		uint16_t slot = dux_reserve_transmit_entry(&cre);

		// Fill out the request entry
		cre->address = 0;
		cre->uuid = kernel_uuid(0, 0x12345678);

		cre->id = 0;

		cre->name = NULL;
		cre->name_len = 0;

		cre->flags = 0;
		cre->offset = total_written;
		cre->data.raw = universal_buffer;
		cre->length = copied;

		cre->opcode = KERNEL_IPC_OP_WRITE;

		// Send the packet
		dux_submit_transmit_entry(slot);

		// Wait for a response
		const struct kernel_ipc_packet *cce;
		for (;;) {
			slot = dux_get_received_entry(&cce);
			if (slot == -1) {
				// Do nothing
			} else if (cce->opcode == KERNEL_IPC_OP_WRITE) {
				// Note the amount written.
				total_written += cce->length;

				dux_pop_received_entry(slot);
				break;
			} else {
				dux_defer_received_entry(slot);
			}
			kernel_io_wait(-1);
		}
	}
}
