#ifndef __POSIX_PTHREAD_H
#define __POSIX_PTHREAD_H

#include <stddef.h>

typedef size_t pthread_t;

/* Boolean value used to ensure a function is executed only once */
// FIXME should be atomic
typedef unsigned char pthread_once_t;

struct sched_param {
};

typedef struct {
} pthread_attr_t;

typedef size_t clockid_t;

#include "pthread/barrier.h"
#include "pthread/cond.h"
#include "pthread/mutex.h"
#include "pthread/rwlock.h"
#include "pthread/spinlock.h"
#include "pthread/storage.h"

enum {
	PTHREAD_CREATE_DETACHED,
	PTHREAD_CREATE_JOINABLE,
};

enum {
	PTHREAD_PRIO_INHERIT,
	PTHREAD_PRIO_NONE,
	PTHREAD_PRIO_PROTECT,
};

enum {
	PTHREAD_INHERIT_SCHED,
	PTHREAD_EXPLICIT_SCHED,
};

enum {
	PTHREAD_SCOPE_SYSTEM,
	PTHREAD_SCOPE_PROCESS,
};

enum {
	PTHREAD_PROCESS_PRIVATE,
	PTHREAD_PROCESS_SHARED,
};

enum {
	PTHREAD_CANCEL_ENABLE,
	PTHREAD_CANCEL_DISABLE
};

enum {
	PTHREAD_CANCEL_DEFERRED,
	PTHREAD_CANCEL_ASYNCHRONOUS
};

extern int pthread_create(pthread_t * restrict thread,
			  const pthread_attr_t * restrict attr,
			  void *(*routine)(void *), void *restrict arg);

extern void pthread_exit(void *ret);

extern int pthread_join(pthread_t thread, void **thread_return);

extern int pthread_detach(pthread_t th);

extern pthread_t pthread_self(void);

extern int pthread_equal(pthread_t thread1, pthread_t thread2);

extern int pthread_attr_init(pthread_attr_t * attr);

extern int pthread_attr_destroy(pthread_attr_t * attr);

extern int pthread_attr_getdetachstate(const pthread_attr_t * attr,
				       int *detachstate);

extern int pthread_attr_setdetachstate(pthread_attr_t * attr, int detachstate);

extern int pthread_attr_getguardsize(const pthread_attr_t * attr,
				     size_t *guardsize);

extern int pthread_attr_setguardsize(pthread_attr_t * attr, size_t guardsize);

extern int pthread_attr_getschedparam(const pthread_attr_t * restrict attr,
				      struct sched_param *restrict param);

extern int pthread_attr_setschedparam(pthread_attr_t * restrict attr,
				      const struct sched_param *restrict param);

extern int pthread_attr_getschedpolicy(const pthread_attr_t * restrict
				       attr, int *restrict policy);

extern int pthread_attr_setschedpolicy(pthread_attr_t * attr, int policy);

extern int pthread_attr_getinheritsched(const pthread_attr_t * restrict
					attr, int *restrict inherit);

extern int pthread_attr_setinheritsched(pthread_attr_t * attr, int inherit);

extern int pthread_attr_getscope(const pthread_attr_t * restrict attr,
				 int *restrict scope);

extern int pthread_attr_setscope(pthread_attr_t * attr, int scope);

extern int pthread_attr_getstackaddr(const pthread_attr_t * restrict
				     attr, void **restrict stackaddr);

extern int pthread_attr_setstackaddr(pthread_attr_t * attr, void *stackaddr);

extern int pthread_attr_getstacksize(const pthread_attr_t * restrict
				     attr, size_t *restrict stacksize);

extern int pthread_attr_setstacksize(pthread_attr_t * attr, size_t stacksize);

extern int pthread_attr_getstack(const pthread_attr_t * restrict attr,
				 void **restrict stackaddr,
				 size_t *restrict stacksize);

extern int pthread_attr_setstack(pthread_attr_t * attr, void *stackaddr,
				 size_t stacksize);

extern int pthread_setschedparam(pthread_t target_thread, int policy,
				 const struct sched_param *param);

extern int pthread_getschedparam(pthread_t target_thread,
				 int *restrict policy,
				 struct sched_param *restrict param);

extern int pthread_setschedprio(pthread_t target_thread, int prio);

extern int pthread_getconcurrency(void);

extern int pthread_setconcurrency(int level);

extern int pthread_yield(void);

extern int pthread_once(pthread_once_t * once_control,
			void (*init_routine)(void));

extern int pthread_setcancelstate(int state, int *oldstate);

extern int pthread_setcanceltype(int type, int *oldtype);

extern int pthread_cancel(pthread_t th);

extern void pthread_testcancel(void);

extern int pthread_getcpuclockid(pthread_t thread_id, clockid_t * clock_id);

extern int pthread_atfork(void (*prepare)(void), void(*parent)(void),
			  void(*child)(void));

#endif
