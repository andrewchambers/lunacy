#define _POSIX_C_SOURCE 200809L
#include "abi.h"
#include <errno.h>
#include <arpa/inet.h>
#include <netinet/in.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

LUNACY_STATIC_ASSERT(sizeof(struct sockaddr_storage) <= sizeof(struct lunacy_storage),
               "lunacy socket address storage is too small for this target");
LUNACY_STATIC_ASSERT(LUNACY_ALIGNOF(struct sockaddr_storage) <= LUNACY_ALIGNOF(struct lunacy_storage),
               "lunacy socket address alignment is too small for this target");
LUNACY_STATIC_ASSERT(sizeof(struct sockaddr_un) <= sizeof(struct sockaddr_storage),
               "sockaddr_un must fit sockaddr_storage");
LUNACY_STATIC_ASSERT(sizeof(ssize_t) <= sizeof(intptr_t), "socket byte count must fit intptr_t");

#define CONSTANT(name, value) int lunacy_##name(void) { return value; }
CONSTANT(af_inet, AF_INET)
CONSTANT(af_inet6, AF_INET6)
CONSTANT(af_unix, AF_UNIX)
CONSTANT(sock_stream, SOCK_STREAM)
CONSTANT(sock_dgram, SOCK_DGRAM)
CONSTANT(sock_seqpacket, SOCK_SEQPACKET)
CONSTANT(shut_rd, SHUT_RD)
CONSTANT(shut_wr, SHUT_WR)
CONSTANT(shut_rdwr, SHUT_RDWR)
CONSTANT(msg_peek, MSG_PEEK)
CONSTANT(msg_oob, MSG_OOB)
CONSTANT(msg_dontroute, MSG_DONTROUTE)

/* BSD socket addresses have an additional length member. */
#if defined(__APPLE__) || defined(__FreeBSD__) || defined(__NetBSD__) || defined(__OpenBSD__) || defined(__DragonFly__)
#define SET_LENGTH(addr, field, size) ((addr).field = (size))
#else
#define SET_LENGTH(addr, field, size) ((void)0)
#endif

size_t lunacy_addr_v4(struct lunacy_storage *out, const unsigned char *ip, uint16_t port) {
    struct sockaddr_in addr = {0};
    addr.sin_family = AF_INET;
    SET_LENGTH(addr, sin_len, sizeof(addr));
    addr.sin_port = htons(port);
    memcpy(&addr.sin_addr, ip, 4);
    memset(out, 0, sizeof(*out));
    memcpy(out, &addr, sizeof(addr));
    return sizeof(addr);
}

size_t lunacy_addr_v6(struct lunacy_storage *out, const unsigned char *ip,
                     uint16_t port, uint32_t flowinfo, uint32_t scope_id) {
    struct sockaddr_in6 addr = {0};
    addr.sin6_family = AF_INET6;
    SET_LENGTH(addr, sin6_len, sizeof(addr));
    addr.sin6_port = htons(port);
    addr.sin6_flowinfo = htonl(flowinfo);
    addr.sin6_scope_id = scope_id;
    memcpy(&addr.sin6_addr, ip, 16);
    memset(out, 0, sizeof(*out));
    memcpy(out, &addr, sizeof(addr));
    return sizeof(addr);
}

int lunacy_addr_unix(struct lunacy_storage *out, const char *path, size_t len, size_t *size) {
    struct sockaddr_un addr = {0};
    if (len == 0 || len >= sizeof(addr.sun_path)) {
        errno = EINVAL;
        return -1;
    }
    addr.sun_family = AF_UNIX;
    memcpy(addr.sun_path, path, len);
    *size = offsetof(struct sockaddr_un, sun_path) + len + 1;
    SET_LENGTH(addr, sun_len, *size);
    memset(out, 0, sizeof(*out));
    memcpy(out, &addr, sizeof(addr));
    return 0;
}

int lunacy_addr_family(const struct lunacy_storage *storage) {
    struct sockaddr_storage addr;
    memcpy(&addr, storage, sizeof(addr));
    return addr.ss_family;
}

int lunacy_addr_get_v4(const struct lunacy_storage *storage, size_t len,
                      unsigned char *ip, uint16_t *port) {
    struct sockaddr_in addr;
    if (len < sizeof(addr) || lunacy_addr_family(storage) != AF_INET) return 0;
    memcpy(&addr, storage, sizeof(addr));
    memcpy(ip, &addr.sin_addr, 4);
    *port = ntohs(addr.sin_port);
    return 1;
}

int lunacy_addr_get_v6(const struct lunacy_storage *storage, size_t len,
                      unsigned char *ip, uint16_t *port, uint32_t *flowinfo, uint32_t *scope_id) {
    struct sockaddr_in6 addr;
    if (len < sizeof(addr) || lunacy_addr_family(storage) != AF_INET6) return 0;
    memcpy(&addr, storage, sizeof(addr));
    memcpy(ip, &addr.sin6_addr, 16);
    *port = ntohs(addr.sin6_port);
    *flowinfo = ntohl(addr.sin6_flowinfo);
    *scope_id = addr.sin6_scope_id;
    return 1;
}

int lunacy_socket(int domain, int type, int protocol) { return socket(domain, type, protocol); }
int lunacy_socketpair(int domain, int type, int protocol, int *fds) {
    return socketpair(domain, type, protocol, fds);
}

static int load_addr(struct sockaddr_storage *out, const struct lunacy_storage *in, size_t len) {
    if (len > sizeof(*out) || (size_t)(socklen_t)len != len) {
        errno = EINVAL;
        return -1;
    }
    memcpy(out, in, sizeof(*out));
    return 0;
}

int lunacy_bind(int fd, const struct lunacy_storage *storage, size_t len) {
    struct sockaddr_storage addr;
    if (load_addr(&addr, storage, len) == -1) return -1;
    return bind(fd, (const struct sockaddr *)&addr, (socklen_t)len);
}

int lunacy_connect(int fd, const struct lunacy_storage *storage, size_t len) {
    struct sockaddr_storage addr;
    if (load_addr(&addr, storage, len) == -1) return -1;
    return connect(fd, (const struct sockaddr *)&addr, (socklen_t)len);
}

int lunacy_listen(int fd, int backlog) { return listen(fd, backlog); }

int lunacy_accept(int fd, struct lunacy_storage *out, size_t *len) {
    if (out == NULL) return accept(fd, NULL, NULL);
    struct sockaddr_storage addr = {0};
    socklen_t size = sizeof(addr);
    int result = accept(fd, (struct sockaddr *)&addr, &size);
    if (result != -1) {
        memset(out, 0, sizeof(*out));
        memcpy(out, &addr, sizeof(addr));
        *len = size;
    }
    return result;
}

int lunacy_getsockname(int fd, struct lunacy_storage *out, size_t *len) {
    struct sockaddr_storage addr = {0};
    socklen_t size = sizeof(addr);
    int result = getsockname(fd, (struct sockaddr *)&addr, &size);
    if (result != -1) {
        memset(out, 0, sizeof(*out));
        memcpy(out, &addr, sizeof(addr));
        *len = size;
    }
    return result;
}

int lunacy_getpeername(int fd, struct lunacy_storage *out, size_t *len) {
    struct sockaddr_storage addr = {0};
    socklen_t size = sizeof(addr);
    int result = getpeername(fd, (struct sockaddr *)&addr, &size);
    if (result != -1) {
        memset(out, 0, sizeof(*out));
        memcpy(out, &addr, sizeof(addr));
        *len = size;
    }
    return result;
}

int lunacy_shutdown(int fd, int how) { return shutdown(fd, how); }
intptr_t lunacy_send(int fd, const unsigned char *buf, size_t len, int flags) {
    return (intptr_t)send(fd, buf, len, flags);
}
intptr_t lunacy_recv(int fd, unsigned char *buf, size_t len, int flags) {
    return (intptr_t)recv(fd, buf, len, flags);
}
