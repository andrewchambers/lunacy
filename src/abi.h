#ifndef LUNACY_ABI_H
#define LUNACY_ABI_H
#include <stddef.h>
#include <stdint.h>

/* Mirrored by sys::Storage. Native sizes/alignment are checked where used. */
struct lunacy_storage {
    _Alignas(16) unsigned char bytes[128];
};

/* Mirrored by poll::PollFd, checked against the native pollfd in readiness.c. */
struct lunacy_pollfd {
    int fd;
    short events;
    short revents;
};
#endif
