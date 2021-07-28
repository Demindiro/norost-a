#ifndef __POSIX_PTHREAD_SPINLOCK_H
#define __POSIX_PTHREAD_SPINLOCK_H

typedef struct {
} pthread_spinlock_t;

extern int pthread_spin_init(pthread_spinlock_t * lock, int pshared);

extern int pthread_spin_destroy(pthread_spinlock_t * lock);

extern int pthread_spin_lock(pthread_spinlock_t * lock);

extern int pthread_spin_trylock(pthread_spinlock_t * lock);

extern int pthread_spin_unlock(pthread_spinlock_t * lock);

#endif
