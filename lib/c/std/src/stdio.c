#include <dux.h>
#include <errno.h>
#include <kernel.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <sys/uio.h>
#include "common.h"
#include "fd.h"
#include "format.h"

// FIXME this is temporary as we currently rely on GCC's stddef, which doesn't have ssize_t
typedef signed long ssize_t;

#define MODE_READ    (0x1)  // "r"
#define MODE_WRITE   (0x2)  // "w"
#define MODE_APPEND  (0x4)  // "a"
#define MODE_UPDATE  (0x8)  // "+"
#define MODE_EXIST   (0x10) // "x"

int fileno(FILE * stream) {
	return (stream - __files_list) / sizeof(*stream);
}

int fputc(int c, FILE * stream)
{
	unsigned char chr = (unsigned char)c;
	struct iovec iov = {
		.iov_base = &chr,
		.iov_len = 1,
	};

	ssize_t ret = writev(fileno(stream), &iov, 1);
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
	int r0 = fputs(s, stdout);
	if (r0 < 0) {
		return r0;
	}
	int r1 = fputs("\n", stdout);
	return r1 < 0 ? r1 : r0 + r1;
}

int fgetc(FILE * stream)
{
	return ENOSYS;
}

char *fgets(char *s, int size, FILE * stream)
{
	char* ret;
	// FIXME this is not the proper way to do it
	do {
		// size - 1 so we can store a null terminator.
		size_t rd = fread(s, size - 1, 1, stdin);
		ret = rd == 0 ? NULL : s;
		s[rd] = '\0';
	} while (ret == NULL);
	return ret;
}

int getc(FILE * stream)
{
	return ENOSYS;
}

int getchar(void)
{
	return ENOSYS;
}

int ungetc(int c, FILE * stream)
{
	return ENOSYS;
}

int printf(const char *format, ...)
{
	va_list vl;
	va_start(vl, format);
	int rc = vfprintf(stdout, format, vl);
	va_end(vl);
	return rc;
}

int fprintf(FILE * stream, const char *format, ...)
{
	va_list vl;
	va_start(vl, format);
	int rc = vfprintf(stream, format, vl);
	va_end(vl);
	return rc;
}

int fclose(FILE *stream)
{
	return ENOSYS;
}

FILE *fopen(const char *path, const char *mode)
{
	unsigned char flags = 0;
	for (const char *c = mode; *c != '\0'; c++) {
		switch (*c) {
		case 'r':
			flags |= MODE_READ;
			break;
		case 'w':
			flags |= MODE_WRITE;
			break;
		case 'a':
			flags |= MODE_APPEND;
			break;
		case '+':
			flags |= MODE_UPDATE;
			break;
		case 'b':
			// Whatever. Everything is binary anyways.
			break;
		default:
			// Don't silently ignore bad mode specifiers.
			// Some libraries supposedly ignore additional characters. I believe that's _probably_
			// a bad idea
			return NULL;
		}
	}

	if (flags == 0 || flags == MODE_UPDATE) {
		// Opening a file without either reading or writing is nonsense.
		return NULL;
	}

	static char path_buf[4096] __attribute__ ((__aligned__(4096)));

	// Take a free file
	FILE *f = __std_pop_free_file();

	size_t len = strlen(path);
	len = len < sizeof(path_buf) - 1 ? len : sizeof(path_buf) - 1;
	memcpy(path_buf, path, len);
	path_buf[len] = '\0';
	f->_address = 0;
	f->_uuid = kernel_uuid(0, 0);
	f->_path = path_buf;
	f->_position = 0;

	return f;
}

size_t fread(void *ptr, size_t size, size_t count, FILE *stream)
{
	// TODO account properly for size
	char *p = ptr;
	size_t read_total = size * count;
	size_t total_read = 0;

	while (read_total > total_read) {
		// Determine the maximum amount of data to be read.
		size_t delta_read = read_total - total_read;
		size_t max_size = universal_buffer_size < delta_read ? universal_buffer_size : delta_read;

		// Get a request entry
		struct kernel_ipc_packet *pkt;
		uint16_t slot = dux_reserve_transmit_entry(&pkt);
		while (pkt == NULL) {
			kernel_io_wait(-1);
			slot = dux_reserve_transmit_entry(&pkt);
		}

		// Fill out the request entry
		pkt->uuid = kernel_uuid(-1, 0);
		pkt->flags = 0;
		pkt->address = stream->_address;
		pkt->offset = stream->_position;
		pkt->name = (void *)stream->_path;
		pkt->name_len = stream->_path != NULL ? strlen(stream->_path) : 0;
		pkt->data.raw = universal_buffer;
		pkt->length = 0x1000; // TODO
		pkt->opcode = KERNEL_IPC_OP_READ;
		
		// Send the packet
		dux_submit_transmit_entry(slot);

		// Wait for a response
		const struct kernel_ipc_packet *cce;
		for (;;) {
			slot = dux_get_received_entry(&cce);
			if (slot == -1) {
				// Do nothing
			} else if (cce->opcode == KERNEL_IPC_OP_READ) {

				// Copy the received data.
				memcpy(p, universal_buffer, cce->length);
				p += cce->length;
				total_read += cce->length;
				stream->_position += cce->length;

				dux_pop_received_entry(slot);
				break;
			} else {
				dux_defer_received_entry(slot);
			}
			kernel_io_wait(-1);
		}

		// Check if the "stream" ended early
		if (cce->length < max_size) {
			break;
		}
	}

	return total_read;
}

size_t fwrite(const void *ptr, size_t size, size_t count, FILE * stream)
{
	// TODO account properly for size
	const char *p = ptr;
	size_t write_total = size * count;
	size_t total_written = 0;

	while (write_total > total_written) {
		// Determine the maximum amount of data to be written.
		size_t delta_write = write_total - total_written;
		size_t max_size = universal_buffer_size < delta_write ? universal_buffer_size : delta_write;

		// Copy the data
		for (size_t i = 0; i < max_size; i++) {
			((char *)universal_buffer)[i] = *p++;
		}

		// Get a request entry
		struct kernel_ipc_packet *pkt;
		uint16_t slot = dux_reserve_transmit_entry(&pkt);
		while (pkt == NULL) {
			kernel_io_wait(-1);
			slot = dux_reserve_transmit_entry(&pkt);
		}

		// Fill out the request entry
		pkt->uuid = kernel_uuid(0, -1);
		pkt->flags = 0;
		pkt->address = stream->_address;
		pkt->offset = stream->_position;
		pkt->name = (void *)stream->_path;
		pkt->name_len = stream->_path != NULL ? strlen(stream->_path) : 0;
		pkt->data.raw = universal_buffer;
		pkt->length = max_size;
		pkt->opcode = KERNEL_IPC_OP_WRITE;

		// Send the packet
		dux_submit_transmit_entry(slot);

		// Wait for a response
		const struct kernel_ipc_packet *cce;
		for (;;) {
			slot = dux_get_received_entry(&cce);
			if (slot == -1) {
				// Do nothing
			} else if (cce->opcode == KERNEL_IPC_OP_WRITE) {

				// Note the amount of written data.
				p += cce->length;
				total_written += cce->length;
				stream->_position += cce->length;

				dux_pop_received_entry(slot);
				break;
			} else {
				dux_defer_received_entry(slot);
			}
			kernel_io_wait(-1);
		}

		// Check if the "stream" ended early
		if (cce->length < max_size) {
			break;
		}
	}

	return total_written;
}

int vfprintf(FILE * stream, const char *format, va_list args)
{
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
				const char *end =
				    __std_determine_format(c, &fty);
				if (end != NULL) {
					// Make a backup of args in case we need to revert
					va_list bka;
					va_copy(bka, args);
					char *e = __std_format(ptr,
							       universal_buffer_size
							       - (ptr - out),
							       &fty,
							       &args);
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
		struct kernel_ipc_packet *pkt;
		uint16_t slot = dux_reserve_transmit_entry(&pkt);

		// Fill out the request entry
		pkt->uuid = stream->_uuid;
		pkt->flags = 0;
		pkt->address = stream->_address;
		pkt->offset = total_written;
		pkt->name = NULL;
		pkt->name_len = 0;
		pkt->data.raw = universal_buffer;
		pkt->length = ptr - out;
		pkt->opcode = KERNEL_IPC_OP_WRITE;

		// Send the packet
		dux_submit_transmit_entry(slot);

		// Wait for a response
		const struct kernel_ipc_packet *cce;
		for (;;) {
			slot = dux_get_received_entry(&cce);
			if (slot == -1) {
				// Do nothing
			} else if (cce->opcode == KERNEL_IPC_OP_WRITE) {

				// Note the amount of written data.
				total_written += cce->length;
				stream->_position += cce->length;

				dux_pop_received_entry(slot);
				break;
			} else {
				for (;;) {}
				dux_defer_received_entry(slot);
			}
			kernel_io_wait(-1);
		}
	}

	// TODO return the correct amount of bytes written
	return total_written;
}
