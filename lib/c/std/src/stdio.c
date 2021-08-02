#include <dux.h>
#include <errno.h>
#include <kernel.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <sys/uio.h>
#include "common.h"
#include "format.h"

// FIXME this is temporary as we currently rely on GCC's stddef, which doesn't have ssize_t
typedef signed long ssize_t;

static FILE _stdin  = { ._address = 0, ._fd = 0, ._uuid = KERNEL_UUID(0, 0) };
static FILE _stdout = { ._address = 0, ._fd = 1, ._uuid = KERNEL_UUID(0, 0) };
static FILE _stderr = { ._address = 0, ._fd = 2, ._uuid = KERNEL_UUID(0, 0) };

FILE *stdin  = &_stdin;
FILE *stdout = &_stdout;
FILE *stderr = &_stderr;

int fputc(int c, FILE * stream)
{
	unsigned char chr = (unsigned char)c;
	struct iovec iov = {
		.iov_base = &chr,
		.iov_len = 1,
	};

	ssize_t ret = writev(stream->_fd, &iov, 1);
	if (ret >= 0) {
		ret = chr;
	}
	return ret;
}

int fputs(const char *s, FILE * stream)
{
	return fwrite(s, strlen(s), 1, stream);
}

int putchar(int c)
{
	return fputc(c, stdout);
}

int puts(const char *s)
{
	struct iovec iov[2] = {
		{
		 // Discarding const is fine as writev won't write to this.
		 .iov_base = (void *)s,
		 .iov_len = strlen(s),
		 },
		{
		 .iov_base = "\n",
		 .iov_len = 1,
		 },
	};

	ssize_t ret = writev(stdout->_fd, iov, 2);

	if (ret >= 0) {
		// ret just has to be a "non-negative number". ssize_t may overflow int so just set it
		// to 0.
		ret = 0;
	}

	return ret;
}

int fgetc(FILE * stream) {
	return ENOSYS;
}

char *fgets(char *s, int size, FILE * stream) {
	if (size == 0) {
		// Paraphrasing man page: "returns NULL while no characters have been read"
		return NULL;
	}
	struct kernel_ipc_packet *rxe = dux_get_receive_entry();
	while (rxe->opcode == 0) {
		kernel_io_wait(0, 0);
	}
	char *ptr = s;
	char *data = rxe->data.raw;
	char *end = data + rxe->length;
	while (data != end) {
		*ptr++ = *data++;
	}
	*ptr = 0;
	kernel_mem_dealloc(rxe->data.raw, 1);
	dux_add_free_range(rxe->data.raw, 1); // FIXME do this properly.
	rxe->opcode = 0;
	return s;
}

int getc(FILE * stream) {
	return ENOSYS;
}

int getchar(void) {
	return ENOSYS;
}

int ungetc(int c, FILE * stream) {
	return ENOSYS;
}

size_t fwrite(const void *ptr, size_t size, size_t count, FILE *stream) {
	struct iovec iov[1] = {
		{
		 // Discarding const is fine as writev won't write to this.
		 .iov_base = (void *)ptr,
		 .iov_len = size * count,
		 },
	};

	ssize_t ret = writev(stream->_fd, iov, 1);

	if (ret >= 0) {
		// ret just has to be a "non-negative number". ssize_t may overflow int so just set it
		// to 0.
		ret = 0;
	}

	return ret;
}

int printf(const char *format, ...) {
	va_list vl;
	va_start(vl, format);
	int rc = vfprintf(stdout, format, vl);
	va_end(vl);
	return rc;
}

int fprintf(FILE *stream, const char *format, ...) {
	va_list vl;
	va_start(vl, format);
	int rc = vfprintf(stream, format, vl);
	va_end(vl);
	return rc;
}

int vfprintf(FILE *stream, const char *format, va_list args) {
	char *out = universal_buffer;
	int total_written = 0;

	const char *c = format;

	while (*c != '\0') {
		// Fill out data
		char *ptr = out;
		while (*c != '\0') {
			if (*c == '%') {
				// It's an argument
				struct std_format_type fty;
				const char *end = __std_determine_format(c, &fty);
				if (end != NULL) {
					// Make a backup of args in case we need to revert
					va_list bka;
					va_copy(bka, args);
					char *e = __std_format(ptr, universal_buffer_size - (ptr - out), &fty, &args);
					if (e != NULL) {
						ptr = e;
						c = end;
					} else {
						// Don't write anything. Instead restore everything and try again next
						// cycle.
						// FIXME this will deadlock if any of the arguments is larger than the
						// buffer capacity.
						va_copy(args, bka);
						break;
					}
				} else {
					// Print invalid arguments normally
					if (ptr - out < universal_buffer_size) {
						*ptr++ = *c++;
					}
				}
			} else {
				// It's a regular ol' char
				if (ptr - out < universal_buffer_size) {
					*ptr++ = *c++;
				}
			}
		}

		// Get a request entry
		struct kernel_ipc_packet *pkt = NULL;
		while (pkt == NULL) {
			kernel_io_wait(0, 0);
			pkt = dux_reserve_transmit_entry();
		}

		// Fill out the request entry
		pkt->flags = 0;
		pkt->address = stream->_address;
		pkt->offset = total_written;
		pkt->data.raw = universal_buffer;
		pkt->length = ptr - out;
		asm volatile ("fence");
		pkt->opcode = KERNEL_IPC_OP_WRITE;

		// TODO check if the request was processed successfully
		/*
		   struct kernel_client_completion_entry *cce = &completion_queue[completion_index];
		   completion_index++;
		   completion_index &= request_mask;

		   return cce->status;
		 */

		total_written += ptr - out;
	}

	// Flush the queue
	kernel_io_wait(0, 0);
	kernel_io_wait(0, 0); // FIXME hacky workaround to ensure the receiving task prints the
	                      // data before we overwrite it again

	// TODO return the correct amount of bytes written
	return total_written;
}
