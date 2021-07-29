#ifndef __POSIX_PTHREAD_STORAGE_H
#define __POSIX_PTHREAD_STORAGE_H

typedef struct {

} pthread_key_t;

extern int pthread_key_create(pthread_key_t * key,
			      void (*destr_function)(void *));
extern int pthread_key_delete(pthread_key_t key);

extern void *pthread_getspecific(pthread_key_t key);

extern int pthread_setspecific(pthread_key_t key, const void *pointer);

#endif
