#include <errno.h>
#include <netinet/in.h>
#include <stdint.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>

#ifndef ENOSYS
#define ENOSYS EINVAL
#endif

#ifndef EAFNOSUPPORT
#define EAFNOSUPPORT EINVAL
#endif

#define LUNACY_NET_AF_INET 1
#define LUNACY_NET_AF_INET6 2
#define LUNACY_NET_AF_UNIX 3
#define LUNACY_NET_SOCK_STREAM 4
#define LUNACY_NET_SOCK_DGRAM 5
#define LUNACY_NET_SOCK_CLOEXEC 6
#define LUNACY_NET_SOCK_NONBLOCK 7
#define LUNACY_NET_IPPROTO_TCP 8
#define LUNACY_NET_IPPROTO_UDP 9
#define LUNACY_NET_SHUT_RD 10
#define LUNACY_NET_SHUT_WR 11
#define LUNACY_NET_SHUT_RDWR 12

#define LUNACY_SOCKET_ADDR_V4 1
#define LUNACY_SOCKET_ADDR_V6 2

#define LUNACY_MISSING_CONSTANT()                                             \
    do {                                                                      \
        errno = ENOSYS;                                                       \
        return -1;                                                            \
    } while (0)

struct lunacy_socket_addr {
    int family;
    unsigned char ip[16];
    uint16_t port;
    uint32_t flowinfo;
    uint32_t scope_id;
};

int lunacy_net_constant(int name, int *out) {
    if (out == NULL) {
        errno = EFAULT;
        return -1;
    }

    switch (name) {
    case LUNACY_NET_AF_INET:
#ifdef AF_INET
        *out = AF_INET;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    case LUNACY_NET_AF_INET6:
#ifdef AF_INET6
        *out = AF_INET6;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    case LUNACY_NET_AF_UNIX:
#ifdef AF_UNIX
        *out = AF_UNIX;
        return 0;
#elif defined(AF_LOCAL)
        *out = AF_LOCAL;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    case LUNACY_NET_SOCK_STREAM:
#ifdef SOCK_STREAM
        *out = SOCK_STREAM;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    case LUNACY_NET_SOCK_DGRAM:
#ifdef SOCK_DGRAM
        *out = SOCK_DGRAM;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    case LUNACY_NET_SOCK_CLOEXEC:
#ifdef SOCK_CLOEXEC
        *out = SOCK_CLOEXEC;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    case LUNACY_NET_SOCK_NONBLOCK:
#ifdef SOCK_NONBLOCK
        *out = SOCK_NONBLOCK;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    case LUNACY_NET_IPPROTO_TCP:
#ifdef IPPROTO_TCP
        *out = IPPROTO_TCP;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    case LUNACY_NET_IPPROTO_UDP:
#ifdef IPPROTO_UDP
        *out = IPPROTO_UDP;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    case LUNACY_NET_SHUT_RD:
#ifdef SHUT_RD
        *out = SHUT_RD;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    case LUNACY_NET_SHUT_WR:
#ifdef SHUT_WR
        *out = SHUT_WR;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    case LUNACY_NET_SHUT_RDWR:
#ifdef SHUT_RDWR
        *out = SHUT_RDWR;
        return 0;
#else
        LUNACY_MISSING_CONSTANT();
#endif
    default:
        errno = EINVAL;
        return -1;
    }
}

static int lunacy_write_sockaddr(const struct lunacy_socket_addr *in,
                                 struct sockaddr_storage *storage,
                                 socklen_t *len) {
    if (in == NULL || storage == NULL || len == NULL) {
        errno = EFAULT;
        return -1;
    }

    memset(storage, 0, sizeof(*storage));

    switch (in->family) {
    case LUNACY_SOCKET_ADDR_V4: {
#ifdef AF_INET
        struct sockaddr_in *addr = (struct sockaddr_in *)storage;
        addr->sin_family = AF_INET;
#if defined(__APPLE__) || defined(__FreeBSD__) || defined(__NetBSD__) ||      \
    defined(__OpenBSD__) || defined(__DragonFly__)
        addr->sin_len = sizeof(*addr);
#endif
        addr->sin_port = htons(in->port);
        memcpy(&addr->sin_addr, in->ip, 4);
        *len = sizeof(*addr);
        return 0;
#else
        errno = EAFNOSUPPORT;
        return -1;
#endif
    }
    case LUNACY_SOCKET_ADDR_V6: {
#ifdef AF_INET6
        struct sockaddr_in6 *addr = (struct sockaddr_in6 *)storage;
        addr->sin6_family = AF_INET6;
#if defined(__APPLE__) || defined(__FreeBSD__) || defined(__NetBSD__) ||      \
    defined(__OpenBSD__) || defined(__DragonFly__)
        addr->sin6_len = sizeof(*addr);
#endif
        addr->sin6_port = htons(in->port);
        addr->sin6_flowinfo = in->flowinfo;
        addr->sin6_scope_id = in->scope_id;
        memcpy(&addr->sin6_addr, in->ip, 16);
        *len = sizeof(*addr);
        return 0;
#else
        errno = EAFNOSUPPORT;
        return -1;
#endif
    }
    default:
        errno = EAFNOSUPPORT;
        return -1;
    }
}

static int lunacy_read_sockaddr(const struct sockaddr *addr, socklen_t len,
                                struct lunacy_socket_addr *out) {
    if (addr == NULL || out == NULL) {
        errno = EFAULT;
        return -1;
    }

    memset(out, 0, sizeof(*out));

    switch (addr->sa_family) {
#ifdef AF_INET
    case AF_INET: {
        if (len < (socklen_t)sizeof(struct sockaddr_in)) {
            errno = EINVAL;
            return -1;
        }

        const struct sockaddr_in *in = (const struct sockaddr_in *)addr;
        out->family = LUNACY_SOCKET_ADDR_V4;
        out->port = ntohs(in->sin_port);
        memcpy(out->ip, &in->sin_addr, 4);
        return 0;
    }
#endif
#ifdef AF_INET6
    case AF_INET6: {
        if (len < (socklen_t)sizeof(struct sockaddr_in6)) {
            errno = EINVAL;
            return -1;
        }

        const struct sockaddr_in6 *in = (const struct sockaddr_in6 *)addr;
        out->family = LUNACY_SOCKET_ADDR_V6;
        out->port = ntohs(in->sin6_port);
        out->flowinfo = in->sin6_flowinfo;
        out->scope_id = in->sin6_scope_id;
        memcpy(out->ip, &in->sin6_addr, 16);
        return 0;
    }
#endif
    default:
        errno = EAFNOSUPPORT;
        return -1;
    }
}

int lunacy_socket_create(int domain, int socket_type, int protocol) {
    return socket(domain, socket_type, protocol);
}

int lunacy_connect_addr(int fd, const struct lunacy_socket_addr *addr) {
    struct sockaddr_storage storage;
    socklen_t len;

    if (lunacy_write_sockaddr(addr, &storage, &len) < 0) {
        return -1;
    }

    return connect(fd, (const struct sockaddr *)&storage, len);
}

int lunacy_bind_addr(int fd, const struct lunacy_socket_addr *addr) {
    struct sockaddr_storage storage;
    socklen_t len;

    if (lunacy_write_sockaddr(addr, &storage, &len) < 0) {
        return -1;
    }

    return bind(fd, (const struct sockaddr *)&storage, len);
}

int lunacy_listen(int fd, int backlog) { return listen(fd, backlog); }

int lunacy_accept_fd(int fd) { return accept(fd, NULL, NULL); }

int lunacy_accept_addr(int fd, struct lunacy_socket_addr *out) {
    struct sockaddr_storage storage;
    socklen_t len = sizeof(storage);
    int accepted = accept(fd, (struct sockaddr *)&storage, &len);

    if (accepted < 0) {
        return -1;
    }

    if (lunacy_read_sockaddr((const struct sockaddr *)&storage, len, out) < 0) {
        int saved_errno = errno;
        close(accepted);
        errno = saved_errno;
        return -1;
    }

    return accepted;
}

int lunacy_shutdown(int fd, int how) { return shutdown(fd, how); }

ssize_t lunacy_send(int fd, const void *buf, size_t len, int flags) {
    return send(fd, buf, len, flags);
}

ssize_t lunacy_recv(int fd, void *buf, size_t len, int flags) {
    return recv(fd, buf, len, flags);
}

ssize_t lunacy_sendto_addr(int fd, const void *buf, size_t len, int flags,
                           const struct lunacy_socket_addr *addr) {
    struct sockaddr_storage storage;
    socklen_t addr_len;

    if (lunacy_write_sockaddr(addr, &storage, &addr_len) < 0) {
        return -1;
    }

    return sendto(fd, buf, len, flags, (const struct sockaddr *)&storage,
                  addr_len);
}

ssize_t lunacy_recvfrom_addr(int fd, void *buf, size_t len, int flags,
                             struct lunacy_socket_addr *out) {
    struct sockaddr_storage storage;
    socklen_t addr_len = sizeof(storage);
    ssize_t received =
        recvfrom(fd, buf, len, flags, (struct sockaddr *)&storage, &addr_len);

    if (received < 0) {
        return -1;
    }

    if (lunacy_read_sockaddr((const struct sockaddr *)&storage, addr_len, out) <
        0) {
        return -1;
    }

    return received;
}
