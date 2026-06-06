#include <errno.h>
#include <stdint.h>
#include <stdlib.h>
#include <sys/ioctl.h>
#include <termios.h>
#include <unistd.h>

struct lunacy_raw_mode {
    int fd;
    int active;
    struct termios original;
};

int lunacy_tty_isatty(int fd) {
    errno = 0;
    if (isatty(fd)) {
        return 1;
    }

    if (errno == 0) {
        return 0;
    }

#ifdef ENOTTY
    if (errno == ENOTTY) {
        return 0;
    }
#endif

#ifdef EINVAL
    if (errno == EINVAL) {
        return 0;
    }
#endif

    return -1;
}

void *lunacy_raw_mode_enable(int fd) {
    struct lunacy_raw_mode *state =
        (struct lunacy_raw_mode *)malloc(sizeof(*state));
    if (state == NULL) {
        errno = ENOMEM;
        return NULL;
    }

    state->fd = fd;
    state->active = 0;
    if (tcgetattr(fd, &state->original) < 0) {
        int saved_errno = errno;
        free(state);
        errno = saved_errno;
        return NULL;
    }

    struct termios raw = state->original;

#ifdef BRKINT
    raw.c_iflag &= ~(tcflag_t)BRKINT;
#endif
#ifdef ICRNL
    raw.c_iflag &= ~(tcflag_t)ICRNL;
#endif
#ifdef INPCK
    raw.c_iflag &= ~(tcflag_t)INPCK;
#endif
#ifdef ISTRIP
    raw.c_iflag &= ~(tcflag_t)ISTRIP;
#endif
#ifdef IXON
    raw.c_iflag &= ~(tcflag_t)IXON;
#endif

#ifdef OPOST
    raw.c_oflag &= ~(tcflag_t)OPOST;
#endif

#ifdef CS8
#ifdef CSIZE
    raw.c_cflag &= ~(tcflag_t)CSIZE;
#endif
    raw.c_cflag |= (tcflag_t)CS8;
#endif

#ifdef ECHO
    raw.c_lflag &= ~(tcflag_t)ECHO;
#endif
#ifdef ICANON
    raw.c_lflag &= ~(tcflag_t)ICANON;
#endif
#ifdef IEXTEN
    raw.c_lflag &= ~(tcflag_t)IEXTEN;
#endif
#ifdef ISIG
    raw.c_lflag &= ~(tcflag_t)ISIG;
#endif

    raw.c_cc[VMIN] = 1;
    raw.c_cc[VTIME] = 0;

    if (tcsetattr(fd, TCSAFLUSH, &raw) < 0) {
        int saved_errno = errno;
        free(state);
        errno = saved_errno;
        return NULL;
    }

    state->active = 1;
    return state;
}

int lunacy_raw_mode_restore(void *state_ptr) {
    struct lunacy_raw_mode *state = (struct lunacy_raw_mode *)state_ptr;
    if (state == NULL) {
        errno = EFAULT;
        return -1;
    }

    if (!state->active) {
        return 0;
    }

    if (tcsetattr(state->fd, TCSAFLUSH, &state->original) < 0) {
        return -1;
    }

    state->active = 0;
    return 0;
}

void lunacy_raw_mode_free(void *state_ptr) { free(state_ptr); }

int lunacy_tty_window_size(int fd, uint16_t *rows, uint16_t *columns) {
    if (rows == NULL || columns == NULL) {
        errno = EFAULT;
        return -1;
    }

    struct winsize size;
    if (ioctl(fd, TIOCGWINSZ, &size) < 0) {
        return -1;
    }

    *rows = (uint16_t)size.ws_row;
    *columns = (uint16_t)size.ws_col;
    return 0;
}
