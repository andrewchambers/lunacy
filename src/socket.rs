//! Single-call BSD socket operations. No retries, buffering, or signal changes.

use crate::{
    Errno,
    fd::{BorrowedFd, OwnedFd},
    sys,
};
use core::{
    ffi::{CStr, c_int},
    ops::{BitOr, BitOrAssign},
    ptr,
};

macro_rules! native_enum {
    ($(#[$doc:meta])* $name:ident { $($(#[$vdoc:meta])* $variant:ident => $symbol:ident),+ $(,)? }) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum $name {
            $($(#[$vdoc])* $variant,)+
            /// A native value supplied by the caller, including platform flags.
            Raw(c_int),
        }
        unsafe extern "C" { $(fn $symbol() -> c_int;)+ }
        impl $name {
            /// Resolves this value using the target's C headers.
            pub fn to_raw(self) -> c_int {
                match self {
                    // SAFETY: Each shim only reads a C constant.
                    $(Self::$variant => unsafe { $symbol() },)+
                    Self::Raw(raw) => raw,
                }
            }
        }
    };
}

native_enum! {
    /// The socket address family (AF_*).
    AddressFamily {
        /// AF_INET.
        Inet => lunacy_af_inet,
        /// AF_INET6.
        Inet6 => lunacy_af_inet6,
        /// AF_UNIX.
        Unix => lunacy_af_unix,
    }
}
native_enum! {
    /// The socket type (SOCK_*).
    SocketType {
        /// SOCK_STREAM.
        Stream => lunacy_sock_stream,
        /// SOCK_DGRAM.
        Datagram => lunacy_sock_dgram,
        /// SOCK_SEQPACKET.
        SeqPacket => lunacy_sock_seqpacket,
    }
}
native_enum! {
    /// Which half of a socket to shut down.
    Shutdown {
        /// SHUT_RD.
        Read => lunacy_shut_rd,
        /// SHUT_WR.
        Write => lunacy_shut_wr,
        /// SHUT_RDWR.
        Both => lunacy_shut_rdwr,
    }
}

/// Native message flags for send and recv.
///
/// Only synchronous buffer access is supported. In particular, MSG_ZEROCOPY
/// cannot be used with these borrowed-buffer operations.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MsgFlags(c_int);

impl MsgFlags {
    /// No flags.
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Wraps native flags for interoperability.
    ///
    /// # Safety
    /// These flags, including their combination with other flags, must not let
    /// libc or the kernel access the supplied buffer after send/recv returns.
    /// MSG_ZEROCOPY does not satisfy this requirement.
    pub const unsafe fn from_raw(raw: c_int) -> Self {
        Self(raw)
    }

    /// Returns the native flags.
    pub const fn to_raw(self) -> c_int {
        self.0
    }

    /// MSG_PEEK: inspect received data without consuming it.
    pub fn peek() -> Self {
        unsafe extern "C" {
            fn lunacy_msg_peek() -> c_int;
        }
        // SAFETY: Reads a C constant.
        Self(unsafe { lunacy_msg_peek() })
    }

    /// MSG_OOB: out-of-band data.
    pub fn oob() -> Self {
        unsafe extern "C" {
            fn lunacy_msg_oob() -> c_int;
        }
        // SAFETY: Reads a C constant.
        Self(unsafe { lunacy_msg_oob() })
    }

    /// MSG_DONTROUTE: bypass routing for send.
    pub fn dont_route() -> Self {
        unsafe extern "C" {
            fn lunacy_msg_dontroute() -> c_int;
        }
        // SAFETY: Reads a C constant.
        Self(unsafe { lunacy_msg_dontroute() })
    }
}

impl BitOr for MsgFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for MsgFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Stack storage for a native socket address and its socklen_t length.
///
/// C builds/interprets the native layout. Address octets are in network order;
/// ports, IPv6 flow information, and scope IDs use host-order Rust integers.
/// This wrapper has no heap allocation. Its storage capacity is checked against
/// the target's sockaddr_storage at C compilation time.
#[derive(Clone)]
pub struct SockAddr {
    storage: sys::Storage,
    len: usize,
}

impl SockAddr {
    /// Empty output storage for accept. Initialize with a constructor before
    /// passing it to bind or connect.
    pub const fn new() -> Self {
        Self {
            storage: sys::Storage([0; 128]),
            len: 0,
        }
    }

    /// Builds sockaddr_in from four address octets and a host-order port.
    pub fn ipv4(ip: [u8; 4], port: u16) -> Self {
        let mut addr = Self::new();
        // SAFETY: C receives correctly sized, nonoverlapping input/output storage.
        addr.len = unsafe { sys::lunacy_addr_v4(&mut addr.storage, ip.as_ptr(), port) };
        addr
    }

    /// Builds sockaddr_in6; numeric fields are in host order.
    pub fn ipv6(ip: [u8; 16], port: u16, flowinfo: u32, scope_id: u32) -> Self {
        let mut addr = Self::new();
        // SAFETY: C reads exactly 16 octets and initializes the output storage.
        addr.len = unsafe {
            sys::lunacy_addr_v6(&mut addr.storage, ip.as_ptr(), port, flowinfo, scope_id)
        };
        addr
    }

    /// Builds a pathname Unix socket address. Empty or overlong paths return
    /// EINVAL. Path removal remains the caller's responsibility. Abstract Unix
    /// addresses are not supported by this constructor.
    pub fn unix(path: &CStr) -> Result<Self, Errno> {
        let mut addr = Self::new();
        // SAFETY: path is readable for its length and outputs are writable.
        sys::cvt(unsafe {
            sys::lunacy_addr_unix(
                &mut addr.storage,
                path.as_ptr(),
                path.to_bytes().len(),
                &mut addr.len,
            )
        })?;
        Ok(addr)
    }

    /// Returns the native address family number.
    pub fn family(&self) -> c_int {
        // SAFETY: The storage is initialized even for an empty address.
        unsafe { sys::lunacy_addr_family(&self.storage) }
    }

    /// Returns IPv4 address octets and a host-order port, if this is sockaddr_in.
    pub fn as_ipv4(&self) -> Option<([u8; 4], u16)> {
        let (mut ip, mut port) = ([0; 4], 0);
        // SAFETY: C checks the family/length and writes bounded output fields.
        let found =
            unsafe { sys::lunacy_addr_get_v4(&self.storage, self.len, ip.as_mut_ptr(), &mut port) };
        (found != 0).then_some((ip, port))
    }

    /// Returns IPv6 octets, port, flow information, and scope ID in host order.
    pub fn as_ipv6(&self) -> Option<([u8; 16], u16, u32, u32)> {
        let (mut ip, mut port, mut flow, mut scope) = ([0; 16], 0, 0, 0);
        // SAFETY: C checks the family/length and writes bounded output fields.
        let found = unsafe {
            sys::lunacy_addr_get_v6(
                &self.storage,
                self.len,
                ip.as_mut_ptr(),
                &mut port,
                &mut flow,
                &mut scope,
            )
        };
        (found != 0).then_some((ip, port, flow, scope))
    }
}

impl Default for SockAddr {
    fn default() -> Self {
        Self::new()
    }
}

fn owned(result: c_int) -> Result<OwnedFd, Errno> {
    let raw = sys::cvt(result)?;
    // SAFETY: Only successful socket/accept results are passed here; these
    // create fresh descriptors whose ownership has not yet been assigned.
    Ok(unsafe { OwnedFd::from_raw_fd(raw) })
}

/// Calls socket once. A protocol of zero requests the family's default.
pub fn socket(domain: AddressFamily, kind: SocketType, protocol: c_int) -> Result<OwnedFd, Errno> {
    // Resolve constants before making the errno-producing call.
    let (domain, kind) = (domain.to_raw(), kind.to_raw());
    // SAFETY: Integer arguments are validated by libc; success creates an owner.
    owned(unsafe { sys::lunacy_socket(domain, kind, protocol) })
}

/// Calls socketpair once, returning two independently owned descriptors.
pub fn socketpair(
    domain: AddressFamily,
    kind: SocketType,
    protocol: c_int,
) -> Result<(OwnedFd, OwnedFd), Errno> {
    let (domain, kind) = (domain.to_raw(), kind.to_raw());
    let mut fds = [-1; 2];
    // SAFETY: fds has space for both returned descriptors.
    sys::cvt(unsafe { sys::lunacy_socketpair(domain, kind, protocol, fds.as_mut_ptr()) })?;
    // SAFETY: Successful socketpair transferred two distinct live descriptors.
    Ok(unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) })
}

/// Calls bind once using the native address.
pub fn bind(fd: BorrowedFd<'_>, addr: &SockAddr) -> Result<(), Errno> {
    // SAFETY: The descriptor and initialized address stay live through the call.
    sys::cvt(unsafe { sys::lunacy_bind(fd.as_raw_fd(), &addr.storage, addr.len) }).map(|_| ())
}

/// Calls connect once. Nonblocking completion (including EINPROGRESS) is left
/// to the caller, as are retries of interrupted calls.
pub fn connect(fd: BorrowedFd<'_>, addr: &SockAddr) -> Result<(), Errno> {
    // SAFETY: The descriptor and initialized address stay live through the call.
    sys::cvt(unsafe { sys::lunacy_connect(fd.as_raw_fd(), &addr.storage, addr.len) }).map(|_| ())
}

/// Calls listen once with the caller's backlog.
pub fn listen(fd: BorrowedFd<'_>, backlog: c_int) -> Result<(), Errno> {
    // SAFETY: The descriptor is borrowed and libc validates the backlog.
    sys::cvt(unsafe { sys::lunacy_listen(fd.as_raw_fd(), backlog) }).map(|_| ())
}

/// Calls accept once. Optionally fills caller-provided peer address storage.
/// Does not add nonblocking or close-on-exec flags to the returned descriptor.
pub fn accept(fd: BorrowedFd<'_>, addr: Option<&mut SockAddr>) -> Result<OwnedFd, Errno> {
    let (storage, len) = match addr {
        Some(addr) => (&mut addr.storage as *mut _, &mut addr.len as *mut _),
        None => (ptr::null_mut(), ptr::null_mut()),
    };
    // SAFETY: Outputs are either both null or writable for the call.
    owned(unsafe { sys::lunacy_accept(fd.as_raw_fd(), storage, len) })
}

/// Calls getsockname once, returning the socket's local address on the stack.
pub fn getsockname(fd: BorrowedFd<'_>) -> Result<SockAddr, Errno> {
    let mut addr = SockAddr::new();
    // SAFETY: Valid borrowed descriptor and writable address storage.
    sys::cvt(unsafe { sys::lunacy_getsockname(fd.as_raw_fd(), &mut addr.storage, &mut addr.len) })?;
    Ok(addr)
}

/// Calls getpeername once, returning the socket's peer address on the stack.
pub fn getpeername(fd: BorrowedFd<'_>) -> Result<SockAddr, Errno> {
    let mut addr = SockAddr::new();
    // SAFETY: Valid borrowed descriptor and writable address storage.
    sys::cvt(unsafe { sys::lunacy_getpeername(fd.as_raw_fd(), &mut addr.storage, &mut addr.len) })?;
    Ok(addr)
}

/// Calls shutdown once without closing or transferring the descriptor.
pub fn shutdown(fd: BorrowedFd<'_>, how: Shutdown) -> Result<(), Errno> {
    let how = how.to_raw();
    // SAFETY: The descriptor remains live and libc validates how.
    sys::cvt(unsafe { sys::lunacy_shutdown(fd.as_raw_fd(), how) }).map(|_| ())
}

/// Calls send once with native flags (MsgFlags::empty for ordinary sending).
/// Returns partial sends directly. Signal handling, including SIGPIPE, is unchanged.
pub fn send(fd: BorrowedFd<'_>, buffer: &[u8], flags: MsgFlags) -> Result<usize, Errno> {
    // SAFETY: The descriptor is live and C can read the entire buffer.
    sys::cvt_size(unsafe {
        sys::lunacy_send(fd.as_raw_fd(), buffer.as_ptr(), buffer.len(), flags.0)
    })
}

/// Calls recv once with native flags. Zero indicates EOF for a stream socket.
/// Some native flags (e.g. MSG_TRUNC) can return a length greater than the buffer;
/// only bytes within the supplied buffer can have been written.
pub fn recv(fd: BorrowedFd<'_>, buffer: &mut [u8], flags: MsgFlags) -> Result<usize, Errno> {
    // SAFETY: The descriptor is live and C has exclusive access to the buffer.
    sys::cvt_size(unsafe {
        sys::lunacy_recv(fd.as_raw_fd(), buffer.as_mut_ptr(), buffer.len(), flags.0)
    })
}
