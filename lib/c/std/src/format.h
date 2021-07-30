#ifndef _STD_FORMAT_H
#define _STD_FORMAT_H

#include <stdarg.h>

enum {
	STD_FORMAT_DEC,
	STD_FORMAT_UDEC,
	STD_FORMAT_OCTAL,
	STD_FORMAT_HEX,
	STD_FORMAT_FLOAT,
	STD_FORMAT_SCIENCE,
	STD_FORMAT_FLOAT_OR_SCIENCE,
	STD_FORMAT_HEX_FLOAT,
	STD_FORMAT_CHAR,
	STD_FORMAT_STRING,
	STD_FORMAT_POINTER,
	STD_FORMAT_COUNT,
	STD_FORMAT_PERCENT,
};

enum {
	STD_FORMAT_LJUST              = 1 << 0,
	STD_FORMAT_SIGNED             = 1 << 1,
	STD_FORMAT_SPACE              = 1 << 2,
	STD_FORMAT_PREFIX_OR_DECIMAL  = 1 << 3,
	STD_FORMAT_ZEROES             = 1 << 4,
	STD_FORMAT_VAR_WIDTH          = 1 << 5,
	STD_FORMAT_VAR_PRECISION      = 1 << 6,
	STD_FORMAT_UPPER              = 1 << 7,
};

enum {
	STD_FORMAT_TYPE_INT,
	// char and short are upcasted to int when used in va_list
	STD_FORMAT_TYPE_CHAR = STD_FORMAT_TYPE_INT,
	STD_FORMAT_TYPE_SHORT = STD_FORMAT_TYPE_INT,
	STD_FORMAT_TYPE_LONG,
	STD_FORMAT_TYPE_LONG_LONG,
	STD_FORMAT_TYPE_INTMAX_T,
	STD_FORMAT_TYPE_SIZE_T,
	STD_FORMAT_TYPE_PTRDIFF_T,
	// sizeof(void *) != sizeof(size_t), see https://stackoverflow.com/a/1572189
	STD_FORMAT_TYPE_POINTER,
	STD_FORMAT_TYPE_LONG_DOUBLE,
	STD_FORMAT_TYPE_NONE,
};

struct std_format_type {
	// The expected amount of buffer space needed.
	unsigned short size;
	// The minimum amount of characters to be printed
	// Limited to 255 for pragmatic reasons
	unsigned char width;
	// TODO describe
	unsigned char precision;
	// The type of the argument to format
	unsigned char specifier;
	// Any modifiers
	unsigned char modifiers;
	// The type of the argument
	unsigned char type;
};

/**
 * Determines how to format a variable according to a format string argument.
 *
 * This *includes* the preceding '%'.
 *
 * On success, the pointer after the consumed characters is returned.
 */
const char *__std_determine_format(const char *input,
				   struct std_format_type *type);

/**
 * Formats an argument in the format string given to e.g. printf & writes it to a buffer.
 *
 * On success, returns the pointer after the characters that have been written.
 *
 * If the buffer is too small, NULL is returned. The output may have been written regardless.
 */
char *__std_format(char *out, size_t size, struct std_format_type *type, va_list * args);

#endif
