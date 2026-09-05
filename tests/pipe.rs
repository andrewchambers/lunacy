use lunacy::{
    Errno,
    fd::{close, dup, dup2, dup2_raw, pipe},
    io::{read, write},
};

#[test]
fn pipe_read_returns_short_counts_and_eof_after_all_writers_close() {
    let (reader, writer) = pipe().unwrap();
    let duplicate = dup(writer.as_fd()).unwrap();
    close(writer).unwrap();
    assert_eq!(write(duplicate.as_fd(), b"hello"), Ok(5));
    close(duplicate).unwrap();

    let mut buffer = [0xa5; 8];
    assert_eq!(read(reader.as_fd(), &mut buffer[..3]), Ok(3));
    assert_eq!(&buffer[..3], b"hel");
    assert_eq!(&buffer[3..], &[0xa5; 5]);
    assert_eq!(read(reader.as_fd(), &mut buffer), Ok(2));
    assert_eq!(&buffer[..2], b"lo");
    assert_eq!(read(reader.as_fd(), &mut buffer), Ok(0));
}

#[test]
fn pipe_ends_preserve_native_access_modes() {
    let (reader, writer) = pipe().unwrap();
    let mut buffer = [0; 1];
    assert_eq!(read(writer.as_fd(), &mut buffer), Err(Errno::EBADF));
    assert_eq!(write(reader.as_fd(), b"x"), Err(Errno::EBADF));
    assert_eq!(read(reader.as_fd(), &mut []), Ok(0));
}

#[test]
fn dup2_replaces_target_without_changing_its_number() {
    let (reader, source) = pipe().unwrap();
    let (old_reader, mut target) = pipe().unwrap();
    let target_number = target.as_raw_fd();
    dup2(source.as_fd(), &mut target).unwrap();
    assert_eq!(target.as_raw_fd(), target_number);

    let mut buffer = [0; 16];
    // dup2 atomically closed the old pipe writer, so its reader reaches EOF.
    assert_eq!(read(old_reader.as_fd(), &mut buffer), Ok(0));
    close(source).unwrap();
    assert_eq!(write(target.as_fd(), b"replacement"), Ok(11));
    close(target).unwrap();
    assert_eq!(read(reader.as_fd(), &mut buffer), Ok(11));
    assert_eq!(&buffer[..11], b"replacement");
    assert_eq!(read(reader.as_fd(), &mut buffer), Ok(0));
}

#[test]
fn raw_dup2_preserves_same_fd_semantics_and_reports_invalid_target() {
    let (reader, writer) = pipe().unwrap();
    let raw = writer.as_raw_fd();
    // SAFETY: Same-fd dup2 does not replace the resource or introduce an owner.
    assert_eq!(unsafe { dup2_raw(writer.as_fd(), raw) }, Ok(raw));
    // SAFETY: -1 cannot name a live target; no owner's invariant can be affected.
    assert_eq!(unsafe { dup2_raw(writer.as_fd(), -1) }, Err(Errno::EBADF));
    assert_eq!(write(writer.as_fd(), b"still open"), Ok(10));
    close(writer).unwrap();
    let mut buffer = [0; 16];
    assert_eq!(read(reader.as_fd(), &mut buffer), Ok(10));
    assert_eq!(&buffer[..10], b"still open");
    assert_eq!(read(reader.as_fd(), &mut buffer), Ok(0));
}
