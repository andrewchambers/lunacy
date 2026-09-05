#define _POSIX_C_SOURCE 200809L
#include "abi.h"
#include <errno.h>
#include <poll.h>
#include <string.h>
#include <sys/select.h>

LUNACY_STATIC_ASSERT(sizeof(fd_set) <= sizeof(struct lunacy_storage),
               "lunacy fd_set storage is too small for this target");
LUNACY_STATIC_ASSERT(LUNACY_ALIGNOF(fd_set) <= LUNACY_ALIGNOF(struct lunacy_storage),
               "lunacy fd_set alignment is too small for this target");
LUNACY_STATIC_ASSERT(sizeof(struct pollfd) == sizeof(struct lunacy_pollfd), "pollfd size mismatch");
LUNACY_STATIC_ASSERT(LUNACY_ALIGNOF(struct pollfd) == LUNACY_ALIGNOF(struct lunacy_pollfd), "pollfd alignment mismatch");
LUNACY_STATIC_ASSERT(offsetof(struct pollfd, fd) == offsetof(struct lunacy_pollfd, fd), "pollfd.fd mismatch");
LUNACY_STATIC_ASSERT(offsetof(struct pollfd, events) == offsetof(struct lunacy_pollfd, events), "pollfd.events mismatch");
LUNACY_STATIC_ASSERT(offsetof(struct pollfd, revents) == offsetof(struct lunacy_pollfd, revents), "pollfd.revents mismatch");

#define EVENT(name, value) short lunacy_##name(void) { return value; }
EVENT(pollin, POLLIN)
EVENT(pollout, POLLOUT)
EVENT(pollpri, POLLPRI)
EVENT(pollerr, POLLERR)
EVENT(pollhup, POLLHUP)
EVENT(pollnval, POLLNVAL)

int lunacy_poll(void *fds, size_t count, int timeout) {
    if ((size_t)(nfds_t)count != count) { errno = EINVAL; return -1; }
    return poll((struct pollfd *)fds, (nfds_t)count, timeout);
}

int lunacy_fd_setsize(void) { return FD_SETSIZE; }

void lunacy_fd_zero(struct lunacy_storage *storage) {
    fd_set set;
    memset(&set, 0, sizeof(set));
    FD_ZERO(&set);
    memset(storage, 0, sizeof(*storage));
    memcpy(storage, &set, sizeof(set));
}

int lunacy_fd_set(struct lunacy_storage *storage, int fd) {
    if (fd < 0 || fd >= FD_SETSIZE) { errno = EINVAL; return -1; }
    fd_set set;
    memcpy(&set, storage, sizeof(set));
    FD_SET(fd, &set);
    memcpy(storage, &set, sizeof(set));
    return 0;
}

int lunacy_fd_clr(struct lunacy_storage *storage, int fd) {
    if (fd < 0 || fd >= FD_SETSIZE) { errno = EINVAL; return -1; }
    fd_set set;
    memcpy(&set, storage, sizeof(set));
    FD_CLR(fd, &set);
    memcpy(storage, &set, sizeof(set));
    return 0;
}

int lunacy_fd_isset(const struct lunacy_storage *storage, int fd) {
    if (fd < 0 || fd >= FD_SETSIZE) return 0;
    fd_set set;
    memcpy(&set, storage, sizeof(set));
    return FD_ISSET(fd, &set) != 0;
}

int lunacy_select(int nfds, struct lunacy_storage *readfds,
                  struct lunacy_storage *writefds, struct lunacy_storage *exceptfds,
                  int64_t *seconds, int64_t *microseconds) {
    if (nfds < 0 || nfds > FD_SETSIZE) { errno = EINVAL; return -1; }
    fd_set readset, writeset, exceptset;
    if (readfds) memcpy(&readset, readfds, sizeof(readset));
    if (writefds) memcpy(&writeset, writefds, sizeof(writeset));
    if (exceptfds) memcpy(&exceptset, exceptfds, sizeof(exceptset));
    struct timeval timeout;
    if (seconds) {
        if (*seconds < 0 || *microseconds < 0 || *microseconds >= 1000000) {
            errno = EINVAL;
            return -1;
        }
        timeout.tv_sec = (time_t)*seconds;
        timeout.tv_usec = (suseconds_t)*microseconds;
        if ((int64_t)timeout.tv_sec != *seconds || (int64_t)timeout.tv_usec != *microseconds) {
            errno = EOVERFLOW;
            return -1;
        }
    }
    int result = select(nfds, readfds ? &readset : NULL, writefds ? &writeset : NULL,
                        exceptfds ? &exceptset : NULL, seconds ? &timeout : NULL);
    if (result != -1) {
        if (readfds) memcpy(readfds, &readset, sizeof(readset));
        if (writefds) memcpy(writefds, &writeset, sizeof(writeset));
        if (exceptfds) memcpy(exceptfds, &exceptset, sizeof(exceptset));
    }
    /* Timeout is unspecified on failure; preserve the caller's value then. */
    if (seconds && result != -1) {
        *seconds = timeout.tv_sec;
        *microseconds = timeout.tv_usec;
    }
    return result;
}
