#include <stddef.h>
#include <string.h>

size_t strlen(const char *s)
{
	const char *e = s;
	while (*e != 0) {
		e++;
	}
	return e - s;
}
