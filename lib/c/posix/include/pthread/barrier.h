#ifndef __POSIX_PTHREAD_BARRIER_H
#define __POSIX_PTHREAD_BARRIER_H

typedef struct {
} pthread_barrier_t;
typedef struct {
} pthread_barrierattr_t;

extern int pthread_barrier_init(pthread_barrier_t * restrict barrier,
				const pthread_barrierattr_t * restrict
				attr, unsigned int count);

extern int pthread_barrier_destroy(pthread_barrier_t * barrier);

extern int pthread_barrier_wait(pthread_barrier_t * barrier);

extern int pthread_barrierattr_init(pthread_barrierattr_t * attr);

extern int pthread_barrierattr_destroy(pthread_barrierattr_t * attr);

extern int pthread_barrierattr_getpshared(const pthread_barrierattr_t *
					  restrict attr, int *restrict pshared);

extern int pthread_barrierattr_setpshared(pthread_barrierattr_t * attr,
					  int pshared);

#endif
