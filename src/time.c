#include <errno.h>
#include <stdint.h>
#include <sys/time.h>
#include <time.h>

#ifndef ENOSYS
#define ENOSYS EINVAL
#endif

#define LUNACY_CLOCK_REALTIME 1
#define LUNACY_CLOCK_MONOTONIC 2
#define LUNACY_CLOCK_PROCESS_CPUTIME_ID 3
#define LUNACY_CLOCK_THREAD_CPUTIME_ID 4
#define LUNACY_CLOCK_MONOTONIC_RAW 5

#define LUNACY_MISSING_CONSTANT()                                             \
    do {                                                                      \
        errno = ENOSYS;                                                       \
        return -1;                                                            \
    } while (0)

struct lunacy_timespec {
    int64_t seconds;
    int32_t nanoseconds;
};

struct lunacy_timeval {
    int64_t seconds;
    int32_t microseconds;
};

int lunacy_clock_constant(int name, int *out) {
    if (out == NULL) {
        errno = EFAULT;
        return -1;
    }

    switch (name) {
    case LUNACY_CLOCK_REALTIME:
#ifdef CLOCK_REALTIME
        *out = CLOCK_REALTIME;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    case LUNACY_CLOCK_MONOTONIC:
#ifdef CLOCK_MONOTONIC
        *out = CLOCK_MONOTONIC;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    case LUNACY_CLOCK_PROCESS_CPUTIME_ID:
#ifdef CLOCK_PROCESS_CPUTIME_ID
        *out = CLOCK_PROCESS_CPUTIME_ID;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    case LUNACY_CLOCK_THREAD_CPUTIME_ID:
#ifdef CLOCK_THREAD_CPUTIME_ID
        *out = CLOCK_THREAD_CPUTIME_ID;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    case LUNACY_CLOCK_MONOTONIC_RAW:
#ifdef CLOCK_MONOTONIC_RAW
        *out = CLOCK_MONOTONIC_RAW;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    default:
        errno = EINVAL;
        return -1;
    }
}

int lunacy_clock_gettime(int clock_id, struct lunacy_timespec *out) {
    if (out == NULL) {
        errno = EFAULT;
        return -1;
    }

    struct timespec ts;
    if (clock_gettime((clockid_t)clock_id, &ts) < 0) {
        return -1;
    }

    out->seconds = (int64_t)ts.tv_sec;
    out->nanoseconds = (int32_t)ts.tv_nsec;
    return 0;
}

int lunacy_gettimeofday(struct lunacy_timeval *out) {
    if (out == NULL) {
        errno = EFAULT;
        return -1;
    }

    struct timeval tv;
    if (gettimeofday(&tv, NULL) < 0) {
        return -1;
    }

    out->seconds = (int64_t)tv.tv_sec;
    out->microseconds = (int32_t)tv.tv_usec;
    return 0;
}

int lunacy_time_seconds(int64_t *out) {
    if (out == NULL) {
        errno = EFAULT;
        return -1;
    }

    time_t now = time(NULL);
    if (now == (time_t)-1) {
        return -1;
    }

    *out = (int64_t)now;
    return 0;
}

static int lunacy_write_timespec(int64_t seconds, int32_t nanoseconds,
                                 struct timespec *out) {
    if (out == NULL) {
        errno = EFAULT;
        return -1;
    }

    if (seconds < 0 || nanoseconds < 0 || nanoseconds >= 1000000000) {
        errno = EINVAL;
        return -1;
    }

    out->tv_sec = (time_t)seconds;
    out->tv_nsec = (long)nanoseconds;
    if ((int64_t)out->tv_sec != seconds || (int32_t)out->tv_nsec != nanoseconds) {
        errno = EINVAL;
        return -1;
    }

    return 0;
}

int lunacy_nanosleep(int64_t seconds, int32_t nanoseconds,
                     int64_t *remaining_seconds,
                     int32_t *remaining_nanoseconds) {
    if (remaining_seconds == NULL || remaining_nanoseconds == NULL) {
        errno = EFAULT;
        return -1;
    }

    struct timespec requested;
    struct timespec remaining;
    if (lunacy_write_timespec(seconds, nanoseconds, &requested) < 0) {
        return -1;
    }

    *remaining_seconds = 0;
    *remaining_nanoseconds = 0;
    if (nanosleep(&requested, &remaining) == 0) {
        return 0;
    }

    if (errno == EINTR) {
        *remaining_seconds = (int64_t)remaining.tv_sec;
        *remaining_nanoseconds = (int32_t)remaining.tv_nsec;
    }

    return -1;
}
