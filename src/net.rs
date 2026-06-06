//! Unix networking wrappers.
//!
//! Socket constants and socket address layout are provided by the C shim. Rust
//! code passes neutral address values and receives ordinary owned file
//! descriptors.

use core::mem::MaybeUninit;

use crate::{
    Errno, Result,
    fd::{FdArg, OwnedFd, RawFd},
};

const NET_AF_INET: crate::ffi::c_int = 1;
const NET_AF_INET6: crate::ffi::c_int = 2;
const NET_AF_UNIX: crate::ffi::c_int = 3;
const NET_SOCK_STREAM: crate::ffi::c_int = 4;
const NET_SOCK_DGRAM: crate::ffi::c_int = 5;
const NET_SOCK_CLOEXEC: crate::ffi::c_int = 6;
const NET_SOCK_NONBLOCK: crate::ffi::c_int = 7;
const NET_IPPROTO_TCP: crate::ffi::c_int = 8;
const NET_IPPROTO_UDP: crate::ffi::c_int = 9;
const NET_SHUT_RD: crate::ffi::c_int = 10;
const NET_SHUT_WR: crate::ffi::c_int = 11;
const NET_SHUT_RDWR: crate::ffi::c_int = 12;

const SOCKET_ADDR_V4: crate::ffi::c_int = 1;
const SOCKET_ADDR_V6: crate::ffi::c_int = 2;

/// A socket address family.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct AddressFamily(crate::ffi::c_int);

impl AddressFamily {
    /// Returns libc's `AF_INET` value.
    pub fn inet() -> Result<Self> {
        net_constant(NET_AF_INET).map(Self)
    }

    /// Returns libc's `AF_INET6` value.
    pub fn inet6() -> Result<Self> {
        net_constant(NET_AF_INET6).map(Self)
    }

    /// Returns libc's `AF_UNIX` value.
    pub fn unix() -> Result<Self> {
        net_constant(NET_AF_UNIX).map(Self)
    }

    /// Wraps a raw address family value.
    pub const fn raw(raw: crate::ffi::c_int) -> Self {
        Self(raw)
    }

    /// Returns the raw address family value.
    pub const fn as_raw(self) -> crate::ffi::c_int {
        self.0
    }
}

/// A socket type, optionally including socket creation flags.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct SocketType(crate::ffi::c_int);

impl SocketType {
    /// Returns libc's `SOCK_STREAM` value.
    pub fn stream() -> Result<Self> {
        net_constant(NET_SOCK_STREAM).map(Self)
    }

    /// Returns libc's `SOCK_DGRAM` value.
    pub fn datagram() -> Result<Self> {
        net_constant(NET_SOCK_DGRAM).map(Self)
    }

    /// Adds libc's `SOCK_CLOEXEC` flag.
    pub fn with_close_on_exec(self) -> Result<Self> {
        net_constant(NET_SOCK_CLOEXEC).map(|flag| Self(self.0 | flag))
    }

    /// Adds libc's `SOCK_NONBLOCK` flag.
    pub fn with_nonblocking(self) -> Result<Self> {
        net_constant(NET_SOCK_NONBLOCK).map(|flag| Self(self.0 | flag))
    }

    /// Wraps a raw socket type value.
    pub const fn raw(raw: crate::ffi::c_int) -> Self {
        Self(raw)
    }

    /// Returns the raw socket type value.
    pub const fn as_raw(self) -> crate::ffi::c_int {
        self.0
    }
}

/// A socket protocol.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Protocol(crate::ffi::c_int);

impl Protocol {
    /// The default protocol for the chosen socket domain and type.
    pub const DEFAULT: Self = Self(0);

    /// Returns libc's `IPPROTO_TCP` value.
    pub fn tcp() -> Result<Self> {
        net_constant(NET_IPPROTO_TCP).map(Self)
    }

    /// Returns libc's `IPPROTO_UDP` value.
    pub fn udp() -> Result<Self> {
        net_constant(NET_IPPROTO_UDP).map(Self)
    }

    /// Wraps a raw protocol value.
    pub const fn raw(raw: crate::ffi::c_int) -> Self {
        Self(raw)
    }

    /// Returns the raw protocol value.
    pub const fn as_raw(self) -> crate::ffi::c_int {
        self.0
    }
}

/// A socket shutdown mode.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Shutdown(crate::ffi::c_int);

impl Shutdown {
    /// Returns libc's `SHUT_RD` value.
    pub fn read() -> Result<Self> {
        net_constant(NET_SHUT_RD).map(Self)
    }

    /// Returns libc's `SHUT_WR` value.
    pub fn write() -> Result<Self> {
        net_constant(NET_SHUT_WR).map(Self)
    }

    /// Returns libc's `SHUT_RDWR` value.
    pub fn both() -> Result<Self> {
        net_constant(NET_SHUT_RDWR).map(Self)
    }

    /// Wraps a raw shutdown mode.
    pub const fn raw(raw: crate::ffi::c_int) -> Self {
        Self(raw)
    }

    /// Returns the raw shutdown mode value.
    pub const fn as_raw(self) -> crate::ffi::c_int {
        self.0
    }
}

/// An IPv4 socket address.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SocketAddrV4 {
    ip: [u8; 4],
    port: u16,
}

impl SocketAddrV4 {
    /// Creates an IPv4 socket address from host-order components.
    pub const fn new(ip: [u8; 4], port: u16) -> Self {
        Self { ip, port }
    }

    /// Returns the IPv4 address bytes in network order.
    pub const fn ip(self) -> [u8; 4] {
        self.ip
    }

    /// Returns the port in host byte order.
    pub const fn port(self) -> u16 {
        self.port
    }
}

/// An IPv6 socket address.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SocketAddrV6 {
    ip: [u8; 16],
    port: u16,
    flowinfo: u32,
    scope_id: u32,
}

impl SocketAddrV6 {
    /// Creates an IPv6 socket address from host-order components.
    pub const fn new(ip: [u8; 16], port: u16, flowinfo: u32, scope_id: u32) -> Self {
        Self {
            ip,
            port,
            flowinfo,
            scope_id,
        }
    }

    /// Returns the IPv6 address bytes in network order.
    pub const fn ip(self) -> [u8; 16] {
        self.ip
    }

    /// Returns the port in host byte order.
    pub const fn port(self) -> u16 {
        self.port
    }

    /// Returns the IPv6 flow label and traffic class field.
    pub const fn flowinfo(self) -> u32 {
        self.flowinfo
    }

    /// Returns the IPv6 scope ID.
    pub const fn scope_id(self) -> u32 {
        self.scope_id
    }
}

/// An IPv4 or IPv6 socket address.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SocketAddr {
    /// An IPv4 address.
    V4(SocketAddrV4),
    /// An IPv6 address.
    V6(SocketAddrV6),
}

impl SocketAddr {
    /// Creates an IPv4 socket address.
    pub const fn v4(ip: [u8; 4], port: u16) -> Self {
        Self::V4(SocketAddrV4::new(ip, port))
    }

    /// Creates an IPv6 socket address.
    pub const fn v6(ip: [u8; 16], port: u16, flowinfo: u32, scope_id: u32) -> Self {
        Self::V6(SocketAddrV6::new(ip, port, flowinfo, scope_id))
    }

    /// Returns this address's port in host byte order.
    pub const fn port(self) -> u16 {
        match self {
            Self::V4(addr) => addr.port(),
            Self::V6(addr) => addr.port(),
        }
    }

    fn to_repr(self) -> SocketAddrRepr {
        let mut repr = SocketAddrRepr::default();

        match self {
            Self::V4(addr) => {
                repr.family = SOCKET_ADDR_V4;
                repr.ip[..4].copy_from_slice(&addr.ip());
                repr.port = addr.port();
            }
            Self::V6(addr) => {
                repr.family = SOCKET_ADDR_V6;
                repr.ip = addr.ip();
                repr.port = addr.port();
                repr.flowinfo = addr.flowinfo();
                repr.scope_id = addr.scope_id();
            }
        }

        repr
    }

    fn from_repr(repr: SocketAddrRepr) -> Self {
        match repr.family {
            SOCKET_ADDR_V4 => Self::v4([repr.ip[0], repr.ip[1], repr.ip[2], repr.ip[3]], repr.port),
            SOCKET_ADDR_V6 => Self::v6(repr.ip, repr.port, repr.flowinfo, repr.scope_id),
            _ => unreachable!("C shim only returns known socket address families"),
        }
    }
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct SocketAddrRepr {
    family: crate::ffi::c_int,
    ip: [u8; 16],
    port: u16,
    flowinfo: u32,
    scope_id: u32,
}

/// Creates a socket.
pub fn socket(
    domain: AddressFamily,
    socket_type: SocketType,
    protocol: Protocol,
) -> Result<OwnedFd> {
    cvt_fd(unsafe {
        lunacy_socket_create(domain.as_raw(), socket_type.as_raw(), protocol.as_raw())
    })
}

/// Connects a socket to a remote address.
pub fn connect(fd: impl FdArg, addr: SocketAddr) -> Result<()> {
    let addr = addr.to_repr();
    cvt_int(unsafe { lunacy_connect_addr(fd.raw_fd(), &addr) }).map(|_| ())
}

/// Binds a socket to a local address.
pub fn bind(fd: impl FdArg, addr: SocketAddr) -> Result<()> {
    let addr = addr.to_repr();
    cvt_int(unsafe { lunacy_bind_addr(fd.raw_fd(), &addr) }).map(|_| ())
}

/// Marks a socket as accepting connections.
pub fn listen(fd: impl FdArg, backlog: crate::ffi::c_int) -> Result<()> {
    cvt_int(unsafe { lunacy_listen(fd.raw_fd(), backlog) }).map(|_| ())
}

/// Accepts one pending connection.
pub fn accept(fd: impl FdArg) -> Result<OwnedFd> {
    cvt_fd(unsafe { lunacy_accept_fd(fd.raw_fd()) })
}

/// Accepts one pending connection and returns its peer address.
pub fn accept_with_addr(fd: impl FdArg) -> Result<(OwnedFd, SocketAddr)> {
    let mut addr = MaybeUninit::<SocketAddrRepr>::uninit();
    let accepted = cvt_fd(unsafe { lunacy_accept_addr(fd.raw_fd(), addr.as_mut_ptr()) })?;
    let addr = SocketAddr::from_repr(unsafe { addr.assume_init() });

    Ok((accepted, addr))
}

/// Shuts down part or all of a full-duplex socket connection.
pub fn shutdown(fd: impl FdArg, how: Shutdown) -> Result<()> {
    cvt_int(unsafe { lunacy_shutdown(fd.raw_fd(), how.as_raw()) }).map(|_| ())
}

/// Sends bytes on a connected socket.
pub fn send(fd: impl FdArg, buf: &[u8], flags: crate::ffi::c_int) -> Result<usize> {
    cvt_ssize(unsafe { lunacy_send(fd.raw_fd(), buf.as_ptr().cast(), buf.len(), flags) })
}

/// Receives bytes from a connected socket.
pub fn recv(fd: impl FdArg, buf: &mut [u8], flags: crate::ffi::c_int) -> Result<usize> {
    cvt_ssize(unsafe { lunacy_recv(fd.raw_fd(), buf.as_mut_ptr().cast(), buf.len(), flags) })
}

/// Sends bytes to an address.
pub fn sendto(
    fd: impl FdArg,
    buf: &[u8],
    flags: crate::ffi::c_int,
    addr: SocketAddr,
) -> Result<usize> {
    let addr = addr.to_repr();
    cvt_ssize(unsafe {
        lunacy_sendto_addr(fd.raw_fd(), buf.as_ptr().cast(), buf.len(), flags, &addr)
    })
}

/// Receives bytes and returns the sender address.
pub fn recvfrom(
    fd: impl FdArg,
    buf: &mut [u8],
    flags: crate::ffi::c_int,
) -> Result<(usize, SocketAddr)> {
    let mut addr = MaybeUninit::<SocketAddrRepr>::uninit();
    let received = cvt_ssize(unsafe {
        lunacy_recvfrom_addr(
            fd.raw_fd(),
            buf.as_mut_ptr().cast(),
            buf.len(),
            flags,
            addr.as_mut_ptr(),
        )
    })?;
    let addr = SocketAddr::from_repr(unsafe { addr.assume_init() });

    Ok((received, addr))
}

fn net_constant(name: crate::ffi::c_int) -> Result<crate::ffi::c_int> {
    let mut value = 0;
    cvt_int(unsafe { lunacy_net_constant(name, &mut value) }).map(|_| value)
}

fn cvt_fd(value: RawFd) -> Result<OwnedFd> {
    if value < 0 {
        Err(Errno::last())
    } else {
        Ok(unsafe { OwnedFd::from_raw_fd(value) })
    }
}

fn cvt_int(value: crate::ffi::c_int) -> Result<crate::ffi::c_int> {
    if value < 0 {
        Err(Errno::last())
    } else {
        Ok(value)
    }
}

fn cvt_ssize(value: crate::ffi::ssize_t) -> Result<usize> {
    if value < 0 {
        Err(Errno::last())
    } else {
        Ok(value as usize)
    }
}

unsafe extern "C" {
    fn lunacy_net_constant(
        name: crate::ffi::c_int,
        out: *mut crate::ffi::c_int,
    ) -> crate::ffi::c_int;
    fn lunacy_socket_create(
        domain: crate::ffi::c_int,
        socket_type: crate::ffi::c_int,
        protocol: crate::ffi::c_int,
    ) -> crate::ffi::c_int;
    fn lunacy_connect_addr(fd: RawFd, addr: *const SocketAddrRepr) -> crate::ffi::c_int;
    fn lunacy_bind_addr(fd: RawFd, addr: *const SocketAddrRepr) -> crate::ffi::c_int;
    fn lunacy_listen(fd: RawFd, backlog: crate::ffi::c_int) -> crate::ffi::c_int;
    fn lunacy_accept_fd(fd: RawFd) -> RawFd;
    fn lunacy_accept_addr(fd: RawFd, out: *mut SocketAddrRepr) -> RawFd;
    fn lunacy_shutdown(fd: RawFd, how: crate::ffi::c_int) -> crate::ffi::c_int;
    fn lunacy_send(
        fd: RawFd,
        buf: *const crate::ffi::c_void,
        len: crate::ffi::size_t,
        flags: crate::ffi::c_int,
    ) -> crate::ffi::ssize_t;
    fn lunacy_recv(
        fd: RawFd,
        buf: *mut crate::ffi::c_void,
        len: crate::ffi::size_t,
        flags: crate::ffi::c_int,
    ) -> crate::ffi::ssize_t;
    fn lunacy_sendto_addr(
        fd: RawFd,
        buf: *const crate::ffi::c_void,
        len: crate::ffi::size_t,
        flags: crate::ffi::c_int,
        addr: *const SocketAddrRepr,
    ) -> crate::ffi::ssize_t;
    fn lunacy_recvfrom_addr(
        fd: RawFd,
        buf: *mut crate::ffi::c_void,
        len: crate::ffi::size_t,
        flags: crate::ffi::c_int,
        out: *mut SocketAddrRepr,
    ) -> crate::ffi::ssize_t;
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream, UdpSocket},
        os::fd::FromRawFd,
        thread,
    };

    use super::{
        AddressFamily, Protocol, Shutdown, SocketAddr, SocketType, accept_with_addr, bind, connect,
        listen, recv, recvfrom, send, sendto, shutdown, socket,
    };
    use crate::fd;

    fn tcp_socket() -> crate::Result<fd::OwnedFd> {
        socket(
            AddressFamily::inet()?,
            SocketType::stream()?,
            Protocol::tcp()?,
        )
    }

    fn udp_socket() -> crate::Result<fd::OwnedFd> {
        socket(
            AddressFamily::inet()?,
            SocketType::datagram()?,
            Protocol::udp()?,
        )
    }

    fn local_tcp_port(fd: &fd::OwnedFd) -> u16 {
        let duplicate = fd::dup(fd).unwrap();
        let listener = unsafe { TcpListener::from_raw_fd(duplicate.into_raw_fd()) };

        listener.local_addr().unwrap().port()
    }

    fn local_udp_port(fd: &fd::OwnedFd) -> u16 {
        let duplicate = fd::dup(fd).unwrap();
        let socket = unsafe { UdpSocket::from_raw_fd(duplicate.into_raw_fd()) };

        socket.local_addr().unwrap().port()
    }

    #[test]
    fn creates_tcp_socket() {
        let socket = tcp_socket().unwrap();

        fd::close(socket).unwrap();
    }

    #[test]
    fn connects_and_exchanges_bytes_with_std_listener() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0; 4];
            stream.read_exact(&mut buf).unwrap();
            assert_eq!(&buf, b"ping");
            stream.write_all(b"pong").unwrap();
        });

        let client = tcp_socket().unwrap();
        connect(&client, SocketAddr::v4([127, 0, 0, 1], port)).unwrap();
        assert_eq!(send(&client, b"ping", 0).unwrap(), 4);

        let mut buf = [0; 4];
        assert_eq!(recv(&client, &mut buf, 0).unwrap(), 4);
        assert_eq!(&buf, b"pong");
        shutdown(&client, Shutdown::both().unwrap()).unwrap();
        fd::close(client).unwrap();
        server.join().unwrap();
    }

    #[test]
    fn binds_listens_accepts_and_exchanges_bytes() {
        let listener = tcp_socket().unwrap();
        bind(&listener, SocketAddr::v4([127, 0, 0, 1], 0)).unwrap();
        listen(&listener, 1).unwrap();

        let port = local_tcp_port(&listener);
        let client = thread::spawn(move || {
            let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
            stream.write_all(b"abc").unwrap();
            let mut buf = [0; 3];
            stream.read_exact(&mut buf).unwrap();
            assert_eq!(&buf, b"xyz");
        });

        let (accepted, peer) = accept_with_addr(&listener).unwrap();
        assert_eq!(peer, SocketAddr::v4([127, 0, 0, 1], peer.port()));

        let mut buf = [0; 3];
        assert_eq!(recv(&accepted, &mut buf, 0).unwrap(), 3);
        assert_eq!(&buf, b"abc");
        assert_eq!(send(&accepted, b"xyz", 0).unwrap(), 3);

        fd::close(accepted).unwrap();
        fd::close(listener).unwrap();
        client.join().unwrap();
    }

    #[test]
    fn sends_and_receives_udp_datagrams() {
        let server = udp_socket().unwrap();
        bind(&server, SocketAddr::v4([127, 0, 0, 1], 0)).unwrap();
        let port = local_udp_port(&server);

        let client = UdpSocket::bind(("127.0.0.1", 0)).unwrap();
        client.send_to(b"hello", ("127.0.0.1", port)).unwrap();

        let mut buf = [0; 8];
        let (received, peer) = recvfrom(&server, &mut buf, 0).unwrap();
        assert_eq!(&buf[..received], b"hello");

        assert_eq!(sendto(&server, b"world", 0, peer).unwrap(), 5);
        let (received, _) = client.recv_from(&mut buf).unwrap();
        assert_eq!(&buf[..received], b"world");

        fd::close(server).unwrap();
    }
}
