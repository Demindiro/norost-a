#ifndef __FD_H
#define __FD_H

#include <stdint.h>
#include <stdio.h>

// FIXME some kind of free file descriptor stack is necessary

static FILE *__std_pop_free_file(void) {
	FILE *f = &__files_list[__files_count];
	__files_count += 1;
	return f;
}

static void __std_push_free_file(int fd) {
	/* TODO */
}

#endif
