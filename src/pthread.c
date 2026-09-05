#define _POSIX_C_SOURCE 200809L
#include "abi.h"
#include <errno.h>
#include <pthread.h>
#include <sched.h>
#include <stdlib.h>
#include <string.h>

_Static_assert(sizeof(pthread_t) <= sizeof(struct lunacy_storage),
               "lunacy pthread ID storage is too small for this target");
_Static_assert(_Alignof(pthread_t) <= _Alignof(struct lunacy_storage),
               "lunacy pthread ID alignment is too small for this target");
_Static_assert(_Alignof(pthread_mutex_t) <= _Alignof(max_align_t),
               "native pthread mutex requires stronger alignment than malloc");

/* pthread functions return error numbers directly, rather than setting errno. */
int lunacy_thread_create(struct lunacy_storage *out, void *(*entry)(void *),
                         void *context, int set_stack_size, size_t stack_size) {
    pthread_t thread;
    int result;
    if (set_stack_size) {
        pthread_attr_t attr;
        result = pthread_attr_init(&attr);
        if (result != 0) return result;
        result = pthread_attr_setstacksize(&attr, stack_size);
        if (result == 0) result = pthread_create(&thread, &attr, entry, context);
        pthread_attr_destroy(&attr);
    } else {
        result = pthread_create(&thread, NULL, entry, context);
    }
    if (result == 0) {
        memset(out, 0, sizeof(*out));
        memcpy(out, &thread, sizeof(thread));
    }
    return result;
}

int lunacy_thread_join(const struct lunacy_storage *storage) {
    pthread_t thread;
    memcpy(&thread, storage, sizeof(thread));
    /* Some libcs do not diagnose self-join, so check while the ID is live. */
    if (pthread_equal(thread, pthread_self())) return EDEADLK;
    return pthread_join(thread, NULL);
}

int lunacy_thread_detach(const struct lunacy_storage *storage) {
    pthread_t thread;
    memcpy(&thread, storage, sizeof(thread));
    return pthread_detach(thread);
}

int lunacy_thread_is_current(const struct lunacy_storage *storage) {
    pthread_t thread;
    memcpy(&thread, storage, sizeof(thread));
    return pthread_equal(thread, pthread_self()) != 0;
}

int lunacy_thread_yield(void) { return sched_yield(); }

int lunacy_mutex_create(void **out) {
    pthread_mutex_t *mutex = malloc(sizeof(*mutex));
    if (mutex == NULL) return ENOMEM;
    pthread_mutexattr_t attr;
    int result = pthread_mutexattr_init(&attr);
    if (result != 0) { free(mutex); return result; }
    /* Explicit ERRORCHECK avoids PTHREAD_MUTEX_DEFAULT's undefined cases and
       returns EDEADLK for recursive locking instead of granting a second guard. */
    result = pthread_mutexattr_settype(&attr, PTHREAD_MUTEX_ERRORCHECK);
    if (result == 0) result = pthread_mutex_init(mutex, &attr);
    pthread_mutexattr_destroy(&attr);
    if (result != 0) { free(mutex); return result; }
    *out = mutex;
    return 0;
}

int lunacy_mutex_lock(void *mutex) { return pthread_mutex_lock(mutex); }
int lunacy_mutex_trylock(void *mutex) { return pthread_mutex_trylock(mutex); }
int lunacy_mutex_unlock(void *mutex) { return pthread_mutex_unlock(mutex); }

int lunacy_mutex_destroy(void *mutex) {
    /* Safe Rust can forget a guard. Do not invoke pthread_mutex_destroy on a
       locked mutex: that is undefined by POSIX, even if a libc returns EBUSY.
       The Rust owner is exclusive here, so no new lockers can race this probe. */
    int result = pthread_mutex_trylock(mutex);
    if (result != 0) return result; /* Deliberately leak a still-locked mutex. */
    result = pthread_mutex_unlock(mutex);
    if (result != 0) return result;
    result = pthread_mutex_destroy(mutex);
    if (result == 0) free(mutex);
    return result;
}
