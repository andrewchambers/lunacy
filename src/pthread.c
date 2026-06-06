#include <errno.h>
#include <pthread.h>
#include <stddef.h>
#include <stdlib.h>

static int lunacy_pthread_status(int status) {
    if (status == 0) {
        return 0;
    }

    errno = status;
    return -1;
}

void *lunacy_pthread_create(void *(*start)(void *), void *arg) {
    pthread_t *thread = (pthread_t *)malloc(sizeof(*thread));
    if (thread == NULL) {
        errno = ENOMEM;
        return NULL;
    }

    int status = pthread_create(thread, NULL, start, arg);
    if (status != 0) {
        errno = status;
        free(thread);
        return NULL;
    }

    return thread;
}

int lunacy_pthread_detach(void *thread) {
    int status = pthread_detach(*(pthread_t *)thread);
    free(thread);

    if (status != 0) {
        errno = status;
        return -1;
    }

    return 0;
}

int lunacy_pthread_join(void *thread) {
    int status = pthread_join(*(pthread_t *)thread, NULL);
    free(thread);

    if (status != 0) {
        errno = status;
        return -1;
    }

    return 0;
}

void *lunacy_pthread_mutex_create(void) {
    pthread_mutex_t *mutex = (pthread_mutex_t *)malloc(sizeof(*mutex));
    if (mutex == NULL) {
        errno = ENOMEM;
        return NULL;
    }

    int status = pthread_mutex_init(mutex, NULL);
    if (status != 0) {
        errno = status;
        free(mutex);
        return NULL;
    }

    return mutex;
}

int lunacy_pthread_mutex_lock(void *mutex) {
    return lunacy_pthread_status(pthread_mutex_lock((pthread_mutex_t *)mutex));
}

int lunacy_pthread_mutex_unlock(void *mutex) {
    return lunacy_pthread_status(pthread_mutex_unlock((pthread_mutex_t *)mutex));
}

int lunacy_pthread_mutex_destroy(void *mutex) {
    int status = pthread_mutex_destroy((pthread_mutex_t *)mutex);
    if (status != 0) {
        errno = status;
        return -1;
    }

    free(mutex);
    return 0;
}
