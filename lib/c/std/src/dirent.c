#include <dirent.h>
#include <errno.h>
#include <string.h>

int alphasort(const struct dirent **lhs, const  struct dirent **rhs) {
	return strncmp((*lhs)->d_name, (*rhs)->d_name, sizeof((*lhs)->d_name));
}

int closedir(DIR *dir) {
	return (errno = 0);
}

int dirfd(DIR *dir) {
	if (dir->_fd == -1) {
		// TODO
	}
	return dir->_fd;
}

DIR *fdopendir(int fd) {
	// FIXME SCHTOOOOOOOOOPID
	// We need to implement an allocator at some point.
	static DIR dir = {};
	dir._fd = fd;
	return &dir;
}

DIR *opendir(const char *path) {
	static struct __std_dirent_entry entries[3] = {
		{
			.name = "foo",
			.name_len = 3,
		},
		{
			.name = "bar",
			.name_len = 3,
		},
		{
			.name = "qux",
			.name_len = 3,
		},
	};
	static DIR dir = {
		._entries = entries,
		._count = 3,
		._index = 0,
	};
	dir._index = 0;
	return &dir;
}

struct dirent *readdir(DIR *dir) {
	static struct dirent ent = {}; // FIXME
	if (dir->_index++ < dir->_count) {
		const struct __std_dirent_entry *e = &dir->_entries[dir->_index - 1];
		ent.d_ino = kernel_uuid(0, 0);
		size_t max_len = sizeof(ent.d_name) - 1; // Account of mandatory null terminator
		max_len = max_len < e->name_len ? max_len : e->name_len;
		memcpy(ent.d_name, e->name, max_len);
		ent.d_name[max_len] = '\0';
		return &ent;
	}
	return NULL;
}

void rewinddir(DIR *dir) {
	dir->_index = 0;
}

int scandir(const char *, struct dirent **, int (*)(const struct dirent *), int (*)(const struct dirent **, const struct dirent **)) {
	return (errno = ENOSYS);
}

void seekdir(DIR *dir, long loc) {
	dir->_index = loc;
}

long telldir(DIR *dir) {
	return dir->_index;
}
