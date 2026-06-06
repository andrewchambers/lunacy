#include <errno.h>
#include <stddef.h>
#include <string.h>

int *lunacy_errno_location(void) {
    return &errno;
}

int lunacy_errno_value(int name, int *out) {
    if (!out) {
        return -1;
    }

    switch (name) {
#ifdef EPERM
    case 1: *out = EPERM; return 0;
#endif
#ifdef ENOENT
    case 2: *out = ENOENT; return 0;
#endif
#ifdef ESRCH
    case 3: *out = ESRCH; return 0;
#endif
#ifdef EINTR
    case 4: *out = EINTR; return 0;
#endif
#ifdef EIO
    case 5: *out = EIO; return 0;
#endif
#ifdef ENXIO
    case 6: *out = ENXIO; return 0;
#endif
#ifdef E2BIG
    case 7: *out = E2BIG; return 0;
#endif
#ifdef ENOEXEC
    case 8: *out = ENOEXEC; return 0;
#endif
#ifdef EBADF
    case 9: *out = EBADF; return 0;
#endif
#ifdef ECHILD
    case 10: *out = ECHILD; return 0;
#endif
#ifdef EAGAIN
    case 11: *out = EAGAIN; return 0;
#endif
#ifdef ENOMEM
    case 12: *out = ENOMEM; return 0;
#endif
#ifdef EACCES
    case 13: *out = EACCES; return 0;
#endif
#ifdef EFAULT
    case 14: *out = EFAULT; return 0;
#endif
#ifdef ENOTBLK
    case 15: *out = ENOTBLK; return 0;
#endif
#ifdef EBUSY
    case 16: *out = EBUSY; return 0;
#endif
#ifdef EEXIST
    case 17: *out = EEXIST; return 0;
#endif
#ifdef EXDEV
    case 18: *out = EXDEV; return 0;
#endif
#ifdef ENODEV
    case 19: *out = ENODEV; return 0;
#endif
#ifdef ENOTDIR
    case 20: *out = ENOTDIR; return 0;
#endif
#ifdef EISDIR
    case 21: *out = EISDIR; return 0;
#endif
#ifdef EINVAL
    case 22: *out = EINVAL; return 0;
#endif
#ifdef ENFILE
    case 23: *out = ENFILE; return 0;
#endif
#ifdef EMFILE
    case 24: *out = EMFILE; return 0;
#endif
#ifdef ENOTTY
    case 25: *out = ENOTTY; return 0;
#endif
#ifdef ETXTBSY
    case 26: *out = ETXTBSY; return 0;
#endif
#ifdef EFBIG
    case 27: *out = EFBIG; return 0;
#endif
#ifdef ENOSPC
    case 28: *out = ENOSPC; return 0;
#endif
#ifdef ESPIPE
    case 29: *out = ESPIPE; return 0;
#endif
#ifdef EROFS
    case 30: *out = EROFS; return 0;
#endif
#ifdef EMLINK
    case 31: *out = EMLINK; return 0;
#endif
#ifdef EPIPE
    case 32: *out = EPIPE; return 0;
#endif
#ifdef EDOM
    case 33: *out = EDOM; return 0;
#endif
#ifdef ERANGE
    case 34: *out = ERANGE; return 0;
#endif
#ifdef EDEADLK
    case 35: *out = EDEADLK; return 0;
#endif
#ifdef ENAMETOOLONG
    case 36: *out = ENAMETOOLONG; return 0;
#endif
#ifdef ENOLCK
    case 37: *out = ENOLCK; return 0;
#endif
#ifdef ENOSYS
    case 38: *out = ENOSYS; return 0;
#endif
#ifdef ENOTEMPTY
    case 39: *out = ENOTEMPTY; return 0;
#endif
#ifdef ELOOP
    case 40: *out = ELOOP; return 0;
#endif
#ifdef EWOULDBLOCK
    case 41: *out = EWOULDBLOCK; return 0;
#endif
#ifdef ENOMSG
    case 42: *out = ENOMSG; return 0;
#endif
#ifdef EIDRM
    case 43: *out = EIDRM; return 0;
#endif
#ifdef ECHRNG
    case 44: *out = ECHRNG; return 0;
#endif
#ifdef EL2NSYNC
    case 45: *out = EL2NSYNC; return 0;
#endif
#ifdef EL3HLT
    case 46: *out = EL3HLT; return 0;
#endif
#ifdef EL3RST
    case 47: *out = EL3RST; return 0;
#endif
#ifdef ELNRNG
    case 48: *out = ELNRNG; return 0;
#endif
#ifdef EUNATCH
    case 49: *out = EUNATCH; return 0;
#endif
#ifdef ENOCSI
    case 50: *out = ENOCSI; return 0;
#endif
#ifdef EL2HLT
    case 51: *out = EL2HLT; return 0;
#endif
#ifdef EBADE
    case 52: *out = EBADE; return 0;
#endif
#ifdef EBADR
    case 53: *out = EBADR; return 0;
#endif
#ifdef EXFULL
    case 54: *out = EXFULL; return 0;
#endif
#ifdef ENOANO
    case 55: *out = ENOANO; return 0;
#endif
#ifdef EBADRQC
    case 56: *out = EBADRQC; return 0;
#endif
#ifdef EBADSLT
    case 57: *out = EBADSLT; return 0;
#endif
#ifdef EDEADLOCK
    case 58: *out = EDEADLOCK; return 0;
#endif
#ifdef EBFONT
    case 59: *out = EBFONT; return 0;
#endif
#ifdef ENOSTR
    case 60: *out = ENOSTR; return 0;
#endif
#ifdef ENODATA
    case 61: *out = ENODATA; return 0;
#endif
#ifdef ETIME
    case 62: *out = ETIME; return 0;
#endif
#ifdef ENOSR
    case 63: *out = ENOSR; return 0;
#endif
#ifdef ENONET
    case 64: *out = ENONET; return 0;
#endif
#ifdef ENOPKG
    case 65: *out = ENOPKG; return 0;
#endif
#ifdef EREMOTE
    case 66: *out = EREMOTE; return 0;
#endif
#ifdef ENOLINK
    case 67: *out = ENOLINK; return 0;
#endif
#ifdef EADV
    case 68: *out = EADV; return 0;
#endif
#ifdef ESRMNT
    case 69: *out = ESRMNT; return 0;
#endif
#ifdef ECOMM
    case 70: *out = ECOMM; return 0;
#endif
#ifdef EPROTO
    case 71: *out = EPROTO; return 0;
#endif
#ifdef EMULTIHOP
    case 72: *out = EMULTIHOP; return 0;
#endif
#ifdef EDOTDOT
    case 73: *out = EDOTDOT; return 0;
#endif
#ifdef EBADMSG
    case 74: *out = EBADMSG; return 0;
#endif
#ifdef EOVERFLOW
    case 75: *out = EOVERFLOW; return 0;
#endif
#ifdef ENOTUNIQ
    case 76: *out = ENOTUNIQ; return 0;
#endif
#ifdef EBADFD
    case 77: *out = EBADFD; return 0;
#endif
#ifdef EREMCHG
    case 78: *out = EREMCHG; return 0;
#endif
#ifdef EFTYPE
    case 79: *out = EFTYPE; return 0;
#endif
#ifdef ELIBACC
    case 80: *out = ELIBACC; return 0;
#endif
#ifdef ELIBBAD
    case 81: *out = ELIBBAD; return 0;
#endif
#ifdef ELIBSCN
    case 82: *out = ELIBSCN; return 0;
#endif
#ifdef ELIBMAX
    case 83: *out = ELIBMAX; return 0;
#endif
#ifdef EILSEQ
    case 84: *out = EILSEQ; return 0;
#endif
#ifdef ERESTART
    case 85: *out = ERESTART; return 0;
#endif
#ifdef ESTRPIPE
    case 86: *out = ESTRPIPE; return 0;
#endif
#ifdef EUSERS
    case 87: *out = EUSERS; return 0;
#endif
#ifdef ENOTSOCK
    case 88: *out = ENOTSOCK; return 0;
#endif
#ifdef EDESTADDRREQ
    case 89: *out = EDESTADDRREQ; return 0;
#endif
#ifdef EMSGSIZE
    case 90: *out = EMSGSIZE; return 0;
#endif
#ifdef EPROTOTYPE
    case 91: *out = EPROTOTYPE; return 0;
#endif
#ifdef ENOPROTOOPT
    case 92: *out = ENOPROTOOPT; return 0;
#endif
#ifdef EPROTONOSUPPORT
    case 93: *out = EPROTONOSUPPORT; return 0;
#endif
#ifdef ESOCKTNOSUPPORT
    case 94: *out = ESOCKTNOSUPPORT; return 0;
#endif
#ifdef ENOTSUP
    case 95: *out = ENOTSUP; return 0;
#endif
#ifdef EPFNOSUPPORT
    case 96: *out = EPFNOSUPPORT; return 0;
#endif
#ifdef EAFNOSUPPORT
    case 97: *out = EAFNOSUPPORT; return 0;
#endif
#ifdef EADDRINUSE
    case 98: *out = EADDRINUSE; return 0;
#endif
#ifdef EADDRNOTAVAIL
    case 99: *out = EADDRNOTAVAIL; return 0;
#endif
#ifdef ENETDOWN
    case 100: *out = ENETDOWN; return 0;
#endif
#ifdef ENETUNREACH
    case 101: *out = ENETUNREACH; return 0;
#endif
#ifdef ENETRESET
    case 102: *out = ENETRESET; return 0;
#endif
#ifdef ECONNABORTED
    case 103: *out = ECONNABORTED; return 0;
#endif
#ifdef ECONNRESET
    case 104: *out = ECONNRESET; return 0;
#endif
#ifdef ENOBUFS
    case 105: *out = ENOBUFS; return 0;
#endif
#ifdef EISCONN
    case 106: *out = EISCONN; return 0;
#endif
#ifdef ENOTCONN
    case 107: *out = ENOTCONN; return 0;
#endif
#ifdef ESHUTDOWN
    case 108: *out = ESHUTDOWN; return 0;
#endif
#ifdef ETOOMANYREFS
    case 109: *out = ETOOMANYREFS; return 0;
#endif
#ifdef ETIMEDOUT
    case 110: *out = ETIMEDOUT; return 0;
#endif
#ifdef ECONNREFUSED
    case 111: *out = ECONNREFUSED; return 0;
#endif
#ifdef EHOSTDOWN
    case 112: *out = EHOSTDOWN; return 0;
#endif
#ifdef EHOSTUNREACH
    case 113: *out = EHOSTUNREACH; return 0;
#endif
#ifdef EALREADY
    case 114: *out = EALREADY; return 0;
#endif
#ifdef EINPROGRESS
    case 115: *out = EINPROGRESS; return 0;
#endif
#ifdef ESTALE
    case 116: *out = ESTALE; return 0;
#endif
#ifdef EUCLEAN
    case 117: *out = EUCLEAN; return 0;
#endif
#ifdef ENOTNAM
    case 118: *out = ENOTNAM; return 0;
#endif
#ifdef ENAVAIL
    case 119: *out = ENAVAIL; return 0;
#endif
#ifdef EISNAM
    case 120: *out = EISNAM; return 0;
#endif
#ifdef EREMOTEIO
    case 121: *out = EREMOTEIO; return 0;
#endif
#ifdef EDQUOT
    case 122: *out = EDQUOT; return 0;
#endif
#ifdef ENOMEDIUM
    case 123: *out = ENOMEDIUM; return 0;
#endif
#ifdef EMEDIUMTYPE
    case 124: *out = EMEDIUMTYPE; return 0;
#endif
#ifdef ECANCELED
    case 125: *out = ECANCELED; return 0;
#endif
#ifdef ENOKEY
    case 126: *out = ENOKEY; return 0;
#endif
#ifdef EKEYEXPIRED
    case 127: *out = EKEYEXPIRED; return 0;
#endif
#ifdef EKEYREVOKED
    case 128: *out = EKEYREVOKED; return 0;
#endif
#ifdef EKEYREJECTED
    case 129: *out = EKEYREJECTED; return 0;
#endif
#ifdef EOWNERDEAD
    case 130: *out = EOWNERDEAD; return 0;
#endif
#ifdef ENOTRECOVERABLE
    case 131: *out = ENOTRECOVERABLE; return 0;
#endif
#ifdef ERFKILL
    case 132: *out = ERFKILL; return 0;
#endif
#ifdef EHWPOISON
    case 133: *out = EHWPOISON; return 0;
#endif
#ifdef EAUTH
    case 134: *out = EAUTH; return 0;
#endif
#ifdef ENEEDAUTH
    case 135: *out = ENEEDAUTH; return 0;
#endif
#ifdef EBADRPC
    case 136: *out = EBADRPC; return 0;
#endif
#ifdef ERPCMISMATCH
    case 137: *out = ERPCMISMATCH; return 0;
#endif
#ifdef EPROGUNAVAIL
    case 138: *out = EPROGUNAVAIL; return 0;
#endif
#ifdef EPROGMISMATCH
    case 139: *out = EPROGMISMATCH; return 0;
#endif
#ifdef EPROCUNAVAIL
    case 140: *out = EPROCUNAVAIL; return 0;
#endif
#ifdef ENOATTR
    case 141: *out = ENOATTR; return 0;
#endif
#ifdef EDOOFUS
    case 142: *out = EDOOFUS; return 0;
#endif
#ifdef EJUSTRETURN
    case 143: *out = EJUSTRETURN; return 0;
#endif
#ifdef ENOIOCTL
    case 144: *out = ENOIOCTL; return 0;
#endif
#ifdef EOPNOTSUPP
    case 145: *out = EOPNOTSUPP; return 0;
#endif
#ifdef ECAPMODE
    case 146: *out = ECAPMODE; return 0;
#endif
#ifdef ENOTCAPABLE
    case 147: *out = ENOTCAPABLE; return 0;
#endif
#ifdef EBADEXEC
    case 148: *out = EBADEXEC; return 0;
#endif
#ifdef EBADARCH
    case 149: *out = EBADARCH; return 0;
#endif
#ifdef ESHLIBVERS
    case 150: *out = ESHLIBVERS; return 0;
#endif
#ifdef EBADMACHO
    case 151: *out = EBADMACHO; return 0;
#endif
    default:
        return -1;
    }
}

int lunacy_errno_description(int err, char *buf, size_t len) {
    const char *msg;
    size_t n;

    if (!buf || !len) {
        return -1;
    }

    msg = strerror(err);
    if (!msg) {
        return -1;
    }

    n = strlen(msg);
    if (n >= len) {
        n = len - 1;
    }
    memcpy(buf, msg, n);
    buf[n] = 0;
    return (int)n;
}
