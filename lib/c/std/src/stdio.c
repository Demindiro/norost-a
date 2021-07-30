#include <errno.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <sys/uio.h>

// FIXME this is temporary as we currently rely on GCC's stddef, which doesn't have ssize_t
typedef signed long ssize_t;

static FILE _stdin  = { ._fd = 0 };
static FILE _stdout = { ._fd = 0 };
static FILE _stderr = { ._fd = 0 };

FILE *stdin  = &_stdin;
FILE *stdout = &_stdout;
FILE *stderr = &_stderr;

int fputc(int c, FILE *stream) {
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

int fputs(const char *s, FILE *stream) {
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

	ssize_t ret = writev(stream->_fd, iov, 2);

	if (ret >= 0) {
		// ret just has to be a "non-negative number". ssize_t may overflow int so just set it
		// to 0.
		ret = 0;
	}

	return ret;
}

int putchar(int c) {
	return fputc(c, stdout);
}

int puts(const char *s) {
	return fputs(s, stdout);
}
