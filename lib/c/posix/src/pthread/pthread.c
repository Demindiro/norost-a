#include "pthread.h"

#include "errno.h"

int pthread_create(pthread_t * restrict thread,
		   const pthread_attr_t * restrict attr,
		   void *(*routine)(void *), void *restrict arg)
{
	return ENOSYS;
}

void pthread_exit(void *ret)
{

}

int pthread_join(pthread_t thread, void **thread_return)
{
	return ENOSYS;
}

int pthread_detach(pthread_t th)
{
	return ENOSYS;
}

pthread_t pthread_self(void)
{
	return 0;
}

int pthread_equal(pthread_t thread1, pthread_t thread2)
{
	return ENOSYS;
}

int pthread_attr_init(pthread_attr_t * attr)
{
	return ENOSYS;
}

int pthread_attr_destroy(pthread_attr_t * attr)
{
	return ENOSYS;
}

int pthread_attr_getdetachstate(const pthread_attr_t * attr, int *detachstate)
{
	return ENOSYS;
}

int pthread_attr_setdetachstate(pthread_attr_t * attr, int detachstate)
{
	return ENOSYS;
}

int pthread_attr_getguardsize(const pthread_attr_t * attr, size_t *guardsize)
{
	return ENOSYS;
}

int pthread_attr_setguardsize(pthread_attr_t * attr, size_t guardsize)
{
	return ENOSYS;
}

int pthread_attr_getschedparam(const pthread_attr_t * restrict attr,
			       struct sched_param *restrict param)
{
	return ENOSYS;
}

int pthread_attr_setschedparam(pthread_attr_t * restrict attr,
			       const struct sched_param *restrict param)
{
	return ENOSYS;
}

int pthread_attr_getschedpolicy(const pthread_attr_t * restrict
				attr, int *restrict policy)
{
	return ENOSYS;
}

int pthread_attr_setschedpolicy(pthread_attr_t * attr, int policy)
{
	return ENOSYS;
}

int pthread_attr_getinheritsched(const pthread_attr_t * restrict
				 attr, int *restrict inherit)
{
	return ENOSYS;
}

int pthread_attr_setinheritsched(pthread_attr_t * attr, int inherit)
{
	return ENOSYS;
}

int pthread_attr_getscope(const pthread_attr_t * restrict attr,
			  int *restrict scope)
{
	return ENOSYS;
}

int pthread_attr_setscope(pthread_attr_t * attr, int scope)
{
	return ENOSYS;
}

int pthread_attr_getstackaddr(const pthread_attr_t * restrict
			      attr, void **restrict stackaddr)
{
	return ENOSYS;
}

int pthread_attr_setstackaddr(pthread_attr_t * attr, void *stackaddr)
{
	return ENOSYS;
}

int pthread_attr_getstacksize(const pthread_attr_t * restrict
			      attr, size_t *restrict stacksize)
{
	return ENOSYS;
}

int pthread_attr_setstacksize(pthread_attr_t * attr, size_t stacksize)
{
	return ENOSYS;
}

int pthread_attr_getstack(const pthread_attr_t * restrict attr,
			  void **restrict stackaddr, size_t *restrict stacksize)
{
	return ENOSYS;
}

int pthread_attr_setstack(pthread_attr_t * attr, void *stackaddr,
			  size_t stacksize)
{
	return ENOSYS;
}

int pthread_setschedparam(pthread_t target_thread, int policy,
			  const struct sched_param *param)
{
	return ENOSYS;
}

int pthread_getschedparam(pthread_t target_thread,
			  int *restrict policy,
			  struct sched_param *restrict param)
{
	return ENOSYS;
}

int pthread_setschedprio(pthread_t target_thread, int prio)
{
	return ENOSYS;
}

int pthread_getconcurrency(void)
{
	return ENOSYS;
}

int pthread_setconcurrency(int level)
{
	return ENOSYS;
}

int pthread_yield(void)
{
	return ENOSYS;
}

int pthread_once(pthread_once_t * once_control, void (*init_routine)(void))
{
	return ENOSYS;
}

int pthread_setcancelstate(int state, int *oldstate)
{
	return ENOSYS;
}

int pthread_setcanceltype(int type, int *oldtype)
{
	return ENOSYS;
}

int pthread_cancel(pthread_t th)
{
	return ENOSYS;
}

void pthread_testcancel(void)
{
}

int pthread_getcpuclockid(pthread_t thread_id, clockid_t * clock_id)
{
	return ENOSYS;
}

int pthread_atfork(void (*prepare)(void), void(*parent)(void),
		   void(*child)(void))
{
	return ENOSYS;
}
