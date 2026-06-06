//! `errno` support that does not depend on `std`.

use alloc::{format, string::String};
use core::fmt;

/// A raw libc `errno` value.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Errno(crate::ffi::c_int);

/// A crate-local result type using [`Errno`] as the error.
pub type Result<T> = core::result::Result<T, Errno>;

macro_rules! errno_names {
    ($($variant:ident = $id:literal => $c_name:literal,)+) => {
        /// A symbolic libc errno name.
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[repr(u16)]
        pub enum ErrnoName {
            $(
                #[doc = concat!("The libc `", $c_name, "` errno symbol.")]
                $variant = $id,
            )+
        }

        impl ErrnoName {
            /// All errno names known to `lunacy`.
            pub const ALL: &'static [Self] = &[$(Self::$variant,)+];

            /// Returns the C errno symbol name.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $c_name,)+
                }
            }

            const fn id(self) -> crate::ffi::c_int {
                self as crate::ffi::c_int
            }
        }
    };
}

errno_names! {
    EPERM = 1 => "EPERM",
    ENOENT = 2 => "ENOENT",
    ESRCH = 3 => "ESRCH",
    EINTR = 4 => "EINTR",
    EIO = 5 => "EIO",
    ENXIO = 6 => "ENXIO",
    E2BIG = 7 => "E2BIG",
    ENOEXEC = 8 => "ENOEXEC",
    EBADF = 9 => "EBADF",
    ECHILD = 10 => "ECHILD",
    EAGAIN = 11 => "EAGAIN",
    ENOMEM = 12 => "ENOMEM",
    EACCES = 13 => "EACCES",
    EFAULT = 14 => "EFAULT",
    ENOTBLK = 15 => "ENOTBLK",
    EBUSY = 16 => "EBUSY",
    EEXIST = 17 => "EEXIST",
    EXDEV = 18 => "EXDEV",
    ENODEV = 19 => "ENODEV",
    ENOTDIR = 20 => "ENOTDIR",
    EISDIR = 21 => "EISDIR",
    EINVAL = 22 => "EINVAL",
    ENFILE = 23 => "ENFILE",
    EMFILE = 24 => "EMFILE",
    ENOTTY = 25 => "ENOTTY",
    ETXTBSY = 26 => "ETXTBSY",
    EFBIG = 27 => "EFBIG",
    ENOSPC = 28 => "ENOSPC",
    ESPIPE = 29 => "ESPIPE",
    EROFS = 30 => "EROFS",
    EMLINK = 31 => "EMLINK",
    EPIPE = 32 => "EPIPE",
    EDOM = 33 => "EDOM",
    ERANGE = 34 => "ERANGE",
    EDEADLK = 35 => "EDEADLK",
    ENAMETOOLONG = 36 => "ENAMETOOLONG",
    ENOLCK = 37 => "ENOLCK",
    ENOSYS = 38 => "ENOSYS",
    ENOTEMPTY = 39 => "ENOTEMPTY",
    ELOOP = 40 => "ELOOP",
    EWOULDBLOCK = 41 => "EWOULDBLOCK",
    ENOMSG = 42 => "ENOMSG",
    EIDRM = 43 => "EIDRM",
    ECHRNG = 44 => "ECHRNG",
    EL2NSYNC = 45 => "EL2NSYNC",
    EL3HLT = 46 => "EL3HLT",
    EL3RST = 47 => "EL3RST",
    ELNRNG = 48 => "ELNRNG",
    EUNATCH = 49 => "EUNATCH",
    ENOCSI = 50 => "ENOCSI",
    EL2HLT = 51 => "EL2HLT",
    EBADE = 52 => "EBADE",
    EBADR = 53 => "EBADR",
    EXFULL = 54 => "EXFULL",
    ENOANO = 55 => "ENOANO",
    EBADRQC = 56 => "EBADRQC",
    EBADSLT = 57 => "EBADSLT",
    EDEADLOCK = 58 => "EDEADLOCK",
    EBFONT = 59 => "EBFONT",
    ENOSTR = 60 => "ENOSTR",
    ENODATA = 61 => "ENODATA",
    ETIME = 62 => "ETIME",
    ENOSR = 63 => "ENOSR",
    ENONET = 64 => "ENONET",
    ENOPKG = 65 => "ENOPKG",
    EREMOTE = 66 => "EREMOTE",
    ENOLINK = 67 => "ENOLINK",
    EADV = 68 => "EADV",
    ESRMNT = 69 => "ESRMNT",
    ECOMM = 70 => "ECOMM",
    EPROTO = 71 => "EPROTO",
    EMULTIHOP = 72 => "EMULTIHOP",
    EDOTDOT = 73 => "EDOTDOT",
    EBADMSG = 74 => "EBADMSG",
    EOVERFLOW = 75 => "EOVERFLOW",
    ENOTUNIQ = 76 => "ENOTUNIQ",
    EBADFD = 77 => "EBADFD",
    EREMCHG = 78 => "EREMCHG",
    EFTYPE = 79 => "EFTYPE",
    ELIBACC = 80 => "ELIBACC",
    ELIBBAD = 81 => "ELIBBAD",
    ELIBSCN = 82 => "ELIBSCN",
    ELIBMAX = 83 => "ELIBMAX",
    EILSEQ = 84 => "EILSEQ",
    ERESTART = 85 => "ERESTART",
    ESTRPIPE = 86 => "ESTRPIPE",
    EUSERS = 87 => "EUSERS",
    ENOTSOCK = 88 => "ENOTSOCK",
    EDESTADDRREQ = 89 => "EDESTADDRREQ",
    EMSGSIZE = 90 => "EMSGSIZE",
    EPROTOTYPE = 91 => "EPROTOTYPE",
    ENOPROTOOPT = 92 => "ENOPROTOOPT",
    EPROTONOSUPPORT = 93 => "EPROTONOSUPPORT",
    ESOCKTNOSUPPORT = 94 => "ESOCKTNOSUPPORT",
    ENOTSUP = 95 => "ENOTSUP",
    EPFNOSUPPORT = 96 => "EPFNOSUPPORT",
    EAFNOSUPPORT = 97 => "EAFNOSUPPORT",
    EADDRINUSE = 98 => "EADDRINUSE",
    EADDRNOTAVAIL = 99 => "EADDRNOTAVAIL",
    ENETDOWN = 100 => "ENETDOWN",
    ENETUNREACH = 101 => "ENETUNREACH",
    ENETRESET = 102 => "ENETRESET",
    ECONNABORTED = 103 => "ECONNABORTED",
    ECONNRESET = 104 => "ECONNRESET",
    ENOBUFS = 105 => "ENOBUFS",
    EISCONN = 106 => "EISCONN",
    ENOTCONN = 107 => "ENOTCONN",
    ESHUTDOWN = 108 => "ESHUTDOWN",
    ETOOMANYREFS = 109 => "ETOOMANYREFS",
    ETIMEDOUT = 110 => "ETIMEDOUT",
    ECONNREFUSED = 111 => "ECONNREFUSED",
    EHOSTDOWN = 112 => "EHOSTDOWN",
    EHOSTUNREACH = 113 => "EHOSTUNREACH",
    EALREADY = 114 => "EALREADY",
    EINPROGRESS = 115 => "EINPROGRESS",
    ESTALE = 116 => "ESTALE",
    EUCLEAN = 117 => "EUCLEAN",
    ENOTNAM = 118 => "ENOTNAM",
    ENAVAIL = 119 => "ENAVAIL",
    EISNAM = 120 => "EISNAM",
    EREMOTEIO = 121 => "EREMOTEIO",
    EDQUOT = 122 => "EDQUOT",
    ENOMEDIUM = 123 => "ENOMEDIUM",
    EMEDIUMTYPE = 124 => "EMEDIUMTYPE",
    ECANCELED = 125 => "ECANCELED",
    ENOKEY = 126 => "ENOKEY",
    EKEYEXPIRED = 127 => "EKEYEXPIRED",
    EKEYREVOKED = 128 => "EKEYREVOKED",
    EKEYREJECTED = 129 => "EKEYREJECTED",
    EOWNERDEAD = 130 => "EOWNERDEAD",
    ENOTRECOVERABLE = 131 => "ENOTRECOVERABLE",
    ERFKILL = 132 => "ERFKILL",
    EHWPOISON = 133 => "EHWPOISON",
    EAUTH = 134 => "EAUTH",
    ENEEDAUTH = 135 => "ENEEDAUTH",
    EBADRPC = 136 => "EBADRPC",
    ERPCMISMATCH = 137 => "ERPCMISMATCH",
    EPROGUNAVAIL = 138 => "EPROGUNAVAIL",
    EPROGMISMATCH = 139 => "EPROGMISMATCH",
    EPROCUNAVAIL = 140 => "EPROCUNAVAIL",
    ENOATTR = 141 => "ENOATTR",
    EDOOFUS = 142 => "EDOOFUS",
    EJUSTRETURN = 143 => "EJUSTRETURN",
    ENOIOCTL = 144 => "ENOIOCTL",
    EOPNOTSUPP = 145 => "EOPNOTSUPP",
    ECAPMODE = 146 => "ECAPMODE",
    ENOTCAPABLE = 147 => "ENOTCAPABLE",
    EBADEXEC = 148 => "EBADEXEC",
    EBADARCH = 149 => "EBADARCH",
    ESHLIBVERS = 150 => "ESHLIBVERS",
    EBADMACHO = 151 => "EBADMACHO",
}

impl Errno {
    /// Creates an errno wrapper from a raw libc error value.
    pub const fn new(raw: crate::ffi::c_int) -> Self {
        Self(raw)
    }

    /// Returns the wrapped raw libc error value.
    pub const fn raw(self) -> crate::ffi::c_int {
        self.0
    }

    /// Returns libc's value for a symbolic errno name.
    ///
    /// Returns `None` when the target libc does not define `name`.
    pub fn named(name: ErrnoName) -> Option<Self> {
        let mut raw = 0;
        let status = unsafe { lunacy_errno_value(name.id(), &mut raw) };
        if status == 0 { Some(Self(raw)) } else { None }
    }

    /// Returns the first symbolic errno name matching this value.
    ///
    /// Aliased errno names may map to the same value. In that case this returns
    /// the first known name in [`ErrnoName::ALL`].
    pub fn name(self) -> Option<ErrnoName> {
        ErrnoName::ALL.iter().copied().find(|name| self.is(*name))
    }

    /// Returns whether this errno equals libc's value for `name`.
    pub fn is(self, name: ErrnoName) -> bool {
        match Self::named(name) {
            Some(errno) => self == errno,
            None => false,
        }
    }

    /// Returns libc's value for `EINTR`.
    ///
    /// This is a function instead of a Rust constant so libcs with dynamic
    /// errno values can provide the value through their C headers.
    pub fn eintr() -> Self {
        Self::named(ErrnoName::EINTR).expect("target libc must define EINTR")
    }

    /// Returns libc's value for `EIO`.
    ///
    /// This is a function instead of a Rust constant so libcs with dynamic
    /// errno values can provide the value through their C headers.
    pub fn eio() -> Self {
        Self::named(ErrnoName::EIO).expect("target libc must define EIO")
    }

    /// Calls `f` with libc's description for this errno value.
    ///
    /// If libc cannot describe the value, the callback receives a fallback of
    /// the form `errno N`.
    pub fn with_description<T>(self, f: impl FnOnce(&str) -> T) -> T {
        let mut buf = [0_u8; 256];
        let len = unsafe {
            lunacy_errno_description(
                self.0,
                buf.as_mut_ptr().cast(),
                buf.len() as crate::ffi::size_t,
            )
        };

        if len >= 0 {
            let bytes = &buf[..len as usize];
            if let Ok(description) = core::str::from_utf8(bytes) {
                return f(description);
            }
        }

        let fallback = format!("errno {}", self.0);
        f(&fallback)
    }

    /// Returns libc's description for this errno value.
    pub fn description(self) -> String {
        self.with_description(|description| String::from(description))
    }

    /// Reads the current thread's libc `errno`.
    pub fn last() -> Self {
        let ptr = unsafe { lunacy_errno_location() };
        Self(unsafe { ptr.read() })
    }

    /// Sets the current thread's libc `errno`.
    pub fn set_last(self) {
        let ptr = unsafe { lunacy_errno_location() };
        unsafe {
            ptr.write(self.0);
        }
    }
}

impl fmt::Debug for Errno {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Errno").field(&self.0).finish()
    }
}

impl fmt::Display for Errno {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "errno {}", self.0)
    }
}

impl fmt::Display for ErrnoName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

unsafe extern "C" {
    fn lunacy_errno_location() -> *mut crate::ffi::c_int;
    fn lunacy_errno_value(
        name: crate::ffi::c_int,
        out: *mut crate::ffi::c_int,
    ) -> crate::ffi::c_int;
    fn lunacy_errno_description(
        err: crate::ffi::c_int,
        buf: *mut crate::ffi::c_char,
        len: crate::ffi::size_t,
    ) -> crate::ffi::c_int;
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use super::{Errno, ErrnoName};

    #[test]
    fn wraps_raw_errno() {
        let errno = Errno::new(22);

        assert_eq!(errno.raw(), 22);
        assert_eq!(format!("{errno}"), "errno 22");
    }

    #[test]
    fn exposes_errno_values_from_c() {
        assert!(Errno::eintr().raw() > 0);
        assert!(Errno::eio().raw() > 0);
        assert_eq!(Errno::named(ErrnoName::EINTR), Some(Errno::eintr()));
        assert!(Errno::eintr().is(ErrnoName::EINTR));
        assert_eq!(Errno::eintr().name(), Some(ErrnoName::EINTR));
        assert_eq!(ErrnoName::EINTR.as_str(), "EINTR");
    }

    #[test]
    fn describes_errno_values() {
        let description = Errno::eintr().description();

        assert!(!description.is_empty());
    }
}
