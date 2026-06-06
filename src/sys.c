#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <poll.h>
#include <stdint.h>
#include <stdlib.h>
#include <stddef.h>
#include <sys/select.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <unistd.h>

#ifndef ENOSYS
#define ENOSYS EINVAL
#endif

#define LUNACY_POLLIN 1
#define LUNACY_POLLOUT 2
#define LUNACY_POLLPRI 3
#define LUNACY_POLLERR 4
#define LUNACY_POLLHUP 5
#define LUNACY_POLLNVAL 6
#define LUNACY_POLLRDHUP 7

#define LUNACY_MISSING_CONSTANT()                                             \
    do {                                                                      \
        errno = ENOSYS;                                                       \
        return -1;                                                            \
    } while (0)

struct lunacy_stat {
    int64_t dev;
    uint64_t ino;
    uint32_t mode;
    uint64_t nlink;
    uint32_t uid;
    uint32_t gid;
    int64_t rdev;
    int64_t size;
    int64_t blksize;
    int64_t blocks;
    int64_t atime;
    int64_t mtime;
    int64_t ctime;
};

struct lunacy_poll_fd {
    int fd;
    int events;
    int revents;
};

struct lunacy_select_fd {
    int fd;
    int ready;
};

static void lunacy_copy_stat(struct lunacy_stat *out, const struct stat *in) {
    out->dev = (int64_t)in->st_dev;
    out->ino = (uint64_t)in->st_ino;
    out->mode = (uint32_t)in->st_mode;
    out->nlink = (uint64_t)in->st_nlink;
    out->uid = (uint32_t)in->st_uid;
    out->gid = (uint32_t)in->st_gid;
    out->rdev = (int64_t)in->st_rdev;
    out->size = (int64_t)in->st_size;
    out->blksize = (int64_t)in->st_blksize;
    out->blocks = (int64_t)in->st_blocks;
    out->atime = (int64_t)in->st_atime;
    out->mtime = (int64_t)in->st_mtime;
    out->ctime = (int64_t)in->st_ctime;
}

int lunacy_fstat(int fd, struct lunacy_stat *out) {
    struct stat st;
    if (fstat(fd, &st) < 0) {
        return -1;
    }
    lunacy_copy_stat(out, &st);
    return 0;
}

int lunacy_stat(const char *path, struct lunacy_stat *out) {
    struct stat st;
    if (stat(path, &st) < 0) {
        return -1;
    }
    lunacy_copy_stat(out, &st);
    return 0;
}

static int lunacy_lseek_impl(int fd, int64_t offset, int whence, int64_t *out) {
    off_t pos = lseek(fd, (off_t)offset, whence);
    if (pos == (off_t)-1) {
        return -1;
    }
    *out = (int64_t)pos;
    return 0;
}

int lunacy_lseek_set(int fd, int64_t offset, int64_t *out) {
    return lunacy_lseek_impl(fd, offset, SEEK_SET, out);
}

int lunacy_lseek_current(int fd, int64_t offset, int64_t *out) {
    return lunacy_lseek_impl(fd, offset, SEEK_CUR, out);
}

int lunacy_lseek_end(int fd, int64_t offset, int64_t *out) {
    return lunacy_lseek_impl(fd, offset, SEEK_END, out);
}

int lunacy_get_cloexec(int fd) {
    int flags = fcntl(fd, F_GETFD);
    if (flags < 0) {
        return -1;
    }
    return !!(flags & FD_CLOEXEC);
}

int lunacy_set_cloexec(int fd, int enabled) {
    int flags = fcntl(fd, F_GETFD);
    if (flags < 0) {
        return -1;
    }
    if (enabled) {
        flags |= FD_CLOEXEC;
    } else {
        flags &= ~FD_CLOEXEC;
    }
    return fcntl(fd, F_SETFD, flags);
}

int lunacy_get_nonblocking(int fd) {
    int flags = fcntl(fd, F_GETFL);
    if (flags < 0) {
        return -1;
    }
    return !!(flags & O_NONBLOCK);
}

int lunacy_set_nonblocking(int fd, int enabled) {
    int flags = fcntl(fd, F_GETFL);
    if (flags < 0) {
        return -1;
    }
    if (enabled) {
        flags |= O_NONBLOCK;
    } else {
        flags &= ~O_NONBLOCK;
    }
    return fcntl(fd, F_SETFL, flags);
}

int lunacy_poll_constant(int name, int *out) {
    if (out == NULL) {
        errno = EFAULT;
        return -1;
    }

    switch (name) {
    case LUNACY_POLLIN:
#ifdef POLLIN
        *out = POLLIN;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    case LUNACY_POLLOUT:
#ifdef POLLOUT
        *out = POLLOUT;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    case LUNACY_POLLPRI:
#ifdef POLLPRI
        *out = POLLPRI;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    case LUNACY_POLLERR:
#ifdef POLLERR
        *out = POLLERR;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    case LUNACY_POLLHUP:
#ifdef POLLHUP
        *out = POLLHUP;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    case LUNACY_POLLNVAL:
#ifdef POLLNVAL
        *out = POLLNVAL;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    case LUNACY_POLLRDHUP:
#ifdef POLLRDHUP
        *out = POLLRDHUP;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    default:
        errno = EINVAL;
        return -1;
    }
}

int lunacy_poll_fds(struct lunacy_poll_fd *fds, size_t count,
                    int64_t timeout_ms) {
    if (timeout_ms < -1 || timeout_ms > INT_MAX) {
        errno = EINVAL;
        return -1;
    }

    if (count == 0) {
        return poll(NULL, 0, (int)timeout_ms);
    }

    if (fds == NULL) {
        errno = EFAULT;
        return -1;
    }

    if (count > SIZE_MAX / sizeof(struct pollfd)) {
        errno = ENOMEM;
        return -1;
    }

    struct pollfd *native = (struct pollfd *)malloc(count * sizeof(*native));
    if (native == NULL) {
        errno = ENOMEM;
        return -1;
    }

    for (size_t i = 0; i < count; i++) {
        native[i].fd = fds[i].fd;
        native[i].events = (short)fds[i].events;
        native[i].revents = 0;
        fds[i].revents = 0;
    }

    int ready = poll(native, (nfds_t)count, (int)timeout_ms);
    int saved_errno = errno;

    if (ready >= 0) {
        for (size_t i = 0; i < count; i++) {
            fds[i].revents = native[i].revents;
        }
    }

    free(native);
    errno = saved_errno;
    return ready;
}

static int lunacy_select_init_set(fd_set *set, struct lunacy_select_fd *fds,
                                  size_t count, int *max_fd) {
    FD_ZERO(set);

    if (count == 0) {
        return 0;
    }

    if (fds == NULL) {
        errno = EFAULT;
        return -1;
    }

    for (size_t i = 0; i < count; i++) {
        fds[i].ready = 0;
        if (fds[i].fd < 0 || fds[i].fd >= FD_SETSIZE) {
            errno = EINVAL;
            return -1;
        }

        FD_SET(fds[i].fd, set);
        if (fds[i].fd > *max_fd) {
            *max_fd = fds[i].fd;
        }
    }

    return 0;
}

static void lunacy_select_write_ready(const fd_set *set,
                                      struct lunacy_select_fd *fds,
                                      size_t count) {
    for (size_t i = 0; i < count; i++) {
        fds[i].ready = FD_ISSET(fds[i].fd, set);
    }
}

int lunacy_select_fds(struct lunacy_select_fd *read_fds, size_t read_count,
                      struct lunacy_select_fd *write_fds, size_t write_count,
                      struct lunacy_select_fd *except_fds, size_t except_count,
                      int64_t timeout_us) {
    fd_set read_set;
    fd_set write_set;
    fd_set except_set;
    fd_set *read_ptr = NULL;
    fd_set *write_ptr = NULL;
    fd_set *except_ptr = NULL;
    int max_fd = -1;

    if (read_count > 0) {
        if (lunacy_select_init_set(&read_set, read_fds, read_count, &max_fd) <
            0) {
            return -1;
        }
        read_ptr = &read_set;
    }

    if (write_count > 0) {
        if (lunacy_select_init_set(&write_set, write_fds, write_count,
                                   &max_fd) < 0) {
            return -1;
        }
        write_ptr = &write_set;
    }

    if (except_count > 0) {
        if (lunacy_select_init_set(&except_set, except_fds, except_count,
                                   &max_fd) < 0) {
            return -1;
        }
        except_ptr = &except_set;
    }

    struct timeval timeout;
    struct timeval *timeout_ptr = NULL;
    if (timeout_us >= 0) {
        int64_t seconds = timeout_us / 1000000;
        int64_t micros = timeout_us % 1000000;

        timeout.tv_sec = (time_t)seconds;
        timeout.tv_usec = (suseconds_t)micros;
        if ((int64_t)timeout.tv_sec != seconds ||
            (int64_t)timeout.tv_usec != micros) {
            errno = EINVAL;
            return -1;
        }

        timeout_ptr = &timeout;
    } else if (timeout_us != -1) {
        errno = EINVAL;
        return -1;
    }

    int ready =
        select(max_fd + 1, read_ptr, write_ptr, except_ptr, timeout_ptr);
    if (ready < 0) {
        return -1;
    }

    if (read_ptr != NULL) {
        lunacy_select_write_ready(read_ptr, read_fds, read_count);
    }
    if (write_ptr != NULL) {
        lunacy_select_write_ready(write_ptr, write_fds, write_count);
    }
    if (except_ptr != NULL) {
        lunacy_select_write_ready(except_ptr, except_fds, except_count);
    }

    return ready;
}
