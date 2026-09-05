use std::{
    fs::File,
    io::Read,
    os::fd::{AsRawFd, FromRawFd, IntoRawFd},
    os::unix::net::UnixStream,
    time::Duration,
};

use lunacy::{
    Errno,
    fd::{BorrowedFd, OwnedFd, close, dup},
    io::write,
};

fn pair() -> (OwnedFd, UnixStream) {
    let (writer, reader) = UnixStream::pair().unwrap();
    reader
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    // SAFETY: into_raw_fd transfers the only owner of this live descriptor.
    (
        unsafe { OwnedFd::from_raw_fd(writer.into_raw_fd()) },
        reader,
    )
}

#[test]
fn write_and_drop_close_the_owned_descriptor() {
    let (writer, mut reader) = pair();
    assert_eq!(write(writer.as_fd(), b"hello"), Ok(5));
    drop(writer);
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"hello");
}

#[test]
fn duplicate_survives_explicit_close_of_original() {
    let (writer, mut reader) = pair();
    let duplicate = dup(writer.as_fd()).unwrap();
    assert_ne!(duplicate.as_raw_fd(), writer.as_raw_fd());
    close(writer).unwrap();
    assert_eq!(write(duplicate.as_fd(), b"dup"), Ok(3));
    close(duplicate).unwrap();
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"dup");
}

#[test]
fn raw_transfer_keeps_descriptor_open() {
    let (writer, mut reader) = pair();
    let raw = writer.into_raw_fd();
    // SAFETY: Transfer the live descriptor to std without an intervening close.
    let writer = unsafe { UnixStream::from_raw_fd(raw) };
    // SAFETY: writer stays alive for the entire borrow.
    let borrowed = unsafe { BorrowedFd::borrow_raw(writer.as_raw_fd()) };
    assert_eq!(write(borrowed, b"raw"), Ok(3));
    drop(writer);
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"raw");
}

#[test]
fn write_reports_native_errno_on_read_only_descriptor() {
    let file = File::open("/dev/null").unwrap();
    // SAFETY: file keeps the descriptor open for the entire borrow.
    let borrowed = unsafe { BorrowedFd::borrow_raw(file.as_raw_fd()) };
    let error = write(borrowed, b"x").unwrap_err();
    assert_eq!(error, Errno::EBADF);
    assert_eq!(
        Some(error.to_raw()),
        std::io::Error::last_os_error().raw_os_error()
    );
}

#[test]
fn nonblocking_write_returns_short_count_and_would_block() {
    let (writer, _reader) = UnixStream::pair().unwrap();
    writer.set_nonblocking(true).unwrap();
    // SAFETY: writer stays open throughout all calls using this borrow.
    let borrowed = unsafe { BorrowedFd::borrow_raw(writer.as_raw_fd()) };
    let buffer = vec![0x5a; 4 * 1024 * 1024];
    let mut short = false;
    for _ in 0..64 {
        match write(borrowed, &buffer) {
            Ok(n) => {
                assert!(n > 0);
                short |= n < buffer.len();
            }
            Err(Errno::EAGAIN) => {
                assert!(short, "expected a partial write before the socket filled");
                return;
            }
            result => panic!("unexpected write result: {result:?}"),
        }
    }
    panic!("socket did not fill within the test's bounded writes");
}
