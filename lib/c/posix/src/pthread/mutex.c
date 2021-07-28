#include "pthread/mutex.h"

#include "errno.h"

int pthread_mutex_init(pthread_mutex_t * mutex,
		       const pthread_mutexattr_t * mutexattr)
{
	return ENOSYS;
}

int pthread_mutex_destroy(pthread_mutex_t * mutex)
{
	return ENOSYS;
}

int pthread_mutex_trylock(pthread_mutex_t * mutex)
{
	return ENOSYS;
}

int pthread_mutex_lock(pthread_mutex_t * mutex)
{
	return ENOSYS;
}

int pthread_mutex_timedlock(pthread_mutex_t * restrict mutex,
			    const struct timespec *restrict abstime)
{
	return ENOSYS;
}

int pthread_mutex_unlock(pthread_mutex_t * mutex)
{
	return ENOSYS;
}

int pthread_mutex_getprioceiling(const pthread_mutex_t *
				 restrict mutex, int *restrict prioceiling)
{
	return ENOSYS;
}

int pthread_mutex_setprioceiling(pthread_mutex_t * restrict mutex,
				 int prioceiling, int *restrict old_ceiling)
{
	return ENOSYS;
}

int pthread_mutex_consistent(pthread_mutex_t * mutex)
{
	return ENOSYS;
}

int pthread_mutexattr_init(pthread_mutexattr_t * attr)
{
	return ENOSYS;
}

int pthread_mutexattr_destroy(pthread_mutexattr_t * attr)
{
	return ENOSYS;
}

int pthread_mutexattr_getpshared(const pthread_mutexattr_t *
				 restrict attr, int *restrict pshared)
{
	return ENOSYS;
}

int pthread_mutexattr_setpshared(pthread_mutexattr_t * attr, int pshared)
{
	return ENOSYS;
}

int pthread_mutexattr_gettype(const pthread_mutexattr_t * restrict
			      attr, int *restrict kind)
{
	return ENOSYS;
}

int pthread_mutexattr_settype(pthread_mutexattr_t * attr, int kind)
{
	return ENOSYS;
}

int pthread_mutexattr_getprotocol(const pthread_mutexattr_t *
				  restrict attr, int *restrict protocol)
{
	return ENOSYS;
}

int pthread_mutexattr_setprotocol(pthread_mutexattr_t * attr, int protocol)
{
	return ENOSYS;
}

int pthread_mutexattr_getprioceiling(const pthread_mutexattr_t *
				     restrict attr, int *restrict prioceiling)
{
	return ENOSYS;
}

int pthread_mutexattr_setprioceiling(pthread_mutexattr_t * attr,
				     int prioceiling)
{
	return ENOSYS;
}

int pthread_mutexattr_getrobust(const pthread_mutexattr_t * attr,
				int *robustness)
{
	return ENOSYS;
}

int pthread_mutexattr_setrobust(pthread_mutexattr_t * attr, int robustness)
{
	return ENOSYS;
}
