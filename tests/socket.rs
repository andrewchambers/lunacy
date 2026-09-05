use lunacy::{
    AddressFamily as Family, BorrowedFd, Errno, MsgFlags, PollEvents, PollFd, Shutdown, SockAddr,
    SocketType, poll,
};
use std::{ffi::CString, fs::File, os::fd::AsRawFd};

fn wait_readable(fd: BorrowedFd<'_>) {
    let mut fds = [PollFd::new(Some(fd), PollEvents::pollin())];
    assert_eq!(poll(&mut fds, 2000), Ok(1));
    assert!(fds[0].revents().contains(PollEvents::pollin()));
}

#[test]
fn socketpair_peek_receive_and_shutdown() {
    let (left, right) = lunacy::socketpair(Family::Unix, SocketType::Stream, 0).unwrap();
    assert_ne!(left.as_raw_fd(), right.as_raw_fd());
    assert_eq!(
        lunacy::send(left.as_fd(), b"hello", MsgFlags::empty()),
        Ok(5)
    );
    wait_readable(right.as_fd());
    let mut buffer = [0; 16];
    assert_eq!(
        lunacy::recv(right.as_fd(), &mut buffer, MsgFlags::peek()),
        Ok(5)
    );
    assert_eq!(&buffer[..5], b"hello");
    assert_eq!(
        lunacy::recv(right.as_fd(), &mut buffer, MsgFlags::empty()),
        Ok(5)
    );
    lunacy::shutdown(left.as_fd(), Shutdown::Write).unwrap();
    wait_readable(right.as_fd());
    assert_eq!(
        lunacy::recv(right.as_fd(), &mut buffer, MsgFlags::empty()),
        Ok(0)
    );
    // Shutting down writing leaves the other direction open.
    assert_eq!(lunacy::send(right.as_fd(), b"x", MsgFlags::empty()), Ok(1));
    wait_readable(left.as_fd());
    assert_eq!(
        lunacy::recv(left.as_fd(), &mut buffer, MsgFlags::empty()),
        Ok(1)
    );
}

#[test]
fn ipv4_listen_connect_accept_and_addresses() {
    let listener = lunacy::socket(Family::Inet, SocketType::Stream, 0).unwrap();
    lunacy::bind(listener.as_fd(), &SockAddr::ipv4([127, 0, 0, 1], 0)).unwrap();
    lunacy::listen(listener.as_fd(), 4).unwrap();
    let address = lunacy::getsockname(listener.as_fd()).unwrap();
    assert_eq!(address.family(), Family::Inet.to_raw());
    let (ip, port) = address.as_ipv4().unwrap();
    assert_eq!(ip, [127, 0, 0, 1]);
    assert_ne!(port, 0);

    let client = lunacy::socket(Family::Inet, SocketType::Stream, 0).unwrap();
    lunacy::connect(client.as_fd(), &address).unwrap();
    wait_readable(listener.as_fd());
    let mut peer = SockAddr::new();
    let server = lunacy::accept(listener.as_fd(), Some(&mut peer)).unwrap();
    assert_eq!(
        peer.as_ipv4(),
        lunacy::getsockname(client.as_fd()).unwrap().as_ipv4()
    );
    assert_eq!(
        lunacy::getpeername(client.as_fd()).unwrap().as_ipv4(),
        Some((ip, port))
    );
    assert_eq!(
        lunacy::send(client.as_fd(), b"tcp", MsgFlags::empty()),
        Ok(3)
    );
    wait_readable(server.as_fd());
    let mut buffer = [0; 3];
    assert_eq!(
        lunacy::recv(server.as_fd(), &mut buffer, MsgFlags::empty()),
        Ok(3)
    );
    assert_eq!(&buffer, b"tcp");
}

#[test]
fn address_fields_round_trip_without_native_layout_assumptions() {
    let v4 = SockAddr::ipv4([192, 0, 2, 19], 0x1234);
    assert_eq!(v4.as_ipv4(), Some(([192, 0, 2, 19], 0x1234)));
    assert_eq!(v4.as_ipv6(), None);
    let ip = [0x20, 1, 0x0d, 0xb8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let v6 = SockAddr::ipv6(ip, 0x5678, 0x12345, 23);
    assert_eq!(v6.family(), Family::Inet6.to_raw());
    assert_eq!(v6.as_ipv6(), Some((ip, 0x5678, 0x12345, 23)));
    assert_eq!(v6.as_ipv4(), None);
    assert_eq!(SockAddr::new().as_ipv4(), None);
    assert_eq!(SockAddr::unix(c"").err(), Some(Errno::EINVAL));
    let long = CString::new(vec![b'x'; 256]).unwrap();
    assert_eq!(SockAddr::unix(&long).err(), Some(Errno::EINVAL));
}

#[test]
fn pathname_unix_bind_connect_and_accept_without_address() {
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let path = std::env::temp_dir().join(format!("lunacy-socket-{}.sock", std::process::id()));
    let _cleanup = Cleanup(path.clone());
    let name = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    let address = SockAddr::unix(&name).unwrap();
    assert_eq!(address.family(), Family::Unix.to_raw());
    let listener = lunacy::socket(Family::Unix, SocketType::Stream, 0).unwrap();
    lunacy::bind(listener.as_fd(), &address).unwrap();
    lunacy::listen(listener.as_fd(), 1).unwrap();
    let client = lunacy::socket(Family::Unix, SocketType::Stream, 0).unwrap();
    lunacy::connect(client.as_fd(), &address).unwrap();
    wait_readable(listener.as_fd());
    let server = lunacy::accept(listener.as_fd(), None).unwrap();
    assert_eq!(
        lunacy::getsockname(server.as_fd()).unwrap().family(),
        Family::Unix.to_raw()
    );
}

#[test]
fn datagram_socketpair_preserves_message_boundaries() {
    let (left, right) = lunacy::socketpair(Family::Unix, SocketType::Datagram, 0).unwrap();
    assert_eq!(lunacy::send(left.as_fd(), b"abc", MsgFlags::empty()), Ok(3));
    assert_eq!(
        lunacy::send(left.as_fd(), b"defg", MsgFlags::empty()),
        Ok(4)
    );
    let mut buffer = [0; 16];
    wait_readable(right.as_fd());
    assert_eq!(
        lunacy::recv(right.as_fd(), &mut buffer, MsgFlags::empty()),
        Ok(3)
    );
    assert_eq!(&buffer[..3], b"abc");
    wait_readable(right.as_fd());
    assert_eq!(
        lunacy::recv(right.as_fd(), &mut buffer, MsgFlags::empty()),
        Ok(4)
    );
    assert_eq!(&buffer[..4], b"defg");
}

#[test]
fn non_socket_reports_enotsock() {
    let file = File::open("/dev/null").unwrap();
    // SAFETY: file keeps the descriptor live throughout these calls.
    let fd = unsafe { BorrowedFd::borrow_raw(file.as_raw_fd()) };
    assert_eq!(lunacy::listen(fd, 1), Err(Errno::ENOTSOCK));
    assert_eq!(
        lunacy::send(fd, b"x", MsgFlags::empty()),
        Err(Errno::ENOTSOCK)
    );
    assert_eq!(lunacy::accept(fd, None).err(), Some(Errno::ENOTSOCK));
}
