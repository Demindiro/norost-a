#include <stddef.h>
#include <stdint.h>
#include "format.h"

/**
 * Inserts the given null-terminated string or inserts '(null)' if value is NULL.
 *
 * Returns NULL if the buffer isn't large enough.
 */
static inline char *format_str(const char *value, char *str, size_t size,
			       size_t max)
{
	value = value ? value : "(null)";

	while (*value != '\0') {
		if (max-- == 0) {
			return str;
		}
		if (size == 0) {
			return NULL;
		}
		*str++ = *value++;
		size--;
	}

	return str;
}

/**
 * Formats the given unsigned number with the given base and modifiers as a human-readable string.
 *
 * Returns NULL if the buffer isn't large enough.
 */
static inline char *format_unsigned_int(uintmax_t value, char *str, size_t size,
					unsigned char base,
					unsigned char modifiers)
{
	char *end = str + size;
	if (modifiers & STD_FORMAT_SIGNED) {
		if (str != end) {
			*str++ = '+';
		} else {
			return NULL;
		}
	} else if (modifiers & STD_FORMAT_SPACE) {
		if (str != end) {
			*str++ = ' ';
		} else {
			return NULL;
		}
	}
	// Write out in reverse
	char *start = str;
	do {
		if (str == end) {
			return NULL;
		}
		unsigned int c = value % base;
		if (c >= 10) {
			c += (modifiers & STD_FORMAT_UPPER ? 'A' : 'a') - 10;
		} else {
			c += '0';
		}
		value /= base;
		*str++ = c;
	} while (value != 0);

	// Reverse chars to put the digits in the correct order
	end = str - 1;
	for (; end > start;) {
		char t = *end;
		*end-- = *start;
		*start++ = t;
	}

	return str;
}

/**
 * Formats the given signed number with the given base and modifiers as a human-readable string.
 *
 * Returns NULL if the buffer isn't large enough.
 */
static inline char *format_signed_int(intmax_t value, char *str, size_t size,
				      unsigned char base,
				      unsigned char modifiers)
{
	if (value < 0) {
		if (size > 0) {
			*str++ = '-';
			size -= 1;
			value = -value;
		} else {
			return NULL;
		}
	} else if (modifiers & STD_FORMAT_SIGNED) {
		// Make sure format_unsigned_int doesn't insert yet another sign
		modifiers &= ~STD_FORMAT_SIGNED;
		if (size > 0) {
			*str++ = '+';
			size -= 1;
		} else {
			return NULL;
		}
	} else if (modifiers & STD_FORMAT_SPACE) {
		// Make sure format_unsigned_int doesn't insert yet another space
		modifiers &= ~STD_FORMAT_SPACE;
		if (size > 0) {
			*str++ = ' ';
			size -= 1;
		} else {
			return NULL;
		}
	}

	return format_unsigned_int((uintmax_t) value, str, size, base,
				   modifiers);
}

const char *__std_determine_format(const char *input,
				   struct std_format_type *type)
{

	// Don't do anything if this isn't an argument
	if (*input++ != '%') {
		return NULL;
	}
	// Ensure there will be no unitialized values
	type->width = 0;
	type->precision = 0;
	type->modifiers = 0;
	type->size = 0;
	type->type = STD_FORMAT_TYPE_NONE;

	// Check if we should just print a '%'
	if (*input == '%') {
		type->specifier = STD_FORMAT_PERCENT;
		type->size = 1;
		return input + 1;
	}
	// Check if there are any modifiers to apply
	for (;;) {
		switch (*input) {
		case '-':
			type->modifiers |= STD_FORMAT_LJUST;
			break;
		case '+':
			type->modifiers |= STD_FORMAT_SIGNED;
			break;
		case ' ':
			type->modifiers |= STD_FORMAT_SPACE;
			break;
		case '#':
			type->modifiers |= STD_FORMAT_PREFIX_OR_DECIMAL;
			break;
		case '0':
			type->modifiers |= STD_FORMAT_ZEROES;
			break;
		default:
			goto modifiers_done;
		}
	}
 modifiers_done:

	// Check if a width has been specified
	if (*input == '*') {
		type->modifiers |= STD_FORMAT_VAR_WIDTH;
	} else {
		while ('0' <= *input && *input <= '9') {
			type->width *= 10;
			type->width += *input - '0';
		}
	}

	// Check if a precision has been specified
	if (*input == '.') {
		input++;
		if (*input == '*') {
			type->modifiers |= STD_FORMAT_VAR_PRECISION;
		} else {
			while ('0' <= *input && *input <= '9') {
				type->precision *= 10;
				type->precision += *input - '0';
			}
		}
	}
	// Check if there is a length specifier
	switch (*input++) {
	case 'h':
		if (*input == 'h') {
			input++;
			type->type = STD_FORMAT_TYPE_CHAR;
		} else {
			type->type = STD_FORMAT_TYPE_SHORT;
		}
		break;
	case 'l':
		if (*input == 'l') {
			input++;
			type->type = STD_FORMAT_TYPE_LONG_LONG;
		} else {
			type->type = STD_FORMAT_TYPE_LONG;
		}
		break;
	case 'j':
		type->type = STD_FORMAT_TYPE_INTMAX_T;
		break;
	case 'z':
		type->type = STD_FORMAT_TYPE_SIZE_T;
		break;
	case 't':
		type->type = STD_FORMAT_TYPE_PTRDIFF_T;
		break;
	case 'L':
		type->type = STD_FORMAT_TYPE_LONG_DOUBLE;
	default:
		type->type = STD_FORMAT_TYPE_INT;
		// Undo so we read the specifier correctly later
		input--;
		break;
	}

	// Check the specifier
	int c = *input++;
	if ('A' <= c && c <= 'Z') {
		type->modifiers |= STD_FORMAT_UPPER;
		c = c - 'A' + 'a';
	}
	switch (c) {
	case 'd':
	case 'i':
		type->specifier = STD_FORMAT_DEC;
		break;
	case 'u':
		type->specifier = STD_FORMAT_UDEC;
		break;
	case 'o':
		type->specifier = STD_FORMAT_OCTAL;
		break;
	case 'x':
		type->specifier = STD_FORMAT_HEX;
		break;
	case 'f':
		type->specifier = STD_FORMAT_FLOAT;
		break;
	case 'e':
		type->specifier = STD_FORMAT_SCIENCE;
		break;
	case 'g':
		type->specifier = STD_FORMAT_FLOAT_OR_SCIENCE;
		break;
	case 'a':
		type->specifier = STD_FORMAT_HEX_FLOAT;
		break;
	case 'c':
		type->specifier = STD_FORMAT_CHAR;
		break;
	case 's':
		type->specifier = STD_FORMAT_STRING;
		type->type = STD_FORMAT_TYPE_NONE;
		break;
	case 'p':
		type->specifier = STD_FORMAT_POINTER;
		break;
	case 'n':
		type->specifier = STD_FORMAT_COUNT;
		break;
	default:
		// The specifier is invalid, so return NULL and let the caller handle it.
		return NULL;
	}

	return input;
}

char *__std_format(char *out, size_t size, struct std_format_type *type,
		   va_list * args)
{
	intmax_t sval = 0;
	uintmax_t val = 0;

	int sign = type->specifier == STD_FORMAT_DEC;

	// Load based on type
	switch (type->type) {
		//case STD_FORMAT_TYPE_CHAR:
		//case STD_FORMAT_TYPE_SHORT:
	case STD_FORMAT_TYPE_INT:
		sign ? (sval = va_arg(*args, signed int)) : (val = va_arg(*args, unsigned
									  int));
		break;
	case STD_FORMAT_TYPE_LONG:
		sign ? (sval = va_arg(*args, signed long)) : (val =
							      va_arg(*args,
								     unsigned
								     long));
		break;
	case STD_FORMAT_TYPE_LONG_LONG:
		sign ? (sval = va_arg(*args, signed long long)) : (val =
								   va_arg(*args,
									  unsigned
									  long
									  long));
		break;
	case STD_FORMAT_TYPE_INTMAX_T:
		sign ? (sval = va_arg(*args, intmax_t)) : (val =
							   va_arg(*args,
								  uintmax_t));
		break;
	case STD_FORMAT_TYPE_SIZE_T:
		// There is technically no ssize_t in standard C.
		val = va_arg(*args, size_t);
		break;
	case STD_FORMAT_TYPE_PTRDIFF_T:
		// Ditto
		sval = va_arg(*args, ptrdiff_t);
		break;
	case STD_FORMAT_TYPE_POINTER:
		// FIXME void * may be larger than uintmax_t! see https://stackoverflow.com/a/1572189
		// It may be worth to add a special type that switches between uintmax_t and uintptr_t
		// depending on largest size.
		val = (uintmax_t) va_arg(*args, void *);
		break;
	case STD_FORMAT_TYPE_NONE:
	default:
		break;
	}

	// Format based on specifier
	switch (type->specifier) {
	case STD_FORMAT_DEC:
		return format_signed_int(sval, out, size, 10, type->modifiers);
	case STD_FORMAT_UDEC:
		return format_unsigned_int(val, out, size, 10, type->modifiers);
	case STD_FORMAT_OCTAL:
		return format_unsigned_int(val, out, size, 8, type->modifiers);
	case STD_FORMAT_HEX:
		return format_unsigned_int(val, out, size, 16, type->modifiers);
	case STD_FORMAT_FLOAT:
		return format_str("(todo)", out, size, -1);
	case STD_FORMAT_SCIENCE:
		return format_str("(todo)", out, size, -1);
	case STD_FORMAT_FLOAT_OR_SCIENCE:
		return format_str("(todo)", out, size, -1);
	case STD_FORMAT_HEX_FLOAT:
		return format_str("(todo)", out, size, -1);
	case STD_FORMAT_CHAR:
		return format_str("(todo)", out, size, -1);
	case STD_FORMAT_STRING:
		return format_str(va_arg(*args, const char *), out, size, -1);
	case STD_FORMAT_POINTER:
		return ((out = format_str("0x", out, size, -1)) != NULL)
		    ? format_unsigned_int(val, out, size - 2, 16,
					  type->modifiers)
		    : NULL;
	case STD_FORMAT_COUNT:
		// TODO set the pointer thingie
		return out;
	case STD_FORMAT_PERCENT:
		if (size > 0) {
			*out++ = '%';
			return out;
		} else {
			return NULL;
		}
	default:
		return NULL;
	}
}
