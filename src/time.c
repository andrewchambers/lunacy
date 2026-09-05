#define _POSIX_C_SOURCE 200809L
#include <errno.h>
#include <stdint.h>
#include <time.h>

/* Convert fields explicitly; Rust does not assume the native timespec layout. */
_Static_assert(sizeof(time_t) <= sizeof(int64_t) && (time_t)-1 < 0,
               "lunacy requires a signed time_t no wider than 64 bits");
_Static_assert(sizeof(clockid_t) <= sizeof(int64_t) && (clockid_t)-1 < 0,
               "lunacy requires a signed clockid_t no wider than 64 bits");
_Static_assert(sizeof(long) <= sizeof(int64_t), "native nanoseconds must fit i64");

int64_t lunacy_clock_realtime(void) { return CLOCK_REALTIME; }
int64_t lunacy_clock_monotonic(void) { return CLOCK_MONOTONIC; }
int64_t lunacy_clock_process_cputime(void) { return CLOCK_PROCESS_CPUTIME_ID; }
int64_t lunacy_clock_thread_cputime(void) { return CLOCK_THREAD_CPUTIME_ID; }

static int clock_id(int64_t raw, clockid_t *native) {
    *native = (clockid_t)raw;
    if ((int64_t)*native != raw) { errno = EOVERFLOW; return -1; }
    return 0;
}

int lunacy_clock_gettime(int64_t raw, int64_t *seconds, int64_t *nanoseconds) {
    clockid_t clock;
    if (clock_id(raw, &clock) == -1) return -1;
    struct timespec value;
    int result = clock_gettime(clock, &value);
    if (result == 0) {
        *seconds = value.tv_sec;
        *nanoseconds = value.tv_nsec;
    }
    return result;
}

int lunacy_clock_getres(int64_t raw, int64_t *seconds, int64_t *nanoseconds) {
    clockid_t clock;
    if (clock_id(raw, &clock) == -1) return -1;
    struct timespec value;
    int result = clock_getres(clock, &value);
    if (result == 0) {
        *seconds = value.tv_sec;
        *nanoseconds = value.tv_nsec;
    }
    return result;
}

int lunacy_nanosleep(int64_t seconds, int64_t nanoseconds,
                     int64_t *remaining_seconds, int64_t *remaining_nanoseconds) {
    if (seconds < 0 || nanoseconds < 0 || nanoseconds >= 1000000000) {
        errno = EINVAL;
        return -1;
    }
    struct timespec request = { .tv_sec = (time_t)seconds, .tv_nsec = (long)nanoseconds };
    if ((int64_t)request.tv_sec != seconds) { errno = EOVERFLOW; return -1; }
    struct timespec remaining;
    int result = nanosleep(&request, remaining_seconds ? &remaining : NULL);
    int error = errno;
    if (result == -1 && error == EINTR && remaining_seconds) {
        *remaining_seconds = remaining.tv_sec;
        *remaining_nanoseconds = remaining.tv_nsec;
    }
    errno = error;
    return result;
}
