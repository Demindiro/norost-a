#ifndef __POSIX_PTHREAD_MUTEX_H
#define __POSIX_PTHREAD_MUTEX_H

#include "sys/time.h"

typedef struct {
} pthread_mutex_t;
typedef struct {
} pthread_mutexattr_t;

enum {
	PTHREAD_MUTEX_NORMAL,
	PTHREAD_MUTEX_ERRORCHECK,
	PTHREAD_MUTEX_RECURSIVE,
	PTHREAD_MUTEX_DEFAULT,
};

enum {
	PTHREAD_MUTEX_STALLED,
	PTHREAD_MUTEX_ROBUST,
};

extern int pthread_mutex_init(pthread_mutex_t * mutex,
			      const pthread_mutexattr_t * mutexattr);

extern int pthread_mutex_destroy(pthread_mutex_t * mutex);

extern int pthread_mutex_trylock(pthread_mutex_t * mutex);

extern int pthread_mutex_lock(pthread_mutex_t * mutex);

extern int pthread_mutex_timedlock(pthread_mutex_t * restrict mutex,
				   const struct timespec *restrict abstime);

extern int pthread_mutex_unlock(pthread_mutex_t * mutex);

extern int pthread_mutex_getprioceiling(const pthread_mutex_t *
					restrict mutex,
					int *restrict prioceiling);

extern int pthread_mutex_setprioceiling(pthread_mutex_t * restrict mutex,
					int prioceiling,
					int *restrict old_ceiling);

extern int pthread_mutex_consistent(pthread_mutex_t * mutex);

extern int pthread_mutexattr_init(pthread_mutexattr_t * attr);

extern int pthread_mutexattr_destroy(pthread_mutexattr_t * attr);

extern int pthread_mutexattr_getpshared(const pthread_mutexattr_t *
					restrict attr, int *restrict pshared);

extern int pthread_mutexattr_setpshared(pthread_mutexattr_t * attr,
					int pshared);

extern int pthread_mutexattr_gettype(const pthread_mutexattr_t * restrict
				     attr, int *restrict kind);

extern int pthread_mutexattr_settype(pthread_mutexattr_t * attr, int kind);

extern int pthread_mutexattr_getprotocol(const pthread_mutexattr_t *
					 restrict attr, int *restrict protocol);

extern int pthread_mutexattr_setprotocol(pthread_mutexattr_t * attr,
					 int protocol);

extern int pthread_mutexattr_getprioceiling(const pthread_mutexattr_t *
					    restrict attr,
					    int *restrict prioceiling);

extern int pthread_mutexattr_setprioceiling(pthread_mutexattr_t * attr,
					    int prioceiling);

extern int pthread_mutexattr_getrobust(const pthread_mutexattr_t * attr,
				       int *robustness);

extern int pthread_mutexattr_setrobust(pthread_mutexattr_t * attr,
				       int robustness);

#endif
