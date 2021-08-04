#ifndef __STD_DIRENT_H
#define __STD_DIRENT_H

#include <dux.h>
#include <sys/types.h>
#include <stdint.h>
#include <kernel.h>
#include <limits.h>

typedef struct {
	kernel_uuid_t uuid;
	pid_t _address;
	struct dux_ipc_list _list;
	size_t _index;
	int _fd;
} DIR;

struct dirent {
	ino_t d_ino;
	char d_name[NAME_MAX];
};

int alphasort(const struct dirent **lhs, const struct dirent **rhs);

int closedir(DIR * dir);

int dirfd(DIR * dir);

DIR *fdopendir(int fd);

DIR *opendir(const char *path);

struct dirent *readdir(DIR * dir);

void rewinddir(DIR * dir);

int scandir(const char *, struct dirent **, int (*)(const struct dirent *),
	    int (*)(const struct dirent **, const struct dirent **));

void seekdir(DIR * dir, long loc);

long telldir(DIR * dir);

#endif
