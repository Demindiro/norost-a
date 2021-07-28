#ifndef __POSIX_PTHREAD_H
#define __POSIX_PTHREAD_H

extern int pthread_rwlock_init(pthread_rwlock_t * restrict rwlock,
			       const pthread_rwlockattr_t * restrict attr);

extern int pthread_rwlock_destroy(pthread_rwlock_t * rwlock);

extern int pthread_rwlock_rdlock(pthread_rwlock_t * rwlock);

extern int pthread_rwlock_tryrdlock(pthread_rwlock_t * rwlock);

extern int pthread_rwlock_timedrdlock(pthread_rwlock_t * restrict rwlock,
				      const struct timespec *restrict abstime);

extern int pthread_rwlock_wrlock(pthread_rwlock_t * rwlock);

extern int pthread_rwlock_trywrlock(pthread_rwlock_t * rwlock);

extern int pthread_rwlock_timedwrlock(pthread_rwlock_t * restrict rwlock,
				      const struct timespec *restrict abstime);

extern int pthread_rwlock_unlock(pthread_rwlock_t * rwlock);

extern int pthread_rwlockattr_init(pthread_rwlockattr_t * attr);

extern int pthread_rwlockattr_destroy(pthread_rwlockattr_t * attr);

extern int pthread_rwlockattr_getpshared(const pthread_rwlockattr_t *
					 restrict attr, int *restrict pshared);

extern int pthread_rwlockattr_setpshared(pthread_rwlockattr_t * attr,
					 int pshared);

#endif
