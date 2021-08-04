#include "common.h"
#include <dirent.h>
#include <dux.h>
#include <errno.h>
#include <kernel.h>
#include <string.h>

#define PAGE_SIZE (4096) /* TODO */

int alphasort(const struct dirent **lhs, const struct dirent **rhs)
{
	return strncmp((*lhs)->d_name, (*rhs)->d_name, sizeof((*lhs)->d_name));
}

int closedir(DIR * dir)
{
	size_t page_count = (dir->_list.data_len + PAGE_SIZE - 1) / PAGE_SIZE;
	kernel_mem_dealloc(dir->_list.data, page_count);
	dux_add_free_range(dir->_list.data, page_count);
	return (errno = 0);
}

int dirfd(DIR * dir)
{
	if (dir->_fd == -1) {
		// TODO
	}
	return dir->_fd;
}

DIR *fdopendir(int fd)
{
	// FIXME SCHTOOOOOOOOOPID
	// We need to implement an allocator at some point.
	static DIR dir = { };
	dir._fd = fd;
	return &dir;
}

DIR *opendir(const char *path)
{
	char *out = universal_buffer;

	// Fill out the path
	char *ptr = out;
	char *end = out + universal_buffer_size;
	const char *c = path;
	while (ptr != end && *c != '\0') {
		*ptr++ = *c++;
	}

	// Get a request entry
	struct kernel_ipc_packet *pkt = NULL;
	while (pkt == NULL) {
		kernel_io_wait(0, 0);
		pkt = dux_reserve_transmit_entry();
	}

	// Fill out the request entry
	pkt->flags = 0;
	pkt->address = 0;
	pkt->uuid = kernel_uuid(0, 0);
	pkt->offset = 0;
	pkt->data.raw = universal_buffer;
	pkt->length = ptr - out;
	asm volatile ("fence");
	pkt->opcode = KERNEL_IPC_OP_LIST;

	void *data;
	size_t data_len;

	struct kernel_ipc_packet *cce = dux_get_receive_entry();
	cce->opcode = 0;
	for (;;) {
		kernel_io_wait(0, 0);
		if (cce->opcode == KERNEL_IPC_OP_LIST) {
			data = cce->data.raw;
			data_len = cce->length;
			cce->opcode = KERNEL_IPC_OP_NONE;
			break;
		}
		//completion_index++;
		//completion_index &= request_mask;

		// Flush the queue
		kernel_io_wait(0, 0);
	}

	static DIR dir = {
		._index = 0,
	};
	dir._list.data = data;
	dir._list.data_len = data_len;
	dir._index = 0;
	return &dir;
}

struct dirent *readdir(DIR * dir)
{
	struct dux_ipc_list_entry e;
	if (dux_ipc_list_get(dir->_list, dir->_index, &e) < 0) {
		return NULL;
	}
	dir->_index += 1;

	static struct dirent ent = { };	// FIXME
	ent.d_ino = kernel_uuid(0, 0);
	size_t max_len = sizeof(ent.d_name) - 1;	// Account of mandatory null terminator
	max_len = max_len < e.name_len ? max_len : e.name_len;
	memcpy(ent.d_name, e.name, max_len);
	ent.d_name[max_len] = '\0';

	return &ent;
}

void rewinddir(DIR * dir)
{
	dir->_index = 0;
}

int scandir(const char *, struct dirent **,
	    int (*)(const struct dirent *),
	    int (*)(const struct dirent **, const struct dirent **))
{
	return (errno = ENOSYS);
}

void seekdir(DIR * dir, long loc)
{
	dir->_index = loc;
}

long telldir(DIR * dir)
{
	return dir->_index;
}
