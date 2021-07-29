#ifndef __POSIX_PTHREAD_COND_H
#define __POSIX_PTHREAD_COND_H

#include "mutex.h"

typedef struct {
} pthread_cond_t;
typedef struct {
} pthread_condattr_t;

extern int pthread_cond_init(pthread_cond_t * restrict cond,
			     const pthread_condattr_t * restrict cond_attr);

extern int pthread_cond_destroy(pthread_cond_t * cond);

extern int pthread_cond_signal(pthread_cond_t * cond);

extern int pthread_cond_broadcast(pthread_cond_t * cond);

extern int pthread_cond_wait(pthread_cond_t * restrict cond,
			     pthread_mutex_t * restrict mutex);

extern int pthread_cond_timedwait(pthread_cond_t * restrict cond,
				  pthread_mutex_t * restrict mutex,
				  const struct timespec *restrict abstime);

extern int pthread_condattr_init(pthread_condattr_t * attr);

extern int pthread_condattr_destroy(pthread_condattr_t * attr);

extern int pthread_condattr_getpshared(const pthread_condattr_t *
				       restrict attr, int *restrict pshared);

extern int pthread_condattr_setpshared(pthread_condattr_t * attr, int pshared);

extern int pthread_condattr_getclock(const pthread_condattr_t *
				     restrict attr,
				     clockid_t * restrict clock_id);

extern int pthread_condattr_setclock(pthread_condattr_t * attr,
				     clockid_t clock_id);

#endif
