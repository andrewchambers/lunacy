#define _POSIX_C_SOURCE 200809L
#include "compat.h"
#include <errno.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/* Read constants through the target's headers; Rust never assumes their values. */
#define LUNACY_ERRNO(name, symbol) int symbol(void) { return name; }
LUNACY_ERRNO(EACCES, lunacy_eacces)
LUNACY_ERRNO(EAGAIN, lunacy_eagain)
LUNACY_ERRNO(EBADF, lunacy_ebadf)
LUNACY_ERRNO(EEXIST, lunacy_eexist)
LUNACY_ERRNO(EFBIG, lunacy_efbig)
LUNACY_ERRNO(EINTR, lunacy_eintr)
LUNACY_ERRNO(EINVAL, lunacy_einval)
LUNACY_ERRNO(EIO, lunacy_eio)
LUNACY_ERRNO(EMFILE, lunacy_emfile)
LUNACY_ERRNO(ENFILE, lunacy_enfile)
LUNACY_ERRNO(ENOENT, lunacy_enoent)
LUNACY_ERRNO(ENOMEM, lunacy_enomem)
LUNACY_ERRNO(ENOSPC, lunacy_enospc)
LUNACY_ERRNO(EPIPE, lunacy_epipe)
LUNACY_ERRNO(EINPROGRESS, lunacy_einprogress)
LUNACY_ERRNO(EALREADY, lunacy_ealready)
LUNACY_ERRNO(EADDRINUSE, lunacy_eaddrinuse)
LUNACY_ERRNO(EADDRNOTAVAIL, lunacy_eaddrnotavail)
LUNACY_ERRNO(EAFNOSUPPORT, lunacy_eafnosupport)
LUNACY_ERRNO(ECONNREFUSED, lunacy_econnrefused)
LUNACY_ERRNO(ECONNRESET, lunacy_econnreset)
LUNACY_ERRNO(ENOTCONN, lunacy_enotconn)
LUNACY_ERRNO(ENOTSOCK, lunacy_enotsock)
LUNACY_ERRNO(EPROTONOSUPPORT, lunacy_eprotonosupport)
LUNACY_ERRNO(EOPNOTSUPP, lunacy_eopnotsupp)
LUNACY_ERRNO(ETIMEDOUT, lunacy_etimedout)
LUNACY_ERRNO(EMSGSIZE, lunacy_emsgsize)
LUNACY_ERRNO(EOVERFLOW, lunacy_eoverflow)
LUNACY_ERRNO(EBUSY, lunacy_ebusy)
LUNACY_ERRNO(EDEADLK, lunacy_edeadlk)
LUNACY_ERRNO(EPERM, lunacy_eperm)
LUNACY_ERRNO(ERANGE, lunacy_erange)
LUNACY_ERRNO(ENOTDIR, lunacy_enotdir)
LUNACY_ERRNO(EISDIR, lunacy_eisdir)
LUNACY_ERRNO(ENOTEMPTY, lunacy_enotempty)
LUNACY_ERRNO(ENOTTY, lunacy_enotty)
LUNACY_ERRNO(ESPIPE, lunacy_espipe)
LUNACY_ERRNO(ELOOP, lunacy_eloop)
LUNACY_ERRNO(ENAMETOOLONG, lunacy_enametoolong)
LUNACY_ERRNO(EROFS, lunacy_erofs)
LUNACY_ERRNO(EXDEV, lunacy_exdev)

int lunacy_errno(void) { return errno; }

intptr_t lunacy_write(int fd, const unsigned char *buffer, size_t len) {
    LUNACY_STATIC_ASSERT(sizeof(ssize_t) <= sizeof(intptr_t), "write result must fit intptr_t");
    return (intptr_t)write(fd, buffer, len);
}

intptr_t lunacy_print(const unsigned char *buffer, size_t len, int to_stderr) {
    return lunacy_write(to_stderr ? STDERR_FILENO : STDOUT_FILENO, buffer, len);
}

int lunacy_dup(int fd) { return dup(fd); }
int lunacy_dup2(int source, int target) { return dup2(source, target); }
int lunacy_pipe(int *fds) { return pipe(fds); }
int lunacy_close(int fd) { return close(fd); }

intptr_t lunacy_read(int fd, unsigned char *buffer, size_t len) {
    LUNACY_STATIC_ASSERT(sizeof(ssize_t) <= sizeof(intptr_t), "read result must fit intptr_t");
    return (intptr_t)read(fd, buffer, len);
}

const char *lunacy_getenv(const char *name) { return getenv(name); }

/* POSIX environ storage is read-only during lookups. Unlike getenv's result,
   reading it does not involve implementation-specific shared return buffers.
   Callers of environment mutation must exclude all simultaneous readers. */
extern char **environ;

size_t lunacy_getenv_copy(const char *name, unsigned char *buffer, size_t capacity) {
    size_t name_len = strlen(name);
    if (name_len == 0 || strchr(name, '=') != NULL || environ == NULL) return 0;
    for (char **entry = environ; *entry != NULL; ++entry) {
        if (strncmp(*entry, name, name_len) == 0 && (*entry)[name_len] == '=') {
            const char *value = *entry + name_len + 1;
            size_t needed = strlen(value) + 1;
            if (needed <= capacity) memcpy(buffer, value, needed);
            return needed;
        }
    }
    return 0;
}

int lunacy_setenv(const char *name, const char *value, int overwrite) {
    return setenv(name, value, overwrite);
}
int lunacy_unsetenv(const char *name) { return unsetenv(name); }

void *lunacy_alloc(size_t size, size_t align) {
    if (align <= LUNACY_MALLOC_ALIGNMENT) {
        return malloc(size);
    }
    void *ptr = NULL;
    if (posix_memalign(&ptr, align, size) != 0) {
        return NULL;
    }
    return ptr;
}

void *lunacy_alloc_zeroed(size_t size, size_t align) {
    if (align <= LUNACY_MALLOC_ALIGNMENT) {
        return calloc(1, size);
    }
    void *ptr = lunacy_alloc(size, align);
    if (ptr != NULL) {
        memset(ptr, 0, size);
    }
    return ptr;
}

void *lunacy_realloc(void *ptr, size_t old_size, size_t align, size_t new_size) {
    if (align <= LUNACY_MALLOC_ALIGNMENT) {
        return realloc(ptr, new_size);
    }
    void *next = lunacy_alloc(new_size, align);
    if (next != NULL) {
        memcpy(next, ptr, old_size < new_size ? old_size : new_size);
        free(ptr);
    }
    return next;
}
