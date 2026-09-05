#define _POSIX_C_SOURCE 200809L
#include "compat.h"
#include <errno.h>
#include <stdint.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

LUNACY_STATIC_ASSERT(sizeof(pid_t) <= sizeof(int64_t) && (pid_t)-1 < 0,
                     "lunacy requires signed pid_t no wider than i64");

int64_t lunacy_fork(void) { return (int64_t)fork(); }

/* POSIX exec does not modify these arrays or their strings, despite the
   historical non-const character type in its prototypes. */
int lunacy_execv(const char *path, const char *const *argv) {
    return execv(path, (char *const *)argv);
}
int lunacy_execve(const char *path, const char *const *argv, const char *const *envp) {
    return execve(path, (char *const *)argv, (char *const *)envp);
}
int lunacy_execvp(const char *file, const char *const *argv) {
    return execvp(file, (char *const *)argv);
}
void lunacy_exit_immediately(int status) { _exit(status); }

int64_t lunacy_waitpid(int64_t raw, int *out, int options) {
    pid_t pid = (pid_t)raw;
    if ((int64_t)pid != raw) { errno = EOVERFLOW; return -1; }
    int status;
    pid_t result = waitpid(pid, out ? &status : NULL, options);
    if (result > 0 && out) *out = status;
    return (int64_t)result;
}

int lunacy_wnohang(void) { return WNOHANG; }
int lunacy_wuntraced(void) { return WUNTRACED; }
int lunacy_wcontinued(void) { return WCONTINUED; }
int lunacy_wifexited(int status) { return WIFEXITED(status); }
int lunacy_wexitstatus(int status) { return WEXITSTATUS(status); }
int lunacy_wifsignaled(int status) { return WIFSIGNALED(status); }
int lunacy_wtermsig(int status) { return WTERMSIG(status); }
int lunacy_wifstopped(int status) { return WIFSTOPPED(status); }
int lunacy_wstopsig(int status) { return WSTOPSIG(status); }
int lunacy_wifcontinued(int status) { return WIFCONTINUED(status); }
