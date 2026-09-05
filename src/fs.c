/* Request the POSIX strerror_r interface, including on glibc. */
#undef _GNU_SOURCE
#define _POSIX_C_SOURCE 200809L
#define _XOPEN_SOURCE 700
#include "compat.h"
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#define FLAG(name, suffix) int lunacy_o_##suffix(void) { return name; }
FLAG(O_RDONLY, rdonly)
FLAG(O_WRONLY, wronly)
FLAG(O_RDWR, rdwr)
FLAG(O_CREAT, creat)
FLAG(O_EXCL, excl)
FLAG(O_TRUNC, trunc)
FLAG(O_APPEND, append)
FLAG(O_CLOEXEC, cloexec)
FLAG(O_DIRECTORY, directory)
FLAG(O_NOFOLLOW, nofollow)
FLAG(O_NONBLOCK, nonblock)
FLAG(O_SYNC, sync)
#define MODE(name, suffix) uint32_t lunacy_s_##suffix(void) { return name; }
MODE(S_IRUSR, irusr)
MODE(S_IWUSR, iwusr)
MODE(S_IXUSR, ixusr)
MODE(S_IRGRP, irgrp)
MODE(S_IWGRP, iwgrp)
MODE(S_IXGRP, ixgrp)
MODE(S_IROTH, iroth)
MODE(S_IWOTH, iwoth)
MODE(S_IXOTH, ixoth)
MODE(S_ISUID, isuid)
MODE(S_ISGID, isgid)
MODE(S_ISVTX, isvtx)

LUNACY_STATIC_ASSERT(sizeof(mode_t) <= sizeof(uint32_t), "mode_t must fit u32");
LUNACY_STATIC_ASSERT(sizeof(off_t) <= sizeof(int64_t) && (off_t)-1 < 0,
               "lunacy requires signed off_t no wider than i64");
LUNACY_STATIC_ASSERT(sizeof(time_t) <= sizeof(int64_t) && (time_t)-1 < 0,
               "lunacy requires signed time_t no wider than i64");
LUNACY_STATIC_ASSERT(sizeof(long) <= sizeof(int64_t), "nanoseconds must fit i64");
#define FITS_U64(type) LUNACY_STATIC_ASSERT(sizeof(type) <= sizeof(uint64_t), #type " must fit u64")
FITS_U64(dev_t);
FITS_U64(ino_t);
FITS_U64(nlink_t);
FITS_U64(uid_t);
FITS_U64(gid_t);

static int native_mode(uint32_t raw, mode_t *mode) {
    *mode = (mode_t)raw;
    if ((uint32_t)*mode != raw) { errno = EOVERFLOW; return -1; }
    return 0;
}

int lunacy_open(const char *path, int flags, uint32_t raw_mode) {
    mode_t mode;
    if (native_mode(raw_mode, &mode) == -1) return -1;
    return open(path, flags, mode);
}
int lunacy_openat(int use_cwd, int fd, const char *path, int flags, uint32_t raw_mode) {
    mode_t mode;
    if (native_mode(raw_mode, &mode) == -1) return -1;
    return openat(use_cwd ? AT_FDCWD : fd, path, flags, mode);
}

/* Fixed shim layout, mirrored by Rust Stat and Timespec, not native struct stat. */
struct lunacy_timespec { int64_t tv_sec, tv_nsec; };
struct lunacy_stat {
    uint64_t st_dev, st_ino, st_nlink, st_uid, st_gid, st_rdev;
    uint32_t st_mode;
    int64_t st_size;
    struct lunacy_timespec st_atim, st_mtim, st_ctim;
};
static void copy_stat(struct lunacy_stat *out, const struct stat *value) {
    out->st_dev = value->st_dev;
    out->st_ino = value->st_ino;
    out->st_nlink = value->st_nlink;
    out->st_uid = value->st_uid;
    out->st_gid = value->st_gid;
    out->st_rdev = value->st_rdev;
    out->st_mode = value->st_mode;
    out->st_size = value->st_size;
#ifdef __APPLE__
    out->st_atim = (struct lunacy_timespec){value->st_atimespec.tv_sec, value->st_atimespec.tv_nsec};
    out->st_mtim = (struct lunacy_timespec){value->st_mtimespec.tv_sec, value->st_mtimespec.tv_nsec};
    out->st_ctim = (struct lunacy_timespec){value->st_ctimespec.tv_sec, value->st_ctimespec.tv_nsec};
#else
    out->st_atim = (struct lunacy_timespec){value->st_atim.tv_sec, value->st_atim.tv_nsec};
    out->st_mtim = (struct lunacy_timespec){value->st_mtim.tv_sec, value->st_mtim.tv_nsec};
    out->st_ctim = (struct lunacy_timespec){value->st_ctim.tv_sec, value->st_ctim.tv_nsec};
#endif
}
int lunacy_stat(const char *path, struct lunacy_stat *out) {
    struct stat value;
    int rc = stat(path, &value);
    if (rc == 0) copy_stat(out, &value);
    return rc;
}
int lunacy_lstat(const char *path, struct lunacy_stat *out) {
    struct stat value;
    int rc = lstat(path, &value);
    if (rc == 0) copy_stat(out, &value);
    return rc;
}
int lunacy_fstat(int fd, struct lunacy_stat *out) {
    struct stat value;
    int rc = fstat(fd, &value);
    if (rc == 0) copy_stat(out, &value);
    return rc;
}
/* These tags belong to the shim, not to the OS. */
int lunacy_file_type(uint32_t raw_mode) {
    mode_t mode = (mode_t)raw_mode;
    if (S_ISREG(mode)) return 1;
    if (S_ISDIR(mode)) return 2;
    if (S_ISLNK(mode)) return 3;
    if (S_ISCHR(mode)) return 4;
    if (S_ISBLK(mode)) return 5;
    if (S_ISFIFO(mode)) return 6;
    if (S_ISSOCK(mode)) return 7;
    return 0;
}
int lunacy_seek_set(void) { return SEEK_SET; }
int lunacy_seek_cur(void) { return SEEK_CUR; }
int lunacy_seek_end(void) { return SEEK_END; }
int64_t lunacy_lseek(int fd, int64_t raw, int whence) {
    off_t offset = (off_t)raw;
    if ((int64_t)offset != raw) { errno = EOVERFLOW; return -1; }
    return (int64_t)lseek(fd, offset, whence);
}
int lunacy_ftruncate(int fd, int64_t raw) {
    off_t length = (off_t)raw;
    if ((int64_t)length != raw) { errno = EOVERFLOW; return -1; }
    return ftruncate(fd, length);
}
int lunacy_unlink(const char *path) { return unlink(path); }
int lunacy_rename(const char *old, const char *new_path) { return rename(old, new_path); }
int lunacy_mkdir(const char *path, uint32_t raw_mode) {
    mode_t mode;
    if (native_mode(raw_mode, &mode) == -1) return -1;
    return mkdir(path, mode);
}
int lunacy_rmdir(const char *path) { return rmdir(path); }
int lunacy_getcwd(char *buffer, size_t capacity) {
    if (capacity == 0) { errno = ERANGE; return -1; }
    return getcwd(buffer, capacity) ? 0 : -1;
}
int lunacy_chdir(const char *path) { return chdir(path); }
int lunacy_isatty(int fd) {
    if (isatty(fd)) return 1;
    if (errno == ENOTTY) return 0;
    return -1;
}
int lunacy_mkstemp(char *template) { return mkstemp(template); }
int lunacy_strerror_r(int error, char *buffer, size_t capacity) {
    if (capacity == 0) return ERANGE;
    int rc = strerror_r(error, buffer, capacity);
    return rc == -1 ? errno : rc;
}

void *lunacy_opendir(const char *path) { return opendir(path); }
int lunacy_readdir(void *directory, const char **name, uint64_t *ino) {
    errno = 0;
    struct dirent *entry = readdir(directory);
    if (entry == NULL) return errno ? -1 : 0;
    *name = entry->d_name;
    *ino = entry->d_ino;
    return 1;
}
int lunacy_closedir(void *directory) { return closedir(directory); }
