use lunacy::{
    Errno,
    poll::{PollEvents, PollFd, poll},
    select::{FdSet, TimeVal, select},
    socket::{self, AddressFamily, MsgFlags, SocketType},
};

fn pair() -> (lunacy::fd::OwnedFd, lunacy::fd::OwnedFd) {
    socket::socketpair(AddressFamily::Unix, SocketType::Stream, 0).unwrap()
}

#[test]
fn poll_timeout_readiness_and_ignored_entries() {
    let (left, right) = pair();
    let mut fds = [
        PollFd::new(Some(right.as_fd()), PollEvents::pollin()),
        PollFd::new(None, PollEvents::pollin() | PollEvents::pollout()),
    ];
    assert_eq!(poll(&mut fds, 0), Ok(0));
    assert_eq!(fds[0].revents(), PollEvents::empty());
    assert_eq!(socket::send(left.as_fd(), b"x", MsgFlags::empty()), Ok(1));
    // The descriptor is already readable, so an infinite timeout also returns.
    assert_eq!(poll(&mut fds, -1), Ok(1));
    assert!(fds[0].revents().contains(PollEvents::pollin()));
    assert_eq!(fds[1].revents(), PollEvents::empty());
    assert_eq!(fds[1].as_raw_fd(), -1);
    fds[0].set_events(PollEvents::empty());
    assert_eq!(poll(&mut fds, 0), Ok(0));
    assert_eq!(fds[0].revents(), PollEvents::empty());
    assert_eq!(poll(&mut [], 1), Ok(0));
}

#[test]
fn poll_counts_entries_not_bits_and_reports_hangup() {
    let (left, right) = pair();
    let mut fds = [PollFd::new(
        Some(right.as_fd()),
        PollEvents::pollin() | PollEvents::pollout(),
    )];
    assert_eq!(socket::send(left.as_fd(), b"x", MsgFlags::empty()), Ok(1));
    assert_eq!(poll(&mut fds, 2000), Ok(1));
    assert!(
        fds[0]
            .revents()
            .contains(PollEvents::pollin() | PollEvents::pollout())
    );
    drop(left);
    fds[0].set_events(PollEvents::empty());
    assert_eq!(poll(&mut fds, 2000), Ok(1));
    assert!(fds[0].revents().contains(PollEvents::pollhup()));
}

#[test]
fn select_updates_sets_and_counts_bits_across_sets() {
    let (left, right) = pair();
    let raw = right.as_raw_fd();
    let mut read = FdSet::new();
    read.insert(right.as_fd()).unwrap();
    read.insert(right.as_fd()).unwrap(); // FD_SET is idempotent.
    assert!(read.contains(raw));
    let mut timeout = TimeVal::default();
    assert_eq!(
        select(raw + 1, Some(&mut read), None, None, Some(&mut timeout)),
        Ok(0)
    );
    assert!(!read.contains(raw));

    socket::send(left.as_fd(), b"x", MsgFlags::empty()).unwrap();
    read.insert(right.as_fd()).unwrap();
    let mut write = read.clone();
    let mut except = read.clone();
    timeout = TimeVal {
        tv_sec: 2,
        tv_usec: 0,
    };
    assert_eq!(
        select(
            raw + 1,
            Some(&mut read),
            Some(&mut write),
            Some(&mut except),
            Some(&mut timeout)
        ),
        Ok(2)
    );
    assert!(read.contains(raw));
    assert!(write.contains(raw));
    assert!(!except.contains(raw));
    assert!(timeout.tv_sec >= 0 && timeout.tv_sec <= 2);
    read.remove(raw).unwrap();
    assert!(!read.contains(raw));
    write.clear();
    assert!(!write.contains(raw));
}

#[test]
fn select_null_timeout_on_ready_descriptor() {
    let (left, right) = pair();
    let mut read = FdSet::new();
    read.insert(right.as_fd()).unwrap();
    socket::send(left.as_fd(), b"x", MsgFlags::empty()).unwrap();
    assert_eq!(
        select(right.as_raw_fd() + 1, Some(&mut read), None, None, None),
        Ok(1)
    );
}

#[test]
fn select_checks_indices_and_timeval_before_native_access() {
    let mut set = FdSet::new();
    assert!(!set.contains(-1));
    assert!(!set.contains(FdSet::capacity()));
    assert_eq!(set.remove(-1), Err(Errno::EINVAL));
    assert_eq!(set.remove(FdSet::capacity()), Err(Errno::EINVAL));
    assert_eq!(select(-1, None, None, None, None), Err(Errno::EINVAL));
    assert_eq!(
        select(FdSet::capacity() + 1, Some(&mut set), None, None, None),
        Err(Errno::EINVAL)
    );
    for mut timeout in [
        TimeVal {
            tv_sec: -1,
            tv_usec: 0,
        },
        TimeVal {
            tv_sec: 0,
            tv_usec: -1,
        },
        TimeVal {
            tv_sec: 0,
            tv_usec: 1_000_000,
        },
    ] {
        let before = timeout;
        assert_eq!(
            select(0, None, None, None, Some(&mut timeout)),
            Err(Errno::EINVAL)
        );
        assert_eq!(timeout, before);
    }
    assert_eq!(
        select(0, None, None, None, Some(&mut TimeVal::default())),
        Ok(0)
    );
}
